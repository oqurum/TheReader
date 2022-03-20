// For .mobi and .prc files

// https://wiki.mobileread.com/wiki/MOBI

use std::{path::{PathBuf, Path}, borrow::Cow};

use mobi::Mobi;

use crate::{Result, BookSearch};
use super::Book;

pub struct EpubBook {
	reader: Mobi
}

impl Book for EpubBook {
	fn load_from_path(path: &str) -> Result<Self> where Self: Sized {
		let this = Self {
			reader: Mobi::from_path(path)?
		};

		// Testing

		let records = this.reader.raw_records();
		let records = records.range(this.reader.readable_records_range());

		println!("Records: {}", records.len());

		let rec = &records[0];

		let mut recompiled = Vec::new();

		// TODO: For some reason random u8's are combined. We remove the space and 96 (tilde) from it. For some reason it always equals 96 after subtracting space and the correct char.
		for v in rec.content {
			if *v < 160 {
				recompiled.push(*v);
			} else {
				recompiled.push(32);
				recompiled.push(*v - 128);
			}
		}

		println!("{:?}", &recompiled[0..128]);

		std::fs::write("../../app/testing.html", String::from_utf8_lossy(&recompiled).to_string()).unwrap();

		// std::fs::write("../../app/testing.html", &this.reader.content_as_string_lossy()).unwrap();

		Ok(this)
	}

	fn find(&self, search: BookSearch<'_>) -> Option<Vec<String>> {
		None
	}

	fn get_unique_id(&self) -> Result<Cow<str>> {
		todo!()
	}

	fn get_root_file_dir(&self) -> &Path {
		todo!()
	}

	fn get_page_path(&self) -> PathBuf {
		todo!()
	}

	fn read_page_raw_as_bytes(&mut self) -> Result<Vec<u8>> {
		todo!()
	}

	fn read_path_as_bytes(&mut self, path: &str) -> Result<Vec<u8>> {
		todo!()
	}

	fn read_page_as_bytes(&mut self, prepend_to_urls: Option<&str>, add_css: Option<&[&str]>) -> Result<Vec<u8>> {
		todo!()
	}


	fn chapter_count(&self) -> usize {
		todo!()
	}

	fn set_chapter(&mut self, value: usize) -> bool {
		todo!()
	}

	fn next_chapter(&mut self) -> bool {
		todo!()
	}

	fn previous_chapter(&mut self) -> bool {
		todo!()
	}

	fn get_chapter(&self) -> usize {
		todo!()
	}
}