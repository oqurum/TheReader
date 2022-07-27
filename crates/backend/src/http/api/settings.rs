use actix_web::{HttpResponse, web, get, post};
use books_common::{setup::SetupConfig, api};

use crate::{database::Database, WebResult, config::does_config_exist, http::passwordless::test_connection};



#[get("/setup")]
pub async fn is_setup() -> web::Json<api::ApiGetIsSetupResponse> {
	web::Json(does_config_exist())
}


#[post("/setup")]
pub async fn save_initial_setup(
	body: web::Json<SetupConfig>,
	_db: web::Data<Database>,
) -> WebResult<HttpResponse> {
	let config = body.into_inner();

	if let Some(email_config) = config.email.as_ref() {
		if !test_connection(email_config)? {
			return Ok(HttpResponse::InternalServerError().body("Test Connection Failed"));
		}
	}

	crate::config::save_config(config).await?;

	Ok(HttpResponse::Ok().finish())
}