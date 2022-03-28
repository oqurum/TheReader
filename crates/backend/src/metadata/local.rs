use anyhow::Result;
use async_trait::async_trait;
use bookie::BookSearch;
use books_common::MetadataItemCached;
use chrono::Utc;

use crate::{database::{table::{File, MetadataItem}, Database}, ThumbnailType};

use super::{Metadata, MetadataReturned};



pub struct LocalMetadata;

#[async_trait]
impl Metadata for LocalMetadata {
	fn get_prefix(&self) ->  &'static str {
		"local"
	}

	async fn try_parse(&mut self, file: &File, db: &Database) -> Result<Option<MetadataReturned>> {
		// Wrapped to prevent "future cannot be sent between threads safely"
		let (mut meta, opt_thumb_url) = {
			let mut book = match bookie::load_from_path(&file.path)? {
				Some(v) => v,
				None => return Ok(None)
			};

			let now = Utc::now();

			let title = book.find(BookSearch::Title).map(|mut v| v.remove(0));
			let opt_thumb_url = book.find(BookSearch::CoverImage)
				.map(|mut v| v.remove(0))
				.map(|url| book.read_path_as_bytes(&url));

			let authors = book.find(BookSearch::Creator);

			let main_author = if let Some(person) = authors.as_ref()
				.and_then(|v| v.first())
				.map(|v| db.get_person_by_name(v))
				.transpose()?
				.flatten()
			{
				Some(person.name)
			} else {
				None
			};


			(MetadataItem {
				id: 0,
				source: format!("{}:{}", self.get_prefix(), book.get_unique_id()?),
				file_item_count: 1,
				title: title.clone(),
				original_title: title,
				description: book.find(BookSearch::Description).map(|mut v| v.remove(0)),
				rating: 0.0,
				thumb_url: None,
				cached: MetadataItemCached::default()
					.publisher_optional(book.find(BookSearch::Publisher).map(|mut v| v.remove(0)))
					.author_optional(main_author),
				refreshed_at: now,
				created_at: now,
				updated_at: now,
				deleted_at: None,
				available_at: None,
				year: None,
				hash: String::new() // TODO: Should not be a file hash. Multiple files can use the same Metadata.
			}, opt_thumb_url)
		};

		meta.thumb_url = match opt_thumb_url {
			Some(book_file_path) => {
				let image = book_file_path?;

				match crate::store_image(ThumbnailType::Local, image).await {
					Ok(path) => Some(ThumbnailType::Local.prefix_text(&path)),
					Err(e) => {
						eprintln!("store_image: {}", e);
						None
					}
				}
			}

			None => None
		};

		Ok(Some(MetadataReturned {
			authors: Vec::new(),
			meta,
		}))
	}
}