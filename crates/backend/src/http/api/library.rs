use actix_web::{get, web};
use books_common::{api, LibraryColl};

use crate::database::Database;



#[get("/api/libraries")]
async fn load_library_list(db: web::Data<Database>) -> web::Json<api::GetLibrariesResponse> {
	web::Json(api::GetLibrariesResponse {
		items: db.list_all_libraries()
			.unwrap()
			.into_iter()
			.map(|file| {
				LibraryColl {
					id: file.id,

					name: file.name,

					created_at: file.created_at.timestamp_millis(),
					scanned_at: file.scanned_at.timestamp_millis(),
					updated_at: file.updated_at.timestamp_millis(),

					directories: Vec::new()
				}
			})
			.collect()
	})
}

