use chrono::{DateTime, Utc};
use common::MemberId;
use rusqlite::{params, OptionalExtension};

use crate::{DatabaseAccess, Result};
use common_local::{util::serialize_datetime, MemberAuthType, Permissions};
use serde::Serialize;

use super::{AdvRow, TableRow};

pub struct NewMemberModel {
    pub name: String,
    pub email: String,

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
            password: None,
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
    pub email: String,
    pub password: Option<String>,

    pub type_of: MemberAuthType,

    pub permissions: Permissions,

    #[serde(serialize_with = "serialize_datetime")]
    pub created_at: DateTime<Utc>,

    #[serde(serialize_with = "serialize_datetime")]
    pub updated_at: DateTime<Utc>,
}

impl From<MemberModel> for common_local::Member {
    fn from(value: MemberModel) -> common_local::Member {
        common_local::Member {
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

            created_at: row.next()?,
            updated_at: row.next()?,
        })
    }
}

impl NewMemberModel {
    pub async fn insert(self, db: &dyn DatabaseAccess) -> Result<MemberModel> {
        let conn = db.write().await;

        conn.execute(r#"
            INSERT INTO members (name, email, type_of, permissions, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            &self.name, &self.email,
            self.type_of, self.permissions,
            self.created_at, self.updated_at
        ])?;

        Ok(self.into_member(MemberId::from(conn.last_insert_rowid() as usize)))
    }
}

impl MemberModel {
    pub async fn find_one_by_email(value: &str, db: &dyn DatabaseAccess) -> Result<Option<Self>> {
        Ok(db
            .read()
            .await
            .query_row(
                r#"SELECT * FROM members WHERE email = ?1"#,
                params![value],
                |v| Self::from_row(v),
            )
            .optional()?)
    }

    pub async fn find_one_by_id(id: MemberId, db: &dyn DatabaseAccess) -> Result<Option<Self>> {
        Ok(db
            .read()
            .await
            .query_row(r#"SELECT * FROM members WHERE id = ?1"#, params![id], |v| {
                Self::from_row(v)
            })
            .optional()?)
    }

    pub async fn accept_invite(&mut self, login_type: MemberAuthType, password: Option<String>, db: &dyn DatabaseAccess) -> Result<usize> {
        if self.type_of != MemberAuthType::Invite {
            return Ok(0);
        }

        self.type_of = login_type;
        self.password = password;
        self.updated_at = Utc::now();

        Ok(db.write().await.execute(
            "UPDATE members SET type_of = ?2, password = ?3, updated_at = ?4 WHERE id = ?1",
            params![ self.id, self.type_of, self.password.as_ref(), self.updated_at ]
        )?)
    }

    pub async fn delete(id: MemberId, db: &dyn DatabaseAccess) -> Result<usize> {
        Ok(db
            .write()
            .await
            .execute("DELETE FROM members WHERE id = ?1", [id])?
        )
    }

    pub async fn get_all(db: &dyn DatabaseAccess) -> Result<Vec<Self>> {
        let read = db.read().await;

        let mut stmt = read.prepare("SELECT * FROM members")?;

        let map = stmt.query_map([], |v| Self::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn count(db: &dyn DatabaseAccess) -> Result<usize> {
        Ok(db
            .read()
            .await
            .query_row(r#"SELECT COUNT(*) FROM members"#, [], |v| v.get(0))?)
    }
}
