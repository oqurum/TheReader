use actix_identity::Identity;
use actix_web::{get, web};
use books_common::api;

use crate::database::Database;



// TODO: Add body requests for specifics
#[get("/api/member")]
pub async fn load_member_self(
	db: web::Data<Database>,
	identity: Identity,
) -> web::Json<api::GetMemberSelfResponse> {
	if let Some(ident) = identity.identity() {
		let member_id: usize = serde_json::from_str(&ident).unwrap();

		if let Some(member) = db.get_member_by_id(member_id).unwrap() {
			return web::Json(api::GetMemberSelfResponse {
				member: Some(member.into())
			});
		}
	}

	web::Json(api::GetMemberSelfResponse {
		member: None
	})
}