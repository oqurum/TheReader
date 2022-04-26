use std::path::Path;

use serde::{Serialize, Deserialize};
use wasm_bindgen::{JsValue, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{RequestInit, Request, RequestMode, Response, Headers, FormData};

use books_common::{api::*, Progression, SearchType, Either};

// TODO: Manage Errors.
// TODO: Correct different integer types.


// Image

pub async fn get_posters_for_meta(metadata_id: usize) -> GetPostersResponse {
	fetch(
		"GET",
		&format!("/api/posters/{}", metadata_id),
		Option::<&()>::None
	).await.unwrap()
}

pub async fn change_poster_for_meta(metadata_id: usize, url_or_id: Either<String, usize>) {
	let _: Option<String> = fetch(
		"POST",
		&format!("/api/posters/{}", metadata_id),
		Some(&ChangePosterBody {
			url_or_id
		})
	).await.ok();
}


// Member

pub async fn get_member_self() -> GetMemberSelfResponse {
	fetch("GET", "/api/member", Option::<&()>::None).await.unwrap()
}


// Libraries

pub async fn get_libraries() -> GetLibrariesResponse {
	fetch("GET", "/api/libraries", Option::<&()>::None).await.unwrap()
}


// Metadata

pub async fn update_metadata(id: usize, value: &PostMetadataBody) {
	let _: Option<String> = fetch(
		"POST",
		&format!("/api/metadata/{}", id),
		Some(value)
	).await.ok();
}

pub async fn get_media_view(metadata_id: usize) -> MediaViewResponse {
	fetch(
		"GET",
		&format!("/api/metadata/{}", metadata_id),
		Option::<&()>::None
	).await.unwrap()
}


pub async fn search_for(search: &str, search_for: SearchType) -> MetadataSearchResponse {
	fetch(
		"GET",
		&format!(
			"/api/metadata/search?query={}&search_type={}",
			urlencoding::encode(search),
			serde_json::to_string(&search_for).unwrap().replace('"', "")
		),
		Option::<&()>::None
	).await.unwrap()
}


// People


pub async fn update_person(id: usize, value: &PostPersonBody) {
	let _: Option<String> = fetch(
		"POST",
		&format!("/api/person/{}", id),
		Some(value)
	).await.ok();
}

pub async fn get_people(query: Option<&str>, offset: Option<usize>, limit: Option<usize>) -> GetPeopleResponse {
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


// Books

pub async fn get_books(
	library: Option<usize>,
	offset: Option<usize>,
	limit: Option<usize>,
	search: Option<SearchQuery>,
) -> GetBookListResponse {
	let url = format!(
		"/api/books?{}",
		serde_urlencoded::to_string(BookListQuery::new(library, offset, limit, search).unwrap()).unwrap()
	);

	fetch("GET", &url, Option::<&()>::None).await.unwrap()
}

pub async fn get_book_info(id: usize) -> GetBookIdResponse {
	fetch("GET", &format!("/api/book/{}", id), Option::<&()>::None).await.unwrap()
}

pub async fn get_book_pages(book_id: usize, start: usize, end: usize) -> GetChaptersResponse {
	fetch("GET", &format!("/api/book/{}/pages/{}-{}", book_id, start, end), Option::<&()>::None).await.unwrap()
}

pub fn compile_book_resource_path(book_id: usize, location: &Path, query: LoadResourceQuery) -> String {
	format!(
		"/api/book/{}/res/{}?{}",
		book_id,
		location.to_str().unwrap(),
		serde_urlencoded::to_string(&query).unwrap_or_default()
	)
}


// Progress

pub async fn update_book_progress(book_id: usize, progression: &Progression) {
	let _: Option<String> = fetch(
		"POST",
		&format!("/api/book/{}/progress", book_id),
		Some(progression)
	).await.ok();
}

pub async fn remove_book_progress(book_id: usize) {
	let _: Option<String> = fetch(
		"DELETE",
		&format!("/api/book/{}/progress", book_id),
		Option::<&()>::None
	).await.ok();
}


// Notes

pub async fn get_book_notes(book_id: usize) -> Option<String> {
	fetch("GET", &format!("/api/book/{}/notes", book_id), Option::<&()>::None).await.ok()
}

pub async fn update_book_notes(book_id: usize, data: String) {
	let _: Option<String> = fetch(
		"POST",
		&format!("/api/book/{}/notes", book_id),
		Some(&data)
	).await.ok();
}

pub async fn remove_book_notes(book_id: usize) {
	let _: Option<String> = fetch(
		"DELETE",
		&format!("/api/book/{}/notes", book_id),
		Option::<&()>::None
	).await.ok();
}


// Options

pub async fn get_options() -> GetOptionsResponse {
	fetch("GET", "/api/options", Option::<&()>::None).await.unwrap()
}

pub async fn update_options_add(options: ModifyOptionsBody) {
	let _: Option<String> = fetch(
		"POST",
		"/api/options/add",
		Some(&options)
	).await.ok();
}

pub async fn update_options_remove(options: ModifyOptionsBody) {
	let _: Option<String> = fetch(
		"POST",
		"/api/options/remove",
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





async fn fetch<V: for<'a> Deserialize<'a>>(method: &str, url: &str, body: Option<&impl Serialize>) -> Result<V, JsValue> {
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

	let text = JsFuture::from(resp.json()?).await?;

	Ok(text.into_serde().unwrap())
}


async fn fetch_url_encoded<V: for<'a> Deserialize<'a>>(method: &str, url: &str, form_data: FormData) -> Result<V, JsValue> {
	let mut opts = RequestInit::new();
	opts.method(method);
	opts.mode(RequestMode::Cors);

	opts.body(Some(&form_data));

	let request = Request::new_with_str_and_init(url, &opts)?;

	let window = gloo_utils::window();
	let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
	let resp: Response = resp_value.dyn_into().unwrap();

	let text = JsFuture::from(resp.json()?).await?;

	Ok(text.into_serde().unwrap())
}