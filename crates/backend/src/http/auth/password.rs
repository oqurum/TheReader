// TODO: Better security. Simple Proof of Concept.


use actix_identity::Identity;
use actix_web::HttpResponse;
use actix_web::web;

use chrono::Utc;
use rand::Rng;
use rand::prelude::ThreadRng;
use serde::{Serialize, Deserialize};

use crate::database::{table, Database};


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
) -> HttpResponse {
	if identity.identity().is_some() {
		return HttpResponse::MethodNotAllowed().finish(); // TODO: What's the proper status?
	}

	let PostPasswordCallback { email, password } = query.into_inner();

	// Create or Update User.
	let member = if let Some(value) = db.get_member_by_email(&email).unwrap() {
		if value.type_of != 2 {
			panic!("Invalid Member. Member does not have a local password associated with it.");
		}

		if bcrypt::verify(&password, value.password.as_ref().unwrap()).unwrap() {
			value
		} else {
			panic!("Unable to very password hash for account");
		}
	} else {
		let hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST).unwrap();

		let new_member = table::NewMember {
			// TODO: Strip email
			name: email.clone(),
			email: Some(email),
			password: Some(hash),
			type_of: 2,
			config: None,
			created_at: Utc::now(),
			updated_at: Utc::now(),
		};

		let inserted_id = db.add_member(&new_member).unwrap();

		new_member.into_member(inserted_id)
	};

	identity.remember(member.id.to_string());

	HttpResponse::Ok().finish()
}

pub fn gen_sample_alphanumeric(amount: usize, rng: &mut ThreadRng) -> String {
	rng.sample_iter(rand::distributions::Alphanumeric)
		.take(amount)
		.map(char::from)
		.collect()
}