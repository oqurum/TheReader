use actix_web::{get, web};
use common::api::WrappingResponse;
use common_local::api;

use crate::{
    database::Database,
    http::{JsonResponse, MemberCookie},
    WebResult,
};

// TODO: Add body requests for specifics
#[get("/member")]
pub async fn load_member_self(
    cookie: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetMemberSelfResponse>> {
    let member = cookie.fetch_or_error(&db).await?;

    Ok(web::Json(WrappingResponse::okay(
        api::GetMemberSelfResponse {
            member: Some(member.into()),
        },
    )))
}
