use crate::{Result, model::file::FileModel};
use async_trait::async_trait;
use bookie::BookSearch;
use common_local::BookItemCached;

use super::{Metadata, MetadataReturned, AuthorInfo, FoundItem, FoundImageLocation};



pub struct LocalMetadata;

#[async_trait]
impl Metadata for LocalMetadata {
    fn get_prefix(&self) ->  &'static str {
        "local"
    }

    async fn get_metadata_from_files(&mut self, files: &[FileModel]) -> Result<Option<MetadataReturned>> {
        for file in files {
            // Wrapped to prevent "future cannot be sent between threads safely"
            let (meta, authors, publisher) = {
                let mut book = match bookie::load_from_path(&file.path)? {
                    Some(v) => v,
                    None => continue,
                };

                let source = self.prefix_text(book.get_unique_id()?);

                let title = book.find(BookSearch::Title).map(|mut v| v.remove(0));
                let thumb_file_data = book.find(BookSearch::CoverImage)
                    .map(|mut v| v.remove(0))
                    .map::<Result<_>, _>(|url| Ok(vec![FoundImageLocation::FileData(book.read_path_as_bytes(&url, None, None)?)]))
                    .transpose()?;

                let publisher = book.find(BookSearch::Publisher).map(|mut v| v.remove(0));
                let authors = book.find(BookSearch::Creator)
                    .map(|items| items.into_iter()
                        .map(|name| AuthorInfo {
                            source: source.as_str().try_into().unwrap(),
                            name,
                            other_names: None,
                            description: None,
                            cover_image_url: None,
                            birth_date: None,
                            death_date: None,
                        })
                        .collect::<Vec<_>>()
                    );

                (FoundItem {
                    source: source.try_into()?,
                    title,
                    description: book.find(BookSearch::Description).map(|mut v| v.remove(0)),
                    rating: 0.0,
                    thumb_locations: thumb_file_data.unwrap_or_default(),
                    cached: BookItemCached::default(),
                    available_at: None,
                    year: None,
                }, authors, publisher)
            };

            // TODO:
            // meta.all_thumbnail_urls = match opt_thumb_url {
            //     Some(book_file_path) => {
            //         let image = book_file_path?;

            //         match crate::store_image(ThumbnailType::Local, image).await {
            //             Ok(path) => path.into(),
            //             Err(e) => {
            //                 eprintln!("store_image: {}", e);
            //                 ThumbnailPath::default()
            //             }
            //         }
            //     }

            //     None => ThumbnailPath::default(),
            // };

            return Ok(Some(MetadataReturned {
                authors,
                publisher,
                meta,
            }));
        }

        Ok(None)
    }
}