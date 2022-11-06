// TODO: Better security. Simple Proof of Concept.

use actix_identity::Identity;
use actix_web::web;
use actix_web::HttpMessage;
use actix_web::HttpRequest;

use chrono::Utc;
use common::api::ApiErrorResponse;
use common::api::WrappingResponse;
use common_local::MemberAuthType;
use common_local::Permissions;
use lettre::Address;
use serde::{Deserialize, Serialize};

use crate::config::get_config;
use crate::config::save_config;
use crate::config::update_config;
use crate::database::Database;
use crate::http::JsonResponse;
use crate::model::auth::AuthModel;
use crate::model::member::MemberModel;
use crate::model::member::NewMemberModel;
use crate::Error;
use crate::WebResult;

pub static PASSWORD_PATH: &str = "/auth/password";

#[derive(Serialize, Deserialize)]
pub struct PostPasswordCallback {
    pub email: String,
    pub password: String,
    // TODO: Check for Login vs Signup
}

pub async fn post_password_oauth(
    request: HttpRequest,
    query: web::Json<PostPasswordCallback>,
    identity: Option<Identity>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<String>> {
    if identity.is_some() {
        return Err(ApiErrorResponse::new("Already logged in").into());
    }

    let PostPasswordCallback {
        email: mut email_str,
        password,
    } = query.into_inner();
    email_str = email_str.trim().to_string();

    // Verify that's is a proper email address.
    let address = email_str.parse::<Address>().map_err(Error::from)?;

    // Create or Update User.
    let member = if let Some(value) =
        MemberModel::find_one_by_email(&email_str, &db.basic()).await?
    {
        if value.type_of != MemberAuthType::Password {
            return Err(ApiErrorResponse::new(
                "Invalid Member. Member does not have a local password associated with it.",
            )
            .into());
        }

        if bcrypt::verify(&password, value.password.as_ref().unwrap()).map_err(Error::from)? {
            value
        } else {
            return Err(ApiErrorResponse::new("Unable to very password hash for account").into());
        }
    } else {
        let hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST).map_err(Error::from)?;

        let mut new_member = NewMemberModel {
            name: address.user().to_string(),
            email: Some(email_str),
            password: Some(hash),
            type_of: MemberAuthType::Password,
            permissions: Permissions::basic(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let has_admin_account = get_config().has_admin_account;

        // Check to see if we don't have the admin account created yet.
        if !has_admin_account {
            new_member.permissions = Permissions::owner();
        }

        let inserted = new_member.insert(&db.basic()).await?;

        // Update config.
        if !has_admin_account {
            update_config(|config| {
                config.has_admin_account = true;
                Ok(())
            })?;

            save_config().await?;
        }

        inserted
    };

    let model = AuthModel::new(Some(member.id));

    model.insert(&db.basic()).await?;

    super::remember_member_auth(&request.extensions(), member.id, model.oauth_token_secret)?;

    Ok(web::Json(WrappingResponse::okay(String::from("success"))))
}
