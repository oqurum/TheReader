use chrono::{DateTime, Utc};
use common::MemberId;
use rusqlite::{params, OptionalExtension};

use crate::{http::gen_sample_alphanumeric, DatabaseAccess, Result};

use super::{AdvRow, TableRow};

/// Used for Pending Authentications and Completed Authentications when member_id is defined.
///
/// It is referenced whenever
pub struct AuthModel {
    pub oauth_token: Option<String>,
    /// Stored in Cookie cache after successful login.
    pub oauth_token_secret: String,

    pub member_id: Option<MemberId>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TableRow<'_> for AuthModel {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            oauth_token: row.next_opt()?,
            oauth_token_secret: row.next()?,
            member_id: row.next_opt()?,
            created_at: row.next()?,
            updated_at: row.next()?,
        })
    }
}

impl AuthModel {
    pub fn new(member_id: Option<MemberId>) -> Self {
        let mut rng = rand::thread_rng();

        Self {
            member_id,
            oauth_token: Some(gen_sample_alphanumeric(32, &mut rng))
                .filter(|_| member_id.is_none()),
            oauth_token_secret: gen_sample_alphanumeric(48, &mut rng),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub async fn insert(&self, db: &dyn DatabaseAccess) -> Result<()> {
        db.write().await.execute(
            "INSERT INTO auth (oauth_token, oauth_token_secret, member_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &self.oauth_token,
                &self.oauth_token_secret,
                self.member_id,
                self.created_at,
                self.updated_at,
            ],
        )?;

        Ok(())
    }

    pub async fn update_with_member_id(
        token_secret: &str,
        member_id: MemberId,
        db: &dyn DatabaseAccess,
    ) -> Result<bool> {
        Ok(db.write().await.execute(
            "UPDATE auth SET member_id = ?2, oauth_token = NULL WHERE oauth_token_secret = ?1",
            params![token_secret, member_id],
        )? != 0)
    }

    pub async fn remove_by_token_secret(
        token_secret: &str,
        db: &dyn DatabaseAccess,
    ) -> Result<bool> {
        Ok(db.write().await.execute(
            "DELETE FROM auth WHERE oauth_token_secret = ?1",
            [token_secret],
        )? != 0)
    }

    // TODO: Replace most used results with does_exist.
    pub async fn find_by_token(token: &str, db: &dyn DatabaseAccess) -> Result<Option<Self>> {
        Ok(db
            .write()
            .await
            .query_row(
                "SELECT * FROM auth WHERE oauth_token = ?1",
                [token],
                |v| Self::from_row(v),
            )
            .optional()?)
    }

    pub async fn find_by_token_secret(token: &str, db: &dyn DatabaseAccess) -> Result<Option<Self>> {
        Ok(db
            .write()
            .await
            .query_row(
                "SELECT * FROM auth WHERE oauth_token_secret = ?1",
                [token],
                |v| Self::from_row(v),
            )
            .optional()?)
    }
}
