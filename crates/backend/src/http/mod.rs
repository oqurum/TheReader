use std::time::Duration;

use actix_identity::{IdentityMiddleware, Identity};
use actix_session::SessionMiddleware;
use actix_session::storage::CookieSessionStore;
use actix_web::HttpResponse;
use actix_web::cookie::Key;
use actix_web::http::header;
use actix_web::{web, App, HttpServer, cookie::SameSite};
use common::api::WrappingResponse;

use crate::CliArgs;
use crate::config::get_config;
use crate::database::Database;

mod api;
mod auth;
mod ws;
pub use self::api::api_route;
pub use self::auth::*;
pub use ws::send_message_to_clients;


pub type JsonResponse<V> = web::Json<WrappingResponse<V>>;


// TODO: Convert to async closure (https://github.com/rust-lang/rust/issues/62290)
async fn default_handler(req: actix_web::HttpRequest) -> std::io::Result<HttpResponse> {
    if !req.path().contains('.') && !crate::config::does_config_exist() && !req.uri().path().contains("/setup") {
        Ok(HttpResponse::TemporaryRedirect()
            .insert_header((header::LOCATION, "/setup"))
            .finish())
    } else {
        Ok(actix_files::NamedFile::open_async("./app/public/dist/index.html").await?.into_response(&req))
    }
}


async fn logout(ident: Identity) -> HttpResponse {
    ident.logout();

    HttpResponse::TemporaryRedirect()
        .insert_header((header::LOCATION, "/logout"))
        .finish()
}


pub async fn register_http_service(cli_args: &CliArgs, db_data: web::Data<Database>) -> std::io::Result<()> {
    let secret_key = Key::from(&get_config().server.auth_key);

    HttpServer::new(move || {
        App::new()
            .app_data(db_data.clone())
            .wrap(IdentityMiddleware::builder()
                .login_deadline(Some(Duration::from_secs(60 * 60 * 24 * 365)))
                .build()
            )
            .wrap(SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_name(String::from("bookie-auth"))
                    .cookie_secure(false)
                    .cookie_same_site(SameSite::Strict)
                    .build()
            )

            // API
            .service(api_route())

            // WS
            .service(ws::ws_index)

            .route(
                "/auth/logout",
                web::get().to(logout)
            )

            // Password
            .route(
                password::PASSWORD_PATH,
                web::post().to(password::post_password_oauth),
            )

            // Passwordless
            .route(
                passwordless::PASSWORDLESS_PATH,
                web::post().to(passwordless::post_passwordless_oauth),
            )
            .route(
                passwordless::PASSWORDLESS_PATH_CB,
                web::get().to(passwordless::get_passwordless_oauth_callback),
            )

            // Other
            .service(actix_files::Files::new("/js", "./app/public/js"))
            .service(actix_files::Files::new("/css", "./app/public/css"))
            .service(actix_files::Files::new("/fonts", "./app/public/fonts"))
            .service(actix_files::Files::new("/images", "./app/public/images"))
            .service(actix_files::Files::new("/dist", "./app/public/dist"))
            .default_service(web::route().to(default_handler))
    })
        .bind(format!("{}:{}", &cli_args.host, cli_args.port))?
        .run()
        .await
}