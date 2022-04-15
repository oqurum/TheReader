use std::path::PathBuf;

use anyhow::Result;
use books_common::ThumbnailPath;
use sha2::{Sha256, Digest};

// TODO: Consolidate into common/specific/thumbnail.

#[derive(Debug, Clone, Copy)]
pub enum ThumbnailType {
	Local,
	Uploaded,
	Metadata,
}

impl ThumbnailType {
	pub fn path_name(self) -> &'static str {
		match self {
			Self::Local => "local",
			Self::Uploaded => "upload",
			Self::Metadata => "meta",
		}
	}

	pub fn prefix_text(&self, value: &str) -> String {
		format!("{}:{}", self.path_name(), value)
	}
}

impl From<&str> for ThumbnailType {
	fn from(value: &str) -> Self {
		match value {
			"local" => Self::Local,
			"upload" => Self::Uploaded,
			"meta" => Self::Metadata,
			_ => unreachable!("ThumbnailType::from()"),
		}
	}
}


#[derive(Debug, Clone)]
pub enum ThumbnailLocation {
	Local(String),
	Uploaded(String),
	Metadata(String),
}

impl ThumbnailLocation {
	pub fn as_type(&self) -> ThumbnailType {
		match self {
			Self::Local(_) => ThumbnailType::Local,
			Self::Uploaded(_) => ThumbnailType::Uploaded,
			Self::Metadata(_) => ThumbnailType::Metadata,
		}
	}

	pub fn as_value(&self) -> &str {
		match self {
			Self::Local(v) |
			Self::Uploaded(v) |
			Self::Metadata(v) => v.as_str(),
		}
	}

	pub fn into_value(self) -> String {
		match self {
			Self::Local(v) |
			Self::Uploaded(v) |
			Self::Metadata(v) => v,
		}
	}

	pub fn from_type(type_of: ThumbnailType, value: String) -> Self {
		match type_of {
			ThumbnailType::Local => Self::Local(value),
			ThumbnailType::Uploaded => Self::Uploaded(value),
			ThumbnailType::Metadata => Self::Metadata(value),
		}
	}
}

impl From<ThumbnailPath> for ThumbnailLocation {
	fn from(value: ThumbnailPath) -> Self {
		let (prefix, suffix) = value.get_prefix_suffix().unwrap();

		match ThumbnailType::from(prefix) {
			ThumbnailType::Local => Self::Local(suffix.to_owned()),
			ThumbnailType::Uploaded => Self::Uploaded(suffix.to_owned()),
			ThumbnailType::Metadata => Self::Metadata(suffix.to_owned())
		}
	}
}

impl From<ThumbnailLocation> for ThumbnailPath {
	fn from(val: ThumbnailLocation) -> Self {
		let type_of = val.as_type();
		type_of.prefix_text(val.as_value()).into()
	}
}


pub async fn store_image(type_of: ThumbnailType, image: Vec<u8>) -> Result<ThumbnailLocation> {
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

	tokio::fs::DirBuilder::new().recursive(true).create(&path).await?;

	path.push(format!("{}.jpg", &hash));

	tokio::fs::write(&path, image).await?;

	Ok(ThumbnailLocation::from_type(type_of, hash))
}

pub fn prefixhash_to_path(type_of: ThumbnailType, hash: &str) -> String {
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