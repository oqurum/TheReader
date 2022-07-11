use actix_web::{HttpResponse, web, get, post};
use books_common::{setup::SetupConfig, api};

use crate::{database::Database, WebResult, config::does_config_exist};



#[get("/setup")]
pub async fn is_setup() -> web::Json<api::ApiGetIsSetupResponse> {
	web::Json(does_config_exist())
}


#[post("/setup")]
pub async fn save_initial_setup(
	body: web::Json<SetupConfig>,
	_db: web::Data<Database>,
) -> WebResult<HttpResponse> {
	crate::config::save_config(body.into_inner()).await?;

	Ok(HttpResponse::Ok().finish())
}