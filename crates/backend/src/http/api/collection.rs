/// Personal Collections

use actix_web::{get, post, web};
use common::{api::WrappingResponse, ThumbnailStore};
use common_local::{api, CollectionId};

use crate::{http::{JsonResponse, MemberCookie}, WebResult, database::Database, model::collection::{CollectionModel, NewCollectionModel}};




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
        description: body.description,

        thumb_url: ThumbnailStore::None,
    }.insert(&db.basic()).await?;

    Ok(web::Json(WrappingResponse::okay(model.into())))
}
