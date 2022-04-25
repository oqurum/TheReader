use actix_web::{web, Scope, dev::{ServiceFactory, ServiceRequest, ServiceResponse}};

use super::LoginRequired;

pub mod book;
pub mod image;
pub mod library;
pub mod member;
pub mod metadata;
pub mod options;
pub mod person;
pub mod task;


pub fn api_route() -> Scope<
	impl ServiceFactory<
		ServiceRequest,
		Config = (),
		Response = ServiceResponse<actix_web::body::BoxBody>,
		Error = actix_web::Error,
		InitError = (),
	>
> {
	web::scope("/api")
		.wrap(LoginRequired)
		// Book
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

		// Image
		.service(image::get_local_image)
		.service(image::get_poster_list)
		.service(image::post_change_poster)
		.service(image::put_upload_poster)

		// Member
		.service(member::load_member_self)

		// Metadata
		.service(metadata::load_metadata_thumbnail)
		.service(metadata::update_item_metadata)
		.service(metadata::get_metadata_search)
		.service(metadata::get_all_metadata_comp)

		// Person
		.service(person::load_author_list)
		.service(person::load_person_thumbnail)
		.service(person::update_person_data)

		// Options
		.service(options::load_options)
		.service(options::update_options_add)
		.service(options::update_options_remove)

		// Library
		.service(library::load_library_list)

		// Task
		.service(task::run_task)
}