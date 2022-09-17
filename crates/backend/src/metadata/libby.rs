use crate::{Result, model::file::FileModel, config::get_config, metadata::SearchItem};
use async_trait::async_trait;
use common::{api::{librarian::{BookSearchResponse, PublicBook}, WrappingResponse}, Agent};
use common_local::{BookItemCached, SearchFor};

use super::{Metadata, MetadataReturned, FoundItem, FoundImageLocation};



pub struct LibbyMetadata;

#[async_trait]
impl Metadata for LibbyMetadata {
    fn get_agent(&self) -> Agent {
        Agent::new_static("libby")
    }

    async fn get_metadata_from_files(&mut self, files: &[FileModel]) -> Result<Option<MetadataReturned>> {
        for file in files {
            if let Some(isbn) = file.identifier.clone() {
                match self.request_query(isbn).await {
                    Ok(Some(v)) => return Ok(Some(v)),
                    a => eprintln!("LibbyMetadata::get_metadata_from_files {:?}", a)
                }
            }
        }

        Ok(None)
    }

    async fn get_metadata_by_source_id(&mut self, value: &str) -> Result<Option<MetadataReturned>> {
        match self.request_singular_id(value).await {
            Ok(Some(v)) => Ok(Some(v)),
            a => {
                eprintln!("LibbyMetadata::get_metadata_by_source_id {:?}", a);

                Ok(None)
            }
        }
    }

    async fn search(&mut self, search: &str, search_for: SearchFor) -> Result<Vec<SearchItem>> {
        match search_for {
            SearchFor::Person => Ok(Vec::new()),

            SearchFor::Book(_specifically) => {
                let libby = get_config().libby;

                let url = format!(
                    "{}/search?query={}&server_id={}&view_private={}",
                    libby.url,
                    urlencoding::encode(search),
                    urlencoding::encode(&libby.token.unwrap()),
                    !libby.public_only
                );

                println!("[METADATA][LIBBY]: Search URL: {}", url);

                let resp = reqwest::get(url).await?;

                if resp.status().is_success() {
                    match resp.json::<BookSearchResponse>().await? {
                        WrappingResponse::Resp(books_cont) => {
                            let mut books = Vec::new();

                            for item in books_cont.items {
                                books.push(SearchItem::Book(FoundItem {
                                    source: self.prefix_text(item.id.to_string()).try_into()?,
                                    title: item.title,
                                    description: item.description,
                                    rating: item.rating,
                                    thumb_locations: vec![FoundImageLocation::Url(item.thumb_url)],
                                    cached: BookItemCached::default(),
                                    available_at: item.available_at.and_then(|v| v.parse().ok()),
                                    year: None,
                                }));
                            }

                            Ok(books)
                        }

                        WrappingResponse::Error(err) => {
                            eprintln!("[METADATA][LIBBY]: Response Error: {}", err);
                            Ok(Vec::new())
                        }
                    }
                } else {
                    return Ok(Vec::new());
                }
            }
        }
    }
}

impl LibbyMetadata {
    pub async fn request_query(&self, value: String) -> Result<Option<MetadataReturned>> {
        let libby = get_config().libby;

        let url = format!(
            "{}/search?query={}&server_id={}&view_private={}",
            libby.url,
            urlencoding::encode(&value),
            urlencoding::encode(&libby.token.unwrap()),
            !libby.public_only
        );

        println!("[METADATA][LIBBY]: Req Query: {}", url);

        let resp = reqwest::get(url).await?;

        let book = if resp.status().is_success() {
            match resp.json::<BookSearchResponse>().await? {
                WrappingResponse::Resp(mut books) => {
                    if books.total == 1 {
                        books.items.remove(0)
                    } else {
                        return Ok(None);
                    }
                }

                WrappingResponse::Error(err) => {
                    eprintln!("[METADATA][LIBBY]: Response Error: {}", err);
                    return Ok(None);
                }
            }
        } else {
            return Ok(None);
        };

        self.compile_book_volume_item(book).await
    }

    pub async fn request_singular_id(&self, id: &str) -> Result<Option<MetadataReturned>> {
        let libby = get_config().libby;

        let resp = reqwest::get(format!(
            "{}/search?query=id:{}&server_id={}&view_private={}",
            libby.url,
            urlencoding::encode(id),
            urlencoding::encode(&libby.token.unwrap()),
            !libby.public_only
        )).await?;

        if resp.status().is_success() {
            self.compile_book_volume_item(resp.json().await?).await
        } else {
            Ok(None)
        }
    }


    async fn compile_book_volume_item(&self, value: PublicBook) -> Result<Option<MetadataReturned>> {
        Ok(Some(MetadataReturned {
            authors: None,
            publisher: None,
            meta: FoundItem {
                source: self.prefix_text(value.id.to_string()).try_into()?,
                title: value.title,
                description: value.description,
                rating: value.rating,
                thumb_locations: vec![FoundImageLocation::Url(value.thumb_url)],
                cached: BookItemCached::default(),
                available_at: value.available_at.and_then(|v| v.parse().ok()),
                year: None,
            }
        }))
    }
}