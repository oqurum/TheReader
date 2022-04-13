use actix_web::{get, web};
use books_common::{api, Poster};

use crate::database::Database;



#[get("/api/posters/{meta_id}")]
async fn get_poster_list(
	path: web::Path<i64>,
	db: web::Data<Database>
) -> web::Json<api::GetPostersResponse> {
	web::Json(api::GetPostersResponse {
		items: db.get_posters_by_linked_id(*path)
			.unwrap()
			.into_iter()
			.map(|poster| Poster {
				id: Some(poster.id),

				path: poster.path.into_url_thumb(poster.link_id),

				created_at: poster.created_at,
			})
			.collect()
	})
}

