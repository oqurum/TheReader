use actix_web::{get, web};
use common::api::WrappingResponse;
use common_local::{api, LibraryColl};

use crate::{database::Database, WebResult, model::library::LibraryModel, http::JsonResponse};



#[get("/libraries")]
async fn load_library_list(db: web::Data<Database>) -> WebResult<JsonResponse<api::ApiGetLibrariesResponse>> {
	Ok(web::Json(WrappingResponse::okay(api::GetLibrariesResponse {
		items: LibraryModel::get_all(&db).await?
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
	})))
}

