use actix_web::{get, post, web};
use common::api::WrappingResponse;
use common_local::MemberPreferences;
use lazy_static::lazy_static;

use crate::{http::{MemberCookie, JsonResponse}, database::Database, WebResult};


lazy_static! {
    static ref DEFAULT_PREFERENCE: MemberPreferences = MemberPreferences::default();
}


#[get("/preferences")]
pub async fn get_preferences(
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<MemberPreferences>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    // TODO: Validate Preferences before parsing. i.e. version isn't the latest server one.

    let prefs = if let Some(pref) = member.preferences {
        serde_json::from_str(&pref).map_err(crate::Error::from)?
    } else {
        MemberPreferences::default()
    };


    Ok(web::Json(WrappingResponse::okay(prefs)))
}

#[post("/preferences")]
pub async fn post_preferences(
    json: web::Json<MemberPreferences>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    let mut member = member.fetch_or_error(&db.basic()).await?;

    let pref = json.into_inner();

    // TODO: Validate Preferences before saving. i.e. clients' version is larger than server one.

    member.preferences = if pref == *DEFAULT_PREFERENCE {
        None
    } else {
        Some(serde_json::to_string(&pref).map_err(crate::Error::from)?)
    };

    member.update(&db.basic()).await?;

    Ok(web::Json(WrappingResponse::okay("ok")))
}
