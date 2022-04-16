use std::path::PathBuf;

use anyhow::Result;
use books_common::{ThumbnailStoreType, ThumbnailStore};
use sha2::{Sha256, Digest};
use tokio::fs;


pub async fn store_image(type_of: ThumbnailStoreType, image: Vec<u8>) -> Result<ThumbnailStore> {
	// TODO: Resize? Function is currently only used for thumbnails.
	let image = image::load_from_memory(&image)?;

	let mut writer = std::io::Cursor::new(Vec::new());
	image.write_to(&mut writer, image::ImageFormat::Jpeg)?;

	let image = writer.into_inner();

	let hash: String = Sha256::digest(&image)
		.iter()
		.map(|v| format!("{:02x}", v))
		.collect();

	let mut path = PathBuf::new();

	path.push("../../app/thumbnails");
	path.push(type_of.path_name());
	path.push(get_directories(&hash));

	fs::DirBuilder::new().recursive(true).create(&path).await?;

	path.push(format!("{}.jpg", &hash));

	if fs::metadata(&path).await.is_err() {
		fs::write(&path, image).await?;
	}

	Ok(ThumbnailStore::from_type(type_of, hash))
}

pub fn prefixhash_to_path(type_of: ThumbnailStoreType, hash: &str) -> String {
	let mut path = PathBuf::new();

	path.push("../../app/thumbnails");
	path.push(type_of.path_name());
	path.push(get_directories(hash));
	path.push(format!("{}.jpg", &hash));

	path.to_string_lossy().to_string()
}


pub fn get_directories(file_name: &str) -> String {
	format!(
		"{}/{}/{}/{}",
		file_name.get(0..1).unwrap(),
		file_name.get(1..2).unwrap(),
		file_name.get(2..3).unwrap(),
		file_name.get(3..4).unwrap()
	)
}