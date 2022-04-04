use std::collections::HashMap;

use anyhow::Result;
use serde::{Serialize, Deserialize};

use super::{KeyItem, TypeValueItem, RecordDescription};


pub async fn get_book_by_id(id: &BookId) -> Result<Option<BookInfo>> {
	let resp = reqwest::get(id.get_json_url()).await?;

	if resp.status().is_success() {
		Ok(Some(resp.json().await?))
	} else {
		Ok(None)
	}
}


pub async fn search_for_books(type_of: BookSearchType, query: &str) -> Result<Option<BookSearchContainer>> {
	let url = type_of.get_api_url(query);

	println!("[METADATA][OPEN LIBRARY]: Search URL: {}", url);

	let resp = reqwest::get(url).await?;

	if resp.status().is_success() {
		Ok(Some(resp.json().await?))
	} else {
		Ok(None)
	}
}



/// https://openlibrary.org/dev/docs/api/books
#[derive(Debug)]
pub enum BookId {
	// OpenLibrary Id
	Work(String),
	Edition(String),

	// Standard
	Isbn(String),
}

impl BookId {
	pub fn key(&self) -> &str {
		match self {
			Self::Work(_) => "works",
			Self::Edition(_) => "books",
			Self::Isbn(_) => "isbn",
		}
	}

	pub fn value(&self) -> &str {
		match self {
			Self::Work(v) => v.as_str(),
			Self::Edition(v) => v.as_str(),
			Self::Isbn(v) => v.as_str(),
		}
	}

	pub fn get_json_url(&self) -> String {
		format!("https://openlibrary.org/{}/{}.json", self.key(), self.value())
	}

	pub fn get_rdf_url(&self) -> String {
		format!("https://openlibrary.org/{}/{}.rdf", self.key(), self.value())
	}

	/// Tries to convert string into one of these values to the best of its' ability.
	pub fn make_assumptions(value: String) -> Option<Self> {
		match value {
			v if v.starts_with("OL") && v.ends_with('W') => Some(Self::Work(v)),
			v if v.starts_with("OL") && v.ends_with('M') => Some(Self::Edition(v)),
			_ => bookie::parse_book_id(&value).into_possible_isbn_value().map(Self::Isbn)
		}
	}
}


pub enum BookSearchType {
	Query,
	Title,
	Author,
}

impl BookSearchType {
	pub fn get_api_url(&self, value: &str) -> String {
		format!("http://openlibrary.org/search.json?{}={}", self.key(), urlencoding::encode(value))
	}

	pub fn key(&self) -> &str {
		match self {
			Self::Query => "q",
			Self::Title => "title",
			Self::Author => "author",
		}
	}
}


#[derive(Debug, Serialize, Deserialize)]
pub struct BookSearchContainer {
	#[serde(rename = "numFound")]
	pub num_found: i64,
	pub start: i64,
	#[serde(rename = "numFoundExact")]
	pub num_found_exact: bool,
	#[serde(rename = "docs")]
	pub items: Vec<BookSearchItem>,
}


#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct BookSearchItem {
	pub key: String,
	#[serde(rename = "type")]
	pub type_of: String,

	pub seed: Option<Vec<String>>,
	pub title: Option<String>,
	pub title_suggest: Option<String>,
	pub has_fulltext: Option<bool>,
	pub edition_count: Option<i64>,
	pub edition_key: Option<Vec<String>>,
	pub publish_date: Option<Vec<String>>,
	pub publish_year: Option<Vec<i64>>,
	pub first_publish_year: Option<i64>,
	pub number_of_pages_median: Option<i64>,
	pub lccn: Option<Vec<String>>,
	pub publish_place: Option<Vec<String>>,
	pub oclc: Option<Vec<String>>,
	pub contributor: Option<Vec<String>>,
	pub lcc: Option<Vec<String>>,
	pub ddc: Option<Vec<String>>,
	pub isbn: Option<Vec<String>>,
	pub last_modified_i: Option<i64>,
	pub ia: Option<Vec<String>>,
	pub ebook_count_i: Option<i64>,
	pub public_scan_b: Option<bool>,
	pub lending_edition_s: Option<String>,
	pub lending_identifier_s: Option<String>,
	pub printdisabled_s: Option<String>,
	pub cover_edition_key: Option<String>,
	pub cover_i: Option<i64>,
	pub first_sentence: Option<Vec<String>>,
	pub publisher: Option<Vec<String>>,
	pub language: Option<Vec<String>>,
	pub author_key: Option<Vec<String>>,
	pub author_name: Option<Vec<String>>,
	pub author_alternative_name: Option<Vec<String>>,
	pub person: Option<Vec<String>>,
	pub place: Option<Vec<String>>,
	pub subject: Option<Vec<String>>,
	pub time: Option<Vec<String>>,
	// TODO: HashMap all of these.
	pub id_abebooks_de: Option<Vec<String>>,
	pub id_alibris_id: Option<Vec<String>>,
	pub id_amazon: Option<Vec<String>>,
	pub id_amazon_ca_asin: Option<Vec<String>>,
	pub id_amazon_co_uk_asin: Option<Vec<String>>,
	pub id_amazon_de_asin: Option<Vec<String>>,
	pub id_amazon_it_asin: Option<Vec<String>>,
	pub id_bibliothèque_nationale_de_france: Option<Vec<String>>,
	#[serde(rename = "id_bodleian__oxford_university")]
	pub id_bodleian_oxford_university: Option<Vec<String>>,
	pub id_british_library: Option<Vec<String>>,
	pub id_british_national_bibliography: Option<Vec<String>>,
	pub id_canadian_national_library_archive: Option<Vec<String>>,
	pub id_yakaboo: Option<Vec<String>>,
	pub id_depósito_legal: Option<Vec<String>>,
	pub id_goodreads: Option<Vec<String>>,
	pub id_google: Option<Vec<String>>,
	pub id_hathi_trust: Option<Vec<String>>,
	pub id_librarything: Option<Vec<String>>,
	pub id_librivox: Option<Vec<String>>,
	pub id_nla: Option<Vec<String>>,
	pub id_overdrive: Option<Vec<String>>,
	pub id_paperback_swap: Option<Vec<String>>,
	pub id_project_gutenberg: Option<Vec<String>>,
	pub id_scribd: Option<Vec<String>>,
	pub id_standard_ebooks: Option<Vec<String>>,
	pub id_wikidata: Option<Vec<String>>,
	pub ia_loaded_id: Option<Vec<String>>,
	pub ia_box_id: Option<Vec<String>>,
	pub ia_collection_s: Option<String>,
	pub publisher_facet: Option<Vec<String>>,
	pub person_key: Option<Vec<String>>,
	pub place_key: Option<Vec<String>>,
	pub time_facet: Option<Vec<String>>,
	pub subtitle: Option<String>,
	pub person_facet: Option<Vec<String>>,
	pub subject_facet: Option<Vec<String>>,
	#[serde(rename = "_version_")]
	pub version: Option<i64>,
	pub place_facet: Option<Vec<String>>,
	pub lcc_sort: Option<String>,
	pub author_facet: Option<Vec<String>>,
	pub subject_key: Option<Vec<String>>,
	pub ddc_sort: Option<String>,
	pub time_key: Option<Vec<String>>,
}




#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct BookInfo {
	pub publishers: Vec<String>,
	pub number_of_pages: Option<usize>,
	pub series: Option<Vec<String>>,
	pub genres: Option<Vec<String>>,
	pub description: Option<RecordDescription>,
	pub contributors: Option<Vec<Contributor>>,
	pub subtitle: Option<String>,
	pub full_title: Option<String>,
	pub work_titles: Option<Vec<String>>,
	pub covers: Option<Vec<i64>>,
	pub notes: Option<RecordDescription>,
	pub ia_box_id: Option<Vec<String>>,
	pub ia_loaded_id: Option<Vec<String>>,
	pub publish_country: Option<String>,
	pub translation_of: Option<String>,
	pub translated_from: Option<Vec<KeyItem>>,
	pub other_titles: Option<Vec<String>>,
	pub dewey_decimal_class: Option<Vec<String>>,
	pub local_id: Option<Vec<String>>,
	pub physical_format: Option<String>,
	pub key: String,
	pub authors: Option<Vec<KeyItem>>,
	pub publish_places: Option<Vec<String>>,
	pub contributions: Option<Vec<String>>,
	pub subjects: Option<Vec<String>>,
	pub edition_name: Option<String>,
	pub pagination: Option<String>,
	pub classifications: Option<serde_json::Value>, // TODO: Unknown.
	pub source_records: Option<Vec<String>>,
	pub title: String,
	pub identifiers: Option<HashMap<String, Vec<String>>>, // TODO: Enum Key names (amazon, google, librarything, goodreads, etc..)
	pub languages: Option<Vec<KeyItem>>,
	pub publish_date: String,
	pub first_sentence: Option<RecordDescription>,
	pub copyright_date: Option<String>,
	pub works: Vec<KeyItem>,
	pub r#type: KeyItem,
	pub physical_dimensions: Option<String>,
	pub ocaid: Option<String>,
	pub isbn_10: Option<Vec<String>>,
	pub isbn_13: Vec<String>,
	pub lccn: Option<Vec<String>>,
	pub oclc_number: Option<Vec<String>>,
	pub oclc_numbers: Option<Vec<String>>,
	pub lc_classifications: Option<Vec<String>>,
	pub latest_revision: usize,
	pub by_statement: Option<String>,
	pub weight: Option<String>,
	pub revision: usize,
	pub table_of_contents: Option<Vec<TableOfContent>>,
	pub created: TypeValueItem,
	pub last_modified: TypeValueItem,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Contributor {
	role: String,
	name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableOfContent {
	level: i64,
	label: Option<String>,
	title: String,
	pagenum: Option<String>,

	#[serde(rename = "type")]
	type_of: Option<KeyItem>,
}