use actix_files::NamedFile;
use actix_web::http::header::ContentDisposition;
use actix_web::{delete, get, post, web, HttpResponse};

use common::api::WrappingResponse;
use common_local::{api, Chapter, FileId, Progression};
use futures::TryStreamExt;
use reqwest::header::HeaderValue;

use crate::database::Database;
use crate::http::{JsonResponse, MemberCookie};
use crate::model::file::FileModel;
use crate::model::note::FileNoteModel;
use crate::model::progress::FileProgressionModel;
use crate::{Result, WebResult};

// Load Book Resources

#[get("/file/{id}/res/{tail:.*}")]
pub async fn load_file_resource(
    path: web::Path<(FileId, String)>,
    res: web::Query<api::LoadResourceQuery>,
    db: web::Data<Database>,
) -> WebResult<HttpResponse> {
    let (file_id, resource_path) = path.into_inner();

    let file = FileModel::find_one_by_id(file_id, &db).await?.unwrap();

    let mut book = bookie::load_from_path(&file.path)?.unwrap();

    // TODO: Check if we're loading a section
    let body = if res.configure_pages {
        match book.read_path_as_bytes(
            &resource_path,
            Some(&format!("/api/file/{}/res", file_id)),
            Some(&[include_str!("../../../../../app/book_stylings.css")]),
        ) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{}", e);
                Vec::new()
            }
        }
    } else {
        match book.read_path_as_bytes(&resource_path, None, None) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{}", e);
                Vec::new()
            }
        }
    };

    let mut ok = HttpResponse::Ok();

    if resource_path.ends_with("xhtml") {
        ok.insert_header(("Content-Type", "application/xhtml+xml"));
    }

    Ok(ok.body(body))
}

#[get("/file/{id}/pages/{pages}")]
pub async fn load_file_pages(
    path: web::Path<(FileId, String)>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetFilePagesByIdResponse>> {
    let (file_id, chapters) = path.into_inner();

    let file = FileModel::find_one_by_id(file_id, &db).await?.unwrap();

    let mut book = bookie::load_from_path(&file.path)?.unwrap();

    let (start_chap, end_chap) = chapters.split_once('-').map_or_else(
        || {
            let chap = chapters.parse::<usize>()?;
            Result::Ok((chap, chap))
        },
        |(a, b)| {
            let start_chap = a.parse::<usize>()?;
            let chap_count = book.chapter_count();
            Result::Ok((
                start_chap,
                if b.trim().is_empty() {
                    chap_count
                } else {
                    chap_count.min(b.parse()?)
                },
            ))
        },
    )?;

    let mut items = Vec::new();

    for chap in start_chap..end_chap {
        book.set_chapter(chap);

        // TODO: Return file names along with Chapter. Useful for redirecting to certain chapter for <a> tags.

        items.push(Chapter {
            file_path: book.get_page_path(),
            value: chap,
        });
    }

    Ok(web::Json(WrappingResponse::okay(
        api::GetChaptersResponse {
            offset: start_chap,
            limit: end_chap - start_chap,
            total: book.chapter_count(),
            items,
        },
    )))
}

// TODO: Add body requests for specifics
#[get("/file/{id}")]
pub async fn load_file(
    member: MemberCookie,
    file_id: web::Path<FileId>,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<Option<api::GetFileByIdResponse>>> {
    Ok(web::Json(WrappingResponse::okay(
        if let Some(file) = FileModel::find_one_by_id(*file_id, &db).await? {
            Some(api::GetFileByIdResponse {
                progress: FileProgressionModel::find_one(member.member_id(), *file_id, &db)
                    .await?
                    .map(|v| v.into()),

                media: file.into(),
            })
        } else {
            None
        },
    )))
}

#[get("/file/{id}/download")]
pub async fn download_file(
    file_id: web::Path<FileId>,
    db: web::Data<Database>,
) -> WebResult<NamedFile> {
    let file_model = FileModel::find_one_by_id(*file_id, &db)
        .await?
        .ok_or(crate::Error::Internal(crate::InternalError::ItemMissing))?;

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

#[get("/file/{id}/debug/{tail:.*}")]
pub async fn load_file_debug(
    web_path: web::Path<(FileId, String)>,
    db: web::Data<Database>,
) -> WebResult<HttpResponse> {
    if let Some(file) = FileModel::find_one_by_id(web_path.0, &db).await? {
        if web_path.1.is_empty() {
            let book = bookie::load_from_path(&file.path)?.unwrap();

            Ok(HttpResponse::Ok().body(
                book.get_files()
                    .into_iter()
                    .map(|v| format!("<a href=\"{}\">{}</a>", v, v))
                    .collect::<Vec<_>>()
                    .join("<br/>"),
            ))
        } else {
            let mut book = bookie::load_from_path(&file.path)?.unwrap();

            Ok(HttpResponse::Ok().body(book.read_path_as_bytes(&web_path.1, None, None)?))
        }
    } else {
        Ok(HttpResponse::Ok().body("Unable to find file from ID"))
    }
}

// Progress

#[post("/file/{id}/progress")]
pub async fn progress_file_add(
    file_id: web::Path<FileId>,
    body: web::Json<Progression>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    if let Some(book_id) = FileModel::find_one_by_id(*file_id, &db)
        .await?
        .and_then(|v| v.book_id)
    {
        FileProgressionModel::insert_or_update(
            member.member_id(),
            book_id,
            *file_id,
            body.into_inner(),
            &db,
        )
        .await?;
    }

    Ok(web::Json(WrappingResponse::okay("success")))
}

#[delete("/file/{id}/progress")]
pub async fn progress_file_delete(
    file_id: web::Path<FileId>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    FileProgressionModel::delete_one(member.member_id(), *file_id, &db).await?;
    Ok(web::Json(WrappingResponse::okay("success")))
}

// Notes

#[get("/file/{id}/notes")]
pub async fn notes_file_get(
    file_id: web::Path<FileId>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<api::ApiGetFileNotesByIdResponse>> {
    let v = FileNoteModel::find_one(*file_id, member.member_id(), &db).await?;
    Ok(web::Json(WrappingResponse::okay(v.map(|v| v.data))))
}

#[post("/file/{id}/notes")]
pub async fn notes_file_add(
    file_id: web::Path<FileId>,
    mut payload: web::Payload,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.try_next().await? {
        body.extend_from_slice(&chunk);
    }

    let data = unsafe { String::from_utf8_unchecked(body.to_vec()) };

    FileNoteModel::new(*file_id, member.member_id(), data)
        .insert_or_update(&db)
        .await?;

    Ok(web::Json(WrappingResponse::okay("success")))
}

#[delete("/file/{id}/notes")]
pub async fn notes_file_delete(
    file_id: web::Path<FileId>,
    member: MemberCookie,
    db: web::Data<Database>,
) -> WebResult<JsonResponse<&'static str>> {
    FileNoteModel::delete_one(*file_id, member.member_id(), &db).await?;

    Ok(web::Json(WrappingResponse::okay("success")))
}
