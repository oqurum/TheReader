use actix_web::web;
use actix_web::HttpMessage;
use actix_web::HttpRequest;

use common::api::ApiErrorResponse;
use common::api::WrappingResponse;

use crate::SqlPool;
use crate::http::JsonResponse;
use crate::model::AuthModel;
use crate::model::NewClientModel;
use crate::model::NewMemberModel;
use crate::WebResult;

use super::MemberCookie;

pub static GUEST_PATH: &str = "/auth/guest";

pub async fn post_guest_oauth(
    request: HttpRequest,
    member_cookie: Option<MemberCookie>,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<String>> {
    if member_cookie.is_some() {
        return Err(ApiErrorResponse::new("Already logged in").into());
    }

    let mut conn = db.acquire().await?;

    let member = NewMemberModel::new_guest();

    let member = member.insert(&mut *conn).await?;

    // TODO: Consolidate these in function inside Auth.
    let auth = AuthModel::new(Some(member.id));

    auth.insert(&mut *conn).await?;

    if let Some(header) = request.headers().get(reqwest::header::USER_AGENT).and_then(|v| v.to_str().ok()) {
        NewClientModel::new(
            auth.oauth_token_secret.clone(),
            String::from("Web"),
            header,
        ).insert(&mut *conn).await?;
    }

    super::remember_member_auth(&request.extensions(), member.id, auth.oauth_token_secret)?;

    Ok(web::Json(WrappingResponse::okay(String::from("success"))))
}
