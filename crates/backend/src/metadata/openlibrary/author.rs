// https://www.w3.org/RDF/

use anyhow::Result;

use super::book::BookId;

// Call rfd after calling book.

pub async fn get_authors_from_book_by_rfd(id: &BookId) -> Result<Vec<rfd::AuthorDescription>> {
	let resp = reqwest::get(id.get_rdf_url()).await?;

	let text = resp.text().await?;

	let record: rfd::RfdModel = serde_xml_rs::from_str(&text)?;

	let authors = record.description.into_iter()
		.filter_map(|v| v.authors)
		.flatten()
		.flat_map(|v| v.description)
		.collect::<Vec<_>>();

	Ok(authors)
}


pub async fn get_author_from_url(url_or_path: &str) -> Result<json::AuthorJson> {
	let resp = reqwest::get(into_url(url_or_path)).await?;
	Ok(resp.json().await?)
}

fn into_url(url_or_path: &str) -> String {
	if url_or_path.starts_with("/authors") {
		format!("https://openlibrary.org{}.json", url_or_path)
	}
	else if url_or_path.starts_with("OL") {
		format!("https://openlibrary.org/authors/{}.json", url_or_path)
	} else {
		url_or_path.to_string()
	}
}


pub mod json {
	use std::collections::HashMap;

	use serde::{Deserialize, Serialize};
	use crate::metadata::openlibrary::{KeyItem, TypeValueItem, RecordDescription};

	#[derive(Debug, Serialize, Deserialize)]
	#[serde(deny_unknown_fields)]
	pub struct AuthorJson {
		pub id: Option<i64>,
		pub bio: Option<RecordDescription>,
		pub r#type: KeyItem,
		pub remote_ids: Option<HashMap<String, String>>, // TODO: Figure out all names: viaf, isni, wikidata
		pub name: String,
		pub entity_type: Option<String>,
		pub title: Option<String>,
		pub personal_name: Option<String>,
		pub source_records: Option<Vec<String>>,
		pub alternate_names: Option<Vec<String>>,
		pub photos: Option<Vec<i64>>,
		pub key: String,
		pub links: Option<Vec<Link>>,
		pub wikipedia: Option<String>,
		pub birth_date: Option<String>,
		pub death_date: Option<String>,
		pub latest_revision: Option<usize>,
		pub revision: usize,
		pub created: Option<TypeValueItem>,
		pub last_modified: Option<TypeValueItem>,
	}

	#[derive(Debug, Serialize, Deserialize)]
	pub struct Link {
		pub url: String,
		pub title: String,
		pub r#type: KeyItem,
	}
}


pub mod rfd {
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Serialize, Deserialize)]
	pub struct RfdModel {
		#[serde(rename = "Description")]
		pub description: Vec<RfdDescriptionItem>,
	}

	#[derive(Debug, Serialize, Deserialize)]
	pub struct RfdDescriptionItem {
			pub about: String,
			#[serde(rename = "authorList")]
			pub authors: Option<Vec<AuthorList>>,

			pub contributor: Option<Vec<String>>,

			// title: String,
			// publisher: String,
			// #[serde(rename = "placeOfPublication")]
			// publication_place: Option<String>,
			// issued: String,
			// extent: String,

			// edition: Option<String>,
	}

	#[derive(Debug, Serialize, Deserialize)]
	pub struct AuthorList {
		#[serde(rename = "Description")]
		pub description: Vec<AuthorDescription>,
	}

	#[derive(Debug, Serialize, Deserialize)]
	pub struct AuthorDescription {
		pub about: String,
		pub name: String,
	}
}