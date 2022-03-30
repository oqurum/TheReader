// TODO: Ping/Pong if currently viewing book. View time. How long been on page. Etc.

use std::io::Read;

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{get, web, App, HttpServer, cookie::SameSite, HttpResponse, post, delete};

use books_common::{Chapter, MediaItem, api, Progression, LibraryColl, MetadataItemCached};
use bookie::Book;
use futures::TryStreamExt;

use crate::database::{Database, table::FileWithMetadata};

pub mod config;
pub mod database;
pub mod image;
pub mod metadata;
pub mod scanner;
pub mod task;

pub use task::{queue_task, Task};
pub use self::image::{ThumbnailLocation, ThumbnailType, store_image};




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

#[get("/api/book/{id}/thumbnail")]
async fn load_book_thumbnail(path: web::Path<i64>, db: web::Data<Database>) -> HttpResponse {
	let book_id = path.into_inner();

	let file = db.find_file_by_id_with_metadata(book_id).unwrap().unwrap();

	if let Some(path) = file.meta.and_then(|v| v.thumb_path) {
		let loc = ThumbnailLocation::from(path);

		let path = crate::image::prefixhash_to_path(loc.as_type(), loc.as_value());

		HttpResponse::Ok().body(std::fs::read(path).unwrap())
	} else {
		HttpResponse::NotFound().finish()
	}
}


#[derive(serde::Serialize)]
struct ChapterInfo {
	chapters: Vec<Chapter>
}

#[get("/api/book/{id}/pages/{pages}")]
async fn load_pages(path: web::Path<(i64, String)>, db: web::Data<Database>) -> web::Json<api::GetChaptersResponse> {
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
			file_path: book.get_page_path(),
			value: chap,
			html: book.read_page_as_string(Some(&path), Some(&[
				include_str!("../../../app/book_stylings.css")
			])).unwrap()
		});
	}

	web::Json(api::GetChaptersResponse {
		offset: start_chap,
		limit: end_chap - start_chap,
		total: book.chapter_count(),
		chapters
	})
}


// TODO: Add body requests for specifics
#[get("/api/book/{id}")]
async fn load_book(file_id: web::Path<i64>, db: web::Data<Database>) -> web::Json<Option<api::GetBookIdResponse>> {
	web::Json(if let Some(file) = db.find_file_by_id(*file_id).unwrap() {
		// TODO: Make bookie::load_from_path(&file.path).unwrap().unwrap();
		let book = bookie::epub::EpubBook::load_from_path(&file.path).unwrap();
		// TODO: Not needed ^

		Some(api::GetBookIdResponse {
			progress: db.get_progress(0, *file_id).unwrap().map(|v| v.into()),

			media: MediaItem {
				id: file.id,

				title: book.package.metadata.dcmes_elements.get("title").unwrap().iter().find_map(|v| v.value.as_ref().cloned()).unwrap_or_default(),
				cached: MetadataItemCached::default()
					.author_optional(book.package.metadata.get_creators().first().map(|v| v.to_string())),
				icon_path: None, // TODO

				chapter_count: book.chapter_count(),

				path: file.path,

				file_name: file.file_name,
				file_type: file.file_type,
				file_size: file.file_size,

				modified_at: file.modified_at.timestamp_millis(),
				accessed_at: file.accessed_at.timestamp_millis(),
				created_at: file.created_at.timestamp_millis(),
			}
		})
	} else {
		None
	})
}


#[get("/api/book/{id}/debug/{tail:.*}")]
async fn load_book_debug(web_path: web::Path<(i64, String)>, db: web::Data<Database>) -> HttpResponse {
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


// Notes

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


// Options

#[get("/api/options")]
async fn load_options(db: web::Data<Database>) -> web::Json<api::GetOptionsResponse> {
	let libraries = db.list_all_libraries().unwrap();
	let mut directories = db.get_all_directories().unwrap();

	web::Json(api::GetOptionsResponse {
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
	})
}

// TODO: Move to utils file.
fn take_from_and_swap<V, P: Fn(&V) -> bool>(array: &mut Vec<V>, predicate: P) -> Vec<V> {
	let mut ret = Vec::new();

	for i in (0..array.len()).rev() {
		if predicate(&array[i]) {
			ret.push(array.swap_remove(i));
		}
	}

	ret.reverse();

	ret
}

#[post("/api/options/add")]
async fn update_options_add(modify: web::Json<api::ModifyOptionsBody>, db: web::Data<Database>) -> HttpResponse {
	let api::ModifyOptionsBody {
		library,
		directory
	} = modify.into_inner();

	if let Some(library) = library {
		db.add_library(library.name.unwrap()).unwrap();
	}

	if let Some(directory) = directory {
		// TODO: Don't trust that the path is correct. Also remove slashes at the end of path.
		db.add_directory(directory.library_id, directory.path).unwrap();
	}

	HttpResponse::Ok().finish()
}

#[post("/api/options/remove")]
async fn update_options_remove(modify: web::Json<api::ModifyOptionsBody>, db: web::Data<Database>) -> HttpResponse {
	let api::ModifyOptionsBody {
		library,
		directory
	} = modify.into_inner();

	if let Some(library) = library {
		db.remove_library(library.id.unwrap()).unwrap();
	}

	if let Some(directory) = directory {
		db.remove_directory(&directory.path).unwrap();
	}

	HttpResponse::Ok().finish()
}


// Task

// TODO: Actually optimize.
#[post("/api/task")]
async fn run_task(modify: web::Json<api::RunTaskBody>) -> HttpResponse {
	let modify = modify.into_inner();

	if modify.run_search {
		queue_task(task::TaskLibraryScan);
	}

	if modify.run_metadata {
		queue_task(task::TaskUpdateInvalidMetadata::new(task::UpdatingMetadata::Invalid));
	}

	HttpResponse::Ok().finish()
}


// Metadata

// TODO: Use for frontend updating instead of attempting to auto-match. Will retreive metadata source name.
#[post("/api/metadata")]
async fn update_item_metadata(body: web::Json<api::PostMetadataBody>) -> HttpResponse {
	std::thread::spawn(move || {
		let rt = tokio::runtime::Runtime::new().unwrap();

		rt.block_on(async {
			match body.into_inner() {
				api::PostMetadataBody::File(file_id) => {
					queue_task(task::TaskUpdateInvalidMetadata::new(task::UpdatingMetadata::Single(file_id)));
				}
			}
		});
	});

	HttpResponse::Ok().finish()
}


// TODO: Add body requests for specific books
#[get("/api/books")]
async fn load_book_list(db: web::Data<Database>, query: web::Query<api::BookListQuery>) -> web::Json<api::GetBookListResponse> {
	web::Json(api::GetBookListResponse {
		count: db.get_file_count().unwrap(),
		items: db.get_files_with_metadata_by(query.library, query.offset.unwrap_or(0), query.limit.unwrap_or(50))
			.unwrap()
			.into_iter()
			.map(|FileWithMetadata { file, meta }| {
				let (title, cached, icon_path) = if let Some(meta) = meta {
					(
						meta.title.or(meta.original_title).unwrap_or_default(),
						meta.cached,
						meta.thumb_path.map(|url| format!("/api/book/{}/res/{}", file.id, url))
					)
				} else {
					(String::new(), MetadataItemCached::default(), None)
				};

				MediaItem {
					id: file.id,

					title,
					cached,
					// TODO: Cache images
					icon_path,

					chapter_count: file.chapter_count as usize,

					path: file.path,

					file_name: file.file_name,
					file_type: file.file_type,
					file_size: file.file_size,

					modified_at: file.modified_at.timestamp_millis(),
					accessed_at: file.accessed_at.timestamp_millis(),
					created_at: file.created_at.timestamp_millis(),
				}
			})
			.collect()
	})
}


#[get("/api/libraries")]
async fn load_library_list(db: web::Data<Database>) -> web::Json<api::GetLibrariesResponse> {
	web::Json(api::GetLibrariesResponse {
		items: db.list_all_libraries()
			.unwrap()
			.into_iter()
			.map(|file| {
				LibraryColl {
					id: file.id,

					name: file.name,

					created_at: file.created_at.timestamp_millis(),
					scanned_at: file.scanned_at.timestamp_millis(),
					updated_at: file.updated_at.timestamp_millis(),

					directories: Vec::new()
				}
			})
			.collect()
	})
}


// TODO: Convert to async closure (https://github.com/rust-lang/rust/issues/62290)
async fn default_handler() -> impl actix_web::Responder {
	actix_files::NamedFile::open_async("../frontend/dist/index.html").await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let db = database::init().await.unwrap();

	let db_data = web::Data::new(db);

	task::start_task_manager(db_data.clone());

	println!("Starting HTTP Server");

	HttpServer::new(move || {
		App::new()
			.app_data(db_data.clone())
			.wrap(IdentityService::new(
				CookieIdentityPolicy::new(&[0; 32])
					.name("bookie-auth")
					.secure(false)
					.same_site(SameSite::Strict)
			))

			.service(load_book_debug)
			.service(load_book)
			.service(load_pages)
			.service(load_book_thumbnail)
			.service(load_resource)
			.service(load_book_list)
			.service(progress_book_add)
			.service(progress_book_delete)
			.service(notes_book_get)
			.service(notes_book_add)
			.service(notes_book_delete)
			.service(load_library_list)
			.service(load_options)
			.service(update_options_add)
			.service(update_options_remove)
			.service(run_task)
			.service(update_item_metadata)

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