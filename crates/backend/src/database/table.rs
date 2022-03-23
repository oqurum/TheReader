use books_common::Progression;
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Row;
use serde::{Serialize, Serializer};


// Metadata

#[derive(Debug, Clone, Serialize)]
pub struct MetadataItem {
	pub id: i64,

	pub source: String,
	pub file_item_count: i64,
	pub title: Option<String>,
	pub original_title: Option<String>,
	pub description: Option<String>,
	pub rating: f64,
	pub thumb_url: Option<String>,

	pub creator: Option<String>,
	pub publisher: Option<String>,

	pub tags_genre: Option<String>,
	pub tags_collection: Option<String>,
	pub tags_author: Option<String>,
	pub tags_country: Option<String>,

	#[serde(serialize_with = "serialize_datetime")]
	pub refreshed_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub created_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub updated_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime_opt")]
	pub deleted_at: Option<DateTime<Utc>>,

	pub available_at: Option<i64>,
	pub year: Option<i64>,

	pub hash: String
}

impl<'a> TryFrom<&Row<'a>> for MetadataItem {
	type Error = rusqlite::Error;

	fn try_from(value: &Row<'a>) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			id: value.get(0)?,
			source: value.get(1)?,
			file_item_count: value.get(2)?,
			title: value.get(3)?,
			original_title: value.get(4)?,
			description: value.get(5)?,
			rating: value.get(6)?,
			thumb_url: value.get(7)?,
			creator: value.get(8)?,
			publisher: value.get(9)?,
			tags_genre: value.get(10)?,
			tags_collection: value.get(11)?,
			tags_author: value.get(12)?,
			tags_country: value.get(13)?,
			available_at: value.get(14)?,
			year: value.get(15)?,
			refreshed_at: Utc.timestamp_millis(value.get(16)?),
			created_at: Utc.timestamp_millis(value.get(17)?),
			updated_at: Utc.timestamp_millis(value.get(18)?),
			deleted_at: value.get::<_, Option<_>>(19)?.map(|v| Utc.timestamp_millis(v)),
			hash: value.get(20)?
		})
	}
}


// Notes

#[derive(Debug, Serialize)]
pub struct FileNote {
	pub file_id: i64,
	pub user_id: i64,

	pub data: String,
	pub data_size: i64,

	#[serde(serialize_with = "serialize_datetime")]
	pub updated_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub created_at: DateTime<Utc>,
}

impl FileNote {
	pub fn new(file_id: i64, user_id: i64, data: String) -> Self {
		Self {
			file_id,
			user_id,
			data_size: data.len() as i64,
			data,
			updated_at: Utc::now(),
			created_at: Utc::now(),
		}
	}
}


impl<'a> TryFrom<&Row<'a>> for FileNote {
	type Error = rusqlite::Error;

	fn try_from(value: &Row<'a>) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			file_id: value.get(0)?,
			user_id: value.get(1)?,

			data: value.get(2)?,

			data_size: value.get(3)?,

			updated_at: Utc.timestamp_millis(value.get(4)?),
			created_at: Utc.timestamp_millis(value.get(5)?),
		})
	}
}

// File Progression

#[derive(Debug, Serialize)]
pub struct FileProgression {
	pub file_id: i64,
	pub user_id: i64,

	pub type_of: u8,

	// Ebook/Audiobook
	pub chapter: Option<i64>,

	// Ebook
	pub page: Option<i64>, // TODO: Remove page. Change to byte pos. Most accurate since screen sizes can change.
	pub char_pos: Option<i64>,

	// Audiobook
	pub seek_pos: Option<i64>,

	#[serde(serialize_with = "serialize_datetime")]
	pub updated_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub created_at: DateTime<Utc>,
}

impl FileProgression {
	pub fn new(progress: Progression, user_id: i64, file_id: i64) -> Self {
		match progress {
			Progression::Complete => Self {
				file_id,
				user_id,
				type_of: 0,
				chapter: None,
				page: None,
				char_pos: None,
				seek_pos: None,
				updated_at: Utc::now(),
				created_at: Utc::now(),
			},

			Progression::Ebook { chapter, page, char_pos } => Self {
				file_id,
				user_id,
				type_of: 1,
				char_pos: Some(char_pos),
				chapter: Some(chapter),
				page: Some(page),
				seek_pos: None,
				updated_at: Utc::now(),
				created_at: Utc::now(),
			},

			Progression::AudioBook { chapter, seek_pos } => Self {
				file_id,
				user_id,
				type_of: 2,
				chapter: Some(chapter),
				page: None,
				char_pos: None,
				seek_pos: Some(seek_pos),
				updated_at: Utc::now(),
				created_at: Utc::now(),
			}
		}
	}
}

impl<'a> TryFrom<&Row<'a>> for FileProgression {
	type Error = rusqlite::Error;

	fn try_from(value: &Row<'a>) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			file_id: value.get(0)?,
			user_id: value.get(1)?,

			type_of: value.get(2)?,

			chapter: value.get(3)?,

			page: value.get(4)?,
			char_pos: value.get(5)?,

			seek_pos: value.get(6)?,

			updated_at: Utc.timestamp_millis(value.get(7)?),
			created_at: Utc.timestamp_millis(value.get(8)?),
		})
	}
}

impl From<FileProgression> for Progression {
    fn from(val: FileProgression) -> Self {
        match val.type_of {
			0 => Progression::Complete,

			1 => Progression::Ebook {
				char_pos: val.char_pos.unwrap(),
				chapter: val.chapter.unwrap(),
				page: val.page.unwrap(),
			},

			2 => Progression::AudioBook {
				chapter: val.chapter.unwrap(),
				seek_pos: val.seek_pos.unwrap(),
			},

			_ => unreachable!()
		}
    }
}


// Library

pub struct NewLibrary {
	pub name: String,
	pub type_of: String,

	pub scanned_at: DateTime<Utc>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct Library {
	pub id: i64,

	pub name: String,
	pub type_of: String,

	#[serde(serialize_with = "serialize_datetime")]
	pub scanned_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub created_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub updated_at: DateTime<Utc>,
}

impl<'a> TryFrom<&Row<'a>> for Library {
	type Error = rusqlite::Error;

	fn try_from(value: &Row<'a>) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			id: value.get(0)?,
			name: value.get(1)?,
			type_of: value.get(2)?,
			scanned_at: Utc.timestamp_millis(value.get(3)?),
			created_at: Utc.timestamp_millis(value.get(4)?),
			updated_at: Utc.timestamp_millis(value.get(5)?),
		})
	}
}


// Directory

pub struct Directory {
	pub library_id: i64,
	pub path: String,
}

impl<'a> TryFrom<&Row<'a>> for Directory {
	type Error = rusqlite::Error;

	fn try_from(value: &Row<'a>) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			library_id: value.get(0)?,
			path: value.get(1)?,
		})
	}
}


// File

pub struct NewFile {
	pub path: String,

	pub file_name: String,
	pub file_type: String,
	pub file_size: i64,

	pub library_id: i64,
	pub metadata_id: Option<i64>,
	pub chapter_count: i64,

	pub modified_at: DateTime<Utc>,
	pub accessed_at: DateTime<Utc>,
	pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct File {
	pub id: i64,

	pub path: String,

	pub file_name: String,
	pub file_type: String,
	pub file_size: i64,

	pub library_id: i64,
	pub metadata_id: Option<i64>,
	pub chapter_count: i64,

	#[serde(serialize_with = "serialize_datetime")]
	pub modified_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub accessed_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub created_at: DateTime<Utc>,
}

impl<'a> TryFrom<&Row<'a>> for File {
	type Error = rusqlite::Error;

	fn try_from(value: &Row<'a>) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			id: value.get(0)?,

			path: value.get(1)?,

			file_name: value.get(2)?,
			file_type: value.get(3)?,
			file_size: value.get(4)?,

			library_id: value.get(5)?,
			metadata_id: value.get(6)?,
			chapter_count: value.get(7)?,

			modified_at: Utc.timestamp_millis(value.get(8)?),
			accessed_at: Utc.timestamp_millis(value.get(9)?),
			created_at: Utc.timestamp_millis(value.get(10)?),
		})
	}
}


fn serialize_datetime<S>(value: &DateTime<Utc>, s: S) -> std::result::Result<S::Ok, S::Error> where S: Serializer {
	s.serialize_i64(value.timestamp_millis())
}

fn serialize_datetime_opt<S>(value: &Option<DateTime<Utc>>, s: S) -> std::result::Result<S::Ok, S::Error> where S: Serializer {
	match value {
		Some(v) => s.serialize_i64(v.timestamp_millis()),
		None => s.serialize_none()
	}
}




// Non Table Items


pub struct FileWithMetadata {
	pub file: File,
	pub meta: Option<MetadataItem>
}

impl<'a> TryFrom<&Row<'a>> for FileWithMetadata {
	type Error = rusqlite::Error;

	fn try_from(value: &Row<'a>) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			file: File {
				id: value.get(0)?,

				path: value.get(1)?,

				file_name: value.get(2)?,
				file_type: value.get(3)?,
				file_size: value.get(4)?,

				library_id: value.get(5)?,
				metadata_id: value.get(6)?,
				chapter_count: value.get(7)?,

				modified_at: Utc.timestamp_millis(value.get(8)?),
				accessed_at: Utc.timestamp_millis(value.get(9)?),
				created_at: Utc.timestamp_millis(value.get(10)?),
			},

			meta: value.get(11)
				.ok()
				.map(|_: i64| std::result::Result::<_, Self::Error>::Ok(MetadataItem {
					id: value.get(11)?,
					source: value.get(12)?,
					file_item_count: value.get(13)?,
					title: value.get(14)?,
					original_title: value.get(15)?,
					description: value.get(16)?,
					rating: value.get(17)?,
					thumb_url: value.get(18)?,
					creator: value.get(19)?,
					publisher: value.get(20)?,
					tags_genre: value.get(21)?,
					tags_collection: value.get(22)?,
					tags_author: value.get(23)?,
					tags_country: value.get(24)?,
					available_at: value.get(25)?,
					year: value.get(26)?,
					refreshed_at: Utc.timestamp_millis(value.get(27)?),
					created_at: Utc.timestamp_millis(value.get(28)?),
					updated_at: Utc.timestamp_millis(value.get(29)?),
					deleted_at: value.get::<_, Option<_>>(30)?.map(|v| Utc.timestamp_millis(v)),
					hash: value.get(31)?
				}))
				.transpose()?
		})
	}
}