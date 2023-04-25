use actix_files::NamedFile;
use actix_web::{
    delete, get,
    http::header::{ContentDisposition, HeaderValue},
    post, web,
};

use chrono::Utc;
use common::{
    api::{ApiErrorResponse, DeletionResponse, WrappingResponse},
    BookId, Either, ImageType, PersonId, MISSING_THUMB_PATH,
};
use common_local::{
    api::{self, BookPresetListType, BookProgression},
    DisplayItem, ModifyValuesBy, Poster, SearchFor, SearchForBooksBy, SearchType, BookType,
};
use serde_qs::actix::QsQuery;

use crate::{
    database::Database,
    http::{JsonResponse, MemberCookie},
    metadata::{self, ActiveAgents},
    model::{
        BookModel,
        BookPersonModel,
        FileModel,
        ImageLinkModel, UploadedImageModel,
        PersonModel,
        FileProgressionModel, LibraryModel,
    },
    queue_task, store_image,
    task::{self, queue_task_priority},
    Error, WebResult,
};

const QUERY_LIMIT: usize = 100;

#[get("/books")]
pub async fn load_book_list(
    query: QsQuery<api::BookListQuery>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetBookListResponse>> {
    let query = query.into_inner();

    let filters = query.filters.unwrap_or_default();

    let member = member.fetch_or_error(&db.basic()).await?;

    // Ensure we can access this library.
    if let Some(library) = query.library {
        let model = LibraryModel::find_one_by_id(library, &db.basic())
            .await?
            .ok_or_else(|| Error::from(crate::InternalError::ItemMissing))?;

        let lib_access = member.parse_library_access_or_default()?;

        if !member.permissions.is_owner() && !lib_access.is_accessible(model.id, model.is_public) {
            return Err(ApiErrorResponse::new("Not accessible").into());
        }
    } else if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not accessible. Not owner.").into());
    }


    let count = BookModel::count_search_by(&filters, query.library, &db.basic()).await?;

    let items = if count == 0 {
        Vec::new()
    } else {
        BookModel::search_by(
            &filters,
            query.library,
            query.offset.unwrap_or(0),
            query.limit.unwrap_or(50).min(QUERY_LIMIT),
            &db.basic(),
        )
        .await?
        .into_iter()
        .map(|book| DisplayItem {
            id: book.id,
            title: book.title.or(book.original_title).unwrap_or_default(),
            cached: book.cached,
            thumb_path: book.thumb_path,
        })
        .collect()
    };

    Ok(web::Json(WrappingResponse::okay(
        api::GetBookListResponse { items, count },
    )))
}

// TODO: Place into GET /books
#[get("/books/preset")]
pub async fn load_book_preset_list(
    query: QsQuery<api::BookPresetListQuery>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::GetBookPresetListResponse>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    match query.preset {
        BookPresetListType::Progressing => {
            let mut items = Vec::new();

            for (a, book) in
                FileProgressionModel::get_member_progression_and_books(
                    member.id,
                    query.offset.unwrap_or(0),
                    query.limit.unwrap_or(50).min(QUERY_LIMIT),
                    &db.basic()
                )
                    .await?
            {
                let file = FileModel::find_one_by_id(a.file_id, &db.basic())
                    .await?
                    .unwrap();

                let book = DisplayItem {
                    id: book.id,
                    title: book.title.or(book.original_title).unwrap_or_default(),
                    cached: book.cached,
                    thumb_path: book.thumb_path,
                };

                items.push(BookProgression {
                    progress: a.into(),
                    book,
                    file: file.into(),
                });
            }

            Ok(web::Json(WrappingResponse::okay(
                api::GetBookPresetListResponse { items, count: 0 },
            )))
        }
    }
}

#[post("/book")]
pub async fn update_books(
    body: web::Json<api::MassEditBooks>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    let edit = body.into_inner();

    let member = member.fetch_or_error(&db.basic()).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    // TODO: YES, I KNOW! I'm just lazy.
    // People
    match edit.people_list_mod {
        ModifyValuesBy::Overwrite => {
            for book_id in edit.book_ids {
                BookPersonModel::delete_by_book_id(book_id, &db.basic()).await?;

                for person_id in edit.people_list.iter().copied() {
                    BookPersonModel { book_id, person_id }
                        .insert_or_ignore(&db.basic())
                        .await?;
                }

                // Update the cached author name
                if let Some(person_id) = edit.people_list.first().copied() {
                    let person = PersonModel::find_one_by_id(person_id, &db.basic()).await?;
                    let book = BookModel::find_one_by_id(book_id, &db.basic()).await?;

                    if let Some((person, mut book)) = person.zip(book) {
                        book.cached.author = Some(person.name);
                        book.update(&db.basic()).await?;
                    }
                }
            }
        }

        ModifyValuesBy::Append => {
            for book_id in edit.book_ids {
                for person_id in edit.people_list.iter().copied() {
                    BookPersonModel { book_id, person_id }
                        .insert_or_ignore(&db.basic())
                        .await?;
                }
            }
        }

        ModifyValuesBy::Remove => {
            for book_id in edit.book_ids {
                for person_id in edit.people_list.iter().copied() {
                    BookPersonModel { book_id, person_id }
                        .delete(&db.basic())
                        .await?;
                }

                // TODO: Check if we removed cached author
                // If book has no other people referenced we'll update the cached author name.
                if BookPersonModel::find_by(Either::Left(book_id), &db.basic())
                    .await?
                    .is_empty()
                {
                    let book = BookModel::find_one_by_id(book_id, &db.basic()).await?;

                    if let Some(mut book) = book {
                        book.cached.author = None;
                        book.update(&db.basic()).await?;
                    }
                }
            }
        }
    }

    Ok(web::Json(WrappingResponse::okay("success")))
}

// Book
#[get("/book/{id}")]
pub async fn load_book_info(
    member: MemberCookie,
    book_id: web::Path<BookId>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetBookByIdResponse>> {
    let book = BookModel::find_one_by_id(*book_id, &db.basic())
        .await?
        .unwrap();

    let mut found_progression = false;
    let (mut media, mut progress) = (Vec::new(), Vec::new());

    if book.type_of == BookType::Book {
        for file in FileModel::find_by_book_id(book.id, &db.basic()).await? {
            let prog = if !found_progression {
                let prog =
                    FileProgressionModel::find_one(member.member_id(), file.id, &db.basic()).await?;

                found_progression = prog.is_some();

                prog
            } else {
                None
            };

            media.push(file.into());
            progress.push(prog.map(|v| v.into()));
        }
    } else {
        let db_rw = db.basic();

        let (mut proc_media, mut proc_progress) = (Vec::new(), Vec::new());
        let (mut proc_pro_media, mut proc_pro_progress) = (Vec::new(), Vec::new());

        // If we're viewing a comic
        for section_model in BookModel::find_by_parent_id(book.id, &db_rw).await? {
            for file_model in BookModel::find_by_parent_id(section_model.id, &db_rw).await? {
                // Copy of above
                for file in FileModel::find_by_book_id(file_model.id, &db_rw).await? {
                    let prog = if !found_progression {
                        let prog =
                            FileProgressionModel::find_one(member.member_id(), file.id, &db_rw).await?;

                        found_progression = prog.is_some();

                        prog
                    } else {
                        None
                    };

                    if section_model.index.unwrap() == 0 {
                        proc_pro_media.push((file_model.index.unwrap(), file));
                        proc_pro_progress.push((file_model.index.unwrap(), prog));
                    } else {
                        proc_media.push((file_model.index.unwrap(), file));
                        proc_progress.push((file_model.index.unwrap(), prog));
                    }
                }
            }
        }

        // Sort by index
        proc_media.sort_by_key(|&(i, _)| i);
        proc_progress.sort_by_key(|&(i, _)| i);
        proc_pro_media.sort_by_key(|&(i, _)| i);
        proc_pro_progress.sort_by_key(|&(i, _)| i);

        // Place into main vec
        for (file, prog) in proc_pro_media.into_iter().zip(proc_pro_progress.into_iter()) {
            media.push(file.1.into());
            progress.push(prog.1.map(|v| v.into()));
        }

        for (file, prog) in proc_media.into_iter().zip(proc_progress.into_iter()) {
            media.push(file.1.into());
            progress.push(prog.1.map(|v| v.into()));
        }
    }

    let people = PersonModel::find_by_book_id(book.id, &db.basic()).await?;

    Ok(web::Json(WrappingResponse::okay(api::GetBookResponse {
        book: book.into(),
        media,
        progress,
        people: people.into_iter().map(|p| p.into()).collect(),
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

    let member = member.fetch_or_error(&db.basic()).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    match body.into_inner() {
        api::PostBookBody::UnMatch => {
            queue_task(task::TaskUpdateInvalidBook::new(
                task::UpdatingBook::UnMatch(book_id),
            ));
        }

        api::PostBookBody::RefreshBookId => {
            queue_task(task::TaskUpdateInvalidBook::new(
                task::UpdatingBook::Refresh(book_id),
            ));
        }

        api::PostBookBody::AutoMatchBookIdByFiles => {
            queue_task(task::TaskUpdateInvalidBook::new(
                task::UpdatingBook::AutoUpdateBookIdByFiles(book_id),
            ));
        }

        api::PostBookBody::AutoMatchBookId => {
            queue_task(task::TaskUpdateInvalidBook::new(
                task::UpdatingBook::AutoUpdateBookId(book_id),
            ));
        }

        api::PostBookBody::UpdateBookBySource(source) => {
            queue_task_priority(task::TaskUpdateInvalidBook::new(
                task::UpdatingBook::UpdateBookWithSource { book_id, source },
            ));
        }

        api::PostBookBody::Edit(edit) => {
            BookModel::edit_book_by_id(book_id, edit, &db.basic()).await?;
        }
    }

    Ok(web::Json(WrappingResponse::okay("success")))
}

#[get("/book/{id}/download")]
pub async fn download_book(
    book_id: web::Path<BookId>,
    db: web::Data<Database>,
) -> WebResult<NamedFile> {
    let mut files = FileModel::find_by_book_id(*book_id, &db.basic()).await?;

    if files.is_empty() {
        return Err(crate::Error::Internal(crate::InternalError::ItemMissing).into());
    }

    let mut index = 0;

    for (i, file) in files.iter().enumerate().skip(1) {
        if file.file_size > files[index].file_size {
            index = i;
        }
    }

    let file_model = files.remove(index);

    Ok(NamedFile::open_async(file_model.path)
        .await
        .map_err(crate::Error::from)?
        .set_content_disposition(ContentDisposition::from_raw(&HeaderValue::from_str(
            &format!(
                r#"attachment; filename="{}.{}""#,
                file_model.file_name.replace('"', ""), // Shouldn't have " in the file_name but just in-case.
                file_model.file_type,
            ),
        )?)?))
}

#[get("/book/{id}/posters")]
async fn get_book_posters(
    path: web::Path<BookId>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetPosterByBookIdResponse>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    let book = BookModel::find_one_by_id(*path, &db.basic())
        .await?
        .unwrap();

    // TODO: For Open Library we need to go from an Edition to Work.
    // Work is the main book. Usually consisting of more posters.
    // We can do they by works[0].key = "/works/OLXXXXXXW"

    let mut items: Vec<Poster> =
        ImageLinkModel::find_with_link_by_link_id(**path, ImageType::Book, &db.basic())
            .await?
            .into_iter()
            .map(|poster| Poster {
                id: Some(poster.image_id),

                selected: poster.path == book.thumb_path,

                path: poster
                    .path
                    .into_value()
                    .map(|v| format!("/api/image/{v}"))
                    .unwrap_or_else(|| String::from(MISSING_THUMB_PATH)),

                created_at: poster.created_at,
            })
            .collect();

    let search = crate::metadata::search_all_agents(
        &format!(
            "{} {}",
            book.title
                .as_deref()
                .or(book.title.as_deref())
                .unwrap_or_default(),
            book.cached.author.as_deref().unwrap_or_default(),
        ),
        common_local::SearchFor::Book(common_local::SearchForBooksBy::Query),
        &ActiveAgents::default(),
    )
    .await?;

    for item in search.0.into_values().flatten() {
        if let crate::metadata::SearchItem::Book(item) = item {
            for path in item
                .thumb_locations
                .into_iter()
                .filter_map(|v| v.into_url_value())
            {
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
        items,
    })))
}

#[post("/book/{id}/posters")]
async fn insert_or_update_book_image(
    book_id: web::Path<BookId>,
    body: web::Json<api::ChangePosterBody>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    let mut book = BookModel::find_one_by_id(*book_id, &db.basic())
        .await?
        .unwrap();

    match body.into_inner().url_or_id {
        Either::Left(url) => {
            let resp = reqwest::get(url)
                .await
                .map_err(Error::from)?
                .bytes()
                .await
                .map_err(Error::from)?;

            let image_model = store_image(resp.to_vec(), &db.basic()).await?;

            book.thumb_path = image_model.path.clone();

            ImageLinkModel::new_book(image_model.id, book.id)
                .insert(&db.basic())
                .await?;
        }

        Either::Right(id) => {
            let poster = UploadedImageModel::get_by_id(id, &db.basic())
                .await?
                .unwrap();

            if book.thumb_path == poster.path {
                return Ok(web::Json(WrappingResponse::okay("success")));
            }

            book.thumb_path = poster.path;
        }
    }

    book.update(&db.basic()).await?;

    Ok(web::Json(WrappingResponse::okay("success")))
}

#[get("/book/{id}/progress")]
async fn get_book_progress(
    book_id: web::Path<BookId>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetBookProgressResponse>> {
    let model =
        FileProgressionModel::find_one_by_book_id(member.member_id(), *book_id, &db.basic())
            .await?;

    Ok(web::Json(WrappingResponse::okay(model.map(|v| v.into()))))
}

#[get("/book/search")]
pub async fn book_search(
    body: web::Query<api::GetBookSearch>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetBookSearchResponse>> {
    let member = member.fetch_or_error(&db.basic()).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    let search = metadata::search_all_agents(
        &body.query,
        match body.search_type {
            // TODO: Allow for use in Query.
            SearchType::Book => SearchFor::Book(SearchForBooksBy::Query),
            SearchType::Person => SearchFor::Person,
        },
        &ActiveAgents::default(),
    )
    .await?;

    Ok(web::Json(WrappingResponse::okay(api::BookSearchResponse {
        items: search
            .0
            .into_iter()
            .map(|(a, b)| {
                (
                    a.into_owned(),
                    b.into_iter()
                        .map(|v| match v {
                            metadata::SearchItem::Book(book) => {
                                api::SearchItem::Book(api::MetadataBookSearchItem {
                                    source: book.source,
                                    author: book.cached.author,
                                    description: book.description,
                                    name: book
                                        .title
                                        .unwrap_or_else(|| String::from("Unknown title")),
                                    thumbnail_url: book
                                        .thumb_locations
                                        .first()
                                        .and_then(|v| v.as_url_value())
                                        .map(|v| v.to_string())
                                        .unwrap_or_default(),
                                })
                            }

                            metadata::SearchItem::Author(author) => {
                                api::SearchItem::Person(api::MetadataPersonSearchItem {
                                    source: author.source,

                                    cover_image: author
                                        .cover_image_url
                                        .and_then(|v| v.into_url_value()),

                                    name: author.name,
                                    other_names: author.other_names,
                                    description: author.description,

                                    birth_date: author.birth_date,
                                    death_date: author.death_date,
                                })
                            }
                        })
                        .collect(),
                )
            })
            .collect(),
    })))
}

#[post("/book/{book_id}/person/{person_id}")]
async fn insert_book_person(
    ids: web::Path<(BookId, PersonId)>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<String>> {
    let (book_id, person_id) = ids.into_inner();

    let member = member.fetch_or_error(&db.basic()).await?;

    if !member.permissions.is_owner() {
        return Ok(web::Json(WrappingResponse::error(
            "You cannot do this! No Permissions!",
        )));
    }

    // If book had no other people referenced we'll update the cached author name.
    if BookPersonModel::find_by(Either::Left(book_id), &db.basic())
        .await?
        .is_empty()
    {
        let person = PersonModel::find_one_by_id(person_id, &db.basic()).await?;
        let book = BookModel::find_one_by_id(book_id, &db.basic()).await?;

        if let Some((person, mut book)) = person.zip(book) {
            book.cached.author = Some(person.name);
            book.update(&db.basic()).await?;
        }
    }

    BookPersonModel { book_id, person_id }
        .insert_or_ignore(&db.basic())
        .await?;

    Ok(web::Json(WrappingResponse::okay(String::from("success"))))
}

#[delete("/book/{book_id}/person/{person_id}")]
async fn delete_book_person(
    ids: web::Path<(BookId, PersonId)>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<DeletionResponse>> {
    let (book_id, person_id) = ids.into_inner();

    let member = member.fetch_or_error(&db.basic()).await?;

    if !member.permissions.is_owner() {
        return Ok(web::Json(WrappingResponse::error(
            "You cannot do this! No Permissions!",
        )));
    }

    BookPersonModel { book_id, person_id }
        .delete(&db.basic())
        .await?;

    // If book has no other people referenced we'll update the cached author name.
    if BookPersonModel::find_by(Either::Left(book_id), &db.basic())
        .await?
        .is_empty()
    {
        let book = BookModel::find_one_by_id(book_id, &db.basic()).await?;

        if let Some(mut book) = book {
            book.cached.author = None;
            book.update(&db.basic()).await?;
        }
    }

    // TODO: Return total deleted
    Ok(web::Json(WrappingResponse::okay(DeletionResponse {
        total: 1,
    })))
}
