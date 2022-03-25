use anyhow::Result;
use async_trait::async_trait;
use bookie::BookSearch;
use chrono::Utc;

use crate::database::{table::{File, MetadataItem}, Database};

use super::Metadata;



pub struct LocalMetadata;

#[async_trait]
impl Metadata for LocalMetadata {
	fn get_prefix(&self) ->  &'static str {
		"local"
	}

	async fn try_parse(&mut self, file: &File, _db: &Database) -> Result<Option<MetadataItem>> {
		let book = match bookie::load_from_path(&file.path)? {
			Some(v) => v,
			None => return Ok(None)
		};

		let now = Utc::now();

		let title = book.find(BookSearch::Title).map(|mut v| v.remove(0));

		Ok(Some(MetadataItem {
			id: 0,
			source: format!("{}:{}", self.get_prefix(), book.get_unique_id()?),
			file_item_count: 1,
			title: title.clone(),
			original_title: title,
			description: book.find(BookSearch::Description).map(|mut v| v.remove(0)),
			rating: 0.0,
			thumb_url: book.find(BookSearch::CoverImage).map(|mut v| v.remove(0)),
			creator: book.find(BookSearch::Creator).map(|mut v| v.remove(0)),
			publisher: book.find(BookSearch::Publisher).map(|mut v| v.remove(0)),
			tags_genre: None,
			tags_collection: None,
			tags_author: None,
			tags_country: None,
			refreshed_at: now,
			created_at: now,
			updated_at: now,
			deleted_at: None,
			available_at: None,
			year: None,
			hash: String::new() // TODO: Should not be a file hash. Multiple files can use the same Metadata.
		}))
	}
}