use actix_web::{post, web};
use books_common::api;

use crate::{queue_task, task, http::MemberCookie, database::Database, WebResult};


// TODO: Actually optimize.
#[post("/task")]
pub async fn run_task(
	modify: web::Json<api::RunTaskBody>,
	member: MemberCookie,
	db: web::Data<Database>,
) -> WebResult<web::Json<api::WrappingResponse<&'static str>>> {
	let modify = modify.into_inner();

	let member = member.fetch_or_error(&db).await?;

	if !member.permissions.is_owner() {
		return Err(api::ApiErrorResponse::new("Not owner").into());
	}


	if modify.run_search {
		queue_task(task::TaskLibraryScan);
	}

	Ok(web::Json(api::WrappingResponse::okay("success")))
}
