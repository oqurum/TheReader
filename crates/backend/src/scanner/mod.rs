use std::{collections::VecDeque, path::{PathBuf, Path}, time::UNIX_EPOCH};

use crate::{
    database::DatabaseAccess,
    http::send_message_to_clients,
    metadata::{get_metadata_from_files, MetadataReturned},
    model::{
        book::BookModel,
        book_person::BookPersonModel,
        directory::DirectoryModel,
        file::{FileModel, NewFileModel},
        image::{ImageLinkModel, UploadedImageModel},
        library::LibraryModel,
    },
    Result,
};
use bookie::BookSearch;
use chrono::{TimeZone, Utc};
use common::parse_book_id;
use common_local::{
    ws::{TaskId, TaskType, WebsocketNotification},
    LibraryId, LibraryType,
};
use tokio::fs;
use tracing::{error, info, trace, debug};

pub static WHITELISTED_FILE_TYPES: [&str; 2] = ["epub", "cbz"];

pub async fn library_scan(
    library: &LibraryModel,
    mut directories: Vec<DirectoryModel>,
    task_id: TaskId,
    db: &dyn DatabaseAccess,
) -> Result<()> {
    if directories.is_empty() {
        return Ok(());
    }

    let mut dirs = directories
        .into_iter()
        .map(|v| PathBuf::from(&v.path))
        .collect::<Vec<_>>();
    dirs.reverse();

    let mut folders: VecDeque<PathBuf> = VecDeque::new();

    let (mut checked_items, mut imported_items, mut overwritten_files) = (0, 0, 0);

    let mut inside_root_dir = dirs.pop().unwrap();
    folders.push_front(inside_root_dir.clone());

    while let Some(dir) = folders.pop_front() {
        if folders.is_empty() {
            if let Some(next_dir) = dirs.pop() {
                inside_root_dir = next_dir;
                folders.push_front(inside_root_dir.clone());
            }
        }

        let mut dir = fs::read_dir(&dir).await?;

        while let Some(entry) = dir.next_entry().await? {
            let file_type = entry.file_type().await?;
            let file_name = entry.file_name();
            let path = entry.path();
            let meta = entry.metadata().await?;

            if file_type.is_dir() {
                folders.push_back(path);
            } else if file_type.is_file() {
                let file_name = file_name.into_string().unwrap();
                let (file_name, file_type) = match file_name.rsplit_once('.') {
                    Some((v1, v2)) => (v1.to_string(), v2.to_string().to_lowercase()),
                    None => (file_name, String::new()),
                };

                if WHITELISTED_FILE_TYPES.contains(&file_type.as_str()) && library.type_of.is_filetype_valid(file_type.as_str()) {
                    send_message_to_clients(WebsocketNotification::update_task(
                        task_id,
                        TaskType::LibraryScan(file_name.clone()),
                        true,
                    ));

                    let file_size = fs::metadata(&path).await?.len();

                    checked_items += 1;

                    let mut book = match bookie::load_from_path(&path.to_string_lossy()) {
                        Ok(book) => {
                            if let Some(book) = book {
                                book
                            } else {
                                error!(target: "scanner", file = ?path, "Unable to find book from path");
                                continue;
                            }
                        }

                        Err(e) => {
                            error!(target: "scanner", error = ?e, file = ?path);
                            continue;
                        }
                    };

                    let path = path.to_str().unwrap().replace('\\', "/");

                    let Some(hash) = book.compute_hash() else {
                        error!(target: "scanner", file = path, "Unable to compute hash");
                        continue;
                    };

                    let chapter_count = book.chapter_count() as i64;

                    // If file exists, check to see if the one currently in the database is valid.
                    let file = if let Some(mut model) =
                        FileModel::find_one_by_hash_or_path(&path, &hash, db).await?
                    {
                        // We found it by path, no need to verify anything since it exists.
                        // Update stored model with the new one that matched the hash.
                        // TODO: Optimize? I don't want to check the FS for EVERY SINGLE FILE.
                        if model.path != path && tokio::fs::metadata(&model.path).await.is_err() {
                            model.path = path;
                            model.file_name = file_name;
                            model.file_type = file_type;
                            model.file_size = file_size as i64;
                            model.library_id = library.id;
                            model.chapter_count = chapter_count;
                            model.hash = hash;

                            model.modified_at = Utc.timestamp_millis(
                                meta.modified()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                            );
                            model.accessed_at = Utc.timestamp_millis(
                                meta.accessed()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                            );
                            model.created_at = Utc.timestamp_millis(
                                meta.created()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                            );
                            model.deleted_at = None;

                            info!(target: "scanner", id = ?model.id, "Overwriting Missing File");

                            overwritten_files += 1;

                            model.update(db).await?;
                        }

                        model
                    } else {
                        let identifier = if let Some(found) = book.find(BookSearch::Identifier) {
                            let parsed = found
                                .into_iter()
                                .map(|v| parse_book_id(&v))
                                .collect::<Vec<_>>();

                            parsed
                                .iter()
                                .find_map(|v| v.as_isbn_13())
                                .or_else(|| parsed.iter().find_map(|v| v.as_isbn_10()))
                        } else {
                            None
                        };

                        let file = NewFileModel {
                            path,

                            file_name,
                            file_type,
                            file_size: file_size as i64,

                            library_id: library.id,
                            book_id: None,
                            chapter_count,

                            identifier,
                            hash,

                            modified_at: Utc.timestamp_millis(
                                meta.modified()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                            ),
                            accessed_at: Utc.timestamp_millis(
                                meta.accessed()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                            ),
                            created_at: Utc.timestamp_millis(
                                meta.created()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                            ),
                            deleted_at: None,
                        };

                        imported_items += 1;

                        file.insert(db).await?
                    };

                    if file.book_id.is_none() {
                        let file_id = file.id;

                        if library.type_of == LibraryType::Book {
                            // TODO: Run Concurrently.
                            if let Err(e) = file_match_or_create_book(file, library.id, db).await {
                                error!(error = ?e, "File #{file_id} file_match_or_create_metadata");
                            }
                        } else if let Err(e) = file_match_or_create_comic_book(file, &inside_root_dir, library.id, db).await {
                            error!(error = ?e, "File #{file_id} file_match_or_create_metadata");
                        }
                    }
                } else {
                    trace!(file = ?path, "Skipping File. Not a whitelisted file type.");
                }
            }
        }
    }

    info!("Checked {checked_items} Files, Imported {imported_items} Files, Overwritten {overwritten_files} Files");

    Ok(())
}

async fn file_match_or_create_book(
    file: FileModel,
    library_id: LibraryId,
    db: &dyn DatabaseAccess,
) -> Result<()> {
    let file_id = file.id;

    let meta = get_metadata_from_files(&[file], &Default::default()).await?;

    if let Some(mut ret) = meta {
        let (main_author, author_ids) = ret.add_or_ignore_authors_into_database(db).await?;

        let MetadataReturned {
            mut meta,
            publisher,
            ..
        } = ret;

        // TODO: This is For Local File Data. Need specify.
        if let Some(item) = meta.thumb_locations.iter_mut().find(|v| v.is_file_data()) {
            item.download(db).await?;
        }

        let mut book_model: BookModel = meta.into();

        book_model.library_id = library_id;

        // TODO: Store Publisher inside Database
        book_model.cached = book_model
            .cached
            .publisher_optional(publisher)
            .author_optional(main_author);

        let book_model = book_model.insert_or_increment(db).await?;
        FileModel::update_book_id(file_id, book_model.id, db).await?;

        if let Some(thumb_path) = book_model.thumb_path.as_value() {
            if let Some(image) = UploadedImageModel::get_by_path(thumb_path, db).await? {
                ImageLinkModel::new_book(image.id, book_model.id)
                    .insert(db)
                    .await?;
            }
        }

        for person_id in author_ids {
            BookPersonModel {
                book_id: book_model.id,
                person_id,
            }
            .insert_or_ignore(db)
            .await?;
        }
    }

    Ok(())
}


async fn file_match_or_create_comic_book(
    file: FileModel,
    root_dir_path: &Path,
    library_id: LibraryId,
    db: &dyn DatabaseAccess,
) -> Result<()> {
    let local_path = file.path.strip_prefix(&root_dir_path.display().to_string().replace("\\", "/"))
        .expect("File Path is not a child of the root directory");

    debug!("{:?}", local_path);
    debug!("{:?}", file);

    Ok(())
}
