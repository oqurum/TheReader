pub mod password;
pub mod passwordless;


// TODO: Slim User for Identity

use actix_identity::Identity;

use crate::database::{table, Database};


pub fn get_auth_value(identity: &Identity) -> Option<i64> {
	let ident = identity.identity()?;
	serde_json::from_str(&ident).ok()
}

pub fn get_auth_member(identity: &Identity, db: &Database) -> Option<table::Member> {
	let id = get_auth_value(identity)?;
	db.get_member_by_id(id).ok().flatten()
}

pub fn remember_member_auth(member_id: i64, identity: &Identity) {
	identity.remember(member_id.to_string());
}