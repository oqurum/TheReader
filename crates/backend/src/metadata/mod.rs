use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;

use crate::database::{table::{MetadataItem, File}, Database};

use self::{
	google_books::GoogleBooksMetadata,
	local::LocalMetadata,
	openlibrary::OpenLibraryMetadata
};

pub mod audible;
pub mod commonsensemedia;
pub mod google_books;
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

	async fn search(&mut self, _search: &str) -> Result<Vec<SearchItem>> {
		Ok(Vec::new())
	}
}

// TODO: Utilize current metadata in try_parse.
// TODO: Order which metadata should be tried.
pub async fn get_metadata(file: &File, _meta: Option<&MetadataItem>, db: &Database) -> Result<Option<MetadataItem>> {
	return_if_found!(OpenLibraryMetadata.try_parse(file, db).await);
	return_if_found!(GoogleBooksMetadata.try_parse(file, db).await);

	// TODO: Don't re-scan file if we already have metadata from file.
	LocalMetadata.try_parse(file, db).await
}


pub async fn search_all_agents(search: &str) -> Result<HashMap<&'static str, Vec<SearchItem>>> {
	let mut map = HashMap::new();

	map.insert(
		OpenLibraryMetadata.get_prefix(),
		OpenLibraryMetadata.search(search).await?,
	);

	map.insert(
		GoogleBooksMetadata.get_prefix(),
		GoogleBooksMetadata.search(search).await?,
	);

	Ok(map)
}


pub struct SearchItem {
	//
}