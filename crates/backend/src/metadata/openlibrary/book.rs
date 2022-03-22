use std::collections::HashMap;

use serde::{Serialize, Deserialize};


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

	pub fn get_rfd_url(&self) -> String {
		format!("https://openlibrary.org/{}/{}.rfd", self.key(), self.value())
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
pub struct Record {
	pub publishers: Vec<String>,
	pub number_of_pages: usize,
	pub description: Option<RecordDescription>,
	pub contributors: Option<Vec<Contributor>>,
	pub series: Option<Vec<String>>,
	pub covers: Vec<usize>,
	pub local_id: Option<Vec<String>>,
	pub physical_format: Option<String>,
	pub key: String,
	pub authors: Option<Vec<KeyItem>>,
	pub publish_places: Option<Vec<String>>,
	pub contributions: Option<Vec<String>>,
	pub subjects: Option<Vec<String>>,
	pub edition_name: Option<String>,
	pub pagination: Option<String>,
	// pub classifications: ,
	pub source_records: Option<Vec<String>>,
	pub title: String,
	pub identifiers: HashMap<String, Vec<String>>, // TODO: Enum Key names (amazon, google, librarything, goodreads, etc..)
	pub languages: Vec<KeyItem>,
	pub publish_date: String,
	pub copyright_date: Option<String>,
	pub works: Vec<KeyItem>,
	pub r#type: KeyItem,
	pub physical_dimensions: Option<String>,
	pub ocaid: String,
	pub isbn_10: Vec<String>,
	pub isbn_13: Vec<String>,
	pub lccn: Option<Vec<String>>,
	pub oclc_numbers: Option<Vec<String>>,
	pub lc_classifications: Option<Vec<String>>,
	pub latest_revision: usize,
	pub revision: usize,
	pub created: TypeValueItem,
	pub last_modified: TypeValueItem
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RecordDescription {
	Text(String),
	SpecificType(TypeValueItem)
}

impl RecordDescription {
	pub fn content(&self) -> &str {
		match self {
			Self::Text(v) => v.as_str(),
			Self::SpecificType(v) => v.value.as_str(),
		}
	}
}



#[derive(Debug, Serialize, Deserialize)]
pub struct Contributor {
	role: String,
	name: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyItem {
	key: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeValueItem {
	r#type: String, // TODO: Handle Types
	value: String
}
