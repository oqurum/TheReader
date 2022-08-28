use actix_web::{web, get, post, HttpResponse, http::header, HttpRequest};
use common_local::{setup::{SetupConfig, Config, LibraryConnection}, api};
use chrono::Utc;
use common::api::{ApiErrorResponse, WrappingResponse, librarian::{AuthFormLink, Scope, AuthQueryHandshake}, reader::VerifyAgentQuery};
use rand::distributions::{DistString, Alphanumeric};
use reqwest::Url;
use serde::{Serialize, Deserialize};
use serde_qs::actix::QsQuery;

use crate::{database::Database, WebResult, config::{does_config_exist, CONFIG_PATH, CONFIG_FILE, get_config, save_config, update_config}, http::{passwordless::test_connection, MemberCookie, JsonResponse}, model::{library::NewLibraryModel, directory::DirectoryModel, auth::AuthModel}, Result};



#[get("/setup")]
pub async fn is_setup(
    member: Option<MemberCookie>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetIsSetupResponse>> {
    if let Some(member) = member.as_ref() {
        let member = member.fetch_or_error(&db).await?;

        if !member.permissions.is_owner() {
            return Err(ApiErrorResponse::new("Not owner").into());
        }

        Ok(web::Json(WrappingResponse::okay(does_config_exist())))
    } else {
        Ok(web::Json(WrappingResponse::okay(false)))
    }
}


#[post("/setup")]
pub async fn save_initial_setup(
    body: web::Json<SetupConfig>,
    member: Option<MemberCookie>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<String>> {
    if let Some(member) = member {
        let member = member.fetch_or_error(&db).await?;

        if !member.permissions.is_owner() {
            return Err(ApiErrorResponse::new("Not owner").into());
        }
    } else if !does_config_exist() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }


    let config = body.into_inner();

    if let Some(email_config) = config.email.as_ref() {
        if !test_connection(email_config)? {
            return Ok(web::Json(WrappingResponse::error("Test Connection Failed")));
        }
    }

    for path in &config.directories {
        let now = Utc::now();

        let lib = NewLibraryModel {
            name: format!("New Library {}", now.timestamp_millis()),
            created_at: now,
            scanned_at: now,
            updated_at: now,
        }.insert(&db).await?;

        // TODO: Don't trust that the path is correct. Also remove slashes at the end of path.
        DirectoryModel { library_id: lib.id, path: path.clone() }.insert(&db).await?;
    }

    save_setup_config(config).await?;

    Ok(web::Json(WrappingResponse::okay(String::new())))
}



// External Metadata Initiation
// Client -> Server - Initial request
// Server -> Client - unique ids and such
//     |-> Server - verification

#[post("/setup/agent")]
pub async fn post_setup_agent(
    req: HttpRequest,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<HttpResponse> {
    let member = member.fetch_or_error(&db).await?;

    if !member.permissions.is_owner() {
        return Ok(HttpResponse::NotAcceptable().body("Not owner"));
    }


    let config = get_config();

    if config.libby.token.is_some() {
        return Ok(HttpResponse::NotAcceptable().body("Agent is already setup"));
    }

    let redirect_uri = {
        let host = match req.headers().get("host").and_then(|v| v.to_str().ok()) {
            Some(v) => format!("{}://{v}", config.server.is_secure.then(|| "https").unwrap_or("http")),
            None => return Ok(HttpResponse::NotAcceptable().body("Missing host")),
        };

        let mut location_uri = Url::parse(&host).unwrap();
        location_uri.set_path("api/setup/agent/verify");
        location_uri.to_string()
    };


    let state = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);

    // TODO: Should I store it in AuthModel?
    AuthModel {
        oauth_token: state.clone(),
        oauth_token_secret: String::new(),
        created_at: Utc::now(),
    }.insert(&db).await?;


    let mut location_uri = Url::parse(&config.libby.url).unwrap();
    location_uri.set_path("authorize");
    location_uri.set_query(Some(&serde_qs::to_string(&AuthFormLink {
        server_owner_name: Some(member.name),
        server_name: Some(config.server.name),
        server_id: None,
        redirect_uri,
        state,
        scope: Scope::ServerRegister,
    }).unwrap()));

    Ok(HttpResponse::SeeOther()
        .insert_header((header::LOCATION, location_uri.to_string()))
        .body("redirecting..."))
}


#[get("/setup/agent/verify")]
pub async fn get_setup_agent_verify(
    _req: HttpRequest,
    query: QsQuery<VerifyAgentQuery>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<HttpResponse> {
    let query = query.into_inner();
    let member = member.fetch_or_error(&db).await?;

    if !member.permissions.is_owner() {
        return Ok(HttpResponse::NotAcceptable().body("Not owner"));
    }

    let config = get_config();

    if config.libby.token.is_some() {
        return Ok(HttpResponse::NotAcceptable().body("Agent is already setup"));
    }


    if AuthModel::remove_by_oauth_token(&query.state, &db).await? {
        let mut location_uri = Url::parse(&config.libby.url).unwrap();
        location_uri.set_path("auth/handshake");
        location_uri.set_query(Some(&serde_qs::to_string(&AuthQueryHandshake {
            public_id: query.public_id.clone(),
            server_id: query.server_id.clone(),
            state: Some(query.state),
            scope: query.scope,
        }).unwrap()));

        let resp = reqwest::get(location_uri).await
            .map_err(crate::Error::from)?;

        if resp.status().is_success() {
            update_config(move |config| {
                config.libby.token = Some(query.server_id);
                config.libby.pubid = Some(query.public_id);

                Ok(())
            })?;

            save_config().await?;

            Ok(HttpResponse::TemporaryRedirect()
                .insert_header((header::LOCATION, "/options"))
                .body("Redirecting..."))
        } else {
            let body = resp.bytes().await.map_err(crate::Error::from)?;
            Ok(HttpResponse::InternalServerError().body(body))
        }
    } else {
        Ok(HttpResponse::InternalServerError().body("Incorrect State. Try Linking again."))
    }

}



#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterAgentQuery {
    pub server_owner_name: Option<String>,
    pub server_name: Option<String>,
    pub redirect_uri: String,
    /// Unqiue ID for continuity
    pub state: String,
    pub scope: String,
}


async fn save_setup_config(value: SetupConfig) -> Result<()> {
    let config = Config {
        server: value.server,
        libby: LibraryConnection::default(),
        email: value.email,
        authenticators: value.authenticators,
    };

    tokio::fs::write(
        CONFIG_PATH,
        toml_edit::ser::to_string_pretty(&config)?,
    ).await?;

    *CONFIG_FILE.lock().unwrap() = Some(config);

    Ok(())
}