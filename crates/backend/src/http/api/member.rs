use actix_web::{get, post, web};
use common::{
    api::{ApiErrorResponse, ErrorCodeResponse, WrappingResponse},
    MemberId,
};
use common_local::{api, MemberUpdate};

use crate::{
    http::{JsonResponse, MemberCookie},
    model::{MemberModel, NewMemberModel},
    SqlPool, WebResult,
};

// TODO: Add body requests for specifics
#[get("/member")]
pub async fn load_member_self(
    member: Option<MemberCookie>,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<api::ApiGetMemberSelfResponse>> {
    if let Some(member) = &member {
        let member = member.fetch_or_error(&mut *db.acquire().await?).await?;

        Ok(web::Json(WrappingResponse::okay(
            api::GetMemberSelfResponse {
                member: Some(member.into()),
            },
        )))
    } else {
        Ok(web::Json(WrappingResponse::error_code(
            "Not Signed in.",
            ErrorCodeResponse::NotLoggedIn,
        )))
    }
}

#[post("/member")]
pub async fn update_member(
    member: MemberCookie,
    update: web::Json<api::UpdateMember>,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<&'static str>> {
    let member = member.fetch_or_error(&mut *db.acquire().await?).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    match update.into_inner() {
        api::UpdateMember::Delete { id } => {
            MemberModel::delete(id, &mut *db.acquire().await?).await?;
        }

        api::UpdateMember::Invite { email } => {
            NewMemberModel::from_email(email)
                .insert(&mut *db.acquire().await?)
                .await?;

            // TODO: Send an email.
        }
    }

    Ok(web::Json(WrappingResponse::okay("ok")))
}

#[get("/members")]
pub async fn load_members_list(
    member: MemberCookie,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<api::ApiGetMembersListResponse>> {
    let member = member.fetch_or_error(&mut *db.acquire().await?).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    let count = MemberModel::count(&mut *db.acquire().await?).await?;

    let members = MemberModel::get_all(&mut *db.acquire().await?).await?;

    Ok(web::Json(WrappingResponse::okay(
        api::GetMembersListResponse {
            items: members.into_iter().map(|v| v.into()).collect(),
            count: count as usize,
        },
    )))
}

// TODO: Remove api::UpdateMember::Delete and replace with #[delete("/member/{id}")]
// TODO: Allow self to update their own member.
// Update Member
#[post("/member/{id}")]
pub async fn update_member_id(
    id: web::Path<MemberId>,
    member: MemberCookie,
    update: web::Json<MemberUpdate>,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<api::GetMemberSelfResponse>> {
    let member_owner = member.fetch_or_error(&mut *db.acquire().await?).await?;

    if !member_owner.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    let Some(mut member_updating) = MemberModel::find_one_by_id(*id, &mut *db.acquire().await?).await? else {
        return Err(ApiErrorResponse::new("Unable to find Member to Update").into());
    };

    member_updating
        .update_with(update.into_inner(), &mut *db.acquire().await?)
        .await?;

    Ok(web::Json(WrappingResponse::okay(
        api::GetMemberSelfResponse {
            member: Some(member_updating.into()),
        },
    )))
}
