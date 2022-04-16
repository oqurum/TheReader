use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use books_common::{SearchFor, Source, ThumbnailPath};
use chrono::Utc;

use crate::{database::{table::{MetadataItem, File, self}, Database}, ThumbnailType};

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

	// Metadata
	async fn get_metadata_from_files(&mut self, files: &[File]) -> Result<Option<MetadataReturned>>;

	async fn get_metadata_by_source_id(&mut self, _value: &str) -> Result<Option<MetadataReturned>> {
		Ok(None)
	}

	// Person

	async fn get_person_by_source_id(&mut self, _value: &str) -> Result<Option<AuthorInfo>> {
		Ok(None)
	}


	// Both

	async fn search(&mut self, _search: &str, _search_for: SearchFor) -> Result<Vec<SearchItem>> {
		Ok(Vec::new())
	}
}

// TODO: Utilize current metadata in get_metadata_from_files.
// TODO: Order which metadata should be tried.
/// Attempts to return the first valid Metadata from Files.
pub async fn get_metadata_from_files(files: &[File], _meta: Option<&MetadataItem>, db: &Database) -> Result<Option<MetadataReturned>> {
	return_if_found!(OpenLibraryMetadata.get_metadata_from_files(files).await);
	return_if_found!(GoogleBooksMetadata.get_metadata_from_files(files).await);

	// TODO: Don't re-scan file if we already have metadata from file.
	LocalMetadata.get_metadata_from_files(files).await
}

pub async fn get_metadata_by_source(source: &Source) -> Result<Option<MetadataReturned>> {
	match source.agent.as_str() {
		v if v == OpenLibraryMetadata.get_prefix() => OpenLibraryMetadata.get_metadata_by_source_id(&source.value).await,
		v if v == GoogleBooksMetadata.get_prefix() => GoogleBooksMetadata.get_metadata_by_source_id(&source.value).await,

		_ => Ok(None)
	}
}



pub async fn search_all_agents(search: &str, search_for: SearchFor) -> Result<HashMap<String, Vec<SearchItem>>> {
	let mut map = HashMap::new();

	// Checks to see if we can use get_metadata_by_source (source:id)
	if let Ok(source) = Source::try_from(search) {
		// Check if it's a Metadata Source.
		if let Some(val) = get_metadata_by_source(&source).await? {
			map.insert(
				source.agent,
				vec![SearchItem::Book(val.meta)],
			);

			return Ok(map);
		}
	}

	// Search all sources
	let prefixes = [OpenLibraryMetadata.get_prefix(), GoogleBooksMetadata.get_prefix()];
	let asdf = futures::future::join_all(
		[OpenLibraryMetadata.search(search, search_for), GoogleBooksMetadata.search(search, search_for)]
	).await;

	for (val, prefix) in asdf.into_iter().zip(prefixes) {
		match val {
			Ok(val) => {
				map.insert(
					prefix.to_string(),
					val,
				);
			}

			Err(e) => eprintln!("{:?}", e),
		}
	}

	Ok(map)
}


pub async fn get_person_by_source(source: &Source) -> Result<Option<AuthorInfo>> {
	match source.agent.as_str() {
		v if v == OpenLibraryMetadata.get_prefix() => OpenLibraryMetadata.get_person_by_source_id(&source.value).await,
		v if v == GoogleBooksMetadata.get_prefix() => GoogleBooksMetadata.get_person_by_source_id(&source.value).await,

		_ => Ok(None)
	}
}




#[derive(Debug)]
pub enum SearchItem {
	Author(AuthorInfo),
	Book(MetadataItem)
}



#[derive(Debug)]
pub struct AuthorInfo {
	pub source: Source,

	pub cover_image_url: Option<String>,

	pub name: String,
	pub other_names: Option<Vec<String>>,
	pub description: Option<String>,

	pub birth_date: Option<String>,
	pub death_date: Option<String>,
}


#[derive(Debug)]
pub struct MetadataReturned {
	// Person, Alt Names
	pub authors: Option<Vec<AuthorInfo>>,
	pub publisher: Option<String>,
	// TODO: Add More.

	pub meta: MetadataItem
}

impl MetadataReturned {
	/// Returns (Main Author, Person IDs)
	pub async fn add_or_ignore_authors_into_database(&mut self, db: &Database) -> Result<(Option<String>, Vec<usize>)> {
		let mut main_author = None;
		let mut person_ids = Vec::new();

		if let Some(authors_with_alts) = self.authors.take() {
			for author_info in authors_with_alts {
				// Check if we already have a person by that name anywhere in the two database tables.
				if let Some(person) = db.get_person_by_name(&author_info.name)? {
					person_ids.push(person.id);

					if main_author.is_none() {
						main_author = Some(person.name);
					}

					continue;
				}

				let mut thumb_url = ThumbnailPath::default();

				// Download thumb url and store it.
				if let Some(url) = author_info.cover_image_url {
					let resp = reqwest::get(url).await?;

					if resp.status().is_success() {
						let bytes = resp.bytes().await?;

						match crate::store_image(ThumbnailType::Metadata, bytes.to_vec()).await {
							Ok(path) => thumb_url = path.into(),
							Err(e) => {
								eprintln!("add_or_ignore_authors_into_database Error: {}", e);
							}
						}
					} else {
						let text = resp.text().await;
						eprintln!("add_or_ignore_authors_into_database Error: {:?}", text);
					}
				}

				let author = table::NewTagPerson {
					source: author_info.source,
					name: author_info.name,
					description: author_info.description,
					birth_date: author_info.birth_date,
					thumb_url,
					// TODO: death_date: author_info.death_date,
					updated_at: Utc::now(),
					created_at: Utc::now(),
				};

				let person_id = db.add_person(&author)?;

				if let Some(alts) = author_info.other_names {
					for name in alts {
						// Ignore errors. Errors should just be UNIQUE constraint failed
						if let Err(e) = db.add_person_alt(&table::TagPersonAlt {
							person_id,
							name,
						}) {
							eprintln!("[OL]: Add Alt Name Error: {e}");
						}
					}
				}

				person_ids.push(person_id);

				if main_author.is_none() {
					main_author = Some(author.name);
				}
			}
		}

		Ok((main_author, person_ids))
	}
}