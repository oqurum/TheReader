use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{web, App, HttpServer, cookie::SameSite};

use crate::database::Database;


mod api;
pub use self::api::*;



// TODO: Convert to async closure (https://github.com/rust-lang/rust/issues/62290)
async fn default_handler() -> impl actix_web::Responder {
	actix_files::NamedFile::open_async("../frontend/dist/index.html").await
}


pub async fn register_http_service(db_data: web::Data<Database>) -> std::io::Result<()> {
	HttpServer::new(move || {
		App::new()
			.app_data(db_data.clone())
			.wrap(IdentityService::new(
				CookieIdentityPolicy::new(&[0; 32])
					.name("bookie-auth")
					.secure(false)
					.same_site(SameSite::Strict)
			))

			.service(book::load_book_debug)
			.service(book::load_book)
			.service(book::load_pages)
			.service(book::load_resource)
			.service(book::progress_book_add)
			.service(book::progress_book_delete)
			.service(book::notes_book_get)
			.service(book::notes_book_add)
			.service(book::notes_book_delete)
			.service(book::load_book_list)
			.service(metadata::load_metadata_thumbnail)
			.service(metadata::update_item_metadata)
			.service(metadata::get_metadata_search)
			.service(metadata::get_all_metadata_comp)
			.service(person::load_author_list)
			.service(options::load_options)
			.service(options::update_options_add)
			.service(options::update_options_remove)
			.service(library::load_library_list)
			.service(task::run_task)

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