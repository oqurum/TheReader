//! The Client the user is using to view the website.
//!
//! References by AuthModel.

use chrono::{DateTime, Utc};
use common::{ClientId};
use lazy_static::lazy_static;
use rusqlite::{params, OptionalExtension};
use uaparser::{Parser, UserAgentParser};

use crate::{DatabaseAccess, Result, http::gen_sample_alphanumeric};

use super::{AdvRow, TableRow};

lazy_static! {
    static ref USER_AGENT_PARSER: UserAgentParser = UserAgentParser::from_bytes(include_bytes!("../../../../app/user_agents.yaml")).expect("User Agent Parsing");
}


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

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TableRow<'_> for ClientModel {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.next()?,

            oauth: row.next()?,
            identifier: row.next()?,

            client: row.next()?,
            device: row.next()?,
            platform: row.next_opt()?,

            created_at: row.next()?,
            updated_at: row.next()?,
        })
    }
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

    pub async fn insert(self, db: &dyn DatabaseAccess) -> Result<ClientModel> {
        let now = Utc::now();

        let mut rng = rand::thread_rng();
        let identifier = gen_sample_alphanumeric(32, &mut rng);
        // TODO: Ensure identifier doesn't already exist.

        let writer = db.write().await;

        writer.execute(
            "INSERT INTO client (oauth, identifier, client, device, platform, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &self.oauth,
                &identifier,
                &self.client,
                &self.device,
                &self.platform,
                now,
                now,
            ],
        )?;

        Ok(ClientModel {
            id: ClientId::from(writer.last_insert_rowid() as usize),
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
    pub async fn find_by_token(token: &str, db: &dyn DatabaseAccess) -> Result<Option<Self>> {
        Ok(db
            .write()
            .await
            .query_row(
                "SELECT * FROM client WHERE oauth_token = ?1",
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
                "SELECT * FROM client WHERE oauth_token_secret = ?1",
                [token],
                |v| Self::from_row(v),
            )
            .optional()?)
    }
}
