use std::{path::PathBuf, collections::VecDeque, time::UNIX_EPOCH};

use crate::{Result, database::{Database, table}, metadata::{get_metadata_from_files, MetadataReturned}, model::{image::{ImageLinkModel, UploadedImageModel}, library::LibraryModel, directory::DirectoryModel, metadata::MetadataModel, file::{NewFileModel, FileModel}}};
use bookie::BookSearch;
use chrono::{Utc, TimeZone};
use tokio::fs;


pub static WHITELISTED_FILE_TYPES: [&str; 1] = ["epub"];


pub async fn library_scan(library: &LibraryModel, directories: Vec<DirectoryModel>, db: &Database) -> Result<()> {
	let mut folders: VecDeque<PathBuf> = directories.into_iter().map(|v| PathBuf::from(&v.path)).collect::<VecDeque<_>>();

	while let Some(path) = folders.pop_front() {
		let mut dir = fs::read_dir(path).await?;

		while let Some(entry) = dir.next_entry().await? {
			let file_type = entry.file_type().await?;
			let file_name = entry.file_name();
			let path = entry.path();
			let meta = entry.metadata().await?;

			if file_type.is_dir() {
				folders.push_back(path);
			} else if file_type.is_file() {
				let file_name = file_name.into_string().unwrap();
				let (file_name, file_type) = match file_name.rsplit_once('.') {
					Some((v1, v2)) => (v1.to_string(), v2.to_string().to_lowercase()),
					None => (file_name, String::new())
				};

				if WHITELISTED_FILE_TYPES.contains(&file_type.as_str()) {
					let file_size = fs::metadata(&path).await?.len(); // TODO: Remove fs::read

					let (chapter_count, identifier) = match bookie::load_from_path(&path.to_string_lossy().to_string()) {
						Ok(book) => {
							if let Some(book) = book {
								let identifier = if let Some(found) = book.find(BookSearch::Identifier) {
									let parsed = found.into_iter()
										.map(|v| bookie::parse_book_id(&v))
										.collect::<Vec<_>>();

									parsed.iter()
										.find_map(|v| v.as_isbn_13())
										.or_else(|| parsed.iter().find_map(|v| v.as_isbn_10()))
								} else {
									None
								};

								(book.chapter_count() as i64, identifier)
							} else {
								(0, None)
							}
						},

						Err(e) => {
							eprintln!("library_scan: {:?}", e);
							continue;
						}
					};

					let file = NewFileModel {
						path: path.to_str().unwrap().replace('\\', "/"),

						file_name,
						file_type,
						file_size: file_size as i64,

						library_id: library.id,
						metadata_id: None,
						chapter_count,

						identifier,

						modified_at: Utc.timestamp_millis(meta.modified()?.duration_since(UNIX_EPOCH)?.as_millis() as i64),
						accessed_at: Utc.timestamp_millis(meta.accessed()?.duration_since(UNIX_EPOCH)?.as_millis() as i64),
						created_at: Utc.timestamp_millis(meta.created()?.duration_since(UNIX_EPOCH)?.as_millis() as i64),
					};

					if !FileModel::path_exists(&file.path, db)? {
						let file = file.insert(db)?;
						let file_id = file.id;

						// TODO: Run Concurrently.
						if let Err(e) = file_match_or_create_metadata(file, db).await {
							eprintln!("File #{file_id} file_match_or_create_metadata Error: {e}");
						}
					}
				} else {
					log::info!("Skipping File {:?}. Not a whitelisted file type.", path);
				}
			}
		}
	}

	println!("Found {} Files", FileModel::get_file_count(db)?);

	Ok(())
}


async fn file_match_or_create_metadata(file: FileModel, db: &Database) -> Result<()> {
	if file.metadata_id.is_none() {
		let file_id = file.id;

		let meta = get_metadata_from_files(&[file]).await?;

		if let Some(mut ret) = meta {
			let (main_author, author_ids) = ret.add_or_ignore_authors_into_database(db).await?;

			let MetadataReturned { mut meta, publisher, .. } = ret;

			// TODO: This is For Local File Data. Need specify.
			if let Some(item) = meta.thumb_locations.iter_mut().find(|v| v.is_file_data()) {
				item.download(db).await?;
			}

			let mut meta: MetadataModel = meta.into();

			// TODO: Store Publisher inside Database
			meta.cached = meta.cached.publisher_optional(publisher).author_optional(main_author);

			let meta = meta.add_or_increment(db)?;
			FileModel::update_file_metadata_id(file_id, meta.id, db)?;

			if let Some(image) = UploadedImageModel::get_by_path(meta.thumb_path.as_value(), db).await? {
				ImageLinkModel::new_book(image.id, meta.id).insert(db).await?;
			}


			for person_id in author_ids {
				db.add_meta_person(&table::MetadataPerson {
					metadata_id: meta.id,
					person_id,
				})?;
			}
		}
	}

	Ok(())
}