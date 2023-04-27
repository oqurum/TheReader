use actix_web::{get, post, web};
use common::api::{ApiErrorResponse, WrappingResponse};
use common_local::{
    api,
    ws::{TaskId, TaskInfo},
};

use crate::{
    http::{ws::RUNNING_TASKS, JsonResponse, MemberCookie},
    queue_task, task, SqlPool, WebResult,
};

// TODO: Actually optimize.
#[post("/task")]
pub async fn run_task(
    modify: web::Json<api::RunTaskBody>,
    member: MemberCookie,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<&'static str>> {
    let modify = modify.into_inner();

    let member = member.fetch_or_error(&mut *db.acquire().await?).await?;

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

#[get("/tasks")]
pub async fn get_tasks(
    member: MemberCookie,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<Vec<(TaskId, TaskInfo)>>> {
    let member = member.fetch_or_error(&mut *db.acquire().await?).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    let resp = RUNNING_TASKS
        .lock()
        .unwrap()
        .iter()
        .map(|(a, b)| (*a, b.clone()))
        .collect::<Vec<_>>();

    Ok(web::Json(WrappingResponse::okay(resp)))
}
