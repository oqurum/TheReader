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