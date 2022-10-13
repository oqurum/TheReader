use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

pub mod cb;
pub mod epub;
pub mod mobi;

pub mod error;
pub use error::*;

// TODO: path: &str -> path: &Path

pub trait Book {
    fn load_from_path(path: &str) -> Result<Self>
    where
        Self: Sized;

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
    fn read_page_as_bytes(
        &mut self,
        prepend_to_urls: Option<&str>,
        add_css: Option<&[&str]>,
    ) -> Result<Vec<u8>>;

    /// Get the page with urls relative to the internal zip structure
    fn read_page_as_string(
        &mut self,
        prepend_to_urls: Option<&str>,
        add_css: Option<&[&str]>,
    ) -> Result<String> {
        Ok(String::from_utf8(
            self.read_page_as_bytes(prepend_to_urls, add_css)?,
        )?)
    }

    fn read_path_as_bytes(
        &mut self,
        path: &str,
        prepend_to_urls: Option<&str>,
        add_css: Option<&[&str]>,
    ) -> Result<Vec<u8>>;
    fn read_path_as_string(
        &mut self,
        path: &str,
        prepend_to_urls: Option<&str>,
        add_css: Option<&[&str]>,
    ) -> Result<String> {
        Ok(String::from_utf8(self.read_path_as_bytes(
            path,
            prepend_to_urls,
            add_css,
        )?)?)
    }

    fn get_files(&self) -> Vec<String> {
        Vec::new()
    }

    fn chapter_count(&self) -> usize;
    fn get_chapter(&self) -> usize;

    fn set_chapter(&mut self, value: usize) -> bool;
    fn next_chapter(&mut self) -> bool;
    fn previous_chapter(&mut self) -> bool;

    fn get_root_file_dir(&self) -> &Path;

    fn find(&self, search: BookSearch<'_>) -> Option<Vec<String>>;

    fn compute_hash(&mut self) -> Option<String>;
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

    Other(&'a str),
}

impl<'a> From<&'a str> for BookSearch<'a> {
    fn from(value: &'a str) -> Self {
        Self::Other(value)
    }
}

pub fn load_from_path(path: &str) -> Result<Option<Box<dyn Book>>> {
    Ok(match path.rsplit_once('.').map(|v| v.1) {
        Some("cbz") => Some(Box::new(cb::ComicBook::load_from_path(path)?)),
        Some("epub") => Some(Box::new(epub::EpubBook::load_from_path(path)?)),

        _ => None,
    })
}
