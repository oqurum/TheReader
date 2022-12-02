use actix_web::{get, web, post};
use common::api::{WrappingResponse, ApiErrorResponse};
use common_local::api;

use crate::{
    database::Database,
    http::{JsonResponse, MemberCookie},
    WebResult, model::member::MemberModel,
};

// TODO: Add body requests for specifics
#[get("/member")]
pub async fn load_member_self(
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetMemberSelfResponse>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    Ok(web::Json(WrappingResponse::okay(
        api::GetMemberSelfResponse {
            member: Some(member.into()),
        },
    )))
}

#[post("/member")]
pub async fn update_member(
    member: MemberCookie,
    update: web::Json<api::UpdateMember>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    match update.into_inner() {
        api::UpdateMember::Delete { id } => {
            MemberModel::delete(id, &db.basic()).await?;
        }

        api::UpdateMember::Update { id: _id } => {
            //
        }
    }

    Ok(web::Json(WrappingResponse::okay("ok")))
}


#[get("/members")]
pub async fn load_members_list(
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetMembersListResponse>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    let count = MemberModel::count(&db.basic()).await?;

    let members = MemberModel::get_all(&db.basic()).await?;

    Ok(web::Json(WrappingResponse::okay(
        api::GetMembersListResponse {
            items: members.into_iter().map(|v| v.into()).collect(),
            count,
        },
    )))
}