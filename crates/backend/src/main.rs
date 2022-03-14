// TODO: Ping/Pong if currently viewing book. View time. How long been on page. Etc.

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{get, web, App, HttpServer, cookie::SameSite, HttpResponse, post, delete};

use books_common::{Chapter, MediaItem, api, Progression};
use bookie::Book;
use futures::TryStreamExt;

use crate::database::Database;

pub mod config;
pub mod database;
pub mod metadata;
pub mod scanner;




#[get("/api/book/{id}/res/{tail:.*}")]
async fn load_resource(path: web::Path<(i64, String)>, db: web::Data<Database>) -> HttpResponse {
	let (book_id, resource_path) = path.into_inner();

	let file = db.find_file_by_id(book_id).unwrap().unwrap();

	let mut book = bookie::load_from_path(&file.path).unwrap().unwrap();

	let body = match book.read_path_as_bytes(&resource_path) {
		Ok(v) => v,

		Err(e) => {
			eprintln!("{}", e);

			Vec::new()
		}
	};

	HttpResponse::Ok()
		.body(body)
}


#[derive(serde::Serialize)]
struct ChapterInfo {
	chapters: Vec<Chapter>
}

#[get("/api/book/{id}/pages/{pages}")]
async fn load_pages(path: web::Path<(i64, String)>, db: web::Data<Database>) -> web::Json<ChapterInfo> {
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
				(start_chap, if b.trim().is_empty() { book.chapter_count().saturating_sub(1) } else { b.parse::<usize>().unwrap() })
			}
		);

	let path = format!("/api/book/{}/res", book_id);

	let mut chapters = Vec::new();

	for chap in start_chap..end_chap {
		book.set_chapter(chap);

		// TODO: Return file names along with Chapter. Useful for redirecting to certain chapter for <a> tags.

		chapters.push(Chapter {
			value: chap,
			html: book.read_page_as_string(Some(&path), Some(&[
				include_str!("../../../app/book_stylings.css")
			])).unwrap()
		});
	}

	web::Json(ChapterInfo {
		chapters
	})
}


// TODO: Add body requests for specifics
#[get("/api/book/{id}")]
async fn load_book(file_id: web::Path<i64>, db: web::Data<Database>) -> web::Json<Option<api::GetBookIdResponse>> {
	web::Json(if let Some(file) = db.find_file_by_id(*file_id).unwrap() {
		// TODO: Make bookie::load_from_path(&file.path).unwrap().unwrap();
		let book = bookie::epub::EpubBook::load_from_path(&file.path).unwrap();

		Some(api::GetBookIdResponse {
			progress: db.get_progress(0, *file_id).unwrap().map(|v| v.into()),

			media: MediaItem {
				id: file.id,

				title: book.package.metadata.dcmes_elements.get("title").unwrap().iter().find_map(|v| v.value.as_ref().cloned()).unwrap_or_default(),
				author: book.package.metadata.get_creators().first().map(|v| v.to_string()).unwrap_or_default(),
				icon_path: None, // TODO

				chapter_count: book.chapter_count(),

				path: file.path,

				file_name: file.file_name,
				file_type: file.file_type,
				file_size: file.file_size,

				modified_at: file.modified_at,
				accessed_at: file.accessed_at,
				created_at: file.created_at,
			}
		})
	} else {
		None
	})
}


#[post("/api/book/{id}/progress")]
async fn progress_book_add(file_id: web::Path<i64>, body: web::Json<Progression>, db: web::Data<Database>) -> HttpResponse {
	match db.add_or_update_progress(0, *file_id, body.into_inner()) {
		Ok(_) => HttpResponse::Ok().finish(),
		Err(e) => HttpResponse::BadRequest().body(format!("{}", e))
	}
}

#[delete("/api/book/{id}/progress")]
async fn progress_book_delete(file_id: web::Path<i64>, db: web::Data<Database>) -> HttpResponse {
	match db.delete_progress(0, *file_id) {
		Ok(_) => HttpResponse::Ok().finish(),
		Err(e) => HttpResponse::BadRequest().body(format!("{}", e))
	}
}

// db.get_notes(0, *file_id).unwrap().map(|v| v.data)


#[get("/api/book/{id}/notes")]
async fn notes_book_get(file_id: web::Path<i64>, db: web::Data<Database>) -> HttpResponse {
	match db.get_notes(0, *file_id) {
		Ok(v) => HttpResponse::Ok().body(v.map(|v| v.data).unwrap_or_default()),
		Err(e) => HttpResponse::BadRequest().body(format!("{}", e))
	}
}

#[post("/api/book/{id}/notes")]
async fn notes_book_add(file_id: web::Path<i64>, mut payload: web::Payload, db: web::Data<Database>) -> actix_web::Result<HttpResponse> {
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
async fn notes_book_delete(file_id: web::Path<i64>, db: web::Data<Database>) -> HttpResponse {
	match db.delete_notes(0, *file_id) {
		Ok(_) => HttpResponse::Ok().finish(),
		Err(e) => HttpResponse::BadRequest().body(format!("{}", e))
	}
}


// TODO: Add body requests for specific books
#[get("/api/books")]
async fn load_book_list(db: web::Data<Database>) -> web::Json<Vec<MediaItem>> {
	web::Json(db.list_all_files()
		.unwrap()
		.into_iter()
		.filter(|v| v.file_type == "epub")
		.map(|file| {
			// TODO: Make bookie::load_from_path(&file.path).unwrap().unwrap();
			let book = bookie::epub::EpubBook::load_from_path(&file.path).unwrap();

			MediaItem {
				id: file.id,

				title: book.package.metadata.dcmes_elements.get("title").unwrap().iter().find_map(|v| v.value.as_ref().cloned()).unwrap_or_default(),
				author: book.package.metadata.get_creators().first().map(|v| v.to_string()).unwrap_or_default(),
				icon_path: None, // TODO

				chapter_count: book.chapter_count(),

				path: file.path,

				file_name: file.file_name,
				file_type: file.file_type,
				file_size: file.file_size,

				modified_at: file.modified_at,
				accessed_at: file.accessed_at,
				created_at: file.created_at,
			}
		})
		.collect())
}


// TODO: Convert to async closure (https://github.com/rust-lang/rust/issues/62290)
async fn default_handler() -> impl actix_web::Responder {
	actix_files::NamedFile::open_async("../frontend/dist/index.html").await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let db = database::init().await.unwrap();
	db.add_library("Y:/books/J. K. Rowling").unwrap();

	for library in db.list_all_libraries().unwrap() {
		let directories = db.get_directories(library.id).unwrap();

		scanner::library_scan(&library, directories, &db).await.unwrap();
	}

	let db_data = web::Data::new(db);

	HttpServer::new(move || {
		App::new()
			.app_data(db_data.clone())
			.wrap(IdentityService::new(
				CookieIdentityPolicy::new(&[0; 32])
					.name("bookie-auth")
					.secure(false)
					.same_site(SameSite::Strict)
			))

			.service(load_book)
			.service(load_pages)
			.service(load_resource)
			.service(load_book_list)
			.service(progress_book_add)
			.service(progress_book_delete)
			.service(notes_book_get)
			.service(notes_book_add)
			.service(notes_book_delete)

			.service(actix_files::Files::new("/js", "../../app/public/js"))
			.service(actix_files::Files::new("/css", "../../app/public/css"))
			.service(actix_files::Files::new("/fonts", "../../app/public/fonts"))
			.service(actix_files::Files::new("/images", "../../app/public/images"))
			.service(actix_files::Files::new("/", "../frontend/dist").index_file("index.html"))
			.default_service(web::route().to(default_handler))
	})
		.bind("0.0.0.0:8084")?
		.run()
		.await
}