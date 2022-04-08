use actix_web::{web, get, post, HttpResponse};
use books_common::api;
use chrono::Utc;

use crate::{database::{Database, table::{TagPersonAlt, MetadataPerson}}, task::{self, queue_task_priority}, queue_task, ThumbnailLocation};


// Get List Of People and Search For People
#[get("/api/people")]
pub async fn load_author_list(
	db: web::Data<Database>,
	query: web::Query<api::SimpleListQuery>,
) -> web::Json<api::GetPeopleResponse> {
	let offset = query.offset.unwrap_or(0);
	let limit = query.offset.unwrap_or(50);

	// Return Searched People
	if let Some(query) = query.query.as_deref() {
		let items = db.search_person_list(query, offset, limit)
			.unwrap()
			.into_iter()
			.map(|v| v.into())
			.collect();

		web::Json(api::GetPeopleResponse {
			offset,
			limit,
			total: 0, // TODO
			items
		})
	}

	// Return All People
	else {
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
}


// Person Thumbnail
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


// Person Tasks - Update Person, Overwrite Person with another source.
#[post("/api/person/{id}")]
pub async fn update_person_data(meta_id: web::Path<i64>, body: web::Json<api::PostPersonBody>, db: web::Data<Database>) -> HttpResponse {
	let person_id = *meta_id;

	match body.into_inner() {
		api::PostPersonBody::AutoMatchById => {
			queue_task(task::TaskUpdatePeople::new(task::UpdatingPeople::AutoUpdateById(person_id)));
		}

		api::PostPersonBody::UpdateBySource(source) => {
			queue_task_priority(task::TaskUpdatePeople::new(task::UpdatingPeople::UpdatePersonWithSource { person_id, source }));
		}

		api::PostPersonBody::CombinePersonWith(into_person_id) => {
			// TODO: Tests for this to ensure it's correct.

			let old_person = db.get_person_by_id(person_id).unwrap().unwrap();
			let mut into_person = db.get_person_by_id(into_person_id).unwrap().unwrap();

			// Transfer Alt Names to Other Person
			db.transfer_person_alt(old_person.id, into_person.id).unwrap();

			// Delete remaining Alt Names
			db.remove_person_alt_by_person_id(old_person.id).unwrap();

			// Make Old Person Name an Alt Name
			let _ = db.add_person_alt(&TagPersonAlt {
				name: old_person.name,
				person_id: into_person.id,
			});

			// Transfer Old Person Metadata to New Person
			for met_per in db.get_meta_person_list(old_person.id).unwrap() {
				let _ = db.add_meta_person(&MetadataPerson {
					metadata_id: met_per.metadata_id,
					person_id: into_person.id,
				});
			}

			db.remove_meta_person_by_person_id(old_person.id).unwrap();

			if into_person.birth_date.is_none() {
				into_person.birth_date = old_person.birth_date;
			}

			if into_person.description.is_none() {
				into_person.description = old_person.description;
			}

			if into_person.thumb_url.is_none() {
				into_person.thumb_url = old_person.thumb_url;
			}

			into_person.updated_at = Utc::now();

			// Update New Person
			db.update_person(&into_person).unwrap();

			// Delete Old Person
			db.remove_person_by_id(old_person.id).unwrap();

			// TODO: Update Metadata cache
		}
	}

	HttpResponse::Ok().finish()
}