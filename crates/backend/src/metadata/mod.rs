use anyhow::Result;
use async_trait::async_trait;

use crate::database::table::{MetadataItem, File};

pub mod audible;
pub mod commonsensemedia;
pub mod goodreads;
pub mod local;
pub mod openlibrary;
pub mod ratedreads;

// "source" column: [prefix]:[id]


#[async_trait]
pub trait Metadata {
	fn get_prefix(&self) -> &'static str;

	async fn try_parse(&mut self, file: &File) -> Result<Option<MetadataItem>>;
}

pub async fn get_metadata(file: &File) -> Result<Option<MetadataItem>> {
	local::LocalMetadata.try_parse(file).await
}