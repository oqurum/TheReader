use actix_web::{web, get};
use books_common::api;

use crate::database::Database;



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