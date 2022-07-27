use actix_web::{web, get, post};
use books_common::{setup::SetupConfig, api};
use chrono::Utc;

use crate::{database::Database, WebResult, config::does_config_exist, http::passwordless::test_connection, model::{library::NewLibraryModel, directory::DirectoryModel}};



#[get("/setup")]
pub async fn is_setup() -> web::Json<api::ApiGetIsSetupResponse> {
	web::Json(does_config_exist())
}


#[post("/setup")]
pub async fn save_initial_setup(
	body: web::Json<SetupConfig>,
	db: web::Data<Database>,
) -> WebResult<web::Json<api::WrappingResponse<String>>> {
	let config = body.into_inner();

	if let Some(email_config) = config.email.as_ref() {
		if !test_connection(email_config)? {
			return Ok(web::Json(api::WrappingResponse::error("Test Connection Failed")));
		}
	}

	for path in &config.directories {
		let now = Utc::now();

		let lib = NewLibraryModel {
			name: format!("New Library {}", now.timestamp_millis()),
			created_at: now,
			scanned_at: now,
			updated_at: now,
		}.insert(&db).await?;

		// TODO: Don't trust that the path is correct. Also remove slashes at the end of path.
		DirectoryModel { library_id: lib.id, path: path.clone() }.insert(&db).await?;
	}

	crate::config::save_config(config).await?;

	Ok(web::Json(api::WrappingResponse::okay(String::new())))
}