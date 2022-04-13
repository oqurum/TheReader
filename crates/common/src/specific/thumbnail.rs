use std::ops::Deref;

use serde::{Deserialize, Deserializer, Serialize, Serializer};


pub static MISSING_THUMB_PATH: &str = "/images/missingthumbnail.jpg";


#[derive(Debug, Clone, Default, PartialEq)]
pub struct ThumbnailPath(pub Option<String>);

impl ThumbnailPath {
	pub fn get_prefix_suffix(&self) -> Option<(&str, &str)> {
		self.0.as_ref().and_then(|v| v.split_once(':'))
	}

	pub fn is_url(&self) -> bool {
		self.0.as_ref().map(|v| v.contains('.')).unwrap_or_default()
	}

	pub fn into_url_thumb(self, meta_id: i64) -> String {
		if self.is_url() {
			self.0.unwrap()
		} else if self.is_some() {
			format!("/api/metadata/{meta_id}/thumbnail")
		} else {
			String::from(MISSING_THUMB_PATH)
		}
	}

	// TODO: Deref, Ref error.
}

impl ToString for ThumbnailPath {
    fn to_string(&self) -> String {
        self.0.clone().unwrap_or_default()
    }
}

impl From<&str> for ThumbnailPath {
	fn from(value: &str) -> Self {
		Self(Some(value.to_owned()))
	}
}

impl From<String> for ThumbnailPath {
	fn from(value: String) -> Self {
		Self(Some(value))
	}
}


impl From<Option<String>> for ThumbnailPath {
	fn from(value: Option<String>) -> Self {
		Self(value)
	}
}

impl From<Option<&str>> for ThumbnailPath {
	fn from(value: Option<&str>) -> Self {
		Self(value.map(|v| v.to_owned()))
	}
}

impl AsRef<Option<String>> for ThumbnailPath {
	fn as_ref(&self) -> &Option<String> {
		&self.0
	}
}

impl Deref for ThumbnailPath {
	type Target = Option<String>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}


impl<'de> Deserialize<'de> for ThumbnailPath {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de> {
		Ok(Self(Option::<String>::deserialize(deserializer)?))
	}
}

impl Serialize for ThumbnailPath {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer {
		if let Some(v) = self.0.as_deref() {
			serializer.serialize_str(v)
		} else {
			serializer.serialize_none()
		}
	}
}


