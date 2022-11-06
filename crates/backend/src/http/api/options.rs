use actix_web::{delete, get, post, web};
use chrono::Utc;
use common::api::{ApiErrorResponse, WrappingResponse};
use common_local::{api, util::take_from_and_swap, LibraryColl};

use crate::{
    config::{get_config, save_config, update_config},
    database::Database,
    http::{JsonResponse, MemberCookie},
    model::{
        directory::DirectoryModel,
        library::{LibraryModel, NewLibraryModel},
    },
    WebResult,
};

#[get("/options")]
async fn load_options(
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetOptionsResponse>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    let libraries = LibraryModel::get_all(&db.basic()).await?;
    let mut directories = DirectoryModel::get_all(&db.basic()).await?;

    Ok(web::Json(WrappingResponse::okay(api::GetOptionsResponse {
        libraries: libraries
            .into_iter()
            .map(|lib| LibraryColl {
                id: lib.id,
                name: lib.name,
                scanned_at: lib.scanned_at.timestamp_millis(),
                created_at: lib.created_at.timestamp_millis(),
                updated_at: lib.updated_at.timestamp_millis(),
                directories: take_from_and_swap(&mut directories, |v| v.library_id == lib.id)
                    .into_iter()
                    .map(|v| v.path)
                    .collect(),
            })
            .collect(),

        config: member.permissions.is_owner().then(|| {
            let mut config = get_config();

            config.email = None;
            config.libby.token = config.libby.token.map(|_| String::new());
            config.server.auth_key.clear();

            config
        }),
    })))
}

#[post("/options")]
async fn update_options_add(
    modify: web::Json<api::ModifyOptionsBody>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    let api::ModifyOptionsBody {
        library,
        libby_public_search,
    } = modify.into_inner();

    if let Some(mut library) = library {
        if let Some(name) = library.name {
            let lib = NewLibraryModel {
                name,
                created_at: Utc::now(),
                scanned_at: Utc::now(),
                updated_at: Utc::now(),
            }
            .insert(&db.basic())
            .await?;

            // TODO: Properly handle.
            if let Some(id) = library.id {
                if id != lib.id {
                    panic!("POST /options error. library id is already set and different than the new library.");
                }
            }

            library.id = Some(lib.id);
        }

        if let Some((directories, library_id)) = library.directories.zip(library.id) {
            // TODO: Don't trust that the path is correct. Also remove slashes at the end of path.
            for path in directories {
                DirectoryModel { library_id, path }
                    .insert(&db.basic())
                    .await?;
            }
        }
    }

    if let Some(libby_search) = libby_public_search {
        update_config(move |config| {
            config.libby.public_only = libby_search;

            Ok(())
        })?;

        save_config().await?;
    }

    Ok(web::Json(WrappingResponse::okay("success")))
}

#[delete("/options")]
async fn update_options_remove(
    modify: web::Json<api::ModifyOptionsBody>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    let api::ModifyOptionsBody { library, .. } = modify.into_inner();

    if let Some(library) = library {
        if let Some(id) = library.id {
            LibraryModel::delete_by_id(id, &db.basic()).await?;
        }

        if let Some(directory) = library.directories {
            for path in directory {
                DirectoryModel::remove_by_path(&path, &db.basic()).await?;
            }
        }
    }

    Ok(web::Json(WrappingResponse::okay("success")))
}
