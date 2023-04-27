use actix_web::{get, http::header, post, web, HttpRequest, HttpResponse};
use chrono::Utc;
use common::api::{
    librarian::{AuthFormLink, AuthQueryHandshake, Scope},
    reader::VerifyAgentQuery,
    ApiErrorResponse, WrappingResponse,
};
use common_local::{
    api,
    setup::{Config, LibraryConnection, SetupConfig},
    LibraryType, PublicServerSettings,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery;

use crate::{
    config::{
        get_config, is_setup as iss_setup, save_config, update_config, CONFIG_FILE, CONFIG_PATH,
        IS_SETUP,
    },
    http::{passwordless::test_connection, JsonResponse, MemberCookie},
    model::{AuthModel, DirectoryModel, LibraryModel, NewLibraryModel},
    Result, SqlPool, WebResult,
};

#[get("/settings")]
pub async fn get_server_settings(
    _member: Option<MemberCookie>,
    // db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<PublicServerSettings>> {
    let config = get_config();

    Ok(web::Json(WrappingResponse::okay(PublicServerSettings {
        is_auth_guest_enabled: config.is_public_access,
        is_auth_password_enabled: config.authenticators.email_pass,
        is_auth_passwordless_enabled: config.authenticators.email_no_pass,
    })))
}

#[get("/setup")]
pub async fn get_configg(
    member: Option<MemberCookie>,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<api::ApiGetSetupResponse>> {
    if iss_setup() {
        if let Some(member) = member.as_ref() {
            let member = member.fetch_or_error(&mut *db.acquire().await?).await?;

            if !member.permissions.is_owner() {
                return Err(ApiErrorResponse::new("Not owner").into());
            }
        } else {
            return Err(ApiErrorResponse::new("Not owner").into());
        }
    }

    let mut config = get_config();

    config.libby.token = config.libby.token.map(|_| String::new());
    config.server.auth_key.clear();

    Ok(web::Json(WrappingResponse::okay(config)))
}

#[post("/setup")]
pub async fn save_initial_setup(
    body: web::Json<SetupConfig>,
    member: Option<MemberCookie>,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<String>> {
    if let Some(member) = member {
        let member = member.fetch_or_error(&mut *db.acquire().await?).await?;

        if !member.permissions.is_owner() {
            return Err(ApiErrorResponse::new("Not owner").into());
        }
    } else if iss_setup() {
        return Err(ApiErrorResponse::new("already setup").into());
    }

    let config = body.into_inner();

    if let Some(email_config) = config.email.as_ref() {
        if !test_connection(email_config)? {
            return Ok(web::Json(WrappingResponse::error("Test Connection Failed")));
        }
    }

    let name_trim = config.server.name.trim();
    if name_trim.len() < 3 || name_trim.len() > 32 {
        return Ok(web::Json(WrappingResponse::error("Invalid Server Name")));
    }

    let mut library_count = LibraryModel::count(&mut *db.acquire().await?).await?;

    for path in &config.directories {
        if tokio::fs::metadata(path).await.is_err() {
            return Ok(web::Json(WrappingResponse::error(format!(
                "Invalid Directory: {path:?}"
            ))));
        }

        let now = Utc::now().naive_utc();

        let lib = NewLibraryModel {
            name: format!("New Library #{library_count}"),
            type_of: LibraryType::Book, // TODO: Specify type.

            is_public: true,
            settings: None,

            created_at: now,
            scanned_at: now,
            updated_at: now,
        }
        .insert(&mut *db.acquire().await?)
        .await?;

        // TODO: Don't trust that the path is correct. Also remove slashes at the end of path.
        // Initial directory when doing the library scan task.
        DirectoryModel {
            library_id: lib.id,
            path: path.clone(),
        }
        .insert(&mut *db.acquire().await?)
        .await?;

        crate::task::queue_task(crate::task::TaskLibraryScan { library_id: lib.id });

        library_count += 1;
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
    db: web::Data<SqlPool>,
) -> WebResult<HttpResponse> {
    let member = member.fetch_or_error(&mut *db.acquire().await?).await?;

    if !member.permissions.is_owner() {
        return Ok(HttpResponse::NotAcceptable().body("Not owner"));
    }

    let config = get_config();

    if config.libby.token.is_some() {
        return Ok(HttpResponse::NotAcceptable().body("Agent is already setup"));
    }

    let redirect_uri = {
        let host = match req.headers().get("host").and_then(|v| v.to_str().ok()) {
            Some(v) => format!(
                "{}://{v}",
                if config.server.is_secure {
                    "https"
                } else {
                    "http"
                }
            ),
            None => return Ok(HttpResponse::NotAcceptable().body("Missing host")),
        };

        let mut location_uri = Url::parse(&host).unwrap();
        location_uri.set_path("api/setup/agent/verify");
        location_uri.to_string()
    };

    // TODO: Should I store it in AuthModel?
    let auth_model = AuthModel::new(None);
    auth_model.insert(&mut *db.acquire().await?).await?;

    let mut location_uri = Url::parse(&config.libby.url).unwrap();
    location_uri.set_path("authorize");
    location_uri.set_query(Some(
        &serde_qs::to_string(&AuthFormLink {
            server_owner_name: Some(member.name),
            server_name: Some(config.server.name),
            server_id: None,
            redirect_uri,
            state: auth_model.oauth_token.unwrap(),
            scope: Scope::ServerRegister,
        })
        .unwrap(),
    ));

    Ok(HttpResponse::SeeOther()
        .insert_header((header::LOCATION, location_uri.to_string()))
        .body("redirecting..."))
}

#[get("/setup/agent/verify")]
pub async fn get_setup_agent_verify(
    query: QsQuery<VerifyAgentQuery>,
    member: MemberCookie,
    db: web::Data<SqlPool>,
) -> WebResult<HttpResponse> {
    let query = query.into_inner();
    let member = member.fetch_or_error(&mut *db.acquire().await?).await?;

    if !member.permissions.is_owner() {
        return Ok(HttpResponse::NotAcceptable().body("Not owner"));
    }

    let config = get_config();

    if config.libby.token.is_some() {
        return Ok(HttpResponse::NotAcceptable().body("Agent is already setup"));
    }

    // TODO: Somehow validate that the `auth_model` secret is the same as when we initiated the request.
    if let Some(auth_model) =
        AuthModel::find_by_token(&query.state, &mut *db.acquire().await?).await?
    {
        AuthModel::remove_by_token_secret(
            &auth_model.oauth_token_secret,
            &mut *db.acquire().await?,
        )
        .await?;

        let mut location_uri = Url::parse(&config.libby.url).unwrap();
        location_uri.set_path("auth/handshake");
        location_uri.set_query(Some(
            &serde_qs::to_string(&AuthQueryHandshake {
                public_id: query.public_id.clone(),
                server_id: query.server_id.clone(),
                state: Some(query.state),
                scope: query.scope,
            })
            .unwrap(),
        ));

        let resp = reqwest::get(location_uri)
            .await
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

async fn save_setup_config(mut value: SetupConfig) -> Result<()> {
    let temp_config = get_config();

    value.server.auth_key = temp_config.server.auth_key;

    let config = Config {
        server: value.server,
        libby: LibraryConnection::default(),
        email: value.email,
        authenticators: value.authenticators,

        has_admin_account: false,
        is_public_access: false,
    };

    tokio::fs::write(CONFIG_PATH, toml_edit::ser::to_string_pretty(&config)?).await?;

    *IS_SETUP.lock().unwrap() = config.is_fully_setup();
    *CONFIG_FILE.lock().unwrap() = config;

    Ok(())
}
