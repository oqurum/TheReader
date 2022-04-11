use std::io::Read;

use actix_web::{get, web, HttpResponse, post, delete};

use books_common::{Chapter, api, Progression, DisplayItem};
use bookie::Book;
use futures::TryStreamExt;

use crate::database::Database;



// Load Book Resources

#[get("/api/book/{id}/res/{tail:.*}")]
pub async fn load_resource(
	path: web::Path<(i64, String)>,
	res: web::Query<api::LoadResourceQuery>,
	db: web::Data<Database>
) -> HttpResponse {
	let (book_id, resource_path) = path.into_inner();

	let file = db.find_file_by_id(book_id).unwrap().unwrap();

	let mut book = bookie::load_from_path(&file.path).unwrap().unwrap();

	// TODO: Check if we're loading a section
	if res.configure_pages {
		let body = match book.read_path_as_bytes(
			&resource_path,
			Some(&format!("/api/book/{}/res", book_id)),
			Some(&[include_str!("../../../../../app/book_stylings.css")])
		) {
			Ok(v) => v,
			Err(e) => {
				eprintln!("{}", e);
				Vec::new()
			}
		};

		HttpResponse::Ok()
			.insert_header(("Content-Type", "application/xhtml+xml"))
			.body(body)
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

		HttpResponse::Ok().body(body)
	}
}


#[get("/api/book/{id}/pages/{pages}")]
pub async fn load_pages(path: web::Path<(i64, String)>, db: web::Data<Database>) -> web::Json<api::GetChaptersResponse> {
	let (book_id, chapters) = path.into_inner();

	let file = db.find_file_by_id(book_id).unwrap().unwrap();

	let mut book = bookie::load_from_path(&file.path).unwrap().unwrap();

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

	web::Json(api::GetChaptersResponse {
		offset: start_chap,
		limit: end_chap - start_chap,
		total: book.chapter_count(),
		items
	})
}


// TODO: Add body requests for specifics
#[get("/api/book/{id}")]
pub async fn load_book(file_id: web::Path<i64>, db: web::Data<Database>) -> web::Json<Option<api::GetBookIdResponse>> {
	web::Json(if let Some(file) = db.find_file_by_id(*file_id).unwrap() {
		Some(api::GetBookIdResponse {
			progress: db.get_progress(0, *file_id).unwrap().map(|v| v.into()),

			media: file.into()
		})
	} else {
		None
	})
}


#[get("/api/book/{id}/debug/{tail:.*}")]
pub async fn load_book_debug(web_path: web::Path<(i64, String)>, db: web::Data<Database>) -> HttpResponse {
	if let Some(file) = db.find_file_by_id(web_path.0).unwrap() {
		if web_path.1.is_empty() {
			let book = bookie::epub::EpubBook::load_from_path(&file.path).unwrap();

			HttpResponse::Ok().body(
				book.container.file_names_in_archive()
				.map(|v| format!("<a href=\"{}\">{}</a>", v, v))
				.collect::<Vec<_>>()
				.join("<br/>")
			)
		} else {
			// TODO: Make bookie::load_from_path(&file.path).unwrap().unwrap();
			let mut book = bookie::epub::EpubBook::load_from_path(&file.path).unwrap();

			// Init Package Document
			let mut file = book.container.archive.by_name(&web_path.1).unwrap();

			let mut data = Vec::new();
			file.read_to_end(&mut data).unwrap();

			HttpResponse::Ok().body(data)
		}
	} else {
		HttpResponse::Ok().body("Unable to find file from ID")
	}
}


// Progress

#[post("/api/book/{id}/progress")]
pub async fn progress_book_add(file_id: web::Path<i64>, body: web::Json<Progression>, db: web::Data<Database>) -> HttpResponse {
	match db.add_or_update_progress(0, *file_id, body.into_inner()) {
		Ok(_) => HttpResponse::Ok().finish(),
		Err(e) => HttpResponse::BadRequest().body(format!("{}", e))
	}
}

#[delete("/api/book/{id}/progress")]
pub async fn progress_book_delete(file_id: web::Path<i64>, db: web::Data<Database>) -> HttpResponse {
	match db.delete_progress(0, *file_id) {
		Ok(_) => HttpResponse::Ok().finish(),
		Err(e) => HttpResponse::BadRequest().body(format!("{}", e))
	}
}


// Notes

#[get("/api/book/{id}/notes")]
pub async fn notes_book_get(file_id: web::Path<i64>, db: web::Data<Database>) -> HttpResponse {
	match db.get_notes(0, *file_id) {
		Ok(v) => HttpResponse::Ok().body(v.map(|v| v.data).unwrap_or_default()),
		Err(e) => HttpResponse::BadRequest().body(format!("{}", e))
	}
}

#[post("/api/book/{id}/notes")]
pub async fn notes_book_add(file_id: web::Path<i64>, mut payload: web::Payload, db: web::Data<Database>) -> actix_web::Result<HttpResponse> {
	let mut body = web::BytesMut::new();
	while let Some(chunk) = payload.try_next().await? {
		body.extend_from_slice(&chunk);
	}

	let data = unsafe { String::from_utf8_unchecked(body.to_vec()) };

	Ok(match db.add_or_update_notes(0, *file_id, data) {
		Ok(_) => HttpResponse::Ok().finish(),
		Err(e) => HttpResponse::BadRequest().body(format!("{}", e))
	})
}

#[delete("/api/book/{id}/notes")]
pub async fn notes_book_delete(file_id: web::Path<i64>, db: web::Data<Database>) -> HttpResponse {
	match db.delete_notes(0, *file_id) {
		Ok(_) => HttpResponse::Ok().finish(),
		Err(e) => HttpResponse::BadRequest().body(format!("{}", e))
	}
}


// TODO: Add body requests for specific books
#[get("/api/books")]
pub async fn load_book_list(db: web::Data<Database>, query: web::Query<api::BookListQuery>) -> web::Json<api::GetBookListResponse> {
	web::Json(api::GetBookListResponse {
		count: db.get_file_count().unwrap(),
		items: db.get_metadata_by(query.library, query.offset.unwrap_or(0), query.limit.unwrap_or(50))
			.unwrap()
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
	})
}




fn default_true() -> bool {
	true
}