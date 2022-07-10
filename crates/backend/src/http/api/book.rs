use actix_web::{get, web, HttpResponse, post};

use books_common::{api, SearchType, SearchFor, SearchForBooksBy, Poster, MetadataId, DisplayItem};
use chrono::Utc;
use common::{MemberId, ImageType, Either};

use crate::{database::Database, task::{queue_task_priority, self}, queue_task, metadata, WebResult, Error, store_image, model::{image::{ImageLinkModel, UploadedImageModel}, metadata::MetadataModel, file::FileModel, progress::FileProgressionModel, person::PersonModel}};




#[get("/books")]
pub async fn load_book_list(
	query: web::Query<api::BookListQuery>,
	db: web::Data<Database>,
) -> WebResult<web::Json<api::ApiGetBookListResponse>> {
	let (items, count) = if let Some(search) = query.search_query() {
		let search = search?;

		let count = MetadataModel::count_search_by(&search, query.library, &db).await?;

		let items = if count == 0 {
			Vec::new()
		} else {
			MetadataModel::search_by(
				&search,
				query.library,
				query.offset.unwrap_or(0),
				query.limit.unwrap_or(50),
				&db,
			).await?
				.into_iter()
				.map(|meta| {
					DisplayItem {
						id: meta.id,
						title: meta.title.or(meta.original_title).unwrap_or_default(),
						cached: meta.cached,
						has_thumbnail: meta.thumb_path.is_some()
					}
				})
				.collect()
		};

		(items, count)
	} else {
		let count = MetadataModel::count_search_by(
			&api::SearchQuery { query: None, source: None },
			query.library,
			&db,
		).await?;

		let items = MetadataModel::find_by(
			query.library,
			query.offset.unwrap_or(0),
			query.limit.unwrap_or(50),
			&db,
		).await?
			.into_iter()
			.map(|meta| {
				DisplayItem {
					id: meta.id,
					title: meta.title.or(meta.original_title).unwrap_or_default(),
					cached: meta.cached,
					has_thumbnail: meta.thumb_path.is_some()
				}
			})
			.collect();

		(items, count)
	};

	Ok(web::Json(api::GetBookListResponse {
		items,
		count,
	}))
}


#[get("/book/{id}/thumbnail")]
async fn load_book_thumbnail(path: web::Path<MetadataId>, db: web::Data<Database>) -> WebResult<HttpResponse> {
	let meta_id = path.into_inner();

	let meta = MetadataModel::find_one_by_id(meta_id, &db).await?;

	if let Some(loc) = meta.map(|v| v.thumb_path) {
		let path = crate::image::prefixhash_to_path(loc.as_value());

		Ok(HttpResponse::Ok().body(std::fs::read(path).map_err(Error::from)?))
	} else {
		Ok(HttpResponse::NotFound().finish())
	}
}


// Metadata
#[get("/book/{id}")]
pub async fn load_book_info(meta_id: web::Path<MetadataId>, db: web::Data<Database>) -> WebResult<web::Json<api::ApiGetMetadataByIdResponse>> {
	let meta = MetadataModel::find_one_by_id(*meta_id, &db).await?.unwrap();

	let (mut media, mut progress) = (Vec::new(), Vec::new());

	for file in FileModel::find_by_metadata_id(meta.id, &db).await? {
		let prog = FileProgressionModel::find_one(MemberId::none(), file.id, &db).await?;

		media.push(file.into());
		progress.push(prog.map(|v| v.into()));
	}

	let people = PersonModel::find_by_meta_id(meta.id, &db).await?;

	Ok(web::Json(api::MediaViewResponse {
		metadata: meta.into(),
		media,
		progress,
		people: people.into_iter()
			.map(|p| p.into())
			.collect(),
	}))
}

#[post("/book/{id}")]
pub async fn update_book_info(meta_id: web::Path<MetadataId>, body: web::Json<api::PostMetadataBody>) -> HttpResponse {
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


#[get("/book/{id}/posters")]
async fn get_book_posters(
	path: web::Path<MetadataId>,
	db: web::Data<Database>
) -> WebResult<web::Json<api::ApiGetPosterByMetaIdResponse>> {
	let meta = MetadataModel::find_one_by_id(*path, &db).await?.unwrap();

	// TODO: For Open Library we need to go from an Edition to Work.
	// Work is the main book. Usually consisting of more posters.
	// We can do they by works[0].key = "/works/OLXXXXXXW"

	let mut items: Vec<Poster> = ImageLinkModel::find_with_link_by_link_id(**path, ImageType::Book, &db).await?
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

#[post("/book/{id}/posters")]
async fn insert_or_update_book_image(
	metadata_id: web::Path<MetadataId>,
	body: web::Json<api::ChangePosterBody>,
	db: web::Data<Database>
) -> WebResult<HttpResponse> {
	let mut meta = MetadataModel::find_one_by_id(*metadata_id, &db).await?.unwrap();

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

	meta.update(&db).await?;

	Ok(HttpResponse::Ok().finish())
}


#[get("/book/search")]
pub async fn book_search(body: web::Query<api::GetMetadataSearch>) -> WebResult<web::Json<api::ApiGetMetadataSearchResponse>> {
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