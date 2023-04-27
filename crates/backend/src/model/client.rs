//! The Client the user is using to view the website.
//!
//! References by AuthModel.

use chrono::{Utc, NaiveDateTime};
use common::{ClientId};
use lazy_static::lazy_static;
use sqlx::{FromRow, SqliteConnection};
use uaparser::{Parser, UserAgentParser};

use crate::{Result, http::gen_sample_alphanumeric};

lazy_static! {
    static ref USER_AGENT_PARSER: UserAgentParser = UserAgentParser::from_bytes(include_bytes!("../../../../app/user_agents.yaml")).expect("User Agent Parsing");
}


// We have this separated from the Auth Model because we use the auth model for more than just User Clients.
pub struct NewClientModel {
    /// Oauth Secret Token
    pub oauth: String,

    /// Client Type Name (egg. Oqurum Web, Oqurum for Android, ...)
    pub client: String,
    /// Device Name (egg. Firefox ???)
    pub device: String,
    /// Platform Name (egg. Windows 11)
    pub platform: Option<String>,
}

#[derive(FromRow)]
pub struct ClientModel {
    pub id: ClientId,

    /// Unique ID for the frontend
    pub identifier: String,

    /// Oauth Secret Token
    pub oauth: String,

    /// Client Type Name (egg. Oqurum Web, Oqurum for Android, ...)
    pub client: String,
    /// Device Name (egg. Firefox ???)
    pub device: String,
    /// Platform Name (egg. Windows 11)
    pub platform: Option<String>,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl NewClientModel {
    pub fn new(oauth_token_secret: String, client: String, user_agent: &str) -> Self {
        let ua = USER_AGENT_PARSER.parse(user_agent);

        Self {
            oauth: oauth_token_secret,
            client,
            device: {
                if let Some(major) = ua.user_agent.major {
                    format!("{} {major}", ua.user_agent.family)
                } else {
                    ua.user_agent.family.to_string()
                }
            },
            platform: Some({
                if let Some(major) = ua.os.major {
                    format!("{} {major}", ua.os.family)
                } else {
                    ua.os.family.to_string()
                }
            }).filter(|v| !v.trim().is_empty()),
        }
    }

    pub async fn insert(self, db: &mut SqliteConnection) -> Result<ClientModel> {
        let now = Utc::now().naive_utc();

        let mut rng = rand::thread_rng();
        let identifier = gen_sample_alphanumeric(32, &mut rng);
        // TODO: Ensure identifier doesn't already exist.

        let res = sqlx::query(
            "INSERT INTO client (oauth, identifier, client, device, platform, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $6)",
        )
        .bind(&self.oauth)
        .bind(&identifier)
        .bind(&self.client)
        .bind(&self.device)
        .bind(&self.platform)
        .bind(now)
        .execute(db).await?;

        Ok(ClientModel {
            id: ClientId::from(res.last_insert_rowid()),
            identifier,
            oauth: self.oauth,
            client: self.client,
            device: self.device,
            platform: self.platform,
            created_at: now,
            updated_at: now,
        })
    }
}

impl ClientModel {
    pub async fn find_by_token_secret(token: &str, db: &mut SqliteConnection) -> Result<Option<Self>> {
        Ok(sqlx::query_as("SELECT * FROM client WHERE oauth = $1").bind(token).fetch_optional(db).await?)
    }
}
