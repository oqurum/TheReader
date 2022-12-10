use actix_web::{get, post, web};
use common::api::WrappingResponse;
use common_local::{api, LibraryColl, LibraryId};

use crate::{
    database::Database,
    http::JsonResponse,
    model::{directory::DirectoryModel, library::LibraryModel},
    WebResult,
};

#[get("/libraries")]
async fn load_library_list(
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetLibrariesResponse>> {
    Ok(web::Json(WrappingResponse::okay(
        api::GetLibrariesResponse {
            items: LibraryModel::get_all(&db.basic())
                .await?
                .into_iter()
                .map(|file| LibraryColl {
                    id: file.id,

                    name: file.name,

                    created_at: file.created_at.timestamp_millis(),
                    scanned_at: file.scanned_at.timestamp_millis(),
                    updated_at: file.updated_at.timestamp_millis(),

                    directories: Vec::new(),
                })
                .collect(),
        },
    )))
}

#[get("/library/{id}")]
async fn load_library_id(
    id: web::Path<LibraryId>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetLibraryIdResponse>> {
    let model = LibraryModel::find_one_by_id(*id, &db.basic())
        .await?
        .ok_or_else(|| crate::Error::from(crate::InternalError::ItemMissing))?;

    let directories = DirectoryModel::find_directories_by_library_id(*id, &db.basic()).await?;

    let library = LibraryColl {
        id: model.id,

        name: model.name,

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
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    let body = body.into_inner();

    let mut model = LibraryModel::find_one_by_id(*id, &db.basic())
        .await?
        .ok_or_else(|| crate::Error::from(crate::InternalError::ItemMissing))?;

    let mut is_updated = false;

    // TODO: Update Directories.

    if let Some(name) = body.name {
        // TODO: Add Checks
        model.name = name;
        is_updated = true;
    }

    if !body.remove_directories.is_empty() {
        // TODO: Don't trust that the path is correct. Also remove slashes at the end of path.
        for path in body.remove_directories {
            DirectoryModel::remove_by_path(&path, &db.basic()).await?;
        }
    }

    if !body.add_directories.is_empty() {
        // TODO: Don't trust that the path is correct. Also remove slashes at the end of path.
        for path in body.add_directories {
            DirectoryModel {
                library_id: *id,
                path,
            }
            .insert(&db.basic())
            .await?;
        }
    }

    if is_updated {
        model.update(&db.basic()).await?;
    }

    Ok(web::Json(WrappingResponse::okay("ok")))
}
