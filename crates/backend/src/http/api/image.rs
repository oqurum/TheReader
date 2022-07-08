use actix_files::NamedFile;
use actix_web::{get, web, Responder};


#[get("/image/{id}")]
async fn get_local_image(path: web::Path<String>) -> impl Responder {
	let id = path.into_inner();

	let path = crate::image::prefixhash_to_path(
		&id
	);

	NamedFile::open_async(path).await
}