use actix_web::{get, web, post, delete};
use common_local::{api, LibraryColl, util::take_from_and_swap};
use chrono::Utc;
use common::api::{WrappingResponse, ApiErrorResponse};

use crate::{database::Database, WebResult, model::{library::{LibraryModel, NewLibraryModel}, directory::DirectoryModel}, http::{MemberCookie, JsonResponse}};


#[get("/options")]
async fn load_options(db: web::Data<Database>) -> WebResult<web::Json<api::ApiGetOptionsResponse>> {
	let libraries = LibraryModel::get_all(&db).await?;
	let mut directories = DirectoryModel::get_all(&db).await?;

	Ok(web::Json(api::GetOptionsResponse {
		libraries: libraries.into_iter()
			.map(|lib| {
				LibraryColl {
					id: lib.id,
					name: lib.name,
					scanned_at: lib.scanned_at.timestamp_millis(),
					created_at: lib.created_at.timestamp_millis(),
					updated_at: lib.updated_at.timestamp_millis(),
					directories: take_from_and_swap(&mut directories, |v| v.library_id == lib.id)
						.into_iter()
						.map(|v| v.path)
						.collect()
				}
			})
			.collect()
	}))
}

#[post("/options")]
async fn update_options_add(
	modify: web::Json<api::ModifyOptionsBody>,
	member: MemberCookie,
	db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
	let member = member.fetch_or_error(&db).await?;

	if !member.permissions.is_owner() {
		return Err(ApiErrorResponse::new("Not owner").into());
	}

	let api::ModifyOptionsBody { library } = modify.into_inner();

	if let Some(mut library) = library {
		if let Some(name) = library.name {
			let lib = NewLibraryModel {
				name,
				created_at: Utc::now(),
				scanned_at: Utc::now(),
				updated_at: Utc::now(),
			}.insert(&db).await?;

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
				DirectoryModel { library_id, path }.insert(&db).await?;
			}
		}
	}

	Ok(web::Json(WrappingResponse::okay("success")))
}

#[delete("/options")]
async fn update_options_remove(
	modify: web::Json<api::ModifyOptionsBody>,
	member: MemberCookie,
	db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
	let member = member.fetch_or_error(&db).await?;

	if !member.permissions.is_owner() {
		return Err(ApiErrorResponse::new("Not owner").into());
	}

	let api::ModifyOptionsBody { library } = modify.into_inner();

	if let Some(library) = library {
		if let Some(id) = library.id {
			LibraryModel::delete_by_id(id, &db).await?;
		}

		if let Some(directory) = library.directories {
			for path in directory {
				DirectoryModel::remove_by_path(&path, &db).await?;
			}
		}
	}

	Ok(web::Json(WrappingResponse::okay("success")))
}