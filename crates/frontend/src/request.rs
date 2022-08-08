use std::path::Path;

use common::{ImageId, PersonId, Either, api::{WrappingResponse, ApiErrorResponse, DeletionResponse}, BookId, ImageIdType};
use serde::{Serialize, Deserialize};
use serde_json::json;
use wasm_bindgen::{JsValue, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{RequestInit, Request, RequestMode, Response, Headers};

use common_local::{api::*, Progression, SearchType, setup::SetupConfig, LibraryId, FileId};

// TODO: Manage Errors.


// Setup
pub async fn check_if_setup() -> ApiGetIsSetupResponse {
    fetch(
        "GET",
        "/api/setup",
        Option::<&()>::None,
    ).await.unwrap()
}

pub async fn finish_setup(value: SetupConfig) -> WrappingResponse<String> {
    fetch(
        "POST",
        "/api/setup",
        Some(&value),
    ).await.unwrap_or_else(def)
}


// Member

pub async fn get_member_self() -> Option<ApiGetMemberSelfResponse> {
    fetch("GET", "/api/member", Option::<&()>::None).await.ok()
}


// Libraries

pub async fn get_libraries() -> ApiGetLibrariesResponse {
    fetch("GET", "/api/libraries", Option::<&()>::None).await.unwrap()
}


// People

pub async fn update_person(id: PersonId, value: &PostPersonBody) {
    let _: Option<String> = fetch(
        "POST",
        &format!("/api/person/{}", id),
        Some(value)
    ).await.ok();
}

pub async fn get_people(query: Option<&str>, offset: Option<usize>, limit: Option<usize>) -> ApiGetPeopleResponse {
    let mut url = String::from("/api/people?");

    if let Some(value) = offset {
        url += "offset=";
        url += &value.to_string();
        url += "&";
    }

    if let Some(value) = limit {
        url += "limit=";
        url += &value.to_string();
        url += "&";
    }

    if let Some(value) = query {
        url += "query=";
        url += &urlencoding::encode(value);
    }

    fetch(
        "GET",
        &url,
        Option::<&()>::None
    ).await.unwrap()
}

pub async fn get_person(id: PersonId) -> GetPersonResponse {
    fetch(
        "GET",
        &format!("/api/person/{}", id),
        Option::<&()>::None
    ).await.unwrap()
}

pub async fn update_person_thumbnail(id: PersonId, url_or_id: Either<String, ImageId>) {
    let _: Option<String> = fetch(
        "POST",
        &format!("/api/person/{id}/thumbnail"),
        Some(&ChangePosterBody { url_or_id })
    ).await.ok();
}



// Person Book Link

pub async fn add_person_to_book(book_id: BookId, person_id: PersonId) -> WrappingResponse<String> {
    fetch(
        "POST",
        &format!("/api/book/{book_id}/person/{person_id}"),
        Option::<&()>::None
    ).await.unwrap_or_else(def)
}

pub async fn delete_person_from_book(book_id: BookId, person_id: PersonId) -> WrappingResponse<DeletionResponse> {
    fetch(
        "DELETE",
        &format!("/api/book/{book_id}/person/{person_id}"),
        Option::<&()>::None
    ).await.unwrap_or_else(def)
}


// Books


pub async fn update_book(id: BookId, value: &PostBookBody) {
    let _: Option<String> = fetch(
        "POST",
        &format!("/api/book/{}", id),
        Some(value)
    ).await.ok();
}

pub async fn get_media_view(book_id: BookId) -> ApiGetBookByIdResponse {
    fetch(
        "GET",
        &format!("/api/book/{}", book_id),
        Option::<&()>::None
    ).await.unwrap()
}


pub async fn search_for(search: &str, search_for: SearchType) -> ApiGetBookSearchResponse {
    fetch(
        "GET",
        &format!(
            "/api/book/search?query={}&search_type={}",
            urlencoding::encode(search),
            serde_json::to_string(&search_for).unwrap().replace('"', "")
        ),
        Option::<&()>::None
    ).await.unwrap()
}


pub async fn get_books(
    library: Option<LibraryId>,
    offset: Option<usize>,
    limit: Option<usize>,
    search: Option<SearchQuery>,
) -> ApiGetBookListResponse {
    let url = format!(
        "/api/books?{}",
        serde_urlencoded::to_string(BookListQuery::new(library, offset, limit, search).unwrap()).unwrap()
    );

    fetch("GET", &url, Option::<&()>::None).await.unwrap()
}

pub async fn get_book_info(id: FileId) -> ApiGetFileByIdResponse {
    fetch("GET", &format!("/api/file/{}", id), Option::<&()>::None).await.unwrap()
}

pub async fn get_book_pages(book_id: FileId, start: usize, end: usize) -> ApiGetFilePagesByIdResponse {
    fetch("GET", &format!("/api/file/{}/pages/{}-{}", book_id, start, end), Option::<&()>::None).await.unwrap()
}

pub fn compile_book_resource_path(book_id: FileId, location: &Path, query: LoadResourceQuery) -> String {
    format!(
        "/api/file/{}/res/{}?{}",
        book_id,
        location.to_str().unwrap(),
        serde_urlencoded::to_string(&query).unwrap_or_default()
    )
}

pub async fn get_posters_for_book(id: BookId) -> ApiGetPosterByBookIdResponse {
    fetch(
        "GET",
        &format!("/api/book/{id}/posters"),
        Option::<&()>::None
    ).await.unwrap()
}

pub async fn change_poster_for_book(id: BookId, url_or_id: Either<String, ImageId>) {
    let _: Option<String> = fetch(
        "POST",
        &format!("/api/book/{id}/posters"),
        Some(&ChangePosterBody {
            url_or_id
        })
    ).await.ok();
}


// Progress

pub async fn update_book_progress(book_id: FileId, progression: &Progression) {
    let _: Option<String> = fetch(
        "POST",
        &format!("/api/file/{}/progress", book_id),
        Some(progression)
    ).await.ok();
}

pub async fn remove_book_progress(book_id: FileId) {
    let _: Option<String> = fetch(
        "DELETE",
        &format!("/api/file/{}/progress", book_id),
        Option::<&()>::None
    ).await.ok();
}


// Notes

pub async fn get_book_notes(book_id: FileId) -> ApiGetFileNotesByIdResponse {
    fetch("GET", &format!("/api/file/{}/notes", book_id), Option::<&()>::None).await.ok()
}

pub async fn update_book_notes(book_id: FileId, data: String) {
    let _: Option<String> = fetch(
        "POST",
        &format!("/api/file/{}/notes", book_id),
        Some(&data)
    ).await.ok();
}

pub async fn remove_book_notes(book_id: FileId) {
    let _: Option<String> = fetch(
        "DELETE",
        &format!("/api/file/{}/notes", book_id),
        Option::<&()>::None
    ).await.ok();
}


// Image

pub async fn get_posters_for(img_id_type: ImageIdType) -> GetPostersResponse {
    fetch(
        "GET",
        &format!("/api/images/{}", img_id_type),
        Option::<&()>::None
    ).await.unwrap()
}

// Options

pub async fn get_options() -> ApiGetOptionsResponse {
    fetch("GET", "/api/options", Option::<&()>::None).await.unwrap()
}

pub async fn update_options_add(options: ModifyOptionsBody) {
    let _: Option<String> = fetch(
        "POST",
        "/api/options",
        Some(&options)
    ).await.ok();
}

pub async fn update_options_remove(options: ModifyOptionsBody) {
    let _: Option<String> = fetch(
        "DELETE",
        "/api/options",
        Some(&options)
    ).await.ok();
}

pub async fn run_task() { // TODO: Use common::api::RunTaskBody
    let _: Option<String> = fetch(
        "POST",
        "/api/task",
        Some(&serde_json::json!({
            "run_search": true,
            "run_metadata": true
        }))
    ).await.ok();
}


// Login In

pub async fn login_with_password(email: String, password: String) -> WrappingResponse<String> {
    fetch(
        "POST",
        "/auth/password",
        Some(&json!({
            "email": email,
            "password": password,
        }))
    ).await.unwrap_or_else(def)
}

pub async fn login_without_password(email: String) -> WrappingResponse<String> {
    fetch(
        "POST",
        "/auth/passwordless",
        Some(&json!({
            "email": email,
        }))
    ).await.unwrap_or_else(def)
}



async fn fetch<V: for<'a> Deserialize<'a>>(method: &str, url: &str, body: Option<&impl Serialize>) -> Result<V, JsValue> {
    let text = fetch_jsvalue(method, url, body).await?;

    Ok(text.into_serde().unwrap())
}

async fn fetch_jsvalue(method: &str, url: &str, body: Option<&impl Serialize>) -> Result<JsValue, JsValue> {
    let mut opts = RequestInit::new();
    opts.method(method);
    opts.mode(RequestMode::Cors);

    if let Some(body) = body {
        opts.body(Some(&JsValue::from_str(&serde_json::to_string(body).unwrap())));

        let headers = Headers::new()?;
        headers.append("Content-Type", "application/json")?;
        opts.headers(&headers);
    }

    let request = Request::new_with_str_and_init(url, &opts)?;

    let window = gloo_utils::window();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into().unwrap();

    JsFuture::from(resp.json()?).await
}

fn def<V>(e: JsValue) -> WrappingResponse<V> {
    WrappingResponse {
        resp: None,
        error: Some(ApiErrorResponse {
            description: {
                use std::fmt::Write;

                let mut s = String::new();
                let _ = write!(&mut s, "{:?}", e);

                s
            }
        })
    }
}