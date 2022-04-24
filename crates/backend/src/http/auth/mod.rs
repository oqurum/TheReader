use std::pin::Pin;

use actix_identity::Identity;
use actix_web::{FromRequest, HttpRequest, dev::Payload, error::ErrorUnauthorized, web};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{Result, database::{table, Database}};

pub mod password;
pub mod passwordless;


#[derive(Serialize, Deserialize)]
pub struct CookieAuth {
	pub member_id: usize,
	pub stored_since: i64,
}


pub fn get_auth_value(identity: &Identity) -> Option<CookieAuth> {
	let ident = identity.identity()?;
	serde_json::from_str(&ident).ok()
}

pub fn get_auth_member(identity: &Identity, db: &Database) -> Option<table::Member> {
	let store = get_auth_value(identity)?;
	db.get_member_by_id(store.member_id).ok().flatten()
}

pub fn remember_member_auth(member_id: usize, identity: &Identity) -> Result<()> {
	let value = serde_json::to_string(&CookieAuth {
		member_id,
		stored_since: Utc::now().timestamp_millis(),
	})?;

	identity.remember(value);

	Ok(())
}

// Retrive Member from Identity
pub struct MemberCookie(CookieAuth);

impl MemberCookie {
	pub fn member_id(&self) -> usize {
		self.0.member_id
	}
}


impl FromRequest for MemberCookie {
	type Error = actix_web::Error;
	type Future = Pin<Box<dyn std::future::Future<Output = std::result::Result<MemberCookie, actix_web::Error>>>>;

	fn from_request(req: &HttpRequest, pl: &mut Payload) -> Self::Future {
		let fut = Identity::from_request(req, pl);

		let db = req.app_data::<web::Data<Database>>().unwrap().clone();

		Box::pin(async move {
			if let Some(id) = get_auth_value(&fut.await?) {
				Ok(MemberCookie(id))
			} else {
				Err(ErrorUnauthorized("unauthorized"))
			}
		})
	}
}