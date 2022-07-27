use std::fmt::Write;
use std::{num::ParseIntError, sync::PoisonError};
use std::io::Error as IoError;
use std::time::SystemTimeError;

use books_common::api::{WrappingResponse, ApiErrorResponse};
use thiserror::Error as ThisError;

use bcrypt::BcryptError;
use common::error::Error as CommonError;
use rusqlite::Error as RusqliteError;
use lettre::error::Error as LettreError;
use lettre::address::AddressError;
use lettre::transport::smtp::Error as SmtpError;
use image::ImageError;
use reqwest::Error as HttpError;
use serde_urlencoded::ser::Error as UrlEncodedSerError;
use serde_json::Error as JsonError;
use serde_xml_rs::Error as XmlError;
use serde::de::value::Error as SerdeValueError;
use toml_edit::{de::Error as TomlDeError, ser::Error as TomlSerError};
use books_common::Error as LocalCommonError;
use bookie::Error as BookieError;

use actix_multipart::MultipartError;
use actix_web::Error as ActixError;
use actix_web::error::PayloadError;
use actix_web::ResponseError;

pub type Result<T> = std::result::Result<T, Error>;
pub type WebResult<T> = std::result::Result<T, WebError>;

// Used specifically for Actix Errors since Actix cannot be used between threads.

#[derive(Debug, ThisError)]
pub enum WebError {
	#[error("ActixWeb Error: {0}")]
	Actix(#[from] ActixError),
	#[error("Multipart Error: {0}")]
	Multipart(#[from] MultipartError),
	#[error("Payload Error: {0}")]
	Payload(#[from] PayloadError),

	#[error(transparent)]
	All(#[from] Error),

	#[error(transparent)]
	Common(#[from] CommonError),

	#[error(transparent)]
	LocalCommon(#[from] LocalCommonError),

	#[error(transparent)]
	Bookie(#[from] BookieError),

	#[error(transparent)]
	ApiResponse(#[from] ApiErrorResponse),
}


impl ResponseError for WebError {
	fn error_response(&self) -> actix_web::HttpResponse<actix_web::body::BoxBody> {
		let resp_value = match self {
			Self::ApiResponse(r) => WrappingResponse::<()> {
				resp: None,
				error: Some(r.clone())
			},

			this => {
				let mut description = String::new();
				let _ = write!(&mut description, "{}", this);
				WrappingResponse::<()>::error(description)
			},
		};

		let mut res = actix_web::HttpResponse::new(self.status_code());

		res.headers_mut().insert(
			actix_web::http::header::CONTENT_TYPE,
			actix_web::http::header::HeaderValue::from_static("text/plain; charset=utf-8")
		);

		res.set_body(actix_web::body::BoxBody::new(
			serde_json::to_string(&resp_value).unwrap()
		))
	}
}



// Used for all Errors in Application.
#[derive(Debug, ThisError)]
pub enum Error {
	#[error("Internal Error: {0}")]
	Internal(#[from] InternalError),

	#[error("Poison Error")]
	Poisoned,

	#[error("Json Error: {0}")]
	Json(#[from] JsonError),
	#[error("XML Error: {0}")]
	Xml(#[from] XmlError),
	#[error("Serde Value Error: {0}")]
	SerdeValue(#[from] SerdeValueError),
	#[error("Url Encoded Ser Error: {0}")]
	UrlEncodedSer(#[from] UrlEncodedSerError),
	#[error("TOML Deserialize Error: {0}")]
	TomlDeValue(#[from] TomlDeError),
	#[error("TOML Serialize Error: {0}")]
	TomlSerValue(#[from] TomlSerError),

	#[error("IO Error: {0}")]
	Io(#[from] IoError),
	#[error("SystemTime Error: {0}")]
	SystemTime(#[from] SystemTimeError),
	#[error("HTTP Error: {0}")]
	Http(#[from] HttpError),
	#[error("Parse Int Error: {0}")]
	ParseInt(#[from] ParseIntError),

	#[error("Image Error: {0}")]
	Image(#[from] ImageError),
	#[error("Lettre Error: {0}")]
	Lettre(#[from] LettreError),
	#[error("SMTP Error: {0}")]
	Smtp(#[from] SmtpError),
	#[error("Address Error: {0}")]
	Address(#[from] AddressError),
	#[error("Rusqlite Error: {0}")]
	Rusqlite(#[from] RusqliteError),
	#[error("Bcrypt Error: {0}")]
	Bcrypt(#[from] BcryptError),

	#[error(transparent)]
	Common(#[from] CommonError),
	#[error(transparent)]
	Bookie(#[from] BookieError),
}

#[derive(Debug, ThisError)]
pub enum InternalError {
	// Actix

	#[error("The user does not exist")]
	UserMissing,
}

impl<V> From<PoisonError<V>> for Error {
	fn from(_: PoisonError<V>) -> Self {
		Self::Poisoned
	}
}