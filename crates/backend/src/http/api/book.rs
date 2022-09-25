use actix_web::{get, web, post, delete};

use common_local::{api, SearchType, SearchFor, SearchForBooksBy, Poster, DisplayItem, filter::FilterContainer, ModifyValuesBy};
use chrono::Utc;
use common::{ImageType, Either, api::{ApiErrorResponse, WrappingResponse, DeletionResponse}, PersonId, MISSING_THUMB_PATH, BookId};
use serde_qs::actix::QsQuery;

use crate::{database::Database, task::{queue_task_priority, self}, queue_task, metadata::{self, ActiveAgents}, WebResult, Error, store_image, model::{image::{ImageLinkModel, UploadedImageModel}, book::BookModel, file::FileModel, progress::FileProgressionModel, person::PersonModel, book_person::BookPersonModel}, http::{MemberCookie, JsonResponse}};




#[get("/books")]
pub async fn load_book_list(
    query: QsQuery<api::BookListQuery>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetBookListResponse>> {
    let (items, count) = if query.has_query() {
        let search = &query.filters;

        let count = BookModel::count_search_by(search, query.library, &db).await?;

        let items = if count == 0 {
            Vec::new()
        } else {
            BookModel::search_by(
                search,
                query.library,
                query.offset.unwrap_or(0),
                query.limit.unwrap_or(50),
                &db,
            ).await?
                .into_iter()
                .map(|book| {
                    DisplayItem {
                        id: book.id,
                        title: book.title.or(book.original_title).unwrap_or_default(),
                        cached: book.cached,
                        thumb_path: book.thumb_path,
                    }
                })
                .collect()
        };

        (items, count)
    } else {
        let count = BookModel::count_search_by(
            &FilterContainer::default(),
            query.library,
            &db,
        ).await?;

        let items = BookModel::find_by(
            query.library,
            query.offset.unwrap_or(0),
            query.limit.unwrap_or(50),
            None,
            &db,
        ).await?
            .into_iter()
            .map(|book| {
                DisplayItem {
                    id: book.id,
                    title: book.title.or(book.original_title).unwrap_or_default(),
                    cached: book.cached,
                    thumb_path: book.thumb_path,
                }
            })
            .collect();

        (items, count)
    };

    Ok(web::Json(WrappingResponse::okay(api::GetBookListResponse {
        items,
        count,
    })))
}


#[post("/book")]
pub async fn update_books(
    body: web::Json<api::MassEditBooks>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    let edit = body.into_inner();

    let member = member.fetch_or_error(&db).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    // TODO: YES, I KNOW! I'm just lazy.
    // People
    match edit.people_list_mod {
        ModifyValuesBy::Overwrite => {
            for book_id in edit.book_ids {
                BookPersonModel::delete_by_book_id(book_id, &db).await?;

                for person_id in edit.people_list.iter().copied() {
                    BookPersonModel { book_id, person_id }.insert_or_ignore(&db).await?;
                }

                // Update the cached author name
                if let Some(person_id) = edit.people_list.first().copied() {
                    let person = PersonModel::find_one_by_id(person_id, &db).await?;
                    let book = BookModel::find_one_by_id(book_id, &db).await?;

                    if let Some((person, mut book)) = person.zip(book) {
                        book.cached.author = Some(person.name);
                        book.update(&db).await?;
                    }
                }
            }
        }

        ModifyValuesBy::Append => {
            for book_id in edit.book_ids {
                for person_id in edit.people_list.iter().copied() {
                    BookPersonModel { book_id, person_id }.insert_or_ignore(&db).await?;
                }
            }
        }

        ModifyValuesBy::Remove => {
            for book_id in edit.book_ids {
                for person_id in edit.people_list.iter().copied() {
                    BookPersonModel { book_id, person_id }.delete(&db).await?;
                }

                // TODO: Check if we removed cached author
                // If book has no other people referenced we'll update the cached author name.
                if BookPersonModel::find_by(Either::Left(book_id), &db).await?.is_empty() {
                    let book = BookModel::find_one_by_id(book_id, &db).await?;

                    if let Some(mut book) = book {
                        book.cached.author = None;
                        book.update(&db).await?;
                    }
                }
            }
        }
    }

    Ok(web::Json(WrappingResponse::okay("success")))
}



// Book
#[get("/book/{id}")]
pub async fn load_book_info(member: MemberCookie, book_id: web::Path<BookId>, db: web::Data<Database>) -> WebResult<JsonResponse<api::ApiGetBookByIdResponse>> {
    let book = BookModel::find_one_by_id(*book_id, &db).await?.unwrap();

    let (mut media, mut progress) = (Vec::new(), Vec::new());

    for file in FileModel::find_by_book_id(book.id, &db).await? {
        let prog = FileProgressionModel::find_one(member.member_id(), file.id, &db).await?;

        media.push(file.into());
        progress.push(prog.map(|v| v.into()));
    }

    let people = PersonModel::find_by_book_id(book.id, &db).await?;

    Ok(web::Json(WrappingResponse::okay(api::GetBookResponse {
        book: book.into(),
        media,
        progress,
        people: people.into_iter()
            .map(|p| p.into())
            .collect(),
    })))
}

#[post("/book/{id}")]
pub async fn update_book_info(
    book_id: web::Path<BookId>,
    body: web::Json<api::PostBookBody>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    let book_id = *book_id;

    let member = member.fetch_or_error(&db).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    match body.into_inner() {
        api::PostBookBody::AutoMatchBookIdByFiles => {
            queue_task(task::TaskUpdateInvalidBook::new(task::UpdatingBook::AutoUpdateBookIdByFiles(book_id)));
        }

        api::PostBookBody::AutoMatchBookId => {
            queue_task(task::TaskUpdateInvalidBook::new(task::UpdatingBook::AutoUpdateBookId(book_id)));
        }

        api::PostBookBody::UpdateBookBySource(source) => {
            queue_task_priority(task::TaskUpdateInvalidBook::new(task::UpdatingBook::UpdateBookWithSource { book_id, source }));
        }

        api::PostBookBody::Edit(edit) => {
            BookModel::edit_book_by_id(book_id, edit, &db).await?;
        }
    }

    Ok(web::Json(WrappingResponse::okay("success")))
}


#[get("/book/{id}/posters")]
async fn get_book_posters(
    path: web::Path<BookId>,
    db: web::Data<Database>
) -> WebResult<JsonResponse<api::ApiGetPosterByBookIdResponse>> {
    let book = BookModel::find_one_by_id(*path, &db).await?.unwrap();

    // TODO: For Open Library we need to go from an Edition to Work.
    // Work is the main book. Usually consisting of more posters.
    // We can do they by works[0].key = "/works/OLXXXXXXW"

    let mut items: Vec<Poster> = ImageLinkModel::find_with_link_by_link_id(**path, ImageType::Book, &db).await?
        .into_iter()
        .map(|poster| Poster {
            id: Some(poster.image_id),

            selected: poster.path == book.thumb_path,

            path: poster.path.into_value()
                .map(|v| format!("/api/image/{v}"))
                .unwrap_or_else(|| String::from(MISSING_THUMB_PATH)),

            created_at: poster.created_at,
        })
        .collect();

    let search = crate::metadata::search_all_agents(
        &format!(
            "{} {}",
            book.title.as_deref().or(book.title.as_deref()).unwrap_or_default(),
            book.cached.author.as_deref().unwrap_or_default(),
        ),
        common_local::SearchFor::Book(common_local::SearchForBooksBy::Query),
        &ActiveAgents::default(),
    ).await?;

    for item in search.0.into_values().flatten() {
        if let crate::metadata::SearchItem::Book(item) = item {
            for path in item.thumb_locations.into_iter().filter_map(|v| v.into_url_value()) {
                items.push(Poster {
                    id: None,

                    selected: false,
                    path,

                    created_at: Utc::now(),
                });
            }
        }
    }

    Ok(web::Json(WrappingResponse::okay(api::GetPostersResponse {
        items
    })))
}

#[post("/book/{id}/posters")]
async fn insert_or_update_book_image(
    book_id: web::Path<BookId>,
    body: web::Json<api::ChangePosterBody>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    let member = member.fetch_or_error(&db).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    let mut book = BookModel::find_one_by_id(*book_id, &db).await?.unwrap();

    match body.into_inner().url_or_id {
        Either::Left(url) => {
            let resp = reqwest::get(url)
                .await.map_err(Error::from)?
                .bytes()
                .await.map_err(Error::from)?;

            let image_model = store_image(resp.to_vec(), &db).await?;

            book.thumb_path = image_model.path.clone();

            ImageLinkModel::new_book(image_model.id, book.id).insert(&db).await?;
        }

        Either::Right(id) => {
            let poster = UploadedImageModel::get_by_id(id, &db).await?.unwrap();

            if book.thumb_path == poster.path {
                return Ok(web::Json(WrappingResponse::okay("success")));
            }

            book.thumb_path = poster.path;
        }
    }

    book.update(&db).await?;

    Ok(web::Json(WrappingResponse::okay("success")))
}


#[get("/book/search")]
pub async fn book_search(body: web::Query<api::GetBookSearch>) -> WebResult<JsonResponse<api::ApiGetBookSearchResponse>> {
    let search = metadata::search_all_agents(
        &body.query,
        match body.search_type {
            // TODO: Allow for use in Query.
            SearchType::Book => SearchFor::Book(SearchForBooksBy::Query),
            SearchType::Person => SearchFor::Person
        },
        &ActiveAgents::default()
    ).await?;

    Ok(web::Json(WrappingResponse::okay(api::BookSearchResponse {
        items: search.0.into_iter()
            .map(|(a, b)| (
                a.into_owned(),
                b.into_iter().map(|v| {
                    match v {
                        metadata::SearchItem::Book(book) => {
                            api::SearchItem::Book(api::MetadataBookSearchItem {
                                source: book.source,
                                author: book.cached.author,
                                description: book.description,
                                name: book.title.unwrap_or_else(|| String::from("Unknown title")),
                                thumbnail_url: book.thumb_locations.first()
                                    .and_then(|v| v.as_url_value())
                                    .map(|v| v.to_string())
                                    .unwrap_or_default(),
                            })
                        }

                        metadata::SearchItem::Author(author) => {
                            api::SearchItem::Person(api::MetadataPersonSearchItem {
                                source: author.source,

                                cover_image: author.cover_image_url.and_then(|v| v.into_url_value()),

                                name: author.name,
                                other_names: author.other_names,
                                description: author.description,

                                birth_date: author.birth_date,
                                death_date: author.death_date,
                            })
                        }
                    }
                }).collect()
            ))
            .collect()
    })))
}





#[post("/book/{book_id}/person/{person_id}")]
async fn insert_book_person(
    ids: web::Path<(BookId, PersonId)>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<String>> {
    let (book_id, person_id) = ids.into_inner();

    let member = member.fetch_or_error(&db).await?;

    if !member.permissions.is_owner() {
        return Ok(web::Json(WrappingResponse::error("You cannot do this! No Permissions!")));
    }

    // If book had no other people referenced we'll update the cached author name.
    if BookPersonModel::find_by(Either::Left(book_id), &db).await?.is_empty() {
        let person = PersonModel::find_one_by_id(person_id, &db).await?;
        let book = BookModel::find_one_by_id(book_id, &db).await?;

        if let Some((person, mut book)) = person.zip(book) {
            book.cached.author = Some(person.name);
            book.update(&db).await?;
        }
    }

    BookPersonModel { book_id, person_id }.insert_or_ignore(&db).await?;

    Ok(web::Json(WrappingResponse::okay(String::from("success"))))
}

#[delete("/book/{book_id}/person/{person_id}")]
async fn delete_book_person(
    ids: web::Path<(BookId, PersonId)>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<DeletionResponse>> {
    let (book_id, person_id) = ids.into_inner();

    let member = member.fetch_or_error(&db).await?;

    if !member.permissions.is_owner() {
        return Ok(web::Json(WrappingResponse::error("You cannot do this! No Permissions!")));
    }

    BookPersonModel { book_id, person_id }.delete(&db).await?;

    // If book has no other people referenced we'll update the cached author name.
    if BookPersonModel::find_by(Either::Left(book_id), &db).await?.is_empty() {
        let book = BookModel::find_one_by_id(book_id, &db).await?;

        if let Some(mut book) = book {
            book.cached.author = None;
            book.update(&db).await?;
        }
    }

    // TODO: Return total deleted
    Ok(web::Json(WrappingResponse::okay(DeletionResponse {
        total: 1,
    })))
}

