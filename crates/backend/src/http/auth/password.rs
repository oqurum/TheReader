// TODO: Better security. Simple Proof of Concept.

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

use super::MemberCookie;

pub static PASSWORD_PATH: &str = "/auth/password";

#[derive(Serialize, Deserialize)]
pub struct PostPasswordCallback {
    pub email: String,
    pub password: String,
}

pub async fn post_password_oauth(
    request: HttpRequest,
    query: web::Json<PostPasswordCallback>,
    member_cookie: Option<MemberCookie>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<String>> {
    // If we're currently logged in as a guest.
    let mut guest_member = None;

    if let Some(cookie) = &member_cookie {
        let member = cookie.fetch(&db.basic()).await?;

        if let Some(member) = member {
            if !member.type_of.is_guest() {
                return Err(ApiErrorResponse::new("Already logged in").into());
            }

            guest_member = Some(member);
        }
    }

    let PostPasswordCallback {
        email: mut email_str,
        password,
    } = query.into_inner();
    email_str = email_str.trim().to_string();

    // Verify that's is a proper email address.
    let address = email_str.parse::<Address>().map_err(Error::from)?;

    // Create or Update User.
    let mut member = if let Some(value) =
        MemberModel::find_one_by_email(&email_str, &db.basic()).await?
    {
        // TODO: Check if we're currently logged in as a guest member?

        if !value.type_of.is_invited() && value.type_of != MemberAuthType::Password {
            return Err(ApiErrorResponse::new(
                "Member does not have a local password associated with it.",
            )
            .into());
        } else if value.type_of == MemberAuthType::Invite
            || bcrypt::verify(&password, value.password.as_ref().unwrap()).map_err(Error::from)?
        {
            value
        } else {
            return Err(ApiErrorResponse::new("Unable to very password hash for account").into());
        }
    } else if let Some(mut guest_member) = guest_member {
        guest_member.convert_to_email(email_str);

        guest_member
    } else if !get_config().has_admin_account {
        // Double check that we don't already have users in the database.
        if MemberModel::count(&db.basic()).await? != 0 {
            update_config(|config| {
                config.has_admin_account = true;
                Ok(())
            })?;

            save_config().await?;

            return Err(ApiErrorResponse::new("We already have people in the database.").into());
        }

        // If we don't have an admin account this means we should create one.
        let new_member = NewMemberModel {
            name: address.user().to_string(),
            email: email_str,
            type_of: MemberAuthType::Password,
            permissions: Permissions::owner(),
            preferences: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut inserted = new_member.insert(&db.basic()).await?;

        let hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST).map_err(Error::from)?;

        // We instantly accept the invite.
        inserted
            .accept_invite(MemberAuthType::Password, Some(hash), &db.basic())
            .await?;

        update_config(|config| {
            config.has_admin_account = true;
            Ok(())
        })?;

        save_config().await?;

        inserted
    } else {
        return Err(
            ApiErrorResponse::new("Hey dum dum! You have no invite with this email!").into(),
        );
    };

    // If we were invited update the invite with correct info.
    if member.type_of == MemberAuthType::Invite {
        let hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST).map_err(Error::from)?;

        member
            .accept_invite(MemberAuthType::Password, Some(hash), &db.basic())
            .await?;
    }

    let model = AuthModel::new(Some(member.id));

    model.insert(&db.basic()).await?;

    super::remember_member_auth(&request.extensions(), member.id, model.oauth_token_secret)?;

    Ok(web::Json(WrappingResponse::okay(String::from("success"))))
}
