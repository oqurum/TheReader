// You supply an email. We email the link to authenticate with.

// TODO: Better security. Simple Proof of Concept.

use actix_web::{http::header, HttpResponse};
use actix_web::{web, HttpMessage, HttpRequest};
use common::api::{ApiErrorResponse, WrappingResponse};
use common_local::setup::ConfigEmail;
use common_local::{MemberAuthType, Permissions};

use crate::config::{get_config, is_setup, save_config, update_config};
use crate::http::JsonResponse;
use crate::model::AuthModel;
use crate::model::{MemberModel, NewMemberModel};
use crate::{Error, Result, WebResult};
use chrono::Utc;
use lettre::message::header::ContentType;
use lettre::message::{MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Address, Message, SmtpTransport, Transport};
use serde::{Deserialize, Serialize};

use crate::database::Database;

use super::MemberCookie;

pub static PASSWORDLESS_PATH: &str = "/auth/passwordless";
pub static PASSWORDLESS_PATH_CB: &str = "/auth/passwordless/response";

#[derive(Serialize, Deserialize)]
pub struct PostPasswordlessCallback {
    pub email: String,
}

pub async fn post_passwordless_oauth(
    req: HttpRequest,
    query: web::Json<PostPasswordlessCallback>,
    member_cookie: Option<MemberCookie>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
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

    if !is_setup() {
        return Err(ApiErrorResponse::new("Already logged in").into());
    }

    let PostPasswordlessCallback {
        email: mut email_str,
    } = query.into_inner();
    email_str = email_str.trim().to_string();

    if guest_member.is_none() && MemberModel::find_one_by_email(&email_str, &db.basic())
        .await?
        .is_none()
        && get_config().has_admin_account
    {
        return Err(
            ApiErrorResponse::new("Hey dum dum! You have no invite with this email!").into(),
        );
    }

    // Verify that's is a proper email address.
    let _ = email_str.parse::<Address>().map_err(Error::from)?;

    let config = get_config();

    let (Some(email_config), Some(host)) = (
        config.email,
        req.headers().get("host").and_then(|v| v.to_str().ok()),
    ) else {
        return Err(
            ApiErrorResponse::new("Missing email from config OR unable to get host").into(),
        );
    };

    let proto = if config.server.is_secure {
        "https"
    } else {
        "http"
    };

    let auth_model = AuthModel::new(None);

    let auth_callback_url = format!(
        "{proto}://{host}{}?{}",
        PASSWORDLESS_PATH_CB,
        serde_urlencoded::to_string(QueryCallback {
            oauth_token: auth_model.oauth_token.clone().unwrap(),
            email: email_str.clone()
        })
        .map_err(Error::from)?
    );

    let main_html = render_email(
        proto,
        host,
        &email_config.display_name,
        &auth_callback_url
    );

    auth_model.insert(&db.basic()).await?;

    send_auth_email(email_str, auth_callback_url, main_html, &email_config)?;

    Ok(web::Json(WrappingResponse::okay("success")))
}

#[derive(Serialize, Deserialize)]
pub struct QueryCallback {
    pub oauth_token: String,
    pub email: String,
}

pub async fn get_passwordless_oauth_callback(
    request: HttpRequest,
    query: web::Query<QueryCallback>,
    member_cookie: Option<MemberCookie>,
    db: web::Data<Database>,
) -> WebResult<HttpResponse> {
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

    let QueryCallback { oauth_token, email } = query.into_inner();
    let email = email.trim().to_string();

    // Verify that's is a proper email address.
    let address = email.parse::<Address>().map_err(Error::from)?;

    if let Some(auth) = AuthModel::find_by_token(&oauth_token, &db.basic()).await? {
        // Create or Update User.
        let mut member = if let Some(value) =
            MemberModel::find_one_by_email(&email, &db.basic()).await?
        {
            value
        } else if let Some(mut guest_member) = guest_member {
            guest_member.convert_to_email(email);

            guest_member
        } else if !get_config().has_admin_account {
            // Double check that we don't already have users in the database.
            if MemberModel::count(&db.basic()).await? != 0 {
                update_config(|config| {
                    config.has_admin_account = true;
                    Ok(())
                })?;

                save_config().await?;

                return Err(
                    ApiErrorResponse::new("We already have people in the database.").into(),
                );
            }

            // If we don't have an admin account this means we should create one.
            let new_member = NewMemberModel {
                name: address.user().to_string(),
                email,
                type_of: MemberAuthType::Passwordless,
                permissions: Permissions::owner(),
                library_access: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            let mut inserted = new_member.insert(&db.basic()).await?;

            // We instantly accept the invite.
            inserted
                .accept_invite(MemberAuthType::Passwordless, None, &db.basic())
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
            member
                .accept_invite(MemberAuthType::Passwordless, None, &db.basic())
                .await?;
        }

        AuthModel::update_with_member_id(&auth.oauth_token_secret, member.id, &db.basic()).await?;

        super::remember_member_auth(&request.extensions(), member.id, auth.oauth_token_secret)?;
    }

    Ok(HttpResponse::Found()
        .append_header((header::LOCATION, "/"))
        .finish())
}

// TODO: Send emails/tests from own thread entirely. lettre uses a loop system.

pub fn test_connection(email_config: &ConfigEmail) -> Result<bool> {
    let creds = Credentials::new(
        email_config.smtp_username.clone(),
        email_config.smtp_password.clone(),
    );

    // Open a remote connection to gmail
    let mailer = SmtpTransport::relay(&email_config.smtp_relay)?
        .credentials(creds)
        .build();

    Ok(mailer.test_connection()?)
}

pub fn send_auth_email(
    sending_to_email: String,
    alt_text: String,
    main_html: String,
    email_config: &ConfigEmail,
) -> Result<()> {
    let email = Message::builder()
        .from(
            format!(
                "{} <{}>",
                email_config.display_name, email_config.sending_email
            )
            .parse()?,
        )
        .reply_to(email_config.sending_email.parse()?)
        .to(sending_to_email.parse()?)
        .subject(&email_config.subject_line)
        .multipart(
            MultiPart::alternative() // This is composed of two parts.
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_PLAIN)
                        .body(alt_text),
                )
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(main_html),
                ),
        )?;

    let creds = Credentials::new(
        email_config.smtp_username.clone(),
        email_config.smtp_password.clone(),
    );

    // Open a remote connection to gmail
    let mailer = SmtpTransport::relay(&email_config.smtp_relay)?
        .credentials(creds)
        .build();

    // Send the email
    mailer.send(&email)?;

    Ok(())
}

// TODO: Change. Based off of peakdesign's passwordless email.
fn render_email(
    website_url_protocol: &str,
    website_http_base_host: &str,
    email_display_name: &str,
    email_callback_url: &str,
) -> String {
    format!(
        r#"
        <!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd">
        <html xmlns="http://www.w3.org/1999/xhtml">
            <head>
                <meta http-equiv="Content-Type" content="text/html; charset=UTF-8">
            </head>
            <body
                leftmargin="0"
                marginwidth="0"
                topmargin="0"
                marginheight="0"
                offset="0"
                style="margin: 0; padding: 0; font-family: Verdana, sans-serif; height: 100%;"
            >
                <center>
                    <table style="width: 600px; mso-table-lspace: 0pt; mso-table-rspace: 0pt; margin: 0; padding: 0; font-family: Verdana,sans-serif; border-collapse: collapse; height: 100%;" align="center" border="0" cellpadding="0" cellspacing="0" height="100%" width="100%" id="bodyTable">
                        <tr>
                            <td
                                align="center"
                                valign="top"
                                style="mso-table-lspace: 0pt; mso-table-rspace: 0pt; margin: 0; padding: 20px; font-family: Verdana, sans-serif; height: 100%;"
                            >
                                <div>
                                    <p style="text-align: center; margin-bottom: 30px;">
                                        <img
                                            src="{website_url_protocol}://{website_http_base_host}/brand.png"
                                            width="120"
                                            alt="{email_display_name}"
                                            style="-ms-interpolation-mode: bicubic; border: 0; height: auto; line-height: 100%; outline: none; text-decoration: none;"
                                        >
                                    </p>

                                    <p style="font-size: 1.2em; line-height: 1.3;">Please click and confirm that you want to sign in to {email_display_name}. This link will expire shortly.</p>

                                    <div style="text-align: center;">
                                        <a
                                            style="text-transform: uppercase; letter-spacing: 1px; color: #ffffff; text-decoration: none; display: inline-block; min-height: 48px; line-height: 48px; padding-top: 0; padding-right: 26px; padding-bottom: 0; margin: 20px 0; padding-left: 26px; border: 0; outline: 0; font-size: 14px; font-style: normal; font-weight: 400; text-align: center; white-space: nowrap; border-radius: 3px; text-overflow: ellipsis; max-width: 280px; overflow: hidden; background: white; color: #333132; border: 1px solid #7c7622;"
                                            href="{email_callback_url}"
                                        >Sign in to {email_display_name}</a>
                                    </div>

                                    <p>Or sign in using this link:</p>

                                    <p>
                                        <a
                                            style="font-size: 12px; color: #5c581c; text-decoration: none; word-break: break-all;"
                                            href="{email_callback_url}"
                                        >{email_callback_url}</a>
                                    </p>

                                    <br>

                                    <span>Thanks!</span>

                                    <br>

                                    <strong>{email_display_name}</strong>

                                    <br><br>

                                    <hr style="border: 2px solid #e3e7ec; border-bottom: 0; margin: 20px 0;">

                                    <p style="text-align: center; color: #5c581c;">If you did not make this request, you can safely ignore this e-mail.</p>
                                </div>
                            </td>
                        </tr>
                    </table>
                </center>
            </body>
        </html>"#
    )
}
