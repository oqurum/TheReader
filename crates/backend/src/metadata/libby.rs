use std::convert::TryFrom;

use crate::{
    config::get_config,
    metadata::{AuthorInfo, SearchItem},
    model::file::FileModel,
    Result,
};
use async_trait::async_trait;
use common::{
    api::{
        librarian::{PublicBook, PublicSearchResponse, PublicSearchType},
        WrappingResponse,
    },
    Agent, Source,
};
use common_local::{BookItemCached, SearchFor};

use super::{FoundImageLocation, FoundItem, Metadata, MetadataReturned};

pub struct LibbyMetadata;

#[async_trait]
impl Metadata for LibbyMetadata {
    fn get_agent(&self) -> Agent {
        Agent::new_static("libby")
    }

    async fn get_metadata_from_files(
        &mut self,
        files: &[FileModel],
    ) -> Result<Option<MetadataReturned>> {
        for file in files {
            if let Some(isbn) = file.identifier.clone() {
                match self.request_book_query(isbn).await {
                    Ok(Some(v)) => return Ok(Some(v)),
                    a => eprintln!("LibbyMetadata::get_metadata_from_files {:?}", a),
                }
            }
        }

        Ok(None)
    }

    async fn get_metadata_by_source_id(&mut self, value: &str) -> Result<Option<MetadataReturned>> {
        match self.request_singular_book_id(value).await {
            Ok(Some(v)) => Ok(Some(v)),
            a => {
                eprintln!("LibbyMetadata::get_metadata_by_source_id {:?}", a);

                Ok(None)
            }
        }
    }

    async fn get_person_by_source_id(&mut self, value: &str) -> Result<Option<AuthorInfo>> {
        match request_authors(value).await? {
            WrappingResponse::Resp(resp) => {
                if let PublicSearchType::AuthorItem(Some(item)) = resp {
                    return Ok(Some(AuthorInfo {
                        source: self.prefix_text(item.id.to_string()).try_into()?,
                        name: item.name,
                        other_names: None,
                        description: item.description,
                        cover_image_url: item.thumb_url.map(FoundImageLocation::Url),
                        birth_date: item.birth_date,
                        death_date: None,
                    }));
                }
            }

            WrappingResponse::Error(err) => {
                eprintln!("[METADATA][LIBBY]: Response Error: {}", err);
            }
        }

        Ok(None)
    }

    async fn search(&mut self, search: &str, search_for: SearchFor) -> Result<Vec<SearchItem>> {
        let libby = get_config().libby;

        match search_for {
            SearchFor::Person => {
                let url = format!(
                    "{}/search/book?query={}&server_id={}&view_private={}",
                    libby.url,
                    urlencoding::encode(search),
                    urlencoding::encode(&libby.token.unwrap()),
                    !libby.public_only
                );

                println!("[METADATA][LIBBY]: Search URL: {}", url);

                match request_authors(&url).await? {
                    WrappingResponse::Resp(resp) => {
                        let mut books = Vec::new();

                        if let PublicSearchType::AuthorList(resp) = resp {
                            for item in resp.items {
                                books.push(SearchItem::Author(AuthorInfo {
                                    source: self.prefix_text(item.id.to_string()).try_into()?,
                                    name: item.name,
                                    other_names: None,
                                    description: item.description,
                                    cover_image_url: item.thumb_url.map(FoundImageLocation::Url),
                                    birth_date: item.birth_date,
                                    death_date: None,
                                }));
                            }
                        }

                        Ok(books)
                    }

                    WrappingResponse::Error(err) => {
                        eprintln!("[METADATA][LIBBY]: Response Error: {}", err);
                        Ok(Vec::new())
                    }
                }
            }

            SearchFor::Book(_specifically) => {
                let url = format!(
                    "{}/search/book?query={}&server_id={}&view_private={}",
                    libby.url,
                    urlencoding::encode(search),
                    urlencoding::encode(&libby.token.unwrap()),
                    !libby.public_only
                );

                println!("[METADATA][LIBBY]: Search URL: {}", url);

                match request_books(&url).await? {
                    WrappingResponse::Resp(resp) => {
                        let mut books = Vec::new();

                        if let PublicSearchType::BookList(books_cont) = resp {
                            for item in books_cont.items {
                                books.push(SearchItem::Book(FoundItem {
                                    source: self.prefix_text(item.id.to_string()).try_into()?,
                                    title: item.title,
                                    description: item.description,
                                    rating: item.rating,
                                    thumb_locations: item
                                        .thumb_url
                                        .map(|v| vec![FoundImageLocation::Url(v)])
                                        .unwrap_or_default(),
                                    cached: BookItemCached::default(),
                                    available_at: item
                                        .available_at
                                        .map(|v| v.and_hms(0, 0, 0).timestamp_millis()),
                                    year: None,
                                }));
                            }
                        }

                        Ok(books)
                    }

                    WrappingResponse::Error(err) => {
                        eprintln!("[METADATA][LIBBY]: Response Error: {}", err);
                        Ok(Vec::new())
                    }
                }
            }
        }
    }
}

impl LibbyMetadata {
    pub async fn request_book_query(&self, value: String) -> Result<Option<MetadataReturned>> {
        let libby = get_config().libby;

        let url = format!(
            "{}/search/book?query={}&server_id={}&view_private={}",
            libby.url,
            urlencoding::encode(&value),
            urlencoding::encode(&libby.token.unwrap()),
            !libby.public_only
        );

        println!("[METADATA][LIBBY]: Req Query: {}", url);

        let book = match request_books(&url).await? {
            WrappingResponse::Resp(resp) => match resp {
                PublicSearchType::BookList(mut books) => {
                    if books.total == 1 {
                        books.items.remove(0)
                    } else {
                        return Ok(None);
                    }
                }

                _ => return Ok(None),
            },

            WrappingResponse::Error(err) => {
                eprintln!("[METADATA][LIBBY]: Response Error: {}", err);
                return Ok(None);
            }
        };

        self.request_singular_book_id(&book.id.to_string()).await
    }

    pub async fn request_singular_author_id(&self, id: &str) -> Result<Option<AuthorInfo>> {
        let libby = get_config().libby;

        let url = format!(
            "{}/search/author?query=id:{}&server_id={}",
            libby.url,
            urlencoding::encode(id),
            urlencoding::encode(&libby.token.unwrap()),
        );

        println!("[METADATA][LIBBY]: Get Single Author URL: {url}");

        match request_authors(&url).await? {
            WrappingResponse::Resp(resp) => match resp {
                PublicSearchType::AuthorItem(Some(author)) => Ok(Some(AuthorInfo {
                    source: Source::try_from(self.prefix_text(author.id.to_string())).unwrap(),
                    cover_image_url: author.thumb_url.map(FoundImageLocation::Url),
                    name: author.name,
                    other_names: Some(author.other_names).filter(|v| !v.is_empty()),
                    description: author.description,
                    birth_date: author.birth_date,
                    death_date: None,
                })),
                _ => Ok(None),
            },

            WrappingResponse::Error(err) => {
                eprintln!("[METADATA][LIBBY]: Response Error: {}", err);
                Ok(None)
            }
        }
    }

    pub async fn request_singular_book_id(&self, id: &str) -> Result<Option<MetadataReturned>> {
        let libby = get_config().libby;

        let url = format!(
            "{}/search/book?query=id:{}&server_id={}",
            libby.url,
            urlencoding::encode(id),
            urlencoding::encode(&libby.token.unwrap()),
        );

        println!("[METADATA][LIBBY]: Get Single Book URL: {url}");

        match request_books(&url).await? {
            WrappingResponse::Resp(resp) => match resp {
                PublicSearchType::BookItem(Some(book)) => self.compile_book_volume_item(book).await,
                _ => Ok(None),
            },

            WrappingResponse::Error(err) => {
                eprintln!("[METADATA][LIBBY]: Response Error: {}", err);
                Ok(None)
            }
        }
    }

    async fn compile_book_volume_item(
        &self,
        value: PublicBook,
    ) -> Result<Option<MetadataReturned>> {
        let mut authors = Vec::new();
        let mut author_name = None;

        for author_id in value.author_ids {
            if let Some(author) = self
                .request_singular_author_id(&author_id.to_string())
                .await?
            {
                if Some(author_id) == value.display_author_id {
                    author_name = Some(author.name.clone());
                }

                authors.push(author);
            }
        }

        Ok(Some(MetadataReturned {
            authors: Some(authors).filter(|v| !v.is_empty()),
            publisher: value.publisher.clone(),
            meta: FoundItem {
                source: self.prefix_text(value.id.to_string()).try_into()?,
                title: value.title,
                description: value.description,
                rating: value.rating,
                thumb_locations: value
                    .thumb_url
                    .map(|v| vec![FoundImageLocation::Url(v)])
                    .unwrap_or_default(),
                cached: BookItemCached::default()
                    .publisher_optional(value.publisher)
                    .author_optional(author_name),
                available_at: value
                    .available_at
                    .map(|v| v.and_hms(0, 0, 0).timestamp_millis()),
                year: None,
            },
        }))
    }
}

async fn request_books(value: &str) -> Result<PublicSearchResponse> {
    Ok(reqwest::get(value).await?.json().await?)
}

async fn request_authors(value: &str) -> Result<PublicSearchResponse> {
    Ok(reqwest::get(value).await?.json().await?)
}
