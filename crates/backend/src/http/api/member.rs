use actix_identity::Identity;
use actix_web::{get, web};
use books_common::api;

use crate::{database::Database, http::get_auth_value, WebResult};



// TODO: Add body requests for specifics
#[get("/member")]
pub async fn load_member_self(
	db: web::Data<Database>,
	identity: Identity,
) -> WebResult<web::Json<api::GetMemberSelfResponse>> {
	if let Some(cookie) = get_auth_value(&identity) {
		if let Some(member) = db.get_member_by_id(cookie.member_id)? {
			return Ok(web::Json(api::GetMemberSelfResponse {
				member: Some(member.into())
			}));
		}
	}

	Ok(web::Json(api::GetMemberSelfResponse {
		member: None
	}))
}