use std::{io, string::FromUtf8Error};

use thiserror::Error as ThisError;
use zip::result::ZipError;
use serde_xml_rs::Error as SerdeXmlError;
use xml::reader::Error as XmlReaderError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(ThisError, Debug)]
pub enum Error {
	#[error(transparent)]
    Io(#[from] io::Error),

	#[error("Zip Error: {0}")]
	Zip(#[from] ZipError),

	#[error("Serde XML Error: {0}")]
	SerdeXml(#[from] SerdeXmlError),

	#[error("XML Reader Error: {0}")]
	XmlReader(#[from] XmlReaderError),

	#[error("FromUtf8 Error: {0}")]
	FromUtf8(#[from] FromUtf8Error),

	#[error("Missing Value For {0}")]
	MissingValueFor(&'static str)
}