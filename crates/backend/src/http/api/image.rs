use actix_web::{get, web};
use books_common::{api, Poster};

use crate::database::Database;



#[get("/api/posters/{meta_id}")]
async fn get_poster_list(
	path: web::Path<i64>,
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

			path: poster.path.into_url_thumb(poster.link_id),

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
			for path in item.all_thumb_urls {
				items.push(Poster {
					id: None,

					path,

					created_at: item.created_at,
				});
			}
		}
	}

	web::Json(api::GetPostersResponse {
		items
	})
}

