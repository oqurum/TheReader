use std::{path::{Path, PathBuf}, borrow::Cow};

pub mod epub;
pub mod mobi;

pub mod error;
pub use error::*;

// TODO: path: &str -> path: &Path

pub trait Book {
	fn load_from_path(path: &str) -> Result<Self> where Self: Sized;

	fn get_page_path(&self) -> PathBuf;
	// TODO: Optional for now. Will be a Result. Unique ID should ALWAYS exist.
	fn get_unique_id(&self) -> Result<Cow<str>>;

	/// Get the raw page
	fn read_page_raw_as_bytes(&mut self) -> Result<Vec<u8>>;

	/// Get the raw page
	fn read_page_raw_as_string(&mut self) -> Result<String> {
		Ok(String::from_utf8(self.read_page_raw_as_bytes()?)?)
	}

	/// Get the page with urls relative to the internal zip structure
	fn read_page_as_bytes(&mut self, prepend_to_urls: Option<&str>, add_css: Option<&[&str]>) -> Result<Vec<u8>>;

	/// Get the page with urls relative to the internal zip structure
	fn read_page_as_string(&mut self, prepend_to_urls: Option<&str>, add_css: Option<&[&str]>) -> Result<String> {
		Ok(String::from_utf8(self.read_page_as_bytes(prepend_to_urls, add_css)?)?)
	}

	fn read_path_as_bytes(&mut self, path: &str, prepend_to_urls: Option<&str>, add_css: Option<&[&str]>) -> Result<Vec<u8>>;
	fn read_path_as_string(&mut self, path: &str, prepend_to_urls: Option<&str>, add_css: Option<&[&str]>) -> Result<String> {
		Ok(String::from_utf8(self.read_path_as_bytes(path, prepend_to_urls, add_css)?)?)
	}

	fn chapter_count(&self) -> usize;
	fn get_chapter(&self) -> usize;

	fn set_chapter(&mut self, value: usize) -> bool;
	fn next_chapter(&mut self) -> bool;
	fn previous_chapter(&mut self) -> bool;

	fn get_root_file_dir(&self) -> &Path;

	fn find(&self, search: BookSearch<'_>) -> Option<Vec<String>>;
}

pub enum BookSearch<'a> {
	// Required
	Title,
	Identifier,
	Language,

	// Optional
    Contributor,
	Coverage,
	CoverImage,
	Creator,
	Date,
	Description,
	Format,
	Publisher,
	Relation,
	Rights,
	Source,
	Subject,
	Type,

	Other(&'a str)
}

impl<'a> From<&'a str> for BookSearch<'a> {
    fn from(value: &'a str) -> Self {
        Self::Other(value)
    }
}


pub fn load_from_path(path: &str) -> Result<Option<Box<dyn Book>>> {
	Ok(match path.rsplit_once('.').map(|v| v.1) {
		Some("epub") => Some(Box::new(epub::EpubBook::load_from_path(path)?)),

		_ => None
	})
}





// Used to help handle ids a little better "amazon:{id}", "amazon_uk:{id}", "goodreads:{id}", "isbn:{id}", "google:{id}", "uuid:{id}", "urn:uuid:{id}", "urn:isbn:{id}"
pub fn parse_book_id(value: &str) -> IdType {
	if let Some((prefix, suffix)) = value.rsplit_once(':') {
		let prefix = prefix.to_lowercase().replace(' ', "");
		let suffix = suffix.trim().to_owned();

		match prefix.as_str() {
			"urn:isbn" |
			"isbn" => IdType::Isbn(suffix),

			"urn:uuid" |
			"uuid" => IdType::Uuid(suffix),

			_ => IdType::UnknownKeyValue(prefix, suffix),
		}
	} else {
		IdType::UnknownValue(value.trim().to_string())
	}
}

pub enum IdType {
	Isbn(String),
	Uuid(String),

	UnknownKeyValue(String, String),
	UnknownValue(String)
}

impl IdType {
	pub fn get_possible_isbn_value(&self) -> Option<&str> {
		match self {
			Self::UnknownValue(v) if v.chars().all(|v| ('0'..='9').contains(&v)) => Some(v.as_str()),
			Self::Isbn(v) => Some(v.as_str()),

			_ => None,
		}
	}

	pub fn into_possible_isbn_value(self) -> Option<String> {
		match self {
			Self::UnknownValue(v) if v.chars().all(|v| ('0'..='9').contains(&v)) => Some(v),
			Self::Isbn(v) => parse_isbn_13(&v).or_else(|| parse_isbn_10(&v)).or(Some(v)),

			_ => None,
		}
	}

	pub fn into_possible_single_value(self) -> Option<String> {
		match self {
			Self::Isbn(v) => Some(v),
			Self::Uuid(v) => Some(v),
			Self::UnknownKeyValue(_, v) => Some(v),
			Self::UnknownValue(v) => Some(v),
		}
	}


	/// Attempts to return an ISBN type of 13 or 12 in that order.
	pub fn as_isbn_13_or_10(&self) -> Option<String> {
		self.as_isbn_13().or_else(|| self.as_isbn_10())
	}

	pub fn as_isbn_13(&self) -> Option<String> {
		match self {
			Self::UnknownValue(v) => parse_isbn_13(v.as_str()),
			Self::Isbn(v) => parse_isbn_13(v.as_str()),

			_ => None,
		}
	}

	pub fn as_isbn_10(&self) -> Option<String> {
		match self {
			Self::UnknownValue(v) => parse_isbn_10(v.as_str()),
			Self::Isbn(v) => parse_isbn_10(v.as_str()),

			_ => None,
		}
	}
}

// TODO: Convert all ISBN-10's to ISBN-13

// TODO: Tests
pub fn parse_isbn_10(value: &str) -> Option<String> {
	let mut s = 0;
	let mut t = 0;

	let mut parse = value.split("").filter(|v| *v != "-" && !v.is_empty());

	let mut compiled = String::new();

	if let Some(v) = parse.next() {
		compiled.push_str(v);
	}

	for dig_str in parse.take(9) {
		let dig = match dig_str.parse::<usize>() {
			Ok(v) => v,
			Err(_) => return None
		};

		compiled.push_str(dig_str);

		t += dig;
		s += t;
	}

	Some(compiled).filter(|v| v.len() == 10 && (s % 11) == 0)
}

// TODO: Tests
pub fn parse_isbn_13(value: &str) -> Option<String> {
	let mut s = 0;

	let mut compiled = String::new();

	for (i, dig_str) in value.split("").filter(|v| *v != "-" && !v.is_empty()).take(13).enumerate() {
		let dig = match dig_str.parse::<usize>() {
			Ok(v) => v,
			Err(_) => return None
		};

		compiled.push_str(dig_str);

		let weight = if i % 2 == 0 { 1 } else { 3 };
		s += dig * weight;
	}

	Some(compiled).filter(|v| v.len() == 13 && (s % 10) == 0)
}