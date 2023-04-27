use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{NaiveDateTime, Utc};
use common::MemberId;
use rand::Rng;
use sqlx::{FromRow, SqliteConnection};

use crate::Result;
use common_local::{LibraryAccess, MemberAuthType, MemberUpdate, Permissions};
use serde::Serialize;

pub static GUEST_INDEX: AtomicUsize = AtomicUsize::new(1);

pub struct NewMemberModel {
    pub name: String,
    pub email: String,

    pub type_of: MemberAuthType,

    // TODO: pub oqurum_oauth: Option<OqurumOauth>,
    pub permissions: Permissions,

    pub library_access: Option<String>,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
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
            library_access: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
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
            library_access: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
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
            library_access: self.library_access,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct MemberModel {
    pub id: MemberId,

    pub name: String,
    pub email: String,
    pub password: Option<String>,

    pub type_of: MemberAuthType,

    pub permissions: Permissions,

    pub library_access: Option<String>,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<MemberModel> for common_local::Member {
    fn from(value: MemberModel) -> common_local::Member {
        common_local::Member {
            id: value.id,
            name: value.name,
            email: value.email,
            type_of: value.type_of,
            permissions: value.permissions,
            library_access: value.library_access,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl NewMemberModel {
    pub async fn insert(self, db: &mut SqliteConnection) -> Result<MemberModel> {
        let res = sqlx::query(
            r#"
                INSERT INTO members (name, email, type_of, permissions, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(&self.name)
        .bind(&self.email)
        .bind(self.type_of)
        .bind(self.permissions)
        .bind(self.created_at)
        .bind(self.updated_at)
        .execute(db)
        .await?;

        Ok(self.into_member(MemberId::from(res.last_insert_rowid())))
    }
}

impl MemberModel {
    pub fn parse_library_access_or_default(&self) -> Result<LibraryAccess> {
        let Some(access) = &self.library_access else {
            return Ok(LibraryAccess::default());
        };

        Ok(serde_json::from_str(access)?)
    }

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

    pub async fn update_with(
        &mut self,
        update: MemberUpdate,
        db: &mut SqliteConnection,
    ) -> Result<()> {
        let mut is_updated = false;

        if let Some(name) = update.name {
            if self.name != name {
                self.name = name;
                is_updated = true;
            }
        }

        if let Some(email) = update.email {
            if self.email != email {
                self.email = email;
                is_updated = true;
            }
        }

        if let Some(type_of) = update.type_of {
            if self.type_of != type_of {
                self.type_of = type_of;
                is_updated = true;
            }
        }

        if let Some(permissions) = update.permissions {
            if self.permissions != permissions {
                self.permissions = permissions;
                is_updated = true;
            }
        }

        // TODO: Update Client Preferences?
        // if let Some(preferences) = update.preferences {
        //     let preferences = Some(serde_json::to_string(&preferences)?);

        //     if self.preferences != preferences {
        //         self.preferences = preferences;
        //         is_updated = true;
        //     }
        // }

        if let Some(library_access) = update.library_access {
            let library_access = Some(serde_json::to_string(&library_access)?);

            if self.library_access != library_access {
                self.library_access = library_access;
                is_updated = true;
            }
        }

        if is_updated {
            // TODO: Replace with own update execution.
            self.update(db).await?;
        }

        Ok(())
    }

    pub async fn update(&mut self, db: &mut SqliteConnection) -> Result<u64> {
        self.updated_at = Utc::now().naive_utc();

        let res = sqlx::query(
            r#"UPDATE members SET
                name = $2,
                email = $3,
                password = $4,
                type_of = $5,
                permissions = $6,
                library_access = $7,
                updated_at = $8
            WHERE id = $1"#,
        )
        .bind(self.id)
        .bind(&self.name)
        .bind(&self.email)
        .bind(&self.password)
        .bind(self.type_of)
        .bind(self.permissions)
        .bind(&self.library_access)
        .bind(self.updated_at)
        .execute(db)
        .await?;

        Ok(res.rows_affected())
    }

    pub async fn find_one_by_email(value: &str, db: &mut SqliteConnection) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT id, name, email, password, type_of, permissions, library_access, created_at, updated_at FROM members WHERE email = $1"
        ).bind(value).fetch_optional(db).await?)
    }

    pub async fn find_one_by_id(id: MemberId, db: &mut SqliteConnection) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT id, name, email, password, type_of, permissions, library_access, created_at, updated_at FROM members WHERE id = $1"
        ).bind(id).fetch_optional(db).await?)
    }

    pub async fn accept_invite(
        &mut self,
        login_type: MemberAuthType,
        password: Option<String>,
        db: &mut SqliteConnection,
    ) -> Result<u64> {
        if self.type_of != MemberAuthType::Invite {
            return Ok(0);
        }

        self.type_of = login_type;
        self.password = password;
        self.updated_at = Utc::now().naive_utc();

        let res = sqlx::query(
            "UPDATE members SET type_of = $2, password = $3, updated_at = $4 WHERE id = $1",
        )
        .bind(self.id)
        .bind(self.type_of)
        .bind(&self.password)
        .bind(self.updated_at)
        .execute(db)
        .await?;

        Ok(res.rows_affected())
    }

    pub async fn delete(id: MemberId, db: &mut SqliteConnection) -> Result<u64> {
        let res = sqlx::query("DELETE FROM members WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;

        Ok(res.rows_affected())
    }

    pub async fn get_all(db: &mut SqliteConnection) -> Result<Vec<Self>> {
        Ok(sqlx::query_as("SELECT id, name, email, password, type_of, permissions, library_access, created_at, updated_at FROM members").fetch_all(db).await?)
    }

    pub async fn count(db: &mut SqliteConnection) -> Result<i32> {
        Ok(sqlx::query_scalar("SELECT COUNT(*) FROM members")
            .fetch_one(db)
            .await?)
    }
}
