use std::{collections::VecDeque, path::PathBuf, time::UNIX_EPOCH};

use crate::{
    database::Database,
    metadata::{get_metadata_from_files, MetadataReturned},
    model::{
        book::BookModel,
        book_person::BookPersonModel,
        directory::DirectoryModel,
        file::{FileModel, NewFileModel},
        image::{ImageLinkModel, UploadedImageModel},
        library::LibraryModel,
    },
    Result,
};
use bookie::BookSearch;
use chrono::{TimeZone, Utc};
use common::parse_book_id;
use common_local::LibraryId;
use tokio::fs;

pub static WHITELISTED_FILE_TYPES: [&str; 2] = ["epub", "cbz"];

pub async fn library_scan(
    library: &LibraryModel,
    directories: Vec<DirectoryModel>,
    db: &Database,
) -> Result<()> {
    let mut folders: VecDeque<PathBuf> = directories
        .into_iter()
        .map(|v| PathBuf::from(&v.path))
        .collect::<VecDeque<_>>();

    let (mut checked_items, mut imported_items, mut overwritten_files) = (0, 0, 0);

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
                    None => (file_name, String::new()),
                };

                if WHITELISTED_FILE_TYPES.contains(&file_type.as_str()) {
                    let file_size = fs::metadata(&path).await?.len();

                    checked_items += 1;

                    let mut book = match bookie::load_from_path(&path.to_string_lossy()) {
                        Ok(book) => {
                            if let Some(book) = book {
                                book
                            } else {
                                eprintln!("library_scan: Unable to find book from path ({path:?})");
                                continue;
                            }
                        }

                        Err(e) => {
                            eprintln!("library_scan: {:?} ({path:?})", e);
                            continue;
                        }
                    };

                    let path = path.to_str().unwrap().replace('\\', "/");

                    let hash = match book.compute_hash() {
                        Some(v) => v,
                        None => {
                            eprintln!("library_scan: Unable to compute hash (\"{path}\")");
                            continue;
                        }
                    };

                    let chapter_count = book.chapter_count() as i64;

                    // If file exists, check to see if the one in the database is valid.
                    if let Some(mut model) =
                        FileModel::find_one_by_hash_or_path(&path, &hash, db).await?
                    {
                        // We found it by path, no need to verify anything since it exists.
                        // Update stored model with the new one that matched the hash.
                        // TODO: Optimize? I don't want to check the FS for EVERY SINGLE FILE.
                        if model.path != path && tokio::fs::metadata(&model.path).await.is_err() {
                            model.path = path;
                            model.file_name = file_name;
                            model.file_type = file_type;
                            model.file_size = file_size as i64;
                            model.library_id = library.id;
                            model.chapter_count = chapter_count;
                            model.hash = hash;

                            model.modified_at = Utc.timestamp_millis(
                                meta.modified()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                            );
                            model.accessed_at = Utc.timestamp_millis(
                                meta.accessed()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                            );
                            model.created_at = Utc.timestamp_millis(
                                meta.created()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                            );
                            model.deleted_at = None;

                            println!("Overwriting Missing File ID: {}", model.id);

                            overwritten_files += 1;

                            model.update(db).await?;
                        }

                        continue;
                    }

                    let identifier = if let Some(found) = book.find(BookSearch::Identifier) {
                        let parsed = found
                            .into_iter()
                            .map(|v| parse_book_id(&v))
                            .collect::<Vec<_>>();

                        parsed
                            .iter()
                            .find_map(|v| v.as_isbn_13())
                            .or_else(|| parsed.iter().find_map(|v| v.as_isbn_10()))
                    } else {
                        None
                    };

                    let file = NewFileModel {
                        path,

                        file_name,
                        file_type,
                        file_size: file_size as i64,

                        library_id: library.id,
                        book_id: None,
                        chapter_count,

                        identifier,
                        hash,

                        modified_at: Utc.timestamp_millis(
                            meta.modified()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                        ),
                        accessed_at: Utc.timestamp_millis(
                            meta.accessed()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                        ),
                        created_at: Utc.timestamp_millis(
                            meta.created()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                        ),
                        deleted_at: None,
                    };

                    let file = file.insert(db).await?;
                    let file_id = file.id;

                    imported_items += 1;

                    // TODO: Run Concurrently.
                    if let Err(e) = file_match_or_create_book(file, library.id, db).await {
                        eprintln!("File #{file_id} file_match_or_create_metadata Error: {e}");
                    }
                } else {
                    // log::info!("Skipping File {:?}. Not a whitelisted file type.", path);
                }
            }
        }
    }

    println!("Checked {checked_items} Files, Imported {imported_items} Files, Overwritten {overwritten_files} Files");

    Ok(())
}

async fn file_match_or_create_book(
    file: FileModel,
    library_id: LibraryId,
    db: &Database,
) -> Result<()> {
    if file.book_id.is_none() {
        let file_id = file.id;

        let meta = get_metadata_from_files(&[file], &Default::default()).await?;

        if let Some(mut ret) = meta {
            let (main_author, author_ids) = ret.add_or_ignore_authors_into_database(db).await?;

            let MetadataReturned {
                mut meta,
                publisher,
                ..
            } = ret;

            // TODO: This is For Local File Data. Need specify.
            if let Some(item) = meta.thumb_locations.iter_mut().find(|v| v.is_file_data()) {
                item.download(db).await?;
            }

            let mut book_model: BookModel = meta.into();

            book_model.library_id = library_id;

            // TODO: Store Publisher inside Database
            book_model.cached = book_model
                .cached
                .publisher_optional(publisher)
                .author_optional(main_author);

            let book_model = book_model.insert_or_increment(db).await?;
            FileModel::update_book_id(file_id, book_model.id, db).await?;

            if let Some(thumb_path) = book_model.thumb_path.as_value() {
                if let Some(image) = UploadedImageModel::get_by_path(thumb_path, db).await? {
                    ImageLinkModel::new_book(image.id, book_model.id)
                        .insert(db)
                        .await?;
                }
            }

            for person_id in author_ids {
                BookPersonModel {
                    book_id: book_model.id,
                    person_id,
                }
                .insert_or_ignore(db)
                .await?;
            }
        }
    }

    Ok(())
}
