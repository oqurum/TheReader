use std::{path::PathBuf, collections::VecDeque};

use anyhow::Result;
use tokio::fs;

use crate::database::{NewFile, Database, Library};

pub async fn library_scan(library: &Library, db: Database) -> Result<()> {
	let mut folders: VecDeque<PathBuf> = std::collections::VecDeque::from([PathBuf::from(&library.path)]);

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
				let file_size = fs::read(&path).await?.len(); // TODO: Remove fs::read

				let file_name = file_name.into_string().unwrap();
				let (file_name, file_type) = match file_name.rsplit_once('.') {
					Some((v1, v2)) => (v1.to_string(), v2.to_string()),
					None => (file_name, String::new())
				};

				let file = NewFile {
					path: path.to_str().unwrap().replace("\\", "/"),

					file_name,
					file_type,
					file_size: file_size as i64,

					library_id: 0,
					metadata_id: 0,
					chapter_count: 0,

					modified_at: meta.modified()?.elapsed()?.as_millis() as i64,
					accessed_at: meta.accessed()?.elapsed()?.as_millis() as i64,
					created_at: meta.created()?.elapsed()?.as_millis() as i64,
				};

				db.add_file(&file)?;
			}
		}
	}

	println!("Found {} Files", db.get_file_count()?);

	Ok(())
}
