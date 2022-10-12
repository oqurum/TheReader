use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::{database::Database, Result};

pub struct AuthModel {
    pub oauth_token: String,
    pub oauth_token_secret: String,
    pub created_at: DateTime<Utc>,
}

impl AuthModel {
    pub async fn insert(&self, db: &Database) -> Result<()> {
        db.write().await.execute(
            r#"
            INSERT INTO auths (oauth_token, oauth_token_secret, created_at)
            VALUES (?1, ?2, ?3)
        "#,
            params![
                &self.oauth_token,
                &self.oauth_token_secret,
                self.created_at.timestamp_millis()
            ],
        )?;

        Ok(())
    }

    pub async fn remove_by_oauth_token(value: &str, db: &Database) -> Result<bool> {
        Ok(db.write().await.execute(
            r#"DELETE FROM auths WHERE oauth_token = ?1"#,
            params![value],
        )? != 0)
    }
}
