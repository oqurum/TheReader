use actix_web::{web, get, post, HttpResponse};
use books_common::api;
use chrono::Utc;
use common::{PersonId, Either};

use crate::{database::Database, task::{self, queue_task_priority}, queue_task, WebResult, Error, model::{metadata_person::MetadataPersonModel, person::PersonModel, person_alt::PersonAltModel}};


// Get List Of People and Search For People
#[get("/people")]
pub async fn load_author_list(
	db: web::Data<Database>,
	query: web::Query<api::SimpleListQuery>,
) -> WebResult<web::Json<api::ApiGetPeopleResponse>> {
	let offset = query.offset.unwrap_or(0);
	let limit = query.offset.unwrap_or(50);

	// Return Searched People
	if let Some(query) = query.query.as_deref() {
		let items = PersonModel::search_by(query, offset, limit, &db).await?
			.into_iter()
			.map(|v| v.into())
			.collect();

		Ok(web::Json(api::GetPeopleResponse {
			offset,
			limit,
			total: 0, // TODO
			items
		}))
	}

	// Return All People
	else {
		let items = PersonModel::find(offset, limit, &db).await?
			.into_iter()
			.map(|v| v.into())
			.collect();

		Ok(web::Json(api::GetPeopleResponse {
			offset,
			limit,
			total: PersonModel::count(&db).await?,
			items
		}))
	}
}


// Person Thumbnail
#[get("/person/{id}/thumbnail")]
async fn load_person_thumbnail(person_id: web::Path<PersonId>, db: web::Data<Database>) -> WebResult<HttpResponse> {
	let meta = PersonModel::find_one_by_id(*person_id, &db).await?;

	if let Some(loc) = meta.map(|v| v.thumb_url) {
		let path = crate::image::prefixhash_to_path(loc.as_value());

		Ok(HttpResponse::Ok().body(std::fs::read(path).map_err(Error::from)?))
	} else {
		Ok(HttpResponse::NotFound().finish())
	}
}


// Person Tasks - Update Person, Overwrite Person with another source.
#[post("/person/{id}")]
pub async fn update_person_data(person_id: web::Path<PersonId>, body: web::Json<api::PostPersonBody>, db: web::Data<Database>) -> WebResult<HttpResponse> {
	let person_id = *person_id;

	match body.into_inner() {
		api::PostPersonBody::AutoMatchById => {
			queue_task(task::TaskUpdatePeople::new(task::UpdatingPeople::AutoUpdateById(person_id)));
		}

		api::PostPersonBody::UpdateBySource(source) => {
			queue_task_priority(task::TaskUpdatePeople::new(task::UpdatingPeople::UpdatePersonWithSource { person_id, source }));
		}

		api::PostPersonBody::CombinePersonWith(into_person_id) => {
			// TODO: Tests for this to ensure it's correct.

			let old_person = PersonModel::find_one_by_id(person_id, &db).await?.unwrap();
			let mut into_person = PersonModel::find_one_by_id(into_person_id, &db).await?.unwrap();

			// Attempt to transfer to other person
			PersonAltModel::transfer_or_ignore(old_person.id, into_person.id, &db).await?;

			// Delete remaining Alt Names
			PersonAltModel::delete_by_id(old_person.id, &db).await?;

			// Make Old Person Name an Alt Name
			let _ = PersonAltModel {
				name: old_person.name,
				person_id: into_person.id,
			}.insert(&db).await;

			// Transfer Old Person Metadata to New Person
			for met_per in MetadataPersonModel::find_by(Either::Right(old_person.id), &db).await? {
				let _ = MetadataPersonModel {
					metadata_id: met_per.metadata_id,
					person_id: into_person.id,
				}.insert_or_ignore(&db).await;
			}

			MetadataPersonModel::delete_by_person_id(old_person.id, &db).await?;

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
			into_person.update(&db).await?;

			// Delete Old Person
			PersonModel::delete_by_id(old_person.id, &db).await?;

			// TODO: Update Metadata cache
		}
	}

	Ok(HttpResponse::Ok().finish())
}