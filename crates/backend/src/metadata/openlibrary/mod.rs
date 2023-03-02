// https://openlibrary.org/developers/api

use crate::{model::file::FileModel, Result};
use async_trait::async_trait;
use chrono::NaiveDate;
use common::Agent;
use common_local::{BookItemCached, SearchForBooksBy};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use self::book::BookSearchType;

use super::{
    AuthorInfo, FoundImageLocation, FoundItem, Metadata, MetadataReturned, SearchFor, SearchItem,
};

pub mod author;
pub mod book;

use book::BookId;

pub struct OpenLibraryMetadata;

#[async_trait]
impl Metadata for OpenLibraryMetadata {
    fn get_agent(&self) -> Agent {
        Agent::new_static("openlibrary")
    }

    async fn get_metadata_from_files(
        &mut self,
        files: &[FileModel],
    ) -> Result<Option<MetadataReturned>> {
        for file in files {
            if let Some(isbn) = file.identifier.clone() {
                let Some(id) = BookId::make_assumptions(isbn) else {
                    continue;
                };

                match self.request(id).await {
                    Ok(Some(v)) => return Ok(Some(v)),
                    Ok(None) => info!("Unable to find book by isbn."),
                    Err(error) => error!(?error),
                }
            }
        }

        Ok(None)
    }

    async fn get_metadata_by_source_id(&mut self, value: &str) -> Result<Option<MetadataReturned>> {
        let Some(id) = BookId::make_assumptions(value.to_string()) else {
            return Ok(None);
        };

        match self.request(id).await {
            Ok(Some(v)) => Ok(Some(v)),
            Ok(None) => {
                info!("Unable to find book by id.");

                Ok(None)
            }

            Err(error) => {
                error!(?error);

                Ok(None)
            }
        }
    }

    async fn get_person_by_source_id(&mut self, value: &str) -> Result<Option<AuthorInfo>> {
        match author::get_author_from_url(value).await? {
            Some(author) => {
                Ok(Some(AuthorInfo {
                    source: self.prefix_text(value).try_into()?,
                    name: author.name.clone(),
                    other_names: author.alternate_names,
                    description: author.bio.map(|v| v.into_content()),
                    // Using value since it should always be value "OLXXXXXA" which is Olid
                    cover_image_url: Some(FoundImageLocation::Url(
                        self::CoverId::Olid(value.to_string()).get_author_cover_url(),
                    )),
                    birth_date: author.birth_date.and_then(|v| v.parse::<NaiveDate>().ok()),
                    death_date: author.death_date.and_then(|v| v.parse::<NaiveDate>().ok()),
                }))
            }

            None => Ok(None),
        }
    }

    async fn search(&mut self, value: &str, search_for: SearchFor) -> Result<Vec<SearchItem>> {
        match search_for {
            SearchFor::Person => {
                if let Some(found) = author::search_for_authors(value).await? {
                    let mut authors = Vec::new();

                    for item in found.items {
                        authors.push(SearchItem::Author(AuthorInfo {
                            source: self.prefix_text(item.key.as_deref().unwrap()).try_into()?,
                            cover_image_url: Some(FoundImageLocation::Url(
                                self::CoverId::Olid(item.key.unwrap()).get_author_cover_url(),
                            )),
                            name: item.name.unwrap(),
                            other_names: item.alternate_names,
                            description: None,
                            birth_date: item.birth_date.and_then(|v| v.parse::<NaiveDate>().ok()),
                            death_date: item.death_date.and_then(|v| v.parse::<NaiveDate>().ok()),
                        }));
                    }

                    Ok(authors)
                } else {
                    Ok(Vec::new())
                }
            }

            SearchFor::Book(specifically) => {
                let type_of_search = match specifically {
                    SearchForBooksBy::AuthorName => BookSearchType::Author,
                    SearchForBooksBy::Contents | SearchForBooksBy::Query => BookSearchType::Query,
                    SearchForBooksBy::Title => BookSearchType::Title,
                };

                if let Some(found) = book::search_for_books(type_of_search, value).await? {
                    let mut books = Vec::new();

                    for item in found.items {
                        books.push(SearchItem::Book(FoundItem {
                            // TODO: Move .replace
                            source: format!(
                                "{}:{}",
                                self.get_agent(),
                                &item.key.replace("/works/", "").replace("/books/", "")
                            )
                            .try_into()?,
                            title: item.title.clone(),
                            description: None,
                            rating: 0.0,
                            thumb_locations: item
                                .cover_edition_key
                                .map(|v| {
                                    vec![FoundImageLocation::Url(
                                        CoverId::Olid(v).get_book_cover_url(),
                                    )]
                                })
                                .unwrap_or_default(),
                            cached: BookItemCached::default(),
                            available_at: None,
                            year: item.first_publish_year,
                        }));
                    }

                    Ok(books)
                } else {
                    Ok(Vec::new())
                }
            }
        }
    }
}

impl OpenLibraryMetadata {
    pub async fn request(&self, id: BookId) -> Result<Option<MetadataReturned>> {
        let Some(mut book_info) = book::get_book_by_id(&id).await? else {
            return Ok(None);
        };

        // Find Authors.
        let authors_rfd = author::get_authors_from_book_by_rfd(&id).await?;

        // Now authors are just Vec< OL00000A >
        let authors_found = if let Some(authors) = book_info.authors.take() {
            let mut author_paths: Vec<String> = authors
                .into_iter()
                .map(|v| strip_url_or_path(v.author_key()))
                .collect();

            for auth in authors_rfd {
                let stripped = strip_url_or_path(auth.about);

                if !author_paths.contains(&stripped) {
                    author_paths.push(stripped);
                }
            }

            author_paths
        } else {
            authors_rfd
                .into_iter()
                .map(|auth| strip_url_or_path(auth.about))
                .collect()
        };

        let mut authors = Vec::new();

        // Now we'll grab the Authors.
        for author_id in authors_found {
            info!(author_id, "Acquiring Author");

            match author::get_author_from_url(&author_id).await {
                Ok(Some(author)) => {
                    authors.push(AuthorInfo {
                        source: self.prefix_text(author_id).try_into()?,
                        name: author.name.clone(),
                        other_names: author.alternate_names,
                        description: author.bio.map(|v| v.into_content()),
                        cover_image_url: Some(FoundImageLocation::Url(
                            self::CoverId::Olid(author.key).get_author_cover_url(),
                        )),
                        birth_date: author.birth_date.and_then(|v| v.parse::<NaiveDate>().ok()),
                        death_date: author.death_date.and_then(|v| v.parse::<NaiveDate>().ok()),
                    });
                }

                // TODO: Should this be an error? I believe yes since the website says they should have it.
                Ok(None) => info!("Unable to find Author"),
                Err(error) => error!(?error),
            }
        }

        // TODO: Parse record.publish_date | Millions of different variations. No specifics' were followed.

        let source_id = book_info.isbn_13.as_ref().and_then(|v| {
            v.first()
                .or_else(|| book_info.isbn_10.as_ref().and_then(|v| v.first()))
                .map(|v| v.to_string())
        }).unwrap_or_else(|| id.value().to_string());

        Ok(Some(MetadataReturned {
            authors: Some(authors).filter(|v| !v.is_empty()),
            publisher: book_info.publishers.and_then(|v| v.first().cloned()),

            meta: FoundItem {
                source: format!("{}:{}", self.get_agent(), source_id).try_into()?,
                title: Some(book_info.title.clone()),
                description: book_info
                    .description
                    .as_ref()
                    .map(|v| v.content().to_owned()),
                rating: 0.0,
                thumb_locations: book_info
                    .covers
                    .into_iter()
                    .flatten()
                    .filter(|v| *v != -1)
                    .map(|id| {
                        FoundImageLocation::Url(CoverId::Id(id.to_string()).get_book_cover_url())
                    })
                    .collect(),
                cached: BookItemCached::default(),
                available_at: None,
                year: None,
            },
        }))
    }
}

// TODO: Rate-Limited:
// The cover access by ids OTHER THAN CoverID and OLID are rate-limited.
// Currently only 100 requests/IP are allowed for every 5 minutes.
pub enum CoverId {
    Id(String), // TODO: number

    Isbn(String),
    Oclc(String),
    Lccn(String),
    Olid(String),

    Goodreads(String),
    LibraryThing(String),
}

impl CoverId {
    pub fn get_book_cover_url(&self) -> String {
        format!(
            "https://covers.openlibrary.org/b/{}/{}-L.jpg",
            self.key(),
            self.value()
        )
    }

    // TODO: Ensure we only use id, olid
    pub fn get_author_cover_url(&self) -> String {
        format!(
            "https://covers.openlibrary.org/a/{}/{}-L.jpg",
            self.key(),
            self.value()
        )
    }

    pub fn key(&self) -> &str {
        match self {
            Self::Id(_) => "id",
            Self::Isbn(_) => "isbn",
            Self::Oclc(_) => "oclc",
            Self::Lccn(_) => "lccn",
            Self::Olid(_) => "olid",
            Self::Goodreads(_) => "goodreads",
            Self::LibraryThing(_) => "librarything",
        }
    }

    pub fn value(&self) -> &str {
        match self {
            Self::Id(v) => v.as_str(),
            Self::Isbn(v) => v.as_str(),
            Self::Oclc(v) => v.as_str(),
            Self::Lccn(v) => v.as_str(),
            Self::Olid(v) => v.as_str(),
            Self::Goodreads(v) => v.as_str(),
            Self::LibraryThing(v) => v.as_str(),
        }
    }
}

fn strip_url_or_path<V: AsRef<str>>(value: V) -> String {
    value
        .as_ref()
        .rsplit('/')
        .find(|v| !v.is_empty())
        .unwrap()
        .to_owned()
}

/*
Types
    /type/text = "Normal Text" (used in: description)
    /type/datetime = "2021-09-30T16:27:03.066859" (used in: create, last_modified)
*/

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyItem {
    key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeValueItem {
    r#type: String, // TODO: Handle Types
    value: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RecordDescription {
    Text(String),
    SpecificType(TypeValueItem),
}

impl RecordDescription {
    pub fn content(&self) -> &str {
        match self {
            Self::Text(v) => v.as_str(),
            Self::SpecificType(v) => v.value.as_str(),
        }
    }

    pub fn into_content(self) -> String {
        match self {
            Self::Text(v) => v,
            Self::SpecificType(v) => v.value,
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::runtime::Runtime;

    use super::*;

    #[test]
    fn test_json_parse_url() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            book::get_book_by_id(&BookId::Edition(String::from("OL7353617M")))
                .await
                .unwrap();
        });
    }
}
