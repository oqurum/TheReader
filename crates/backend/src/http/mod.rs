use std::time::Duration;

use actix_identity::{Identity, IdentityMiddleware};
use actix_session::storage::CookieSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::http::header;
use actix_web::{HttpResponse, HttpRequest};
use actix_web::{cookie::SameSite, web, App, HttpServer};
use common::api::WrappingResponse;
use tracing_actix_web::TracingLogger;

use crate::config::{get_config, is_setup};
use crate::database::Database;
use crate::CliArgs;
use crate::model::auth::AuthModel;

mod api;
mod auth;
mod ws;
pub use self::api::api_route;
pub use self::auth::*;
pub use ws::send_message_to_clients;

pub type JsonResponse<V> = web::Json<WrappingResponse<V>>;

pub const LOGGED_OUT_PATH: &str = "/loggedout";

// TODO: Convert to async closure (https://github.com/rust-lang/rust/issues/62290)
async fn default_handler(
    req: HttpRequest,
    mut ident: Option<Identity>,
    db: web::Data<Database>,
) -> crate::WebResult<HttpResponse> {
    if !req.path().contains('.') && !is_setup() && !req.uri().path().contains("/setup") {
        Ok(HttpResponse::TemporaryRedirect()
            .insert_header((header::LOCATION, "/setup"))
            .finish())
    } else {
        // TODO: Better Place
        // Ensure we're authenticated
        if req.path() != LOGGED_OUT_PATH && !req.path().contains('.') {
            if let Some(ident) = ident.take() {
                if let Some(cookie) = get_auth_value(&ident)? {
                    let token = AuthModel::find_by_token_secret(&cookie.token_secret, &db.basic()).await?;

                    match token {
                        Some(token) => {
                            if token.member_id.is_none() {
                                ident.logout();

                                AuthModel::remove_by_token_secret(&cookie.token_secret, &db.basic()).await?;

                                return Ok(HttpResponse::TemporaryRedirect()
                                    .insert_header((header::LOCATION, LOGGED_OUT_PATH))
                                    .finish())
                            }
                        }

                        None => {
                            ident.logout();

                            return Ok(HttpResponse::TemporaryRedirect()
                                .insert_header((header::LOCATION, LOGGED_OUT_PATH))
                                .finish())
                        }
                    }
                }
            }
        }

        Ok(
            actix_files::NamedFile::open_async("./app/public/dist/index.html")
                .await?
                .into_response(&req),
        )
    }
}

async fn logged_out() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((header::REFRESH, "5; url=/"))
        .body("You have been logged out. Redirecting in 5 seconds...")
}

async fn logout(ident: Identity) -> HttpResponse {
    ident.logout();

    HttpResponse::TemporaryRedirect()
        .insert_header((header::LOCATION, "/logout"))
        .finish()
}

async fn manifest() -> web::Json<serde_json::Value> {
    web::Json(serde_json::json!({
        "short_name": "Oqurum",
        "name": "Oqurum Reader App",
        "display": "fullscreen",
    }))
}

pub async fn register_http_service(
    cli_args: &CliArgs,
    db_data: web::Data<Database>,
) -> std::io::Result<()> {
    let secret_key = Key::from(&get_config().server.auth_key);

    HttpServer::new(move || {
        App::new()
            .app_data(db_data.clone())
            .wrap(TracingLogger::default())
            .wrap(
                IdentityMiddleware::builder()
                    .login_deadline(Some(Duration::from_secs(60 * 60 * 24 * 365)))
                    .build(),
            )
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_name(String::from("bookie-auth"))
                    .cookie_secure(false)
                    .cookie_same_site(SameSite::Strict)
                    .build(),
            )
            // API
            .service(api_route())
            // WS
            .service(ws::ws_index)
            .route("/manifest.json", web::get().to(manifest))

            .route("/auth/logout", web::get().to(logout))
            .route(LOGGED_OUT_PATH, web::get().to(logged_out))
            // Guest
            .route(
                guest::GUEST_PATH,
                web::post().to(guest::post_guest_oauth),
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
