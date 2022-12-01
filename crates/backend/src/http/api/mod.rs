use actix_web::{
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    web, HttpResponse, Scope,
};

use super::LoginRequired;

pub mod book;
pub mod file;
pub mod image;
pub mod library;
pub mod member;
pub mod options;
pub mod person;
pub mod settings;
pub mod storage;
pub mod task;

pub fn api_route() -> Scope<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<actix_web::body::EitherBody<actix_web::body::BoxBody>>,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    web::scope("/api")
        .wrap(LoginRequired)
        // Settings
        .service(settings::get_configg)
        .service(settings::post_setup_agent)
        .service(settings::get_setup_agent_verify)
        .service(settings::save_initial_setup)
        // File
        .service(file::load_file_debug)
        .service(file::load_file)
        .service(file::load_file_pages)
        .service(file::load_file_resource)
        .service(file::progress_file_add)
        .service(file::progress_file_delete)
        .service(file::notes_file_get)
        .service(file::notes_file_add)
        .service(file::notes_file_delete)
        .service(file::download_file)
        // Image
        .service(image::get_local_image)
        // Member
        .service(member::load_member_self)
        // Book
        .service(book::load_book_list)
        .service(book::load_book_preset_list)
        .service(book::update_books)
        .service(book::update_book_info)
        .service(book::book_search)
        .service(book::load_book_info)
        .service(book::get_book_posters)
        .service(book::insert_or_update_book_image)
        .service(book::insert_book_person)
        .service(book::delete_book_person)
        .service(book::download_book)
        // Person
        .service(person::load_person)
        .service(person::load_author_list)
        .service(person::load_person_thumbnail)
        .service(person::update_person_data)
        // Options
        .service(options::load_options)
        .service(options::update_options_add)
        .service(options::update_options_remove)
        // Library
        .service(library::load_library_list)
        .service(library::load_library_id)
        .service(library::update_library_id)
        // Task
        .service(task::run_task)
        // Storage
        .service(storage::get_directory)
        .default_service(web::route().to(default_handler))
}

async fn default_handler() -> HttpResponse {
    HttpResponse::NotFound().finish()
}
