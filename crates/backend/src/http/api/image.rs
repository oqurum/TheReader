use actix_files::NamedFile;
use actix_web::{get, web, Responder};
use books_common::ThumbnailStoreType;


#[get("/image/{type}/{id}")]
async fn get_local_image(path: web::Path<(String, String)>) -> impl Responder {
	let (type_of, id) = path.into_inner();

	let path = crate::image::prefixhash_to_path(
		ThumbnailStoreType::from(type_of.as_str()),
		&id
	);

	NamedFile::open_async(path).await
}