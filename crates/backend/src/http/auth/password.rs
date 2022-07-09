// TODO: Better security. Simple Proof of Concept.


use actix_identity::Identity;
use actix_web::HttpResponse;
use actix_web::web;

use chrono::Utc;
use rand::Rng;
use rand::prelude::ThreadRng;
use serde::{Serialize, Deserialize};

use crate::Error;
use crate::WebResult;
use crate::database::Database;
use crate::model::member::MemberModel;
use crate::model::member::NewMemberModel;


pub static PASSWORD_PATH: &str = "/auth/password";



#[derive(Serialize, Deserialize)]
pub struct PostPasswordCallback {
	pub email: String,
	pub password: String,
	// TODO: Check for Login vs Signup
}

pub async fn post_password_oauth(
	query: web::Form<PostPasswordCallback>,
	identity: Identity,
	db: web::Data<Database>,
) -> WebResult<HttpResponse> {
	if identity.identity().is_some() {
		return Ok(HttpResponse::MethodNotAllowed().finish()); // TODO: What's the proper status?
	}

	let PostPasswordCallback { email, password } = query.into_inner();

	// Create or Update User.
	let member = if let Some(value) = MemberModel::find_by_email(&email, &db)? {
		if value.type_of != 2 {
			panic!("Invalid Member. Member does not have a local password associated with it.");
		}

		if bcrypt::verify(&password, value.password.as_ref().unwrap()).map_err(Error::from)? {
			value
		} else {
			panic!("Unable to very password hash for account");
		}
	} else {
		let hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST).map_err(Error::from)?;

		let new_member = NewMemberModel {
			// TODO: Strip email
			name: email.clone(),
			email: Some(email),
			password: Some(hash),
			type_of: 2,
			config: None,
			created_at: Utc::now(),
			updated_at: Utc::now(),
		};

		new_member.insert(&db)?
	};

	super::remember_member_auth(member.id, &identity)?;

	Ok(HttpResponse::Ok().finish())
}

pub fn gen_sample_alphanumeric(amount: usize, rng: &mut ThreadRng) -> String {
	rng.sample_iter(rand::distributions::Alphanumeric)
		.take(amount)
		.map(char::from)
		.collect()
}