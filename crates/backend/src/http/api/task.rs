use actix_web::{post, web, HttpResponse};
use books_common::api;

use crate::{queue_task, task};


// TODO: Actually optimize.
#[post("/task")]
pub async fn run_task(modify: web::Json<api::RunTaskBody>) -> HttpResponse {
	let modify = modify.into_inner();

	if modify.run_search {
		queue_task(task::TaskLibraryScan);
	}

	if modify.run_metadata {
		queue_task(task::TaskUpdateInvalidMetadata::new(task::UpdatingMetadata::AutoMatchInvalid));
	}

	HttpResponse::Ok().finish()
}
