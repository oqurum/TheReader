use std::{path::{PathBuf, Component, Path, MAIN_SEPARATOR}, ffi::OsStr};

use actix_web::{get, web};
use common_local::api::{self, GetDirectoryQuery};
use common::api::{ApiErrorResponse, WrappingResponse};

use crate::{http::{MemberCookie, JsonResponse}, database::Database, WebResult, config::get_config, Error};



#[get("/directory")]
pub async fn get_directory(
    query: web::Query<GetDirectoryQuery>,
    member: Option<MemberCookie>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetDirectoryResponse>> {
    if let Some(member) = &member {
        let member = member.fetch_or_error(&db).await?;

        if !member.permissions.is_owner() {
            return Err(ApiErrorResponse::new("Not owner").into());
        }
    } else if get_config().has_admin_account {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    let mut path = PathBuf::from(query.path.as_str());

    // We're using this to check if we're on windows to add the Drive Prefix to the directory checks.
    if cfg!(windows) {
        // Does our current path not have a prefix?
        if get_path_prefix(&path).is_none() {
            let curr_dir = std::env::current_dir().map_err(crate::Error::from)?;

            // Does our current_dir check have a disk prefix (C:)
            if let Some(disk_name) = get_path_prefix(&curr_dir) {
                let mut fixed_path = PathBuf::with_capacity(path.capacity() + 1);
                fixed_path.push(disk_name);

                if !path.is_absolute() {
                    fixed_path.push(MAIN_SEPARATOR.to_string());
                }

                fixed_path.extend(path.iter());
                path = fixed_path;
            }
        }
    }


    Ok(web::Json(WrappingResponse::okay(
        api::GetDirectoryResponse {
            items: std::fs::read_dir(&path)
                .map_err(Error::from)?
                .filter_map(|v| {
                    let item = v.ok()?;

                    Some(api::DirectoryEntry {
                        title: item.file_name().to_string_lossy().into_owned(),
                        path: item.path(),
                        is_file: item.metadata().ok()?.is_file(),
                    })
                })
                .collect(),

            path,
        }
    )))
}


fn get_path_prefix(value: &Path) -> Option<&OsStr> {
    if let Some(Component::Prefix(pc)) = value.components().next() {
        Some(pc.as_os_str())
    } else {
        None
    }
}