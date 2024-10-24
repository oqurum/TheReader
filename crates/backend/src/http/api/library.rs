use actix_web::{get, post, web};
use common::api::{ApiErrorResponse, WrappingResponse};
use common_local::{api, LibraryColl, LibraryId};

use crate::{
    http::{JsonResponse, MemberCookie},
    model::{DirectoryModel, LibraryModel},
    SqlPool, WebResult,
};

#[get("/libraries")]
async fn load_library_list(
    member: MemberCookie,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<api::ApiGetLibrariesResponse>> {
    let member = member.fetch_or_error(&mut *db.acquire().await?).await?;
    let lib_access = member.parse_library_access_or_default()?;

    Ok(web::Json(WrappingResponse::okay(
        api::GetLibrariesResponse {
            items: LibraryModel::get_all(&mut *db.acquire().await?)
                .await?
                .into_iter()
                .filter_map(|lib| {
                    if member.permissions.is_owner()
                        || lib_access.is_accessible(lib.id, lib.is_public)
                    {
                        Some(LibraryColl {
                            id: lib.id,

                            name: lib.name,
                            type_of: lib.type_of,

                            is_public: lib.is_public,
                            settings: lib.settings,

                            created_at: lib.created_at.timestamp_millis(),
                            scanned_at: lib.scanned_at.timestamp_millis(),
                            updated_at: lib.updated_at.timestamp_millis(),

                            directories: Vec::new(),
                        })
                    } else {
                        None
                    }
                })
                .collect(),
        },
    )))
}

#[get("/library/{id}")]
async fn load_library_id(
    id: web::Path<LibraryId>,
    member: MemberCookie,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<api::ApiGetLibraryIdResponse>> {
    let model = LibraryModel::find_one_by_id(*id, &mut *db.acquire().await?)
        .await?
        .ok_or_else(|| crate::Error::from(crate::InternalError::ItemMissing))?;

    let member = member.fetch_or_error(&mut *db.acquire().await?).await?;
    let lib_access = member.parse_library_access_or_default()?;

    if !member.permissions.is_owner() && !lib_access.is_accessible(model.id, model.is_public) {
        return Err(ApiErrorResponse::new("Not accessible").into());
    }

    let directories =
        DirectoryModel::find_directories_by_library_id(*id, &mut *db.acquire().await?).await?;

    let library = LibraryColl {
        id: model.id,

        name: model.name,
        type_of: model.type_of,

        is_public: model.is_public,
        settings: model.settings,

        created_at: model.created_at.timestamp_millis(),
        scanned_at: model.scanned_at.timestamp_millis(),
        updated_at: model.updated_at.timestamp_millis(),

        directories: directories.into_iter().map(|v| v.path).collect(),
    };

    Ok(web::Json(WrappingResponse::okay(library)))
}

#[post("/library/{id}")]
async fn update_library_id(
    id: web::Path<LibraryId>,
    body: web::Json<api::UpdateLibrary>,
    member: MemberCookie,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<&'static str>> {
    let body = body.into_inner();

    let member = member.fetch_or_error(&mut *db.acquire().await?).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    let mut model = LibraryModel::find_one_by_id(*id, &mut *db.acquire().await?)
        .await?
        .ok_or_else(|| crate::Error::from(crate::InternalError::ItemMissing))?;

    let mut is_updated = false;

    // TODO: Update Directories.

    if let Some(name) = body.name {
        // TODO: Add Checks
        model.name = name;
        is_updated = true;
    }

    if let Some(is_public) = body.is_public {
        model.is_public = is_public;
        is_updated = true;
    }

    if !body.remove_directories.is_empty() {
        // TODO: Don't trust that the path is correct. Also remove slashes at the end of path.
        for path in body.remove_directories {
            DirectoryModel::remove_by_path(&path, &mut *db.acquire().await?).await?;
        }
    }

    if !body.add_directories.is_empty() {
        // TODO: Don't trust that the path is correct. Also remove slashes at the end of path.
        for path in body.add_directories {
            DirectoryModel {
                library_id: *id,
                path,
            }
            .insert(&mut *db.acquire().await?)
            .await?;
        }
    }

    if is_updated {
        model.update(&mut *db.acquire().await?).await?;
    }

    Ok(web::Json(WrappingResponse::okay("ok")))
}
