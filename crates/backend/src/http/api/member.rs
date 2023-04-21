use actix_web::{get, post, web, HttpRequest, HttpMessage};
use common::{api::{ApiErrorResponse, WrappingResponse}, MemberId};
use common_local::{api, MemberUpdate};

use crate::{
    database::Database,
    http::{JsonResponse, MemberCookie, remember_member_auth},
    model::{member::{MemberModel, NewMemberModel}, auth::AuthModel, client::NewClientModel},
    WebResult, config::get_config,
};

// TODO: Add body requests for specifics
#[get("/member")]
pub async fn load_member_self(
    request: HttpRequest,
    member: Option<MemberCookie>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetMemberSelfResponse>> {
    if let Some(member) = &member {
        let member = member.fetch_or_error(&db.basic()).await?;

        Ok(web::Json(WrappingResponse::okay(
            api::GetMemberSelfResponse {
                member: Some(member.into()),
            },
        )))
    } else {
        let config = get_config();

        if config.has_admin_account && config.is_public_access {
            // TODO: Utilize IP. If we have 20+ guest members on the same ip we'll clear them. It'll prevent mass-creation of guest accounts.

            let member = NewMemberModel::new_guest();

            let member = member.insert(&db.basic()).await?;

            // TODO: Consolidate these in function inside Auth.
            let auth = AuthModel::new(Some(member.id));

            auth.insert(&db.basic()).await?;

            if let Some(header) = request.headers().get(reqwest::header::USER_AGENT).and_then(|v| v.to_str().ok()) {
                NewClientModel::new(
                    auth.oauth_token_secret.clone(),
                    String::from("Web"),
                    header,
                ).insert(&db.basic()).await?;
            }

            remember_member_auth(&request.extensions(), member.id, auth.oauth_token_secret)?;

            Ok(web::Json(WrappingResponse::okay(
                api::GetMemberSelfResponse {
                    member: Some(member.into()),
                },
            )))
        } else {
            Ok(web::Json(WrappingResponse::error("Not Signed in.")))
        }
    }
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

        api::UpdateMember::Invite { email } => {
            NewMemberModel::from_email(email)
                .insert(&db.basic())
                .await?;

            // TODO: Send an email.
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

// TODO: Remove api::UpdateMember::Delete and replace with #[delete("/member/{id}")]
// TODO: Allow self to update their own member.
// Update Member
#[post("/member/{id}")]
pub async fn update_member_id(
    id: web::Path<MemberId>,
    member: MemberCookie,
    update: web::Json<MemberUpdate>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::GetMemberSelfResponse>> {
    let member_owner = member.fetch_or_error(&db.basic()).await?;

    if !member_owner.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    let Some(mut member_updating) = MemberModel::find_one_by_id(*id, &db.basic()).await? else {
        return Err(ApiErrorResponse::new("Unable to find Member to Update").into());
    };

    member_updating.update_with(update.into_inner(), &db.basic()).await?;

    Ok(web::Json(WrappingResponse::okay(api::GetMemberSelfResponse {
        member: Some(member_updating.into()),
    })))
}
