// You supply an email. We email the link to authenticate with.

// TODO: Better security. Simple Proof of Concept.


use actix_identity::Identity;
use actix_web::{http::header, HttpResponse};
use actix_web::{web, HttpRequest, HttpMessage};
use common_local::setup::ConfigEmail;
use common_local::{Permissions, MemberAuthType};
use common::api::{WrappingResponse, ApiErrorResponse};

use crate::config::{is_setup, get_config, update_config, save_config};
use crate::http::JsonResponse;
use crate::model::auth::AuthModel;
use crate::model::member::{NewMemberModel, MemberModel};
use crate::{Result, WebResult, Error};
use chrono::Utc;
use lettre::message::header::ContentType;
use lettre::message::{MultiPart, SinglePart};
use lettre::{Message, SmtpTransport, Transport};
use lettre::transport::smtp::authentication::Credentials;
use rand::Rng;
use rand::prelude::ThreadRng;
use serde::{Serialize, Deserialize};

use crate::database::Database;


pub static PASSWORDLESS_PATH: &str = "/auth/passwordless";
pub static PASSWORDLESS_PATH_CB: &str = "/auth/passwordless/response";


#[derive(Serialize, Deserialize)]
pub struct PostPasswordlessCallback {
    pub email: String,
}

pub async fn post_passwordless_oauth(
    req: HttpRequest,
    query: web::Json<PostPasswordlessCallback>,
    identity: Option<Identity>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<String>> {
    if identity.is_some() || !is_setup() {
        return Err(ApiErrorResponse::new("Already logged in").into());
    }

    let config = get_config();

    let (email_config, host) = match (
        config.email,
        req.headers().get("host").and_then(|v| v.to_str().ok())
    ) {
        (Some(a), Some(b)) => (a, b),
        _ => return Err(ApiErrorResponse::new("Missing email from config OR unable to get host").into()),
    };

    let proto = if config.server.is_secure { "https" } else { "http" };


    let oauth_token = gen_sample_alphanumeric(32, &mut rand::thread_rng());



    let auth_callback_url = format!(
        "{proto}://{host}{}?{}",
        PASSWORDLESS_PATH_CB,
        serde_urlencoded::to_string(QueryCallback {
            oauth_token: oauth_token.clone(),
            email: query.email.clone()
        }).map_err(Error::from)?
    );

    let main_html = render_email(
        proto,
        host,
        &email_config.display_name,
        &auth_callback_url,
    );

    AuthModel {
        oauth_token,
        // TODO:
        oauth_token_secret: String::new(),
        created_at: Utc::now(),
    }.insert(&db).await?;

    send_auth_email(query.0.email, auth_callback_url, main_html, &email_config)?;

    Ok(web::Json(WrappingResponse::okay(String::from("success"))))
}

#[derive(Serialize, Deserialize)]
pub struct QueryCallback {
    pub oauth_token: String,
    pub email: String,
}

pub async fn get_passwordless_oauth_callback(
    request: HttpRequest,
    query: web::Query<QueryCallback>,
    identity: Option<Identity>,
    db: web::Data<Database>,
) -> WebResult<HttpResponse> {
    if identity.is_some() {
        return Ok(HttpResponse::Found()
            .append_header((header::LOCATION, "/"))
            .finish());
    }

    let QueryCallback {
        oauth_token,
        email,
    } = query.into_inner();

    if AuthModel::remove_by_oauth_token(&oauth_token, &db).await? {
        // Create or Update User.
        let member = if let Some(value) = MemberModel::find_one_by_email(&email, &db).await? {
            value
        } else {
            let mut new_member = NewMemberModel {
                // TODO: Strip email
                name: email.clone(),
                email: Some(email),
                password: None,
                type_of: MemberAuthType::Passwordless,
                permissions: Permissions::basic(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            let has_admin_account = get_config().has_admin_account;

            // Check to see if we don't have the admin account created yet.
            if !has_admin_account {
                new_member.permissions = Permissions::owner();
            }


            let inserted = new_member.insert(&db).await?;

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

        super::remember_member_auth(&request.extensions(), member.id)?;
    }

    Ok(HttpResponse::Found()
        .append_header((header::LOCATION, "/"))
        .finish())
}

// TODO: Send emails/tests from own thread entirely. lettre uses a loop system.

pub fn test_connection(email_config: &ConfigEmail) -> Result<bool> {
    let creds = Credentials::new(email_config.smtp_username.clone(), email_config.smtp_password.clone());

    // Open a remote connection to gmail
    let mailer = SmtpTransport::relay(&email_config.smtp_relay)?
        .credentials(creds)
        .build();

    Ok(mailer.test_connection()?)
}

pub fn send_auth_email(sending_to_email: String, alt_text: String, main_html: String, email_config: &ConfigEmail) -> Result<()> {
    let email = Message::builder()
        .from(format!("{} <{}>", email_config.display_name, email_config.sending_email).parse()?)
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

    let creds = Credentials::new(email_config.smtp_username.clone(), email_config.smtp_password.clone());

    // Open a remote connection to gmail
    let mailer = SmtpTransport::relay(&email_config.smtp_relay)?
        .credentials(creds)
        .build();

    // Send the email
    mailer.send(&email)?;

    Ok(())
}

pub fn gen_sample_alphanumeric(amount: usize, rng: &mut ThreadRng) -> String {
    rng.sample_iter(rand::distributions::Alphanumeric)
        .take(amount)
        .map(char::from)
        .collect()
}

// TODO: Change. Based off of peakdesign's passwordless email.
fn render_email(
    website_url_protocol: &str,
    website_http_base_host: &str,
    email_display_name: &str,
    email_callback_url: &str,
) -> String {
    format!(r#"
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