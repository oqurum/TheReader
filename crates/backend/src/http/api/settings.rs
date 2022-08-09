use actix_web::{web, get, post};
use common_local::{setup::SetupConfig, api};
use chrono::Utc;
use common::api::{ApiErrorResponse, WrappingResponse};

use crate::{database::Database, WebResult, config::does_config_exist, http::{passwordless::test_connection, MemberCookie, JsonResponse}, model::{library::NewLibraryModel, directory::DirectoryModel}};



#[get("/setup")]
pub async fn is_setup(
	member: Option<MemberCookie>,
	db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetIsSetupResponse>> {
	if let Some(member) = member.as_ref() {
		let member = member.fetch_or_error(&db).await?;

		if !member.permissions.is_owner() {
			return Err(ApiErrorResponse::new("Not owner").into());
		}

		Ok(web::Json(WrappingResponse::okay(does_config_exist())))
	} else {
		Ok(web::Json(WrappingResponse::okay(false)))
	}
}


#[post("/setup")]
pub async fn save_initial_setup(
	body: web::Json<SetupConfig>,
	member: Option<MemberCookie>,
	db: web::Data<Database>,
) -> WebResult<JsonResponse<String>> {
	if let Some(member) = member {
		let member = member.fetch_or_error(&db).await?;

		if !member.permissions.is_owner() {
			return Err(ApiErrorResponse::new("Not owner").into());
		}
	} else if !does_config_exist() {
		return Err(ApiErrorResponse::new("Not owner").into());
	}


	let config = body.into_inner();

	if let Some(email_config) = config.email.as_ref() {
		if !test_connection(email_config)? {
			return Ok(web::Json(WrappingResponse::error("Test Connection Failed")));
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

	Ok(web::Json(WrappingResponse::okay(String::new())))
}