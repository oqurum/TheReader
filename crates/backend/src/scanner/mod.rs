use std::{path::PathBuf, collections::VecDeque};

use anyhow::Result;
use tokio::fs;

use crate::database::{NewFile, Database, Library, Directory};


pub static WHITELISTED_FILE_TYPES: [&str; 1] = ["epub"];


pub async fn library_scan(library: &Library, directories: Vec<Directory>, db: Database) -> Result<()> {
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
					let file_size = fs::read(&path).await?.len(); // TODO: Remove fs::read

					let file = NewFile {
						path: path.to_str().unwrap().replace("\\", "/"),

						file_name,
						file_type,
						file_size: file_size as i64,

						library_id: library.id,
						metadata_id: 0,
						chapter_count: 0,

						modified_at: meta.modified()?.elapsed()?.as_millis() as i64,
						accessed_at: meta.accessed()?.elapsed()?.as_millis() as i64,
						created_at: meta.created()?.elapsed()?.as_millis() as i64,
					};

					db.add_file(&file)?;
				} else {
					log::info!("Skipping File {:?}. Not a whitelisted file type.", path);
				}
			}
		}
	}

	println!("Found {} Files", db.get_file_count()?);

	Ok(())
}
