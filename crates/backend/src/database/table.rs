use books_common::{Progression, Person, FileId, util::serialize_datetime};
use chrono::{DateTime, TimeZone, Utc};
use common::{PersonId, MemberId, ThumbnailStore, Source};
use rusqlite::Row;
use serde::Serialize;


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