use actix_web::{web, get, post, HttpResponse};
use books_common::api;

use crate::{database::Database, task::{self, queue_task_priority}, queue_task, ThumbnailLocation};


#[get("/api/people")]
pub async fn load_author_list(db: web::Data<Database>, query: web::Query<api::SimpleListQuery>) -> web::Json<api::GetPeopleResponse> {
	let offset = query.offset.unwrap_or(0);
	let limit = query.offset.unwrap_or(50);

	let items = db.get_person_list(offset, limit)
		.unwrap()
		.into_iter()
		.map(|v| v.into())
		.collect();

	web::Json(api::GetPeopleResponse {
		offset,
		limit,
		total: db.get_person_count().unwrap(),
		items
	})
}


#[get("/api/person/{id}/thumbnail")]
async fn load_person_thumbnail(person_id: web::Path<i64>, db: web::Data<Database>) -> HttpResponse {
	let meta = db.get_person_by_id(*person_id).unwrap();

	if let Some(path) = meta.and_then(|v| v.thumb_url) {
		let loc = ThumbnailLocation::from(path);

		let path = crate::image::prefixhash_to_path(loc.as_type(), loc.as_value());

		HttpResponse::Ok().body(std::fs::read(path).unwrap())
	} else {
		HttpResponse::NotFound().finish()
	}
}



#[post("/api/person/{id}")]
pub async fn update_person_data(meta_id: web::Path<i64>, body: web::Json<api::PostPersonBody>) -> HttpResponse {
	let person_id = *meta_id;

	match body.into_inner() {
		api::PostPersonBody::AutoMatchById => {
			queue_task(task::TaskUpdatePeople::new(task::UpdatingPeople::AutoUpdateById(person_id)));
		}

		api::PostPersonBody::UpdateBySource(source) => {
			queue_task_priority(task::TaskUpdatePeople::new(task::UpdatingPeople::UpdatePersonWithSource { person_id, source }));
		}
	}

	HttpResponse::Ok().finish()
}