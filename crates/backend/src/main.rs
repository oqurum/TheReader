#![forbid(unsafe_code)]

use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{get, web, App, HttpServer, cookie::SameSite, HttpResponse};

use books_common::Chapter;
use bookie::Book;

pub mod config;
pub mod database;
pub mod metadata;
pub mod scanner;


#[get("/api/book/{id}/res/{tail:.*}")]
async fn load_resource(path: web::Path<(usize, String)>) -> HttpResponse {
	let (_book_id, resource_path) = path.into_inner();

	let mut book = bookie::epub::EpubBook::load_from_path("../../app/books/Pride and Prejudice by Jane Austen.epub").unwrap();

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
async fn load_pages(path: web::Path<(usize, String)>) -> web::Json<ChapterInfo> {
	let (book_id, chapters) = path.into_inner();

	let mut book = bookie::epub::EpubBook::load_from_path("../../app/books/Pride and Prejudice by Jane Austen.epub").unwrap();

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

	println!("Chapter: {}", book.get_chapter());
	println!("Chapter Total: {}", book.chapter_count());
	println!("Path: {:?}", book.get_page_path());
	println!("Unique ID: {:?}", book.package.manifest.id);
	println!("ID: {:?}", book.get_unique_id());


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


// TODO: Convert to async closure (https://github.com/rust-lang/rust/issues/62290)
async fn default_handler() -> impl actix_web::Responder {
	actix_files::NamedFile::open_async("../frontend/dist/index.html").await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let db = database::init().await.unwrap();
	scanner::scan(db.clone()).await.unwrap();

	HttpServer::new(move || {
		App::new()
			.app_data(web::Data::new(db.clone()))
			.wrap(IdentityService::new(
				CookieIdentityPolicy::new(&[0; 32])
					.name("bookie-auth")
					.secure(false)
					.same_site(SameSite::Strict)
			))

			.service(load_pages)
			.service(load_resource)

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