/// Personal Collections
use actix_web::{delete, get, post, web};
use common::{api::WrappingResponse, BookId, ThumbnailStore};
use common_local::{api, CollectionId, DisplayItem};

use crate::{
    database::Database,
    http::{JsonResponse, MemberCookie},
    model::{
        BookModel,
        CollectionModel, NewCollectionModel,
        CollectionItemModel,
    },
    WebResult,
};

#[get("/collections")]
async fn load_collection_list(
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetCollectionListResponse>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    let items = CollectionModel::find_by_member_id(member.id, &db.basic())
        .await?
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

// TODO: Implement into /books API.
#[get("/collection/{id}/books")]
async fn load_collection_id_books(
    id: web::Path<CollectionId>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetCollectionIdBooksResponse>> {
    let access = db.basic();

    let member = member.fetch_or_error(&access).await?;

    let model = CollectionModel::find_one_by_id(*id, member.id, &access)
        .await?
        .ok_or_else(|| crate::Error::from(crate::InternalError::ItemMissing))?;

    let mut books = Vec::new();

    // TODO: Turn into a single SQL Query
    for item in CollectionItemModel::find_by_collection_id(model.id, &access).await? {
        if let Some(book) = BookModel::find_one_by_id(item.book_id, &access).await? {
            books.push(DisplayItem {
                id: book.id,
                title: book.title.or(book.original_title).unwrap_or_default(),
                cached: book.cached,
                thumb_path: book.thumb_path,
            });
        }
    }

    Ok(web::Json(WrappingResponse::okay(
        api::GetBookListResponse {
            count: books.len(),
            items: books,
        },
    )))
}

#[post("/collection")]
async fn new_collection(
    web::Json(mut body): web::Json<api::NewCollectionBody>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetCollectionIdResponse>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    body.name.truncate(30);

    if let Some(desc) = body.description.as_mut() {
        desc.truncate(100);
    }

    let model = NewCollectionModel {
        member_id: member.id,

        name: body.name,
        description: body
            .description
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty()),

        thumb_url: ThumbnailStore::None,
    }
    .insert(&db.basic())
    .await?;

    Ok(web::Json(WrappingResponse::okay(model.into())))
}

#[post("/collection/{id}/book/{book_id}")]
async fn add_book_to_collection(
    id: web::Path<(CollectionId, BookId)>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    let access = db.basic();

    let member = member.fetch_or_error(&access).await?;

    let _model = CollectionModel::find_one_by_id(id.0, member.id, &access)
        .await?
        .ok_or_else(|| crate::Error::from(crate::InternalError::ItemMissing))?;

    CollectionItemModel {
        collection_id: id.0,
        book_id: id.1,
    }
    .insert_or_ignore(&db.basic())
    .await?;

    Ok(web::Json(WrappingResponse::okay("ok")))
}

#[delete("/collection/{id}/book/{book_id}")]
async fn remove_book_from_collection(
    id: web::Path<(CollectionId, BookId)>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    let access = db.basic();

    let member = member.fetch_or_error(&access).await?;

    let _model = CollectionModel::find_one_by_id(id.0, member.id, &access)
        .await?
        .ok_or_else(|| crate::Error::from(crate::InternalError::ItemMissing))?;

    CollectionItemModel {
        collection_id: id.0,
        book_id: id.1,
    }
    .delete_one(&db.basic())
    .await?;

    Ok(web::Json(WrappingResponse::okay("ok")))
}
