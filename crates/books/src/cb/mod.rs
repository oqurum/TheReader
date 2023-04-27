use std::{
    borrow::Cow,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use binstall_zip::ZipArchive;
use common_local::sort::filename_sort;

use super::Book;
use crate::{BookSearch, Result};

// TODO: Rar, 7z , TAR, ACE, ...
//       CBR, CB7, CBT, CBA, ...

// TODO: Handle multiple different types of cb files.
//  - [text content]_v01_p{00index}
//  - 0089-{00index}
//  - 00{index}

const SEPARATORS: [char; 4] = [' ', '_', '-', '.'];

pub struct ComicBook {
    file_name: String,

    archive: ZipArchive<File>,

    files: Vec<String>,
    offset: usize,
}

// impl ComicBook {
//     pub fn get_images_in_chapter(&mut self) -> Result<Vec<(String, Vec<u8>)>> {
//         let mut images = Vec::new();
//
//         let offset = self.offset * IMAGES_PER_CHAPTER;
//
//         for path in self.files.iter().skip(offset).take(IMAGES_PER_CHAPTER) {
//             let mut buf = Vec::new();
//
//             self.archive.by_name(path.as_str())?.read_to_end(&mut buf)?;
//
//             images.push((path.clone(), buf));
//         }
//
//         Ok(images)
//     }
//
//     fn custom_section_to_html(
//         &mut self,
//         path: &str,
//         _prepend_to_urls: Option<&str>,
//         add_css: Option<&[&str]>,
//     ) -> Result<Vec<u8>> {
//         // path should be "section-{number}.html"
//         let regex = Regex::new(r"section-(\d+)\.html").unwrap();
//
//         if let Some(found) = regex.captures(path) {
//             self.set_chapter(found.get(1).unwrap().as_str().parse().unwrap());
//
//             let images = self.get_images_in_chapter()?;
//
//             Ok(wrap_images_in_html_doc(&images, add_css.unwrap_or_default()).into_bytes())
//         } else {
//             Ok(Vec::new())
//         }
//     }
// }

impl Book for ComicBook {
    fn load_from_path(path: &str) -> Result<Self>
    where
        Self: Sized,
    {
        let archive = ZipArchive::new(File::open(path)?)?;

        let mut files = archive
            .file_names()
            .filter_map(|v| (!v.ends_with('/')).then(|| v.to_string()))
            .collect::<Vec<_>>();

        filename_sort(&mut files);

        Ok(Self {
            archive,
            files,

            file_name: path.rsplit_once('/').unwrap().1.to_string(),
            offset: 0,
        })
    }

    fn compute_hash(&mut self) -> Option<String> {
        let mut hasher = blake3::Hasher::new();

        for path in self.files.clone() {
            if let Ok(asdf) = self.read_path_as_bytes(&path, None, None) {
                hasher.update(&asdf);
            }
        }

        Some(hasher.finalize().to_string())
    }

    fn get_files(&self) -> Vec<String> {
        self.files.clone()
    }

    fn find(&self, _search: BookSearch<'_>) -> Option<Vec<String>> {
        None
    }

    fn get_unique_id(&self) -> Result<Cow<str>> {
        Ok(Cow::Borrowed(self.file_name.as_str()))
    }

    fn get_root_file_dir(&self) -> &Path {
        Path::new("")
    }

    fn get_page_path(&self) -> PathBuf {
        self.files[self.offset].clone().into()
    }

    fn read_page_raw_as_bytes(&mut self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();

        self.archive
            .by_name(&self.files[self.offset])?
            .read_to_end(&mut buf)?;

        Ok(buf)
    }

    fn read_path_as_bytes(
        &mut self,
        path: &str,
        _prepend_to_urls: Option<&str>,
        _add_css: Option<&[&str]>,
    ) -> Result<Vec<u8>> {
        if let Some(file) = self.files.iter().find(|v| v == &path) {
            let mut buf = Vec::new();

            self.archive.by_name(file.as_str())?.read_to_end(&mut buf)?;

            Ok(buf)
        } else {
            Ok(Vec::new())
        }
    }

    fn read_page_as_bytes(
        &mut self,
        _prepend_to_urls: Option<&str>,
        _add_css: Option<&[&str]>,
    ) -> Result<Vec<u8>> {
        let mut buf = Vec::new();

        self.archive
            .by_name(&self.files[self.offset])?
            .read_to_end(&mut buf)?;

        Ok(buf)
    }

    fn chapter_count(&self) -> usize {
        self.files.len()
    }

    fn set_chapter(&mut self, value: usize) -> bool {
        if value < self.chapter_count() {
            self.offset = value;

            true
        } else {
            false
        }
    }

    fn next_chapter(&mut self) -> bool {
        self.set_chapter(self.offset + 1)
    }

    fn previous_chapter(&mut self) -> bool {
        if self.offset != 0 {
            self.set_chapter(self.offset - 1)
        } else {
            false
        }
    }

    fn get_chapter(&self) -> usize {
        self.offset
    }
}

// fn encode_image(name: &str, value: &[u8]) -> String {
//     let b64 = base64::encode(value);

//     if let Some((_, type_of)) = name.rsplit_once('.') {
//         format!("data:image/{type_of};charset=utf-8;base64,{b64}")
//     } else {
//         format!("data:image;charset=utf-8;base64,{b64}")
//     }
// }
