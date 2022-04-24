use actix_identity::Identity;
use actix_web::{get, web};
use books_common::api;

use crate::{database::Database, http::get_auth_value};



// TODO: Add body requests for specifics
#[get("/member")]
pub async fn load_member_self(
	db: web::Data<Database>,
	identity: Identity,
) -> web::Json<api::GetMemberSelfResponse> {
	if let Some(cookie) = get_auth_value(&identity) {
		if let Some(member) = db.get_member_by_id(cookie.member_id).unwrap() {
			return web::Json(api::GetMemberSelfResponse {
				member: Some(member.into())
			});
		}
	}

	web::Json(api::GetMemberSelfResponse {
		member: None
	})
}