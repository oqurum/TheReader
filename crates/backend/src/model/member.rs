use chrono::{DateTime, Utc, TimeZone};
use common::MemberId;
use rusqlite::{params, OptionalExtension};

use books_common::{util::serialize_datetime, Permissions, MemberAuthType};
use serde::Serialize;
use crate::{Result, database::Database};

use super::{TableRow, AdvRow};



pub struct NewMemberModel {
	pub name: String,
	pub email: Option<String>,
	pub password: Option<String>,

	pub type_of: MemberAuthType,

	// TODO: pub oqurum_oauth: Option<OqurumOauth>,

	pub permissions: Permissions,

	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl NewMemberModel {
	pub fn into_member(self, id: MemberId) -> MemberModel {
		MemberModel {
			id,
			name: self.name,
			email: self.email,
			password: self.password,
			type_of: self.type_of,
			permissions: self.permissions,
			created_at: self.created_at,
			updated_at: self.updated_at,
		}
	}
}

#[derive(Debug, Clone, Serialize)]
pub struct MemberModel {
	pub id: MemberId,

	pub name: String,
	pub email: Option<String>,
	pub password: Option<String>,

	pub type_of: MemberAuthType,

	pub permissions: Permissions,

	#[serde(serialize_with = "serialize_datetime")]
	pub created_at: DateTime<Utc>,

	#[serde(serialize_with = "serialize_datetime")]
	pub updated_at: DateTime<Utc>,
}


impl From<MemberModel> for books_common::Member {
	fn from(value: MemberModel) -> books_common::Member {
		books_common::Member {
			id: value.id,
			name: value.name,
			email: value.email,
			type_of: value.type_of,
			permissions: value.permissions,
			created_at: value.created_at,
			updated_at: value.updated_at,
		}
	}
}



impl TableRow<'_> for MemberModel {
	fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
		Ok(Self {
			id: row.next()?,
			name: row.next()?,
			email: row.next()?,
			password: row.next()?,
			type_of: row.next()?,
			permissions: row.next()?,

			created_at: Utc.timestamp_millis(row.next()?),
			updated_at: Utc.timestamp_millis(row.next()?),
		})
	}
}


impl NewMemberModel {
    pub async fn insert(self, db: &Database) -> Result<MemberModel> {
		let conn = db.write().await;

		conn.execute(r#"
			INSERT INTO members (name, email, password, type_of, permissions, created_at, updated_at)
			VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
		"#,
		params![
			&self.name, self.email.as_ref(), self.password.as_ref(), self.type_of, self.permissions,
			self.created_at.timestamp_millis(), self.updated_at.timestamp_millis()
		])?;

		Ok(self.into_member(MemberId::from(conn.last_insert_rowid() as usize)))
	}
}


impl MemberModel {
	pub async fn find_one_by_email(value: &str, db: &Database) -> Result<Option<Self>> {
		Ok(db.read().await.query_row(
			r#"SELECT * FROM members WHERE email = ?1 LIMIT 1"#,
			params![value],
			|v| Self::from_row(v)
		).optional()?)
	}

	pub async fn find_one_by_id(id: MemberId, db: &Database) -> Result<Option<Self>> {
		Ok(db.read().await.query_row(
			r#"SELECT * FROM members WHERE id = ?1 LIMIT 1"#,
			params![id],
			|v| Self::from_row(v)
		).optional()?)
	}

	pub async fn count(db: &Database) -> Result<usize> {
		Ok(db.read().await.query_row(
			r#"SELECT COUNT(*) FROM members"#,
			[],
			|v| v.get(0)
		)?)
	}
}