use std::io::Read;

use actix_web::{get, web, HttpResponse, post, delete};

use common_local::{Chapter, api, Progression, FileId};
use bookie::Book;
use common::MemberId;
use futures::TryStreamExt;

use crate::model::file::FileModel;
use crate::model::note::FileNoteModel;
use crate::model::progress::FileProgressionModel;
use crate::{WebResult, Error, Result};
use crate::database::Database;
use crate::http::MemberCookie;



// Load Book Resources

#[get("/file/{id}/res/{tail:.*}")]
pub async fn load_file_resource(
	path: web::Path<(FileId, String)>,
	res: web::Query<api::LoadResourceQuery>,
	db: web::Data<Database>
) -> WebResult<HttpResponse> {
	let (file_id, resource_path) = path.into_inner();

	let file = FileModel::find_one_by_id(file_id, &db).await?.unwrap();

	let mut book = bookie::load_from_path(&file.path)?.unwrap();

	// TODO: Check if we're loading a section
	if res.configure_pages {
		let body = match book.read_path_as_bytes(
			&resource_path,
			Some(&format!("/api/file/{}/res", file_id)),
			Some(&[include_str!("../../../../../app/book_stylings.css")])
		) {
			Ok(v) => v,
			Err(e) => {
				eprintln!("{}", e);
				Vec::new()
			}
		};

		Ok(HttpResponse::Ok().insert_header(("Content-Type","application/xhtml+xml")).body(body))
	} else {
		let body = match book.read_path_as_bytes(
			&resource_path,
			None,
			None
		) {
			Ok(v) => v,
			Err(e) => {
				eprintln!("{}", e);
				Vec::new()
			}
		};

		Ok(HttpResponse::Ok().body(body))
	}
}


#[get("/file/{id}/pages/{pages}")]
pub async fn load_file_pages(path: web::Path<(FileId, String)>, db: web::Data<Database>) -> WebResult<web::Json<api::ApiGetBookPagesByIdResponse>> {
	let (file_id, chapters) = path.into_inner();

	let file = FileModel::find_one_by_id(file_id, &db).await?.unwrap();

	let mut book = bookie::load_from_path(&file.path)?.unwrap();

	let (start_chap, end_chap) = chapters
		.split_once('-')
		.map_or_else(
			|| {
				let chap = chapters.parse::<usize>()?;
				Result::Ok((chap, chap))
			},
			|(a, b)| {
				let start_chap = a.parse::<usize>()?;
				let chap_count = book.chapter_count();
				Result::Ok((start_chap, if b.trim().is_empty() { chap_count } else { chap_count.min(b.parse()?) }))
			}
		)?;

	let mut items = Vec::new();

	for chap in start_chap..end_chap {
		book.set_chapter(chap);

		// TODO: Return file names along with Chapter. Useful for redirecting to certain chapter for <a> tags.

		items.push(Chapter {
			file_path: book.get_page_path(),
			value: chap,
		});
	}

	Ok(web::Json(api::GetChaptersResponse {
		offset: start_chap,
		limit: end_chap - start_chap,
		total: book.chapter_count(),
		items
	}))
}


// TODO: Add body requests for specifics
#[get("/file/{id}")]
pub async fn load_file(file_id: web::Path<FileId>, db: web::Data<Database>) -> WebResult<web::Json<Option<api::GetBookIdResponse>>> {
	Ok(web::Json(if let Some(file) = FileModel::find_one_by_id(*file_id, &db).await? {
		Some(api::GetBookIdResponse {
			progress: FileProgressionModel::find_one(MemberId::none(), *file_id, &db).await?.map(|v| v.into()),

			media: file.into()
		})
	} else {
		None
	}))
}


#[get("/file/{id}/debug/{tail:.*}")]
pub async fn load_file_debug(web_path: web::Path<(FileId, String)>, db: web::Data<Database>) -> WebResult<HttpResponse> {
	if let Some(file) = FileModel::find_one_by_id(web_path.0, &db).await? {
		if web_path.1.is_empty() {
			let book = bookie::epub::EpubBook::load_from_path(&file.path)?;

			Ok(HttpResponse::Ok().body(
				book.container.file_names_in_archive()
				.map(|v| format!("<a href=\"{}\">{}</a>", v, v))
				.collect::<Vec<_>>()
				.join("<br/>")
			))
		} else {
			// TODO: Make bookie::load_from_path(&file.path).unwrap();
			let mut book = bookie::epub::EpubBook::load_from_path(&file.path)?;

			// Init Package Document
			let mut file = book.container.archive.by_name(&web_path.1).unwrap();

			let mut data = Vec::new();
			file.read_to_end(&mut data).map_err(Error::from)?;

			Ok(HttpResponse::Ok().body(data))
		}
	} else {
		Ok(HttpResponse::Ok().body("Unable to find file from ID"))
	}
}


// Progress

#[post("/file/{id}/progress")]
pub async fn progress_file_add(
	file_id: web::Path<FileId>,
	body: web::Json<Progression>,
	member: MemberCookie,
	db: web::Data<Database>,
) -> WebResult<HttpResponse> {
	FileProgressionModel::insert_or_update(member.member_id(), *file_id, body.into_inner(), &db).await?;
	Ok(HttpResponse::Ok().finish())
}

#[delete("/file/{id}/progress")]
pub async fn progress_file_delete(
	file_id: web::Path<FileId>,
	member: MemberCookie,
	db: web::Data<Database>,
) -> WebResult<HttpResponse> {
	FileProgressionModel::delete_one(member.member_id(), *file_id, &db).await?;
	Ok(HttpResponse::Ok().finish())
}


// Notes

#[get("/file/{id}/notes")]
pub async fn notes_file_get(
	file_id: web::Path<FileId>,
	member: MemberCookie,
	db: web::Data<Database>,
) -> WebResult<web::Json<api::ApiGetBookNotesByIdResponse>> {
	let v = FileNoteModel::find_one(*file_id, member.member_id(), &db).await?;
	Ok(web::Json(v.map(|v| v.data)))
}

#[post("/file/{id}/notes")]
pub async fn notes_file_add(
	file_id: web::Path<FileId>,
	mut payload: web::Payload,
	member: MemberCookie,
	db: web::Data<Database>,
) -> WebResult<HttpResponse> {
	let mut body = web::BytesMut::new();
	while let Some(chunk) = payload.try_next().await? {
		body.extend_from_slice(&chunk);
	}

	let data = unsafe { String::from_utf8_unchecked(body.to_vec()) };

	FileNoteModel::new(*file_id, member.member_id(), data)
		.insert_or_update(&db).await?;

	Ok(HttpResponse::Ok().finish())
}

#[delete("/file/{id}/notes")]
pub async fn notes_file_delete(
	file_id: web::Path<FileId>,
	member: MemberCookie,
	db: web::Data<Database>,
) -> WebResult<HttpResponse> {
	FileNoteModel::delete_one(*file_id, member.member_id(), &db).await?;

	Ok(HttpResponse::Ok().finish())
}