#![allow(clippy::manual_map)]

// TODO: Ping/Pong if currently viewing book. View time. How long been on page. Etc.


use actix_web::web;


pub mod config;
pub mod database;
pub mod http;
pub mod image;
pub mod metadata;
pub mod scanner;
pub mod task;

pub use task::{queue_task, Task};
pub use self::image::{ThumbnailLocation, ThumbnailType, store_image};


#[actix_web::main]
async fn main() -> std::io::Result<()> {
	let db = database::init().await.unwrap();

	let db_data = web::Data::new(db);

	task::start_task_manager(db_data.clone());

	println!("Starting HTTP Server on port 8084");

	http::register_http_service(db_data).await
}