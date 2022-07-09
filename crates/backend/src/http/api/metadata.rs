use actix_web::{get, web, HttpResponse, post};

use books_common::{api, SearchType, SearchFor, SearchForBooksBy, Poster, MetadataId};
use chrono::Utc;
use common::{MemberId, ImageType, Either};

use crate::{database::Database, task::{queue_task_priority, self}, queue_task, metadata, WebResult, Error, store_image, model::{image::{ImageLinkModel, UploadedImageModel}, metadata::MetadataModel}};




#[get("/metadata/{id}/thumbnail")]
async fn load_metadata_thumbnail(path: web::Path<MetadataId>, db: web::Data<Database>) -> WebResult<HttpResponse> {
	let meta_id = path.into_inner();

	let meta = MetadataModel::get_by_id(meta_id, &db)?;

	if let Some(loc) = meta.map(|v| v.thumb_path) {
		let path = crate::image::prefixhash_to_path(loc.as_value());

		Ok(HttpResponse::Ok().body(std::fs::read(path).map_err(Error::from)?))
	} else {
		Ok(HttpResponse::NotFound().finish())
	}
}


// Metadata
#[get("/metadata/{id}")]
pub async fn get_all_metadata_comp(meta_id: web::Path<MetadataId>, db: web::Data<Database>) -> WebResult<web::Json<api::ApiGetMetadataByIdResponse>> {
	let meta = MetadataModel::get_by_id(*meta_id, &db)?.unwrap();

	let (mut media, mut progress) = (Vec::new(), Vec::new());

	for file in db.get_files_by_metadata_id(meta.id)? {
		let prog = db.get_progress(MemberId::none(), file.id)?;

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

#[post("/metadata/{id}")]
pub async fn update_item_metadata(meta_id: web::Path<MetadataId>, body: web::Json<api::PostMetadataBody>) -> HttpResponse {
	let meta_id = *meta_id;

	match body.into_inner() {
		api::PostMetadataBody::AutoMatchMetaIdByFiles => {
			queue_task(task::TaskUpdateInvalidMetadata::new(task::UpdatingMetadata::AutoUpdateMetaIdByFiles(meta_id)));
		}

		api::PostMetadataBody::AutoMatchMetaIdBySource => {
			queue_task(task::TaskUpdateInvalidMetadata::new(task::UpdatingMetadata::AutoUpdateMetaIdBySource(meta_id)));
		}

		api::PostMetadataBody::UpdateMetaBySource(source) => {
			queue_task_priority(task::TaskUpdateInvalidMetadata::new(task::UpdatingMetadata::UpdateMetadataWithSource { meta_id, source }));
		}
	}

	HttpResponse::Ok().finish()
}


#[get("/metadata/{id}/posters")]
async fn get_poster_list(
	path: web::Path<MetadataId>,
	db: web::Data<Database>
) -> WebResult<web::Json<api::ApiGetPosterByMetaIdResponse>> {
	let meta = MetadataModel::get_by_id(*path, &db)?.unwrap();

	// TODO: For Open Library we need to go from an Edition to Work.
	// Work is the main book. Usually consisting of more posters.
	// We can do they by works[0].key = "/works/OLXXXXXXW"

	let mut items: Vec<Poster> = ImageLinkModel::get_by_linked_id(**path, ImageType::Book, &db).await?
		.into_iter()
		.map(|poster| Poster {
			id: Some(poster.image_id),

			selected: poster.path == meta.thumb_path,

			path: poster.path.as_url(),

			created_at: poster.created_at,
		})
		.collect();

	let search = crate::metadata::search_all_agents(
		&format!(
			"{} {}",
			meta.title.as_deref().or(meta.title.as_deref()).unwrap_or_default(),
			meta.cached.author.as_deref().unwrap_or_default(),
		),
		books_common::SearchFor::Book(books_common::SearchForBooksBy::Query)
	).await?;

	for item in search.0.into_values().flatten() {
		if let crate::metadata::SearchItem::Book(item) = item {
			for path in item.thumb_locations.into_iter().filter_map(|v| v.into_url_value()) {
				items.push(Poster {
					id: None,

					selected: false,
					path,

					created_at: Utc::now(),
				});
			}
		}
	}

	Ok(web::Json(api::GetPostersResponse {
		items
	}))
}

#[post("/metadata/{id}/posters")]
async fn post_change_poster(
	metadata_id: web::Path<MetadataId>,
	body: web::Json<api::ChangePosterBody>,
	db: web::Data<Database>
) -> WebResult<HttpResponse> {
	let mut meta = MetadataModel::get_by_id(*metadata_id, &db)?.unwrap();

	match body.into_inner().url_or_id {
		Either::Left(url) => {
			let resp = reqwest::get(url)
				.await.map_err(Error::from)?
				.bytes()
				.await.map_err(Error::from)?;

			let image_model = store_image(resp.to_vec(), &db).await?;

			meta.thumb_path = image_model.path.clone();

			ImageLinkModel::new_book(image_model.id, meta.id).insert(&db).await?;
		}

		Either::Right(id) => {
			let poster = UploadedImageModel::get_by_id(id, &db).await?.unwrap();

			if meta.thumb_path == poster.path {
				return Ok(HttpResponse::Ok().finish());
			}

			meta.thumb_path = poster.path;
		}
	}

	meta.update(&db)?;

	Ok(HttpResponse::Ok().finish())
}


#[get("/metadata/search")]
pub async fn get_metadata_search(body: web::Query<api::GetMetadataSearch>) -> WebResult<web::Json<api::ApiGetMetadataSearchResponse>> {
	let search = metadata::search_all_agents(
		&body.query,
		match body.search_type {
			// TODO: Allow for use in Query.
			SearchType::Book => SearchFor::Book(SearchForBooksBy::Query),
			SearchType::Person => SearchFor::Person
		}
	).await?;

	Ok(web::Json(api::MetadataSearchResponse {
		items: search.0.into_iter()
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