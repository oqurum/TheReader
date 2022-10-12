use actix_web::{post, web};
use common::api::{ApiErrorResponse, WrappingResponse};
use common_local::api;

use crate::{
    database::Database,
    http::{JsonResponse, MemberCookie},
    queue_task, task, WebResult,
};

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

    if let Some(library_id) = modify.run_search {
        queue_task(task::TaskLibraryScan { library_id });
    }

    if let Some(library_id) = modify.run_metadata {
        queue_task(task::TaskUpdateInvalidBook::new(
            task::UpdatingBook::UpdateAllWithAgent {
                library_id,
                agent: String::new(),
            },
        ));
    }

    Ok(web::Json(WrappingResponse::okay("success")))
}
