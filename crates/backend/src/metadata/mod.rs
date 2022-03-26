use anyhow::Result;
use async_trait::async_trait;

use crate::database::{table::{MetadataItem, File}, Database};

pub mod audible;
pub mod commonsensemedia;
pub mod goodreads;
pub mod local;
pub mod openlibrary;
pub mod ratedreads;

// "source" column: [prefix]:[id]

/// Simple return if found, println if error.
macro_rules! return_if_found {
	($v: expr) => {
		match $v {
			Ok(Some(v)) => return Ok(Some(v)),
			Ok(None) => (),
			Err(e) => eprintln!("metadata::get_metadata: {}", e)
		}
	};
}


#[async_trait]
pub trait Metadata {
	fn prefix_text<V: AsRef<str>>(&self, value: V) -> String {
		format!("{}:{}", self.get_prefix(), value.as_ref())
	}

	fn get_prefix(&self) -> &'static str;

	async fn try_parse(&mut self, file: &File, db: &Database) -> Result<Option<MetadataItem>>;

	// TODO: Search
}

// TODO: Utilize current metadata in try_parse.
pub async fn get_metadata(file: &File, meta: Option<&MetadataItem>, db: &Database) -> Result<Option<MetadataItem>> {
	return_if_found!(openlibrary::OpenLibraryMetadata.try_parse(file, db).await);

	// TODO: Don't re-scan file if we already have metadata from file.
	local::LocalMetadata.try_parse(file, db).await
}




// TODO: Handle ids better "amazon:{id}", "amazon_uk:{id}", "goodreads:{id}", "isbn:{id}", "google:{id}", "uuid:{id}", "urn:uuid:{id}", "urn:isbn:{id}"
// TODO: Move into another file?
pub fn parse_book_id(value: &str) -> IdType {
	if let Some((prefix, suffix)) = value.rsplit_once(':') {
		let prefix = prefix.to_lowercase().replace(' ', "");
		let suffix = suffix.trim().to_owned();

		match prefix.as_str() {
			"urn:isbn" |
			"isbn" => IdType::Isbn(suffix),

			"urn:uuid" |
			"uuid" => IdType::Uuid(suffix),

			_ => IdType::UnknownKeyValue(prefix, suffix),
		}
	} else {
		IdType::UnknownValue(value.trim().to_string())
	}
}

pub enum IdType {
	Isbn(String),
	Uuid(String),

	UnknownKeyValue(String, String),
	UnknownValue(String)
}

impl IdType {
	pub fn get_possible_isbn_value(&self) -> Option<&str> {
		match self {
			Self::UnknownValue(v) if v.chars().all(|v| ('0'..='9').contains(&v)) => Some(v.as_str()),
			Self::Isbn(v) => Some(v.as_str()),

			_ => None,
		}
	}

	pub fn into_possible_isbn_value(self) -> Option<String> {
		match self {
			Self::UnknownValue(v) if v.chars().all(|v| ('0'..='9').contains(&v)) => Some(v),
			Self::Isbn(v) => Some(v),

			_ => None,
		}
	}
}