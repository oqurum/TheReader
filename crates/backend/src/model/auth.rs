use chrono::{DateTime, NaiveDateTime, Utc};
use common::MemberId;
use sqlx::{FromRow, SqliteConnection};

use crate::{http::gen_sample_alphanumeric, Result, IN_MEM_DB};

/// Used for Pending Authentications and Completed Authentications when member_id is defined.
///
/// It is referenced whenever
#[derive(FromRow)]
pub struct AuthModel {
    pub oauth_token: Option<String>,
    /// Stored in Cookie cache after successful login.
    pub oauth_token_secret: String,

    /// Can be null if we deleted the Member
    pub member_id: Option<MemberId>,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    // TODO: expires_at Date, expires_after Duration
    // TODO: Type of Auth. e.g. Login, Libby Auth
}

impl AuthModel {
    pub fn new(member_id: Option<MemberId>) -> Self {
        let mut rng = rand::thread_rng();

        Self {
            member_id,
            oauth_token: Some(gen_sample_alphanumeric(32, &mut rng))
                .filter(|_| member_id.is_none()),
            oauth_token_secret: gen_sample_alphanumeric(48, &mut rng),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    pub async fn insert(&self, db: &mut SqliteConnection) -> Result<u64> {
        let res = sqlx::query(
            "INSERT INTO auth (oauth_token, oauth_token_secret, member_id, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(&self.oauth_token)
        .bind(&self.oauth_token_secret)
        .bind(self.member_id)
        .bind(self.created_at)
        .bind(self.updated_at)
        .execute(db).await?;

        Ok(res.rows_affected())
    }

    pub async fn update_with_member_id(
        token_secret: &str,
        member_id: MemberId,
        db: &mut SqliteConnection,
    ) -> Result<bool> {
        let res = sqlx::query(
            "UPDATE auth SET member_id = $2, oauth_token = NULL WHERE oauth_token_secret = $1",
        )
        .bind(token_secret)
        .bind(member_id)
        .execute(db)
        .await?;

        Ok(res.rows_affected() != 0)
    }

    pub async fn remove_by_token_secret(
        token_secret: &str,
        db: &mut SqliteConnection,
    ) -> Result<bool> {
        IN_MEM_DB.delete(token_secret).await;

        let res = sqlx::query("DELETE FROM auth WHERE oauth_token_secret = $1")
            .bind(token_secret)
            .execute(db)
            .await?;

        Ok(res.rows_affected() != 0)
    }

    // TODO: Replace most used results with does_exist.
    pub async fn find_by_token(token: &str, db: &mut SqliteConnection) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT oauth_token, oauth_token_secret, member_id, created_at, updated_at FROM auth WHERE oauth_token = $1"
        ).bind(token).fetch_optional(db).await?)
    }

    pub async fn find_by_token_secret(
        token: &str,
        db: &mut SqliteConnection,
    ) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT oauth_token, oauth_token_secret, member_id, created_at, updated_at FROM auth WHERE oauth_token_secret = $1"
        ).bind(token).fetch_optional(db).await?)
    }
}
