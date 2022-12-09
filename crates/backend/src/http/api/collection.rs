/// Personal Collections

use actix_web::{get, post, web};
use common::{api::WrappingResponse, ThumbnailStore};
use common_local::{api, CollectionId};

use crate::{http::{JsonResponse, MemberCookie}, WebResult, database::Database, model::collection::{CollectionModel, NewCollectionModel}};


#[get("/collections")]
async fn load_collection_list(
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetCollectionListResponse>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    let items = CollectionModel::find_by_member_id(member.id, &db.basic()).await?
        .into_iter()
        .map(|v| v.into())
        .collect();

    Ok(web::Json(WrappingResponse::okay(items)))
}



#[get("/collection/{id}")]
async fn load_collection_id(
    id: web::Path<CollectionId>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetCollectionIdResponse>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    let model = CollectionModel::find_one_by_id(*id, member.id, &db.basic())
        .await?
        .ok_or_else(|| crate::Error::from(crate::InternalError::ItemMissing))?;

    Ok(web::Json(WrappingResponse::okay(model.into())))
}




#[post("/collection")]
async fn new_collection(
    web::Json(body): web::Json<api::NewCollectionBody>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetCollectionIdResponse>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    let model = NewCollectionModel {
        member_id: member.id,

        name: body.name,
        description: body.description.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()),

        thumb_url: ThumbnailStore::None,
    }.insert(&db.basic()).await?;

    Ok(web::Json(WrappingResponse::okay(model.into())))
}
