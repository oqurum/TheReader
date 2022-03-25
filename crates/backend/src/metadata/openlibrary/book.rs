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



/// https://openlibrary.org/dev/docs/api/books
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
			v if v.chars().all(|v| ('0'..='9').contains(&v)) => Some(Self::Isbn(v)),

			_ => None
		}
	}
}



#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
	pub first_sentence: Option<String>,
	pub copyright_date: Option<String>,
	pub works: Vec<KeyItem>,
	pub r#type: KeyItem,
	pub physical_dimensions: Option<String>,
	pub ocaid: Option<String>,
	pub isbn_10: Option<Vec<String>>,
	pub isbn_13: Vec<String>,
	pub lccn: Option<Vec<String>>,
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
	label: String,
	title: String,
	pagenum: String,
}