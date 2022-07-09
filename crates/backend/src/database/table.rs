use books_common::{Progression, MetadataItemCached, MediaItem, Person, FileId, MetadataId, LibraryId, util::serialize_datetime};
use chrono::{DateTime, TimeZone, Utc};
use common::{PersonId, MemberId, ThumbnailStore, Source};
use rusqlite::Row;
use serde::Serialize;

use crate::model::metadata::MetadataModel;


// Tag Person Alt

#[derive(Debug, Serialize)]
pub struct MetadataPerson {
	pub metadata_id: MetadataId,
	pub person_id: PersonId,
}

impl<'a> TryFrom<&Row<'a>> for MetadataPerson {
	type Error = rusqlite::Error;

	fn try_from(value: &Row<'a>) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			metadata_id: value.get(0)?,
			person_id: value.get(1)?,
		})
	}
}


// Notes

#[derive(Debug, Serialize)]
pub struct FileNote {
	pub file_id: FileId,
	pub user_id: MemberId,

	pub data: String,
	pub data_size: i64,

	#[serde(serialize_with = "serialize_datetime")]
	pub updated_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub created_at: DateTime<Utc>,
}

impl FileNote {
	pub fn new(file_id: FileId, user_id: MemberId, data: String) -> Self {
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
	pub file_id: FileId,
	pub user_id: MemberId,

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
	pub fn new(progress: Progression, user_id: MemberId, file_id: FileId) -> Self {
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


// File

pub struct NewFile {
	pub path: String,

	pub file_name: String,
	pub file_type: String,
	pub file_size: i64,

	pub library_id: LibraryId,
	pub metadata_id: Option<MetadataId>,
	pub chapter_count: i64,

	pub identifier: Option<String>,

	pub modified_at: DateTime<Utc>,
	pub accessed_at: DateTime<Utc>,
	pub created_at: DateTime<Utc>,
}

impl NewFile {
	pub fn into_file(self, id: FileId) -> File {
		File {
			id,
			path: self.path,
			file_name: self.file_name,
			file_type: self.file_type,
			file_size: self.file_size,
			library_id: self.library_id,
			metadata_id: self.metadata_id,
			chapter_count: self.chapter_count,
			identifier: self.identifier,
			modified_at: self.modified_at,
			accessed_at: self.accessed_at,
			created_at: self.created_at,
		}
	}
}


#[derive(Debug, Serialize)]
pub struct File {
	pub id: FileId,

	pub path: String,

	pub file_name: String,
	pub file_type: String,
	pub file_size: i64,

	pub library_id: LibraryId,
	pub metadata_id: Option<MetadataId>,
	pub chapter_count: i64,

	pub identifier: Option<String>,

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

			identifier: value.get(8)?,

			modified_at: Utc.timestamp_millis(value.get(9)?),
			accessed_at: Utc.timestamp_millis(value.get(10)?),
			created_at: Utc.timestamp_millis(value.get(11)?),
		})
	}
}

impl From<File> for MediaItem {
    fn from(file: File) -> Self {
        Self {
            id: file.id,

			path: file.path,

            file_name: file.file_name,
            file_type: file.file_type,
            file_size: file.file_size,

			library_id: file.library_id,
			metadata_id: file.metadata_id,
			chapter_count: file.chapter_count as usize,

			identifier: file.identifier,

            modified_at: file.modified_at.timestamp_millis(),
            accessed_at: file.accessed_at.timestamp_millis(),
            created_at: file.created_at.timestamp_millis(),
        }
    }
}


// Tags People

#[derive(Debug)]
pub struct NewTagPerson {
	pub source: Source,

	pub name: String,
	pub description: Option<String>,
	pub birth_date: Option<String>,

	pub thumb_url: ThumbnailStore,

	pub updated_at: DateTime<Utc>,
	pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TagPerson {
	pub id: PersonId,

	pub source: Source,

	pub name: String,
	pub description: Option<String>,
	pub birth_date: Option<String>,

	pub thumb_url: ThumbnailStore,

	#[serde(serialize_with = "serialize_datetime")]
	pub updated_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub created_at: DateTime<Utc>,
}

impl<'a> TryFrom<&Row<'a>> for TagPerson {
	type Error = rusqlite::Error;

	fn try_from(value: &Row<'a>) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			id: value.get(0)?,

			source: Source::try_from(value.get::<_, String>(1)?).unwrap(),

			name: value.get(2)?,
			description: value.get(3)?,
			birth_date: value.get(4)?,

			thumb_url: ThumbnailStore::from(value.get::<_, Option<String>>(5)?),

			created_at: Utc.timestamp_millis(value.get(6)?),
			updated_at: Utc.timestamp_millis(value.get(7)?),
		})
	}
}

impl From<TagPerson> for Person {
	fn from(val: TagPerson) -> Self {
		Person {
			id: val.id,
			source: val.source,
			name: val.name,
			description: val.description,
			birth_date: val.birth_date,
			thumb_url: val.thumb_url,
			updated_at: val.updated_at,
			created_at: val.created_at,
		}
	}
}


// Tag Person Alt

#[derive(Debug, Serialize)]
pub struct TagPersonAlt {
	pub person_id: PersonId,
	pub name: String,
}

impl<'a> TryFrom<&Row<'a>> for TagPersonAlt {
	type Error = rusqlite::Error;

	fn try_from(value: &Row<'a>) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			person_id: value.get(0)?,
			name: value.get(1)?,
		})
	}
}


// User

// TODO: type_of 0 = web page, 1 = local passwordless 2 = local password
// TODO: Enum.
pub struct NewMember {
	pub name: String,
	pub email: Option<String>,
	pub password: Option<String>,

	pub type_of: u8,

	// TODO
	pub config: Option<String>,

	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl NewMember {
	pub fn into_member(self, id: MemberId) -> Member {
		Member {
			id,
			name: self.name,
			email: self.email,
			password: self.password,
			type_of: self.type_of,
			config: self.config,
			created_at: self.created_at,
			updated_at: self.updated_at,
		}
	}
}

#[derive(Debug, Clone, Serialize)]
pub struct Member {
	pub id: MemberId,

	pub name: String,
	pub email: Option<String>,
	pub password: Option<String>,

	pub type_of: u8,

	// TODO
	pub config: Option<String>,

	#[serde(serialize_with = "serialize_datetime")]
	pub created_at: DateTime<Utc>,

	#[serde(serialize_with = "serialize_datetime")]
	pub updated_at: DateTime<Utc>,
}

impl<'a> TryFrom<&Row<'a>> for Member {
	type Error = rusqlite::Error;

	fn try_from(value: &Row<'a>) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			id: value.get(0)?,
			name: value.get(1)?,
			email: value.get(2)?,
			password: value.get(3)?,
			type_of: value.get(4)?,
			config: value.get(5)?,
			created_at: Utc.timestamp_millis(value.get(6)?),
			updated_at: Utc.timestamp_millis(value.get(7)?),
		})
	}
}

impl From<Member> for books_common::Member {
	fn from(value: Member) -> books_common::Member {
		books_common::Member {
			id: value.id,
			name: value.name,
			email: value.email,
			type_of: value.type_of,
			config: value.config,
			created_at: value.created_at,
			updated_at: value.updated_at,
		}
	}
}

// Auth

pub struct NewAuth {
	pub oauth_token: String,
	pub oauth_token_secret: String,
	pub created_at: DateTime<Utc>,
}


// Non Table Items

pub struct FileWithMetadata {
	pub file: File,
	pub meta: Option<MetadataModel>
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

				identifier: value.get(8)?,

				modified_at: Utc.timestamp_millis(value.get(9)?),
				accessed_at: Utc.timestamp_millis(value.get(10)?),
				created_at: Utc.timestamp_millis(value.get(11)?),
			},

			meta: value.get(11)
				.ok()
				.map(|_: i64| std::result::Result::<_, Self::Error>::Ok(MetadataModel {
					id: value.get(11)?,
					library_id: value.get(12)?,
					source: Source::try_from(value.get::<_, String>(13)?).unwrap(),
					file_item_count: value.get(14)?,
					title: value.get(15)?,
					original_title: value.get(16)?,
					description: value.get(17)?,
					rating: value.get(18)?,
					thumb_path: ThumbnailStore::from(value.get::<_, Option<String>>(19)?),
					all_thumb_urls: Vec::new(),
					cached: value.get::<_, Option<String>>(20)?
						.map(|v| MetadataItemCached::from_string(&v))
						.unwrap_or_default(),
					available_at: value.get(21)?,
					year: value.get(22)?,
					refreshed_at: Utc.timestamp_millis(value.get(23)?),
					created_at: Utc.timestamp_millis(value.get(24)?),
					updated_at: Utc.timestamp_millis(value.get(25)?),
					deleted_at: value.get::<_, Option<_>>(26)?.map(|v| Utc.timestamp_millis(v)),
					hash: value.get(27)?
				}))
				.transpose()?
		})
	}
}