use anyhow::Result;
use async_trait::async_trait;
use bookie::BookSearch;
use books_common::MetadataItemCached;
use chrono::Utc;

use crate::{database::table::{File, MetadataItem}, ThumbnailType};

use super::{Metadata, MetadataReturned, AuthorInfo};



pub struct LocalMetadata;

#[async_trait]
impl Metadata for LocalMetadata {
	fn get_prefix(&self) ->  &'static str {
		"local"
	}

	async fn get_metadata_from_files(&mut self, files: &[File]) -> Result<Option<MetadataReturned>> {
		for file in files {
			// Wrapped to prevent "future cannot be sent between threads safely"
			let (mut meta, opt_thumb_url, authors, publisher) = {
				let mut book = match bookie::load_from_path(&file.path)? {
					Some(v) => v,
					None => continue,
				};

				let source = self.prefix_text(book.get_unique_id()?);
				let now = Utc::now();

				let title = book.find(BookSearch::Title).map(|mut v| v.remove(0));
				let opt_thumb_url = book.find(BookSearch::CoverImage)
					.map(|mut v| v.remove(0))
					.map(|url| book.read_path_as_bytes(&url, None, None));

				let publisher = book.find(BookSearch::Publisher).map(|mut v| v.remove(0));
				let authors = book.find(BookSearch::Creator)
					.map(|items| items.into_iter()
						.map(|name| AuthorInfo {
							source: source.as_str().try_into().unwrap(),
							name,
							other_names: None,
							description: None,
							cover_image_url: None,
							birth_date: None,
							death_date: None,
						})
						.collect::<Vec<_>>()
					);

				(MetadataItem {
					id: 0,
					source: source.try_into()?,
					library_id: 0,
					file_item_count: 1,
					title: title.clone(),
					original_title: title,
					description: book.find(BookSearch::Description).map(|mut v| v.remove(0)),
					rating: 0.0,
					thumb_path: Default::default(),
					cached: MetadataItemCached::default(),
					refreshed_at: now,
					created_at: now,
					updated_at: now,
					deleted_at: None,
					available_at: None,
					year: None,
					hash: String::new() // TODO: Should not be a file hash. Multiple files can use the same Metadata.
				}, opt_thumb_url, authors, publisher)
			};

			meta.thumb_path = match opt_thumb_url {
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
			}.into();

			return Ok(Some(MetadataReturned {
				authors,
				publisher,
				meta,
			}));
		}

		Ok(None)
	}
}