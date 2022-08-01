use actix_web::{post, web};
use books_common::api;
use common::api::{ApiErrorResponse, WrappingResponse};

use crate::{queue_task, task, http::{MemberCookie, JsonResponse}, database::Database, WebResult};


// TODO: Actually optimize.
#[post("/task")]
pub async fn run_task(
	modify: web::Json<api::RunTaskBody>,
	member: MemberCookie,
	db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
	let modify = modify.into_inner();

	let member = member.fetch_or_error(&db).await?;

	if !member.permissions.is_owner() {
		return Err(ApiErrorResponse::new("Not owner").into());
	}


	if modify.run_search {
		queue_task(task::TaskLibraryScan);
	}

	Ok(web::Json(WrappingResponse::okay("success")))
}
