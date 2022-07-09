use books_common::{util::serialize_datetime};
use chrono::{DateTime, TimeZone, Utc};
use common::{PersonId, MemberId};
use rusqlite::Row;
use serde::Serialize;


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