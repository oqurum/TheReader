// https://developers.google.com/books/docs/v1/getting_started

// TODO: Handle errors

use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use bookie::Book;
use books_common::{MetadataItemCached, SearchForBooksBy};
use chrono::Utc;
use serde::{Serialize, Deserialize};

use crate::{database::table::{MetadataItem, File}, ThumbnailType};
use super::{Metadata, SearchItem, MetadataReturned, SearchFor};

pub struct GoogleBooksMetadata;

#[async_trait]
impl Metadata for GoogleBooksMetadata {
	fn get_prefix(&self) ->  & 'static str {
		"googlebooks"
	}

	async fn get_metadata_from_files(&mut self, files: &[File]) -> Result<Option<MetadataReturned>> {
		for file in files {
			// Wrapped b/c "future cannot be send between threads safely"
			let found = {
				let book = bookie::epub::EpubBook::load_from_path(&file.path).unwrap();
				book.find(bookie::BookSearch::Identifier)
			};

			println!("[METADATA][GB]: get_metadata_from_files with ids: {:?}", found);


			if let Some(idents) = found {
				for ident in idents {
					let id = match bookie::parse_book_id(&ident).into_possible_isbn_value() {
						Some(v) => v,
						None => continue
					};

					match self.request_query(id).await {
						Ok(Some(v)) => return Ok(Some(v)),
						a => eprintln!("GoogleBooksMetadata::get_metadata_from_files {:?}", a)
					}
				}
			}
		}

		Ok(None)
	}

	async fn get_metadata_by_source_id(&mut self, value: &str) -> Result<Option<MetadataReturned>> {
		match self.request_singular_id(value).await {
			Ok(Some(v)) => Ok(Some(v)),
			a => {
				eprintln!("GoogleBooksMetadata::get_metadata_by_source_id {:?}", a);

				Ok(None)
			}
		}
	}

	async fn search(&mut self, search: &str, search_for: SearchFor) -> Result<Vec<SearchItem>> {
		match search_for {
			SearchFor::Person => Ok(Vec::new()),

			SearchFor::Book(specifically) => {
				let url = format!("https://www.googleapis.com/books/v1/volumes?q={}", match specifically {
					SearchForBooksBy::AuthorName => BookSearchKeyword::InAuthor.combile_string(search),
					SearchForBooksBy::Contents |
					SearchForBooksBy::Query => urlencoding::encode(search).to_string(),
					SearchForBooksBy::Title => BookSearchKeyword::InTitle.combile_string(search),
				});


				println!("[METADATA][GOOGLE BOOKS]: Search URL: {}", url);

				let resp = reqwest::get(url).await?;

				if resp.status().is_success() {
					let books_cont = resp.json::<BookVolumesContainer>().await?;

					let mut books = Vec::new();

					for item in books_cont.items {
						books.push(SearchItem::Book(MetadataItem {
							id: 0,
							library_id: 0,
							file_item_count: 1,
							source: self.prefix_text(&item.id).try_into()?,
							title: Some(item.volume_info.title.clone()),
							original_title: Some(item.volume_info.title),
							description: item.volume_info.description,
							rating: item.volume_info.average_rating.unwrap_or_default(),
							thumb_path: format!("https://books.google.com/books/publisher/content/images/frontcover/{}?fife=w400-h600", item.id).into(),
							cached: MetadataItemCached::default(),
							refreshed_at: Utc::now(),
							created_at: Utc::now(),
							updated_at: Utc::now(),
							deleted_at: None,
							available_at: None,
							year: None,
							hash: String::new(),
						}));
					}

					Ok(books)
				} else {
					return Ok(Vec::new());
				}
			}
		}
	}
}

impl GoogleBooksMetadata {
	pub async fn request_query(&self, id: String) -> Result<Option<MetadataReturned>> {
		let resp = reqwest::get(format!("https://www.googleapis.com/books/v1/volumes?q={}", BookSearchKeyword::Isbn.combile_string(&id))).await?;

		let book = if resp.status().is_success() {
			let mut books = resp.json::<BookVolumesContainer>().await?;

			if books.total_items == 1 {
				books.items.remove(0)
			} else {
				return Ok(None);
			}
		} else {
			return Ok(None);
		};

		self.compile_book_volume_item(book).await
	}

	pub async fn request_singular_id(&self, id: &str) -> Result<Option<MetadataReturned>> {
		let resp = reqwest::get(format!("https://www.googleapis.com/books/v1/volumes/{}", id)).await?;

		if resp.status().is_success() {
			self.compile_book_volume_item(resp.json().await?).await
		} else {
			Ok(None)
		}
	}


	async fn compile_book_volume_item(&self, value: BookVolumeItem) -> Result<Option<MetadataReturned>> {
		let mut thumb_url = None;

		// Download thumb url and store it.
		let resp = reqwest::get(format!("https://books.google.com/books/publisher/content/images/frontcover/{}?fife=w400-h600", value.id)).await?;

		if resp.status().is_success() {
			let bytes = resp.bytes().await?;

			match crate::store_image(ThumbnailType::Metadata, bytes.to_vec()).await {
				Ok(path) => thumb_url = Some(ThumbnailType::Metadata.prefix_text(&path)),
				Err(e) => {
					eprintln!("[METADATA][GB] store_image: {}", e);
				}
			}
		}

		let now = Utc::now();

		Ok(Some(MetadataReturned {
			authors: None,
			publisher: None,
			meta: MetadataItem {
				id: 0,
				library_id: 0,
				file_item_count: 1,
				source: self.prefix_text(value.id).try_into()?,
				title: Some(value.volume_info.title.clone()),
				original_title: Some(value.volume_info.title),
				description: value.volume_info.description,
				rating: value.volume_info.average_rating.unwrap_or_default(),
				thumb_path: thumb_url.into(),
				cached: MetadataItemCached::default()
					.publisher_optional(value.volume_info.publisher)
					.author_optional(value.volume_info.authors.and_then(|v| v.first().cloned())),
				refreshed_at: now,
				created_at: now,
				updated_at: now,
				deleted_at: None,
				available_at: None,
				year: None,
				hash: String::new(),
			}
		}))
	}
}





// Search

#[derive(Debug, Clone, Copy)]
pub enum BookSearchKeyword {
	InTitle,
	InAuthor,
	InPublisher,
	Subject,
	Isbn,
	Lccn,
	Oclc
}

impl BookSearchKeyword {
	pub fn combile_string(&self, value: &str) -> String {
		format!("{}:{}", self.key(), urlencoding::encode(value))
	}

	pub fn key(&self) -> &str {
		match self {
			Self::InTitle => "intitle",
			Self::InAuthor => "inauthor",
			Self::InPublisher => "inpublisher",
			Self::Subject => "subject",
			Self::Isbn => "isbn",
			Self::Lccn => "lccn",
			Self::Oclc => "oclc",
		}
	}
}



#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct BookVolumesContainer {
	pub kind: String,
	#[serde(rename = "totalItems")]
	pub total_items: i64,
	pub items: Vec<BookVolumeItem>,
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookVolumeItem {
	pub kind: String,
	pub id: String,
	pub etag: String,
	pub self_link: String,
	pub volume_info: BookVolumeVolumeInfo,
	// pub sale_info: BookVolumeSaleInfo,
	pub access_info: BookVolumeAccessInfo,
	pub search_info: Option<BookVolumeSearchInfo>,
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BookVolumeVolumeInfo {
	pub title: String,
	pub subtitle: Option<String>,
	pub authors: Option<Vec<String>>,
	pub average_rating: Option<f64>,
	pub ratings_count: Option<i64>,
	pub dimensions: Option<HashMap<String, String>>,
	pub publisher: Option<String>,
	pub published_date: Option<String>,
	pub description: Option<String>,
	pub industry_identifiers: Option<Vec<BookVolumeVolumeInfoIndustryIdentifiers>>,
	pub reading_modes: BookVolumeVolumeInfoReadingModes,
	pub page_count: Option<i64>,
	pub printed_page_count: Option<i64>,
	pub series_info: Option<serde_json::Value>,
	pub print_type: String,
	pub categories: Option<Vec<String>>,
	pub maturity_rating: String,
	pub allow_anon_logging: bool,
	pub comics_content: Option<bool>,
	pub content_version: String,
	pub panelization_summary: Option<BookVolumeVolumeInfoPanelizationSummary>,
	pub image_links: Option<BookVolumeVolumeInfoImageLinks>,
	pub language: String,
	pub preview_link: String,
	pub info_link: String,
	pub canonical_volume_link: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct BookVolumeVolumeInfoIndustryIdentifiers {
	#[serde(rename = "type")]
	pub type_of: String,
	pub identifier: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct BookVolumeVolumeInfoReadingModes {
	pub text: bool,
	pub image: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BookVolumeVolumeInfoPanelizationSummary {
	pub contains_epub_bubbles: bool,
	pub epub_bubble_version: Option<String>,
	pub contains_image_bubbles: bool,
	pub image_bubble_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BookVolumeVolumeInfoImageLinks {
	pub small_thumbnail: String,
	pub thumbnail: String,
	pub small: Option<String>,
	pub medium: Option<String>,
	pub large: Option<String>,
	pub extra_large: Option<String>,
}

// TODO: function to return largest size available. Otherwise use current way


#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BookVolumeSaleInfo {
	pub country: String,
	pub saleability: String,
	pub is_ebook: bool,
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BookVolumeAccessInfo {
	pub country: String,
	pub viewability: String,
	pub embeddable: bool,
	pub public_domain: bool,
	pub text_to_speech_permission: String,
	pub epub: BookVolumeAccessInfoEpub,
	pub pdf: BookVolumeAccessInfoPdf,
	pub web_reader_link: String,
	pub access_view_status: String,
	pub quote_sharing_allowed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookVolumeAccessInfoEpub {
	is_available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookVolumeAccessInfoPdf {
	is_available: bool,
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BookVolumeSearchInfo {
	text_snippet: String,
}