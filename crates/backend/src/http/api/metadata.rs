use actix_web::{get, web, HttpResponse, post};

use books_common::{api, SearchType, SearchFor, SearchForBooksBy};

use crate::{database::Database, task::{queue_task_priority, self}, queue_task, metadata, WebResult, Error};




#[get("/metadata/{id}/thumbnail")]
async fn load_metadata_thumbnail(path: web::Path<usize>, db: web::Data<Database>) -> WebResult<HttpResponse> {
	let book_id = path.into_inner();

	let meta = db.get_metadata_by_id(book_id)?;

	if let Some(loc) = meta.map(|v| v.thumb_path) {
		let path = crate::image::prefixhash_to_path(loc.as_type(), loc.as_value());

		Ok(HttpResponse::Ok().body(std::fs::read(path).map_err(Error::from)?))
	} else {
		Ok(HttpResponse::NotFound().finish())
	}
}


// Metadata

#[post("/metadata/{id}")]
pub async fn update_item_metadata(meta_id: web::Path<usize>, body: web::Json<api::PostMetadataBody>) -> HttpResponse {
	let meta_id = *meta_id;

	match body.into_inner() {
		api::PostMetadataBody::AutoMatchByMetaId => {
			queue_task(task::TaskUpdateInvalidMetadata::new(task::UpdatingMetadata::AutoUpdateById(meta_id)));
		}

		api::PostMetadataBody::UpdateMetaBySource(source) => {
			queue_task_priority(task::TaskUpdateInvalidMetadata::new(task::UpdatingMetadata::UpdateMetadataWithSource { meta_id, source }));
		}
	}

	HttpResponse::Ok().finish()
}

#[get("/metadata/{id}")]
pub async fn get_all_metadata_comp(meta_id: web::Path<usize>, db: web::Data<Database>) -> WebResult<web::Json<api::MediaViewResponse>> {
	let meta = db.get_metadata_by_id(*meta_id)?.unwrap();

	let (mut media, mut progress) = (Vec::new(), Vec::new());

	for file in db.get_files_by_metadata_id(meta.id)? {
		let prog = db.get_progress(0, file.id)?;

		media.push(file.into());
		progress.push(prog.map(|v| v.into()));
	}

	let people = db.get_person_list_by_meta_id(meta.id)?;

	Ok(web::Json(api::MediaViewResponse {
		metadata: meta.into(),
		media,
		progress,
		people: people.into_iter()
			.map(|p| p.into())
			.collect(),
	}))
}

#[get("/metadata/search")]
pub async fn get_metadata_search(body: web::Query<api::GetMetadataSearch>) -> WebResult<web::Json<api::MetadataSearchResponse>> {
	let search = metadata::search_all_agents(
		&body.query,
		match body.search_type {
			// TODO: Allow for use in Query.
			SearchType::Book => SearchFor::Book(SearchForBooksBy::Query),
			SearchType::Person => SearchFor::Person
		}
	).await?;

	Ok(web::Json(api::MetadataSearchResponse {
		items: search.into_iter()
			.map(|(a, b)| (
				a,
				b.into_iter().map(|v| {
					match v {
						metadata::SearchItem::Book(book) => {
							api::SearchItem::Book(api::MetadataBookSearchItem {
								source: book.source,
								author: book.cached.author,
								description: book.description,
								name: book.title.unwrap_or_else(|| String::from("Unknown title")),
								thumbnail_url: book.thumb_locations.first()
									.and_then(|v| v.as_url_value())
									.map(|v| v.to_string())
									.unwrap_or_default(),
							})
						}

						metadata::SearchItem::Author(author) => {
							api::SearchItem::Person(api::MetadataPersonSearchItem {
								source: author.source,

								cover_image: author.cover_image_url,

								name: author.name,
								other_names: author.other_names,
								description: author.description,

								birth_date: author.birth_date,
								death_date: author.death_date,
							})
						}
					}
				}).collect()
			))
			.collect()
	}))
}

