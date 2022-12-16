use std::path::Path;

use common::{
    api::{ApiErrorResponse, WrappingResponse},
    BookId, Either, ImageId, ImageIdType, PersonId,
};
use gloo_utils::{format::JsValueSerdeExt, window};
use serde::{Deserialize, Serialize};
use serde_json::json;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, RequestMode, Response};

use common_local::{
    api::*,
    filter::FilterContainer,
    setup::SetupConfig,
    ws::{TaskId, TaskInfo},
    CollectionId, FileId, LibraryId, Progression, SearchType,
};

pub fn get_download_path(value: Either<BookId, FileId>) -> String {
    let path = window().location().origin().unwrap_throw();

    let (type_of, id) = match value {
        Either::Left(v) => ("book", *v),
        Either::Right(v) => ("file", *v),
    };

    format!("{path}/api/{type_of}/{id}/download")
}

// Setup
pub async fn check_if_setup() -> WrappingResponse<ApiGetSetupResponse> {
    fetch("GET", "/api/setup", Option::<&()>::None)
        .await
        .unwrap_or_else(def)
}

pub async fn finish_setup(value: SetupConfig) -> WrappingResponse<String> {
    fetch("POST", "/api/setup", Some(&value))
        .await
        .unwrap_or_else(def)
}

// Member

pub async fn get_member_self() -> WrappingResponse<ApiGetMemberSelfResponse> {
    fetch("GET", "/api/member", Option::<&()>::None)
        .await
        .unwrap_or_else(def)
}

pub async fn get_members() -> WrappingResponse<ApiGetMembersListResponse> {
    fetch("GET", "/api/members", Option::<&()>::None)
        .await
        .unwrap_or_else(def)
}

pub async fn update_member(options: UpdateMember) -> WrappingResponse<String> {
    fetch("POST", "/api/member", Some(&options))
        .await
        .unwrap_or_else(def)
}

// Collections

pub async fn get_collections() -> WrappingResponse<ApiGetCollectionListResponse> {
    fetch("GET", "/api/collections", Option::<&()>::None)
        .await
        .unwrap_or_else(def)
}

pub async fn get_collection(id: CollectionId) -> WrappingResponse<ApiGetCollectionIdResponse> {
    fetch("GET", &format!("/api/collection/{id}"), Option::<&()>::None)
        .await
        .unwrap_or_else(def)
}

pub async fn get_collection_books(
    id: CollectionId,
) -> WrappingResponse<ApiGetCollectionIdBooksResponse> {
    fetch(
        "GET",
        &format!("/api/collection/{id}/books"),
        Option::<&()>::None,
    )
    .await
    .unwrap_or_else(def)
}

pub async fn create_collection(
    value: &NewCollectionBody,
) -> WrappingResponse<ApiGetCollectionIdResponse> {
    fetch("POST", "/api/collection", Some(value))
        .await
        .unwrap_or_else(def)
}

pub async fn add_book_to_collection(id: CollectionId, book_id: BookId) -> WrappingResponse<String> {
    fetch(
        "POST",
        &format!("/api/collection/{id}/book/{book_id}"),
        Option::<&()>::None,
    )
    .await
    .unwrap_or_else(def)
}

pub async fn remove_book_from_collection(
    id: CollectionId,
    book_id: BookId,
) -> WrappingResponse<String> {
    fetch(
        "DELETE",
        &format!("/api/collection/{id}/book/{book_id}"),
        Option::<&()>::None,
    )
    .await
    .unwrap_or_else(def)
}

// Libraries

pub async fn get_libraries() -> WrappingResponse<ApiGetLibrariesResponse> {
    fetch("GET", "/api/libraries", Option::<&()>::None)
        .await
        .unwrap_or_else(def)
}

pub async fn get_library(id: LibraryId) -> WrappingResponse<ApiGetLibraryIdResponse> {
    fetch("GET", &format!("/api/library/{id}"), Option::<&()>::None)
        .await
        .unwrap_or_else(def)
}

pub async fn update_library(id: LibraryId, value: &UpdateLibrary) -> WrappingResponse<String> {
    fetch("POST", &format!("/api/library/{id}"), Some(value))
        .await
        .unwrap_or_else(def)
}

// People

pub async fn update_person(id: PersonId, value: &PostPersonBody) -> WrappingResponse<String> {
    fetch("POST", &format!("/api/person/{}", id), Some(value))
        .await
        .unwrap_or_else(def)
}

pub async fn get_people(
    query: Option<&str>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> WrappingResponse<ApiGetPeopleResponse> {
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

    fetch("GET", &url, Option::<&()>::None)
        .await
        .unwrap_or_else(def)
}

pub async fn get_person(id: PersonId) -> WrappingResponse<GetPersonResponse> {
    fetch("GET", &format!("/api/person/{}", id), Option::<&()>::None)
        .await
        .unwrap_or_else(def)
}

pub async fn update_person_thumbnail(
    id: PersonId,
    url_or_id: Either<String, ImageId>,
) -> WrappingResponse<String> {
    fetch(
        "POST",
        &format!("/api/person/{id}/thumbnail"),
        Some(&ChangePosterBody { url_or_id }),
    )
    .await
    .unwrap_or_else(def)
}

// Books

pub async fn update_books(value: &MassEditBooks) -> WrappingResponse<String> {
    fetch("POST", "/api/book", Some(value))
        .await
        .unwrap_or_else(def)
}

pub async fn update_book(id: BookId, value: &PostBookBody) -> WrappingResponse<String> {
    fetch("POST", &format!("/api/book/{}", id), Some(value))
        .await
        .unwrap_or_else(def)
}

pub async fn get_media_view(book_id: BookId) -> WrappingResponse<ApiGetBookByIdResponse> {
    fetch(
        "GET",
        &format!("/api/book/{}", book_id),
        Option::<&()>::None,
    )
    .await
    .unwrap_or_else(def)
}

pub async fn search_for(
    search: &str,
    search_for: SearchType,
) -> WrappingResponse<ApiGetBookSearchResponse> {
    fetch(
        "GET",
        &format!(
            "/api/book/search?query={}&search_type={}",
            urlencoding::encode(search),
            serde_json::to_string(&search_for).unwrap().replace('"', "")
        ),
        Option::<&()>::None,
    )
    .await
    .unwrap_or_else(def)
}

pub async fn get_books(
    library: Option<LibraryId>,
    offset: Option<usize>,
    limit: Option<usize>,
    search: Option<FilterContainer>,
) -> WrappingResponse<ApiGetBookListResponse> {
    let url = format!(
        "/api/books?{}",
        serde_qs::to_string(&BookListQuery::new(library, offset, limit, search).unwrap()).unwrap()
    );

    fetch("GET", &url, Option::<&()>::None)
        .await
        .unwrap_or_else(def)
}

pub async fn get_books_preset(
    offset: Option<usize>,
    limit: Option<usize>,
    preset: BookPresetListType,
) -> WrappingResponse<ApiGetBookPresetListResponse> {
    let url = format!(
        "/api/books/preset?{}",
        serde_qs::to_string(&BookPresetListQuery {
            offset,
            limit,
            preset
        })
        .unwrap_throw()
    );

    fetch("GET", &url, Option::<&()>::None)
        .await
        .unwrap_or_else(def)
}

pub async fn get_book_info(id: FileId) -> WrappingResponse<ApiGetFileByIdResponse> {
    fetch("GET", &format!("/api/file/{}", id), Option::<&()>::None)
        .await
        .unwrap_or_else(def)
}

pub async fn get_book_pages(
    file_id: FileId,
    start: usize,
    end: usize,
) -> WrappingResponse<ApiGetFilePagesByIdResponse> {
    fetch(
        "GET",
        &format!("/api/file/{file_id}/pages/{start}-{end}"),
        Option::<&()>::None,
    )
    .await
    .unwrap_or_else(def)
}

pub fn compile_book_resource_path(
    file_id: FileId,
    location: &Path,
    query: LoadResourceQuery,
) -> String {
    format!(
        "/api/file/{file_id}/res/{}?{}",
        location.to_str().unwrap(),
        serde_qs::to_string(&query).unwrap_or_default()
    )
}

pub async fn get_posters_for_book(id: BookId) -> WrappingResponse<ApiGetPosterByBookIdResponse> {
    fetch(
        "GET",
        &format!("/api/book/{id}/posters"),
        Option::<&()>::None,
    )
    .await
    .unwrap_or_else(def)
}

pub async fn change_poster_for_book(
    id: BookId,
    url_or_id: Either<String, ImageId>,
) -> WrappingResponse<String> {
    fetch(
        "POST",
        &format!("/api/book/{id}/posters"),
        Some(&ChangePosterBody { url_or_id }),
    )
    .await
    .unwrap_or_else(def)
}

pub async fn get_progress_for_book(id: BookId) -> WrappingResponse<ApiGetBookProgressResponse> {
    fetch(
        "GET",
        &format!("/api/book/{id}/progress"),
        Option::<&()>::None,
    )
    .await
    .unwrap_or_else(def)
}

// Progress

pub async fn update_book_progress(
    book_id: FileId,
    progression: &Progression,
) -> WrappingResponse<String> {
    fetch(
        "POST",
        &format!("/api/file/{}/progress", book_id),
        Some(progression),
    )
    .await
    .unwrap_or_else(def)
}

pub async fn remove_book_progress(book_id: FileId) -> WrappingResponse<String> {
    fetch(
        "DELETE",
        &format!("/api/file/{}/progress", book_id),
        Option::<&()>::None,
    )
    .await
    .unwrap_or_else(def)
}

// Notes

pub async fn get_book_notes(book_id: FileId) -> WrappingResponse<ApiGetFileNotesByIdResponse> {
    fetch(
        "GET",
        &format!("/api/file/{}/notes", book_id),
        Option::<&()>::None,
    )
    .await
    .unwrap_or_else(def)
}

pub async fn update_book_notes(book_id: FileId, data: String) -> WrappingResponse<String> {
    fetch("POST", &format!("/api/file/{}/notes", book_id), Some(&data))
        .await
        .unwrap_or_else(def)
}

// Image

pub async fn get_posters_for(img_id_type: ImageIdType) -> WrappingResponse<GetPostersResponse> {
    fetch(
        "GET",
        &format!("/api/images/{}", img_id_type),
        Option::<&()>::None,
    )
    .await
    .unwrap_or_else(def)
}

// Options

pub async fn get_options() -> WrappingResponse<ApiGetOptionsResponse> {
    fetch("GET", "/api/options", Option::<&()>::None)
        .await
        .unwrap_or_else(def)
}

pub async fn update_options_add(options: ModifyOptionsBody) -> WrappingResponse<String> {
    fetch("POST", "/api/options", Some(&options))
        .await
        .unwrap_or_else(def)
}

pub async fn update_options_remove(options: ModifyOptionsBody) -> WrappingResponse<String> {
    fetch("DELETE", "/api/options", Some(&options))
        .await
        .unwrap_or_else(def)
}

pub async fn run_task(value: RunTaskBody) -> WrappingResponse<String> {
    fetch("POST", "/api/task", Some(&value))
        .await
        .unwrap_or_else(def)
}

pub async fn get_tasks() -> WrappingResponse<Vec<(TaskId, TaskInfo)>> {
    fetch("GET", "/api/tasks", Option::<&()>::None)
        .await
        .unwrap_or_else(def)
}

// Login In

pub async fn login_with_password(email: String, password: String) -> WrappingResponse<String> {
    fetch(
        "POST",
        "/auth/password",
        Some(&json!({
            "email": email,
            "password": password,
        })),
    )
    .await
    .unwrap_or_else(def)
}

pub async fn login_without_password(email: String) -> WrappingResponse<String> {
    fetch(
        "POST",
        "/auth/passwordless",
        Some(&json!({
            "email": email,
        })),
    )
    .await
    .unwrap_or_else(def)
}

// Directory

pub async fn get_directory_contents(path: String) -> WrappingResponse<ApiGetDirectoryResponse> {
    fetch(
        "GET",
        &format!(
            "/api/directory?{}",
            serde_urlencoded::to_string(GetDirectoryQuery { path }).unwrap_throw(),
        ),
        Option::<&()>::None,
    )
    .await
    .unwrap_or_else(def)
}

async fn fetch<V: for<'a> Deserialize<'a>>(
    method: &str,
    url: &str,
    body: Option<&impl Serialize>,
) -> Result<V, JsValue> {
    let text = fetch_jsvalue(method, url, body).await?;

    JsValueSerdeExt::into_serde(&text).map_err(|v| JsValue::from_str(&v.to_string()))
}

async fn fetch_jsvalue(
    method: &str,
    url: &str,
    body: Option<&impl Serialize>,
) -> Result<JsValue, JsValue> {
    let mut opts = RequestInit::new();
    opts.method(method);
    opts.mode(RequestMode::Cors);

    if let Some(body) = body {
        opts.body(Some(&JsValue::from_str(
            &serde_json::to_string(body).unwrap(),
        )));

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
    WrappingResponse::Error(ApiErrorResponse {
        description: {
            use std::fmt::Write;

            let mut s = String::new();
            let _ = write!(&mut s, "{:?}", e);

            s
        },
    })
}
