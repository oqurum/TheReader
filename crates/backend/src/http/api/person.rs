use actix_web::{get, post, web, HttpResponse};
use chrono::Utc;
use common::{
    api::{ApiErrorResponse, WrappingResponse},
    Either, PersonId,
};
use common_local::api;

use crate::{
    http::{JsonResponse, MemberCookie},
    model::{
        BookModel, BookPersonModel, ImageLinkModel, PersonAltModel, PersonModel, UploadedImageModel,
    },
    queue_task, store_image,
    task::{self, queue_task_priority},
    Error, SqlPool, WebResult,
};

const QUERY_LIMIT: i64 = 100;

// Get List Of People and Search For People
#[get("/people")]
pub async fn load_author_list(
    query: web::Query<api::SimpleListQuery>,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<api::ApiGetPeopleResponse>> {
    let offset = query.offset.unwrap_or(0);
    let limit = query.offset.unwrap_or(50).min(QUERY_LIMIT);

    // Return Searched People
    if let Some(query) = query.query.as_deref() {
        let items = PersonModel::search_by(query, offset, limit, &mut *db.acquire().await?)
            .await?
            .into_iter()
            .map(|v| v.into())
            .collect();

        Ok(web::Json(WrappingResponse::okay(api::GetPeopleResponse {
            offset: offset as usize,
            limit: limit as usize,
            total: 0, // TODO
            items,
        })))
    }
    // Return All People
    else {
        let items = PersonModel::find(offset, limit, &mut *db.acquire().await?)
            .await?
            .into_iter()
            .map(|v| v.into())
            .collect();

        Ok(web::Json(WrappingResponse::okay(api::GetPeopleResponse {
            offset: offset as usize,
            limit: limit as usize,
            total: PersonModel::count(&mut *db.acquire().await?).await? as usize,
            items,
        })))
    }
}

// Person Thumbnail
#[get("/person/{id}/thumbnail")]
async fn load_person_thumbnail(
    person_id: web::Path<PersonId>,
    db: web::Data<SqlPool>,
) -> WebResult<HttpResponse> {
    let model = PersonModel::find_one_by_id(*person_id, &mut *db.acquire().await?).await?;

    if let Some(loc) = model.and_then(|v| v.thumb_url.into_value()) {
        let path = crate::image::prefixhash_to_path(&loc);

        Ok(HttpResponse::Ok().body(std::fs::read(path).map_err(Error::from)?))
    } else {
        Ok(HttpResponse::NotFound().finish())
    }
}

#[post("/person/{id}/thumbnail")]
async fn post_change_person_thumbnail(
    id: web::Path<PersonId>,
    body: web::Json<api::ChangePosterBody>,
    member: MemberCookie,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<&'static str>> {
    let member = member.fetch(&mut *db.acquire().await?).await?.unwrap();

    if !member.permissions.is_owner() {
        return Ok(web::Json(WrappingResponse::error(
            "You cannot do this! No Permissions!",
        )));
    }

    let mut person = PersonModel::find_one_by_id(*id, &mut *db.acquire().await?)
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

            let image_model = store_image(resp.to_vec(), &mut *db.acquire().await?).await?;

            person.thumb_url = image_model.path;

            ImageLinkModel::new_person(image_model.id, person.id)
                .insert(&mut *db.acquire().await?)
                .await?;
        }

        Either::Right(id) => {
            let poster = UploadedImageModel::get_by_id(id, &mut *db.acquire().await?)
                .await?
                .unwrap();

            if person.thumb_url == poster.path {
                return Ok(web::Json(WrappingResponse::okay("poster already set")));
            }

            person.thumb_url = poster.path;
        }
    }

    person.update(&mut *db.acquire().await?).await?;

    Ok(web::Json(WrappingResponse::okay("success")))
}

// Person Tasks - Update Person, Overwrite Person with another source.
#[post("/person/{id}")]
pub async fn update_person_data(
    person_id: web::Path<PersonId>,
    body: web::Json<api::PostPersonBody>,
    member: MemberCookie,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<&'static str>> {
    let person_id = *person_id;

    let member = member.fetch_or_error(&mut *db.acquire().await?).await?;

    if !member.permissions.is_owner() {
        return Err(ApiErrorResponse::new("Not owner").into());
    }

    match body.into_inner() {
        api::PostPersonBody::AutoMatchById => {
            queue_task(task::TaskUpdatePeople::new(
                task::UpdatingPeople::AutoUpdateById(person_id),
            ));
        }

        api::PostPersonBody::UpdateBySource(source) => {
            queue_task_priority(task::TaskUpdatePeople::new(
                task::UpdatingPeople::UpdatePersonWithSource { person_id, source },
            ));
        }

        api::PostPersonBody::CombinePersonWith(into_person_id) => {
            // TODO: Tests for this to ensure it's correct.

            let old_person = PersonModel::find_one_by_id(person_id, &mut *db.acquire().await?)
                .await?
                .unwrap();
            let mut into_person =
                PersonModel::find_one_by_id(into_person_id, &mut *db.acquire().await?)
                    .await?
                    .unwrap();

            // Attempt to transfer to other person
            PersonAltModel::transfer_or_ignore(
                old_person.id,
                into_person.id,
                &mut *db.acquire().await?,
            )
            .await?;

            // Delete remaining Alt Names
            PersonAltModel::delete_by_id(old_person.id, &mut *db.acquire().await?).await?;

            // Make Old Person Name an Alt Name
            let _ = PersonAltModel {
                name: old_person.name,
                person_id: into_person.id,
            }
            .insert(&mut *db.acquire().await?)
            .await;

            // Transfer Old Person Book to New Person
            let trans_book_person_vec =
                BookPersonModel::find_by(Either::Right(old_person.id), &mut *db.acquire().await?)
                    .await?;
            for met_per in &trans_book_person_vec {
                let _ = BookPersonModel {
                    book_id: met_per.book_id,
                    person_id: into_person.id,
                }
                .insert_or_ignore(&mut *db.acquire().await?)
                .await;
            }

            BookPersonModel::delete_by_person_id(old_person.id, &mut *db.acquire().await?).await?;

            if into_person.birth_date.is_none() {
                into_person.birth_date = old_person.birth_date;
            }

            if into_person.description.is_none() {
                into_person.description = old_person.description;
            }

            if into_person.thumb_url.is_none() {
                into_person.thumb_url = old_person.thumb_url;
            }

            into_person.updated_at = Utc::now().naive_utc();

            // Update New Person
            into_person.update(&mut *db.acquire().await?).await?;

            // Delete Old Person
            PersonModel::delete_by_id(old_person.id, &mut *db.acquire().await?).await?;

            // Update book cache author name cache
            for met_per in trans_book_person_vec {
                let person =
                    PersonModel::find_one_by_id(into_person_id, &mut *db.acquire().await?).await?;
                let book =
                    BookModel::find_one_by_id(met_per.book_id, &mut *db.acquire().await?).await?;

                if let Some((person, mut book)) = person.zip(book) {
                    book.cached.author = Some(person.name);
                    book.update(&mut *db.acquire().await?).await?;
                }
            }
        }
    }

    Ok(web::Json(WrappingResponse::okay("success")))
}

// Person
#[get("/person/{id}")]
async fn load_person(
    person_id: web::Path<PersonId>,
    db: web::Data<SqlPool>,
) -> WebResult<JsonResponse<api::GetPersonResponse>> {
    let person = PersonModel::find_one_by_id(*person_id, &mut *db.acquire().await?)
        .await?
        .unwrap();

    Ok(web::Json(WrappingResponse::okay(api::GetPersonResponse {
        person: person.into(),
    })))
}
