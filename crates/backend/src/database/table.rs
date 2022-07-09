use chrono::{DateTime, Utc};


// Auth

pub struct NewAuth {
	pub oauth_token: String,
	pub oauth_token_secret: String,
	pub created_at: DateTime<Utc>,
}