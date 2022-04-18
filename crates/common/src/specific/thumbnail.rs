use serde::{Deserialize, Deserializer, Serialize, Serializer};


pub static MISSING_THUMB_PATH: &str = "/images/missingthumbnail.jpg";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThumbnailStoreType {
	Local,
	Uploaded,
	Metadata,
}

impl ThumbnailStoreType {
	pub fn path_name(self) -> &'static str {
		match self {
			Self::Local => "local",
			Self::Uploaded => "upload",
			Self::Metadata => "meta",
		}
	}

	pub fn prefix_text(&self, value: &str) -> String {
		format!("{}:{}", self.path_name(), value)
	}

	pub fn into_thumb_location(self, value: String) -> ThumbnailStore {
		match self {
			ThumbnailStoreType::Local => ThumbnailStore::Local(value),
			ThumbnailStoreType::Uploaded => ThumbnailStore::Uploaded(value),
			ThumbnailStoreType::Metadata => ThumbnailStore::Metadata(value),
		}
	}
}

impl From<&str> for ThumbnailStoreType {
	fn from(value: &str) -> Self {
		match value {
			"local" => Self::Local,
			"upload" => Self::Uploaded,
			"meta" => Self::Metadata,
			_ => unreachable!("ThumbnailType::from({:?})", value),
		}
	}
}


#[derive(Debug, Clone, PartialEq)]
pub enum ThumbnailStore {
	Local(String),
	Uploaded(String),
	Metadata(String),
	None
}

impl ThumbnailStore {
	pub fn is_none(&self) -> bool {
		matches!(self, Self::None)
	}

	pub fn is_some(&self) -> bool {
		!self.is_none()
	}

	pub fn as_url(&self) -> String {
		match self {
			Self::None => String::from(MISSING_THUMB_PATH),
			_ => format!("/api/image/{}/{}", self.as_type().path_name(), self.as_value()),
		}
	}

	pub fn as_type(&self) -> ThumbnailStoreType {
		match self {
			Self::Local(_) => ThumbnailStoreType::Local,
			Self::Uploaded(_) => ThumbnailStoreType::Uploaded,
			Self::Metadata(_) => ThumbnailStoreType::Metadata,
			_ => unreachable!("Self::as_type()"),
		}
	}

	pub fn as_value(&self) -> &str {
		match self {
			Self::Local(v) |
			Self::Uploaded(v) |
			Self::Metadata(v) => v.as_str(),
			_ => unreachable!("Self::as_value()"),
		}
	}

	pub fn into_value(self) -> String {
		match self {
			Self::Local(v) |
			Self::Uploaded(v) |
			Self::Metadata(v) => v,
			_ => unreachable!("Self::into_value()"),
		}
	}

	pub fn from_type(type_of: ThumbnailStoreType, value: String) -> Self {
		match type_of {
			ThumbnailStoreType::Local => Self::Local(value),
			ThumbnailStoreType::Uploaded => Self::Uploaded(value),
			ThumbnailStoreType::Metadata => Self::Metadata(value),
		}
	}

	pub fn to_optional_string(&self) -> Option<String> {
		if self.is_some() {
			Some(self.to_string())
		} else {
			None
		}
	}
}


impl ToString for ThumbnailStore {
	fn to_string(&self) -> String {
		self.as_type().prefix_text(self.as_value())
	}
}

impl From<&str> for ThumbnailStore {
	fn from(value: &str) -> Self {
		let (prefix, suffix) = value.split_once(':').unwrap();
		ThumbnailStoreType::from(prefix).into_thumb_location(suffix.to_string())
	}
}

impl From<String> for ThumbnailStore {
	fn from(value: String) -> Self {
		let (prefix, suffix) = value.split_once(':').unwrap();
		ThumbnailStoreType::from(prefix).into_thumb_location(suffix.to_string())
	}
}


impl From<Option<String>> for ThumbnailStore {
	fn from(value: Option<String>) -> Self {
		value.map(|v| v.into()).unwrap_or(Self::None)
	}
}

impl From<Option<&str>> for ThumbnailStore {
	fn from(value: Option<&str>) -> Self {
		value.map(|v| v.into()).unwrap_or(Self::None)
	}
}


impl<'de> Deserialize<'de> for ThumbnailStore {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de> {
		Ok(Option::<String>::deserialize(deserializer)?.into())
	}
}

impl Serialize for ThumbnailStore {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer {
		if self.is_some() {
			serializer.serialize_str(&self.to_string())
		} else {
			serializer.serialize_none()
		}
	}
}