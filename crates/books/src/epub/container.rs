
use std::{io::{Read, Seek}, ops::{Deref, DerefMut}};

use serde::{Serialize, Deserialize};

use zip::ZipArchive;

use crate::Result;


pub struct AbsContainer<R: Read + Seek> {
	pub(crate) archive: ZipArchive<R>,

	pub(crate) metainf_container: MetaInfContainer,
	// encryption, manifest, metadata, rights, signature
	// https://www.w3.org/publishing/epub3/epub-ocf.html#sec-container-metainf-encryption.xml
}

impl<R: Read + Seek> AbsContainer<R> {
	pub fn new(mut archive: ZipArchive<R>) -> Result<Self> {
		{ // Ensure mimetype file is "application/epub+zip"
			let mut buf = String::new();
			archive.by_name("mimetype")?.read_to_string(&mut buf)?;

			// Remove U+FEFF (Zero Width No-Break Space)
			if !buf.is_empty() && buf.starts_with('\u{feff}') {
				buf = buf.replace('\u{feff}', "");
			}

			// Added a trim since it could contain a NL
			if buf.trim() != "application/epub+zip" {
				panic!("Invalid file 'mimetype' contents: {:?} expected 'application/epub+zip'", buf)
			}
		}

		// TODO: SerdeXml(Syntax { source: Error { pos: 1:1, kind: Syntax("Unexpected characters outside the root element: \u{feff}") } })
		// Possible spot it's happening at.
		let metainf_container = {
			let file = archive.by_name("META-INF/container.xml")?;
			serde_xml_rs::from_reader(file)?
		};

		Ok(Self {
			archive,
			metainf_container
		})
	}

	pub fn root_files(&self) -> &[RootFile] {
		self.metainf_container.roots.0.as_ref()
	}

	pub fn file_names_in_archive(&self) -> impl Iterator<Item = &str> {
		self.archive.file_names()
	}
}


#[derive(Serialize, Deserialize, Debug)]
pub struct MetaInfContainer {
	pub version: String,
	#[serde(rename = "rootfiles")]
	pub roots: RootfilesVec<RootFile>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RootFile {
	#[serde(rename = "full-path")]
	pub full_path: String,
	#[serde(rename = "media-type")]
	pub media_type: String
}


#[derive(PartialEq, Debug, Serialize)]
pub struct RootfilesVec<T>(Vec<T>);

impl<'de, T: serde::de::Deserialize<'de>> serde::de::Deserialize<'de> for RootfilesVec<T> {
	fn deserialize<D>(deserializer: D) -> std::result::Result<RootfilesVec<T>, D::Error>
	where
		D: serde::de::Deserializer<'de>,
	{
		#[derive(PartialEq, Debug, Serialize, Deserialize)]
		struct Helper<U> {
			rootfile: Vec<U>,
		}

		let h: Helper<T> = serde::de::Deserialize::deserialize(deserializer)?;

		Ok(RootfilesVec(h.rootfile))
	}
}

impl<T> Deref for RootfilesVec<T> {
	type Target = [T];

	fn deref(&self) -> &Self::Target {
		self.0.deref()
	}
}

impl<T> DerefMut for RootfilesVec<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.0.deref_mut()
	}
}
