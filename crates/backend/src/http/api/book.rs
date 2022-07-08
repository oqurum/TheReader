use std::io::Read;

use actix_web::{get, web, HttpResponse, post, delete};

use books_common::{Chapter, api, Progression, DisplayItem, FileId};
use bookie::Book;
use common::MemberId;
use futures::TryStreamExt;

use crate::{WebResult, Error, Result};
use crate::database::Database;
use crate::http::MemberCookie;



// Load Book Resources

#[get("/book/{id}/res/{tail:.*}")]
pub async fn load_resource(
	path: web::Path<(FileId, String)>,
	res: web::Query<api::LoadResourceQuery>,
	db: web::Data<Database>
) -> WebResult<HttpResponse> {
	let (file_id, resource_path) = path.into_inner();

	let file = db.find_file_by_id(file_id)?.unwrap();

	let mut book = bookie::load_from_path(&file.path)?.unwrap();

	// TODO: Check if we're loading a section
	if res.configure_pages {
		let body = match book.read_path_as_bytes(
			&resource_path,
			Some(&format!("/api/book/{}/res", file_id)),
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


#[get("/book/{id}/pages/{pages}")]
pub async fn load_pages(path: web::Path<(FileId, String)>, db: web::Data<Database>) -> WebResult<web::Json<api::ApiGetBookPagesByIdResponse>> {
	let (file_id, chapters) = path.into_inner();

	let file = db.find_file_by_id(file_id)?.unwrap();

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
#[get("/book/{id}")]
pub async fn load_book(file_id: web::Path<FileId>, db: web::Data<Database>) -> WebResult<web::Json<Option<api::GetBookIdResponse>>> {
	Ok(web::Json(if let Some(file) = db.find_file_by_id(*file_id)? {
		Some(api::GetBookIdResponse {
			progress: db.get_progress(MemberId::none(), *file_id)?.map(|v| v.into()),

			media: file.into()
		})
	} else {
		None
	}))
}


#[get("/book/{id}/debug/{tail:.*}")]
pub async fn load_book_debug(web_path: web::Path<(FileId, String)>, db: web::Data<Database>) -> WebResult<HttpResponse> {
	if let Some(file) = db.find_file_by_id(web_path.0)? {
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

#[post("/book/{id}/progress")]
pub async fn progress_book_add(
	file_id: web::Path<FileId>,
	body: web::Json<Progression>,
	db: web::Data<Database>,
	member: MemberCookie,
) -> WebResult<HttpResponse> {
	db.add_or_update_progress(member.member_id(), *file_id, body.into_inner())?;
	Ok(HttpResponse::Ok().finish())
}

#[delete("/book/{id}/progress")]
pub async fn progress_book_delete(
	file_id: web::Path<FileId>,
	db: web::Data<Database>,
	member: MemberCookie,
) -> WebResult<HttpResponse> {
	db.delete_progress(member.member_id(), *file_id)?;
	Ok(HttpResponse::Ok().finish())
}


// Notes

#[get("/book/{id}/notes")]
pub async fn notes_book_get(
	file_id: web::Path<FileId>,
	db: web::Data<Database>,
	member: MemberCookie,
) -> WebResult<web::Json<api::ApiGetBookNotesByIdResponse>> {
	let v = db.get_notes(member.member_id(), *file_id)?;
	Ok(web::Json(v.map(|v| v.data)))
}

#[post("/book/{id}/notes")]
pub async fn notes_book_add(
	file_id: web::Path<FileId>,
	mut payload: web::Payload,
	db: web::Data<Database>,
	member: MemberCookie,
) -> WebResult<HttpResponse> {
	let mut body = web::BytesMut::new();
	while let Some(chunk) = payload.try_next().await? {
		body.extend_from_slice(&chunk);
	}

	let data = unsafe { String::from_utf8_unchecked(body.to_vec()) };

	db.add_or_update_notes(member.member_id(), *file_id, data)?;

	Ok(HttpResponse::Ok().finish())
}

#[delete("/book/{id}/notes")]
pub async fn notes_book_delete(
	file_id: web::Path<FileId>,
	db: web::Data<Database>,
	member: MemberCookie,
) -> WebResult<HttpResponse> {
	db.delete_notes(member.member_id(), *file_id)?;

	Ok(HttpResponse::Ok().finish())
}


#[get("/books")]
pub async fn load_book_list(
	query: web::Query<api::BookListQuery>,
	db: web::Data<Database>,
) -> WebResult<web::Json<api::ApiGetBookListResponse>> {
	let (items, count) = if let Some(search) = query.search_query() {
		let search = search?;

		let count = db.count_search_metadata(&search, query.library)?;

		let items = if count == 0 {
			Vec::new()
		} else {
			db.search_metadata_list(
				&search,
				query.library,
				query.offset.unwrap_or(0),
				query.limit.unwrap_or(50),
			)?
				.into_iter()
				.map(|meta| {
					DisplayItem {
						id: meta.id,
						title: meta.title.or(meta.original_title).unwrap_or_default(),
						cached: meta.cached,
						has_thumbnail: meta.thumb_path.is_some()
					}
				})
				.collect()
		};

		(items, count)
	} else {
		let count = db.get_file_count()?;

		let items = db.get_metadata_by(
			query.library,
			query.offset.unwrap_or(0),
			query.limit.unwrap_or(50),
		)?
			.into_iter()
			.map(|meta| {
				DisplayItem {
					id: meta.id,
					title: meta.title.or(meta.original_title).unwrap_or_default(),
					cached: meta.cached,
					has_thumbnail: meta.thumb_path.is_some()
				}
			})
			.collect();

		(items, count)
	};

	Ok(web::Json(api::GetBookListResponse {
		items,
		count,
	}))
}