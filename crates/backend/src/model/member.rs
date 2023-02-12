use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{DateTime, Utc};
use common::MemberId;
use rand::Rng;
use rusqlite::{params, OptionalExtension};

use crate::{DatabaseAccess, Result};
use common_local::{MemberAuthType, Permissions};
use serde::Serialize;

use super::{AdvRow, TableRow};

pub static GUEST_INDEX: AtomicUsize = AtomicUsize::new(1);

pub struct NewMemberModel {
    pub name: String,
    pub email: String,

    pub type_of: MemberAuthType,

    // TODO: pub oqurum_oauth: Option<OqurumOauth>,
    pub permissions: Permissions,
    pub preferences: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl NewMemberModel {
    pub fn new_guest() -> Self {
        let email = format!(
            "guest.account{}{}@oqurum.io",
            rand::thread_rng().gen_range(10_000usize..999_999),
            GUEST_INDEX.fetch_add(1, Ordering::Relaxed),
        );

        let name = if let Some(v) = email.split_once('@').map(|v| v.0) {
            v.to_string()
        } else {
            email.clone()
        };

        Self {
            name,
            email,
            type_of: MemberAuthType::Guest,
            permissions: Permissions::guest(),
            preferences: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn from_email(email: String) -> Self {
        // TODO: unzip once stable
        let name = if let Some(v) = email.split_once('@').map(|v| v.0) {
            v.to_string()
        } else {
            email.clone()
        };

        Self {
            name,
            email,
            type_of: MemberAuthType::Invite,
            permissions: Permissions::basic(),
            preferences: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn into_member(self, id: MemberId) -> MemberModel {
        MemberModel {
            id,
            name: self.name,
            email: self.email,
            password: None,
            type_of: self.type_of,
            permissions: self.permissions,
            preferences: self.preferences,
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
    pub preferences: Option<String>,

    pub created_at: DateTime<Utc>,
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
            preferences: value.preferences,
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
            preferences: row.next_opt()?,

            created_at: row.next()?,
            updated_at: row.next()?,
        })
    }
}

impl NewMemberModel {
    pub async fn insert(self, db: &dyn DatabaseAccess) -> Result<MemberModel> {
        let conn = db.write().await;

        conn.execute(
            r#"
            INSERT INTO members (name, email, type_of, permissions, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
            params![
                &self.name,
                &self.email,
                self.type_of,
                self.permissions,
                self.created_at,
                self.updated_at
            ],
        )?;

        Ok(self.into_member(MemberId::from(conn.last_insert_rowid() as usize)))
    }
}

impl MemberModel {
    /// Converts to email account.
    ///
    /// NOTE: Changes type to Pending Invite.
    pub fn convert_to_email(&mut self, email: String) {
        let name = if let Some(v) = email.split_once('@').map(|v| v.0) {
            v.to_string()
        } else {
            email.clone()
        };

        self.name = name;
        self.email = email;
        self.type_of = MemberAuthType::Invite;
        self.permissions = Permissions::basic();
    }

    pub async fn update(&mut self, db: &dyn DatabaseAccess) -> Result<()> {
        self.updated_at = Utc::now();

        db.write().await.execute(
            r#"
            UPDATE members SET
                name = ?2,
                email = ?3,
                password = ?4,
                type_of = ?5,
                permissions = ?6,
                preferences = ?7,
                updated_at = ?8
            WHERE id = ?1"#,
            params![
                self.id,
                &self.name,
                &self.email,
                &self.password,
                &self.type_of,
                &self.permissions,
                &self.preferences,
                self.updated_at,
            ],
        )?;

        Ok(())
    }

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

    pub async fn accept_invite(
        &mut self,
        login_type: MemberAuthType,
        password: Option<String>,
        db: &dyn DatabaseAccess,
    ) -> Result<usize> {
        if self.type_of != MemberAuthType::Invite {
            return Ok(0);
        }

        self.type_of = login_type;
        self.password = password;
        self.updated_at = Utc::now();

        Ok(db.write().await.execute(
            "UPDATE members SET type_of = ?2, password = ?3, updated_at = ?4 WHERE id = ?1",
            params![
                self.id,
                self.type_of,
                self.password.as_ref(),
                self.updated_at
            ],
        )?)
    }

    pub async fn delete(id: MemberId, db: &dyn DatabaseAccess) -> Result<usize> {
        Ok(db
            .write()
            .await
            .execute("DELETE FROM members WHERE id = ?1", [id])?)
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
