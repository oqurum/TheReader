#![warn(
    clippy::expect_used,
    // clippy::unwrap_used,
)]
#![allow(clippy::manual_map)]

#[macro_use]
extern crate tracing;

// TODO: Ping/Pong if currently viewing book. View time. How long been on page. Etc.

use actix_web::web;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "bundled")]
mod bundle;
pub mod cli;
pub mod database;
pub mod error;
pub mod http;
mod imd;
pub mod metadata;
pub mod model;
pub mod scanner;
pub mod task;
pub mod util;

pub use cli::CliArgs;
pub use database::{SqlConnection, SqlPool};
pub use error::{Error, InternalError, Result, WebError, WebResult};
pub use imd::IN_MEM_DB;
pub use task::{queue_task, Task};
pub use util::*;

#[actix_web::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "books_backend=debug,actix_server=debug,actix_web=debug".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    #[cfg(feature = "bundled")]
    bundle::export().await?;

    let cli_args = CliArgs::parse();

    // Save Config - Otherwise it'll be lazily loaded whenever this fn is first called.
    config::save_config().await?;

    let db = database::init().await?;
    let db_data = web::Data::new(db);

    task::start_task_manager(db_data.clone());

    info!(
        port = cli_args.port,
        host = cli_args.host,
        "Starting HTTP Server"
    );

    http::register_http_service(&cli_args, db_data).await?;

    Ok(())
}
