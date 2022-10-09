use actix_web::{get, web};
use common_local::api;
use common::api::{ApiErrorResponse, WrappingResponse};
use serde::{Deserialize, Serialize};

use crate::{http::{MemberCookie, JsonResponse}, database::Database, WebResult, config::get_config, Error};

#[derive(Serialize, Deserialize)]
pub struct GetDirectoryQuery {
    pub path: String,
}


#[get("/directory")]
pub async fn get_directory(
    query: web::Query<GetDirectoryQuery>,
    member: Option<MemberCookie>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<Vec<api::DirectoryEntry>>> {
    if let Some(member) = &member {
        let member = member.fetch_or_error(&db).await?;

        if !member.permissions.is_owner() {
            return Err(ApiErrorResponse::new("Not owner").into());
        }
    } else if get_config().has_admin_account {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    Ok(web::Json(WrappingResponse::okay(
        std::fs::read_dir(&query.path)
            .map_err(Error::from)?
            .filter_map(|v| {
                let item = v.ok()?;

                Some(api::DirectoryEntry {
                    title: item.file_name().to_string_lossy().into_owned(),
                    path: item.path(),
                    is_file: item.metadata().ok()?.is_file(),
                })
            })
            .collect()
    )))
}
