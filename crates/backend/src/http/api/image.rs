use actix_files::NamedFile;
use actix_web::{get, web, HttpResponse, post, put, Responder};
use books_common::{api, Poster, Either};
use chrono::Utc;

use crate::{database::{Database, table::NewPoster}, store_image, ThumbnailType};



#[get("/api/image/{type}/{id}")]
async fn get_local_image(path: web::Path<(String, String)>) -> impl Responder {
	let (type_of, id) = path.into_inner();

	let path = crate::image::prefixhash_to_path(
		ThumbnailType::from(type_of.as_str()),
		&id
	);

	NamedFile::open_async(path).await
}


#[get("/api/posters/{meta_id}")]
async fn get_poster_list(
	path: web::Path<usize>,
	db: web::Data<Database>
) -> web::Json<api::GetPostersResponse> {
	let meta = db.get_metadata_by_id(*path).unwrap().unwrap();

	// TODO: For Open Library we need to go from an Edition to Work.
	// Work is the main book. Usually consisting of more posters.
	// We can do they by works[0].key = "/works/OLXXXXXXW"

	let mut items: Vec<Poster> = db.get_posters_by_linked_id(*path)
		.unwrap()
		.into_iter()
		.map(|poster| Poster {
			id: Some(poster.id),

			selected: poster.path == meta.thumb_path,

			path: {
				let (prefix, suffix) = poster.path.get_prefix_suffix().unwrap();
				format!("/api/image/{prefix}/{suffix}")
			},

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
	).await.unwrap();

	for item in search.into_values().flatten() {
		if let crate::metadata::SearchItem::Book(item) = item {
			for path in item.thumb_locations {
				items.push(Poster {
					id: None,

					selected: false,
					path: path.into_value(),

					created_at: Utc::now(),
				});
			}
		}
	}

	web::Json(api::GetPostersResponse {
		items
	})
}


#[post("/api/posters/{meta_id}")]
async fn post_change_poster(
	metadata_id: web::Path<usize>,
	body: web::Json<api::ChangePosterBody>,
	db: web::Data<Database>
) -> HttpResponse {
	let mut meta = db.get_metadata_by_id(*metadata_id).unwrap().unwrap();

	match body.into_inner().url_or_id {
		Either::Left(url) => {
			let resp = reqwest::get(url)
				.await
				.unwrap()
				.bytes()
				.await
				.unwrap();

			let hash = store_image(ThumbnailType::Metadata, resp.to_vec()).await.unwrap();


			meta.thumb_path = hash.into();

			db.add_poster(&NewPoster {
				link_id: meta.id,
				path: meta.thumb_path.clone(),
				created_at: Utc::now(),
			}).unwrap();
		}

		Either::Right(id) => {
			let poster = db.get_poster_by_id(id).unwrap().unwrap();

			if meta.thumb_path == poster.path {
				return HttpResponse::Ok().finish();
			}

			meta.thumb_path = poster.path;
		}
	}

	db.update_metadata(&meta).unwrap();

	HttpResponse::Ok().finish()
}


#[put("/api/posters/{meta_id}")]
async fn put_upload_poster(
	path: web::Path<usize>,
	// body: web::Payload,
	db: web::Data<Database>
) -> HttpResponse {
	//

	HttpResponse::Ok().finish()
}