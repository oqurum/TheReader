#![warn(
	clippy::expect_used,
	// clippy::unwrap_used,
)]

#![allow(clippy::manual_map)]

// TODO: Ping/Pong if currently viewing book. View time. How long been on page. Etc.


use actix_web::web;

pub mod database;
pub mod error;
pub mod http;
pub mod metadata;
pub mod scanner;
pub mod task;
pub mod model;
pub mod util;

pub use task::{queue_task, Task};
pub use util::*;
pub use error::{Result, WebResult, WebError, Error, InternalError};


#[actix_web::main]
async fn main() -> Result<()> {
	let db = database::init().await?;

	let db_data = web::Data::new(db);

	task::start_task_manager(db_data.clone());

	println!("Starting HTTP Server on port 8084");

	http::register_http_service(db_data).await?;

	Ok(())
}