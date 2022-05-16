use actix_web::{HttpResponse, web, get, post};
use books_common::setup::SetupConfig;

use crate::{database::Database, WebResult};




#[get("/setup")]
pub async fn is_setup() -> web::Json<bool> {
	web::Json(crate::config::does_config_exist())
}


#[post("/setup")]
pub async fn save_initial_setup(
	body: web::Json<SetupConfig>,
	db: web::Data<Database>,
) -> WebResult<HttpResponse> {
	//

	Ok(HttpResponse::Ok().finish())
}