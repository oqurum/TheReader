// http://idpf.org/epub/20/spec/OPF_2.0.1_draft.htm

// https://www.w3.org/ns/epub/2007/opf/

// https://www.w3.org/publishing/epub3/epub-ocf.html
// https://www.w3.org/publishing/epub3/epub-packages.html

use std::{
    borrow::Cow,
    fs::File,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use binstall_zip::ZipArchive;
use common_local::sort::filename_sort;

pub mod container;
mod file_ncx;
mod modifier;
mod package_document;

use crate::{BookSearch, Result};

use self::file_ncx::FileNCX;

use super::Book;
use container::*;

pub use modifier::*;
pub use package_document::*;

// TODO: Ignore specific file entries? Eg. "META-INF/calibre_bookmarks.txt"
// Would allow for better file hashing to compare against. Eg. One zip may have it, the other may not even though they're the same.

pub struct EpubBook {
    pub container: AbsContainer<File>,
    pub package: PackageDocument,

    root_file_dir: PathBuf,

    pub chapter: usize,
}

impl EpubBook {
    fn init(&mut self) -> Result<()> {
        // println!("{:?}", self.container.archive.file_names().collect::<Vec<_>>());

        // TODO: Incorrect way to do this.
        let path = self.container.root_files()[0].full_path.clone();

        // Init Package Document
        let file = self.container.archive.by_name(&path)?;
        self.package = PackageDocument::parse(file)?;

        self.root_file_dir = PathBuf::from(path);
        self.root_file_dir.pop();

        Ok(())
    }

    fn get_manifest_item_by_spine(&self, value: usize) -> Option<&ManifestItem> {
        let spine_item = self.package.spine.items.get(value)?;

        self.package
            .manifest
            .items
            .iter()
            .find(|mani_item| mani_item.id == spine_item.idref)
    }

    fn get_path_contents(&mut self, path: &str) -> Result<Vec<u8>> {
        // TODO: Optimize
        let path = if path.starts_with(&self.root_file_dir.display().to_string().replace('\\', "/"))
        {
            path.to_string()
        } else {
            self.root_file_dir
                .join(path)
                .display()
                .to_string()
                .replace('\\', "/")
        };

        // FIX: Some books have encoded their manifest item @href. We need to decode them.
        let path = urlencoding::decode(&path)?;

        let mut buf = Vec::new();

        self.container
            .archive
            .by_name(&path)?
            .read_to_end(&mut buf)?;

        Ok(buf)
    }

    fn handle_update_attributes(
        &mut self,
        input: &[u8],
        page_path: PathBuf,
        prepend_to_urls: Option<&str>,
        add_css: Option<&[&str]>,
    ) -> Result<Vec<u8>> {
        update_attributes_with(
            input,
            self,
            |book, element_name, mut attr| {
                attr.value = match (
                    element_name.local_name.as_str(),
                    attr.name.local_name.as_str(),
                ) {
                    ("link", "href") => update_value_with_relative_internal_path(
                        page_path.clone(),
                        &attr.value,
                        prepend_to_urls,
                    ),
                    ("img", "src") => {
                        let path = update_value_with_relative_internal_path(
                            page_path.clone(),
                            &attr.value,
                            None,
                        );
                        let cont = book.get_path_contents(&path);

                        if let Ok(cont) = cont {
                            let b64 = base64::encode(cont);

                            if let Some((_, type_of)) = attr.value.rsplit_once('.') {
                                format!("data:image/{};charset=utf-8;base64,{}", type_of, b64)
                            } else {
                                format!("data:image;charset=utf-8;base64,{}", b64)
                            }
                        } else {
                            update_value_with_relative_internal_path(
                                page_path.clone(),
                                &attr.value,
                                prepend_to_urls,
                            )
                        }
                    }
                    ("image", "href") => update_value_with_relative_internal_path(
                        page_path.clone(),
                        &attr.value,
                        prepend_to_urls,
                    ),
                    _ => return attr,
                };
                attr
            },
            |book, name, attrs, writer| {
                if name.local_name == "link"
                    && attrs.iter().any(|v| {
                        v.name.local_name == "rel" && v.value.to_lowercase() == "stylesheet"
                    })
                {
                    if let Some(attr) = attrs.iter().find(|a| a.name.local_name == "href") {
                        // TODO: handle background-image:url('imagename.png');
                        // Remember to account for directory of the css file when correcting the url. (eg. "/css/" instead of just "/")

                        let path = update_value_with_relative_internal_path(
                            page_path.clone(),
                            &attr.value,
                            None,
                        );

                        if let Some(cont) = book
                            .get_path_contents(&path)
                            .ok()
                            .and_then(|v| String::from_utf8(v).ok())
                        {
                            writer
                                .write(xml::writer::XmlEvent::start_element("style"))
                                .unwrap();
                            writer
                                .write(xml::writer::XmlEvent::characters(&cont))
                                .unwrap();
                            writer.write(xml::writer::XmlEvent::end_element()).unwrap();

                            return true;
                        }
                    }
                }

                false
            },
            if let Some(v) = add_css { v } else { &[] },
        )
    }
}

impl Book for EpubBook {
    fn get_table_of_contents(&mut self) -> Result<Option<Vec<(String, usize)>>> {
        if let Some(path) = self
            .package
            .manifest
            .get_item_by_id("ncx")
            .map(|v| v.href.clone())
        {
            let value = self.read_path_as_bytes(&path, None, None)?;
            let ncx_file = FileNCX::parse(Cursor::new(value))?;

            Ok(Some(
                ncx_file
                    .nav_map
                    .into_iter()
                    .filter_map(|v| {
                        let item = self.package.manifest.get_item_by_href(&v.content.src)?;

                        Some((
                            v.nav_label.text,
                            self.package.spine.position_of_idref(&item.id)?,
                        ))
                    })
                    .collect(),
            ))
        } else {
            // TODO: Would need to parse the file
            // let toc = self.package.guide.find_toc().unwrap();

            Ok(None)
        }
    }

    fn load_from_path(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        let archive = ZipArchive::new(File::open(path)?)?;

        let mut this = Self {
            container: AbsContainer::new(archive)?,
            package: PackageDocument::default(),
            chapter: 0,
            root_file_dir: PathBuf::default(),
        };

        this.init()?;

        Ok(this)
    }

    fn compute_hash(&mut self) -> Option<String> {
        let mut hasher = blake3::Hasher::new();

        for href in self
            .package
            .manifest
            .items
            .iter()
            .map(|v| v.href.clone())
            .collect::<Vec<_>>()
        {
            if let Ok(asdf) = self.read_path_as_bytes(&href, None, None) {
                hasher.update(&asdf);
            }
        }

        Some(hasher.finalize().to_string())
    }

    fn find(&self, search: BookSearch<'_>) -> Option<Vec<String>> {
        match search {
            BookSearch::CoverImage => Some(vec![self
                .package
                .manifest
                .get_item_by_property("cover-image")?
                .href
                .to_owned()]),

            _ => {
                let tag_name = match &search {
                    BookSearch::Title => "title",
                    BookSearch::Identifier => "identifier",
                    BookSearch::Language => "language",
                    BookSearch::Contributor => "contributor",
                    BookSearch::Coverage => "coverage",
                    BookSearch::CoverImage => "cover-image",
                    BookSearch::Creator => "creator",
                    BookSearch::Date => "date",
                    BookSearch::Description => "description",
                    BookSearch::Format => "format",
                    BookSearch::Publisher => "publisher",
                    BookSearch::Relation => "relation",
                    BookSearch::Rights => "rights",
                    BookSearch::Source => "source",
                    BookSearch::Subject => "subject",
                    BookSearch::Type => "type",
                    BookSearch::Other(v) => *v,
                };

                let values: Vec<String> =
                    if let Some(elements) = self.package.metadata.dcmes_elements.get(tag_name) {
                        elements
                            .iter()
                            .filter_map(|v| v.value.as_ref().cloned())
                            .collect()
                    } else {
                        // dc terms are located in the metadata as meta items with said property names.
                        // https://www.dublincore.org/specifications/dublin-core/dcmi-terms/
                        let dc_tag_name = format!("dcterms:{}", tag_name);

                        self.package
                            .metadata
                            .meta_items
                            .iter()
                            .filter_map(|v| {
                                if v.property == dc_tag_name {
                                    Some(v.value.as_ref()?.to_owned())
                                } else {
                                    None
                                }
                            })
                            .collect()
                    };

                if values.is_empty() {
                    None
                } else {
                    Some(values)
                }
            }
        }
    }

    fn get_unique_id(&self) -> Result<Cow<str>> {
        if let Some(identifier_elements) = self.package.metadata.dcmes_elements.get("identifier") {
            // Find the unique ID based off of the specified one in the package attribute.
            let found_id = identifier_elements
                .iter()
                .filter(|v| v.id.is_some() && v.value.is_some())
                .find(|v| {
                    v.id.as_deref().unwrap() == self.package.attributes.unique_identifier.as_str()
                })
                .map(|v| Cow::from(v.value.as_deref().unwrap()));

            if let Some(found) = found_id {
                return Ok(found);
            }

            // Otherwise find the first ID we can that contains both an ID and a VALUE.
            let found_id = identifier_elements
                .iter()
                .find(|v| v.id.is_some() && v.value.is_some())
                .map(|v| Cow::from(v.value.as_deref().unwrap()));

            if let Some(found) = found_id {
                return Ok(found);
            }

            // Just grab the first identifier we have.
            let found_id = identifier_elements
                .iter()
                .find_map(|v| v.value.as_deref().map(Cow::from));

            if let Some(found) = found_id {
                return Ok(found);
            }
        }

        Ok(Cow::from(
            self.package.attributes.unique_identifier.as_str(),
        ))
    }

    fn get_files(&self) -> Vec<String> {
        let mut files = self
            .container
            .file_names_in_archive()
            .map(|v| v.to_string())
            .collect::<Vec<_>>();

        filename_sort(&mut files);

        files
    }

    fn get_root_file_dir(&self) -> &Path {
        self.root_file_dir.as_path()
    }

    fn get_page_path(&self) -> PathBuf {
        let item = self.get_manifest_item_by_spine(self.chapter);
        self.root_file_dir.join(item.unwrap().href.as_str())
    }

    fn read_page_raw_as_bytes(&mut self) -> Result<Vec<u8>> {
        let item = self.get_manifest_item_by_spine(self.chapter);

        if let Some(href) = item.map(|v| v.href.clone()) {
            self.get_path_contents(href.as_str())
        } else {
            Ok(Vec::new())
        }
    }

    fn read_path_as_bytes(
        &mut self,
        path: &str,
        prepend_to_urls: Option<&str>,
        add_css: Option<&[&str]>,
    ) -> Result<Vec<u8>> {
        if prepend_to_urls.is_some() || add_css.is_some() {
            let page_path = PathBuf::from(path);
            let input = self.get_path_contents(path)?;

            self.handle_update_attributes(&input, page_path, prepend_to_urls, add_css)
        } else {
            self.get_path_contents(path)
        }
    }

    fn read_page_as_bytes(
        &mut self,
        prepend_to_urls: Option<&str>,
        add_css: Option<&[&str]>,
    ) -> Result<Vec<u8>> {
        let page_path = self.get_page_path();
        let input = self.read_page_raw_as_bytes()?;

        self.handle_update_attributes(&input, page_path, prepend_to_urls, add_css)
    }

    fn chapter_count(&self) -> usize {
        self.package.spine.items.len()
    }

    fn set_chapter(&mut self, value: usize) -> bool {
        if self.chapter_count() >= value {
            self.chapter = value;
            true
        } else {
            false
        }
    }

    fn next_chapter(&mut self) -> bool {
        if self.chapter_count() > self.chapter + 1 {
            self.chapter += 1;
            true
        } else {
            false
        }
    }

    fn previous_chapter(&mut self) -> bool {
        if self.chapter != 0 {
            self.chapter -= 1;
            true
        } else {
            false
        }
    }

    fn get_chapter(&self) -> usize {
        self.chapter
    }
}

// TODO: General Conformance | https://www.w3.org/publishing/epub3/epub-packages.html#sec-conformance
// TODO: Reading System Conformance | https://www.w3.org/publishing/epub3/epub-packages.html#sec-package-rs-conf

// Prefixes
// dcterms     http://purl.org/dc/terms/
// opf     http://www.idpf.org/2007/opf
// rendition     http://www.idpf.org/vocab/rendition/#

// TODO: Tests

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn load() {
//         //
//     }
// }
