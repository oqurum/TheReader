use std::io::Read;

use actix_web::{get, web, HttpResponse, post, delete};

use books_common::{Chapter, api, Progression, DisplayItem};
use bookie::Book;
use futures::TryStreamExt;

use crate::{WebResult, Error};
use crate::database::Database;
use crate::http::MemberCookie;



// Load Book Resources

#[get("/book/{id}/res/{tail:.*}")]
pub async fn load_resource(
	path: web::Path<(usize, String)>,
	res: web::Query<api::LoadResourceQuery>,
	db: web::Data<Database>
) -> WebResult<HttpResponse> {
	let (book_id, resource_path) = path.into_inner();

	let file = db.find_file_by_id(book_id)?.unwrap();

	let mut book = bookie::load_from_path(&file.path)?.unwrap();

	// TODO: Check if we're loading a section
	if res.configure_pages {
		let body = match book.read_path_as_bytes(
			&resource_path,
			Some(&format!("/book/{}/res", book_id)),
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
pub async fn load_pages(path: web::Path<(usize, String)>, db: web::Data<Database>) -> WebResult<web::Json<api::GetChaptersResponse>> {
	let (book_id, chapters) = path.into_inner();

	let file = db.find_file_by_id(book_id)?.unwrap();

	let mut book = bookie::load_from_path(&file.path)?.unwrap();

	let (start_chap, end_chap) = chapters
		.split_once('-')
		.map_or_else(
			|| {
				let chap = chapters.parse::<usize>().unwrap();
				(chap, chap)
			},
			|(a, b)| {
				let start_chap = a.parse::<usize>().unwrap();
				let chap_count = book.chapter_count();
				(start_chap, if b.trim().is_empty() { chap_count } else { chap_count.min(b.parse().unwrap()) })
			}
		);

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
pub async fn load_book(file_id: web::Path<usize>, db: web::Data<Database>) -> WebResult<web::Json<Option<api::GetBookIdResponse>>> {
	Ok(web::Json(if let Some(file) = db.find_file_by_id(*file_id)? {
		Some(api::GetBookIdResponse {
			progress: db.get_progress(0, *file_id)?.map(|v| v.into()),

			media: file.into()
		})
	} else {
		None
	}))
}


#[get("/book/{id}/debug/{tail:.*}")]
pub async fn load_book_debug(web_path: web::Path<(usize, String)>, db: web::Data<Database>) -> WebResult<HttpResponse> {
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
	file_id: web::Path<usize>,
	body: web::Json<Progression>,
	db: web::Data<Database>,
	member: MemberCookie,
) -> WebResult<HttpResponse> {
	db.add_or_update_progress(member.member_id(), *file_id, body.into_inner())?;
	Ok(HttpResponse::Ok().finish())
}

#[delete("/book/{id}/progress")]
pub async fn progress_book_delete(
	file_id: web::Path<usize>,
	db: web::Data<Database>,
	member: MemberCookie,
) -> WebResult<HttpResponse> {
	db.delete_progress(member.member_id(), *file_id)?;
	Ok(HttpResponse::Ok().finish())
}


// Notes

#[get("/book/{id}/notes")]
pub async fn notes_book_get(
	file_id: web::Path<usize>,
	db: web::Data<Database>,
	member: MemberCookie,
) -> WebResult<HttpResponse> {
	let v = db.get_notes(member.member_id(), *file_id)?;
	Ok(HttpResponse::Ok().body(v.map(|v| v.data).unwrap_or_default()))
}

#[post("/book/{id}/notes")]
pub async fn notes_book_add(
	file_id: web::Path<usize>,
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
	file_id: web::Path<usize>,
	db: web::Data<Database>,
	member: MemberCookie,
) -> WebResult<HttpResponse> {
	db.delete_notes(member.member_id(), *file_id)?;

	Ok(HttpResponse::Ok().finish())
}


// TODO: Add body requests for specific books
#[get("/books")]
pub async fn load_book_list(db: web::Data<Database>, query: web::Query<api::BookListQuery>) -> WebResult<web::Json<api::GetBookListResponse>> {
	Ok(web::Json(api::GetBookListResponse {
		count: db.get_file_count()?,
		items: db.get_metadata_by(query.library, query.offset.unwrap_or(0), query.limit.unwrap_or(50))?
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
	}))
}