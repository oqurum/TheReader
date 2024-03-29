use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use crate::{
    http::send_message_to_clients,
    metadata::{
        get_metadata_by_source, get_metadata_from_files, openlibrary::OpenLibraryMetadata,
        search_all_agents, Metadata, MetadataReturned,
    },
    model::{
        BookModel, BookPersonModel, DirectoryModel, FileModel, ImageLinkModel, LibraryModel,
        NewBookModel, NewFileModel, UploadedImageModel,
    },
    parse::{extract_comic_volume, extract_name_from_path, VolumeType},
    Result,
};
use bookie::BookSearch;
use chrono::{TimeZone, Utc};
use common::parse_book_id;
use common_local::{
    ws::{TaskId, TaskType, WebsocketNotification},
    BookType, LibraryId, LibraryType,
};
use sqlx::SqliteConnection;
use tokio::fs;

pub static WHITELISTED_FILE_TYPES: [&str; 2] = ["epub", "cbz"];

pub async fn library_scan(
    library: &LibraryModel,
    directories: Vec<DirectoryModel>,
    task_id: TaskId,
    db: &mut SqliteConnection,
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

                if WHITELISTED_FILE_TYPES.contains(&file_type.as_str())
                    && library.type_of.is_filetype_valid(file_type.as_str())
                {
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

                            model.modified_at = Utc
                                .timestamp_millis_opt(
                                    meta.modified()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                                )
                                .unwrap()
                                .naive_utc();
                            model.accessed_at = Utc
                                .timestamp_millis_opt(
                                    meta.accessed()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                                )
                                .unwrap()
                                .naive_utc();
                            model.created_at = Utc
                                .timestamp_millis_opt(
                                    meta.created()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                                )
                                .unwrap()
                                .naive_utc();
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

                            modified_at: Utc
                                .timestamp_millis_opt(
                                    meta.modified()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                                )
                                .unwrap()
                                .naive_utc(),
                            accessed_at: Utc
                                .timestamp_millis_opt(
                                    meta.accessed()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                                )
                                .unwrap()
                                .naive_utc(),
                            created_at: Utc
                                .timestamp_millis_opt(
                                    meta.created()?.duration_since(UNIX_EPOCH)?.as_millis() as i64,
                                )
                                .unwrap()
                                .naive_utc(),
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
                        } else if let Err(e) =
                            file_match_or_create_comic_book(file, &inside_root_dir, library.id, db)
                                .await
                        {
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
    db: &mut SqliteConnection,
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

        let mut book_model: NewBookModel = meta.into();

        book_model.library_id = library_id;

        // TODO: Store Publisher inside Database
        book_model.cached = book_model
            .cached
            .publisher_optional(publisher)
            .author_optional(main_author);

        let book_model = book_model.insert_or_increment(db).await?;
        FileModel::update_book_id(file_id, book_model.id, db).await?;

        if let Some(thumb_path) = book_model.thumb_url.as_value() {
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
    db: &mut SqliteConnection,
) -> Result<()> {
    let Some(local_path) = file
        .path
        .strip_prefix(&root_dir_path.display().to_string().replace('\\', "/"))
    else {
        error!("File Path is not a child of the root directory");

        return Ok(());
    };

    let stripped_book_name = extract_name_from_path(local_path);

    debug!("{stripped_book_name:?} - {local_path:?}");

    // TODO: Cache the searches.

    // We have to search by the comic book title.
    let items = search_all_agents(
        &stripped_book_name,
        common_local::SearchFor::Book(common_local::SearchForBooksBy::Title),
        &Default::default(),
    )
    .await?;

    let mut source = None;

    // Find first exact match in the OpenLibrary Agent.
    if let Some(source2) = items.iter().find_map(|(agent, items)| {
        if agent == &OpenLibraryMetadata.get_agent() {
            items.iter().find_map(|item| {
                if let Some(book) = item.as_book() {
                    if let Some(title) = book.title.as_deref() {
                        if title == stripped_book_name {
                            return Some(book.source.clone());
                        }
                    }
                }

                None
            })
        } else {
            None
        }
    }) {
        source = Some(source2);
    } else {
        let sim = items.sort_items_by_similarity(&stripped_book_name);

        // Find closest match in the OpenLibrary Agent.
        if let Some(item) = sim.iter().find_map(|&(amt, ref item)| {
            if let Some(book) = item.as_book() {
                if amt > 0.75 && book.source.agent == OpenLibraryMetadata.get_agent() {
                    return Some(item);
                }
            }

            None
        }) {
            source = Some(item.as_book().unwrap().source.clone());
        }
        // Just find closest match in any Agent.
        else if let Some(&(amt, ref found)) = sim.first() {
            if amt > 0.75 {
                source = Some(found.as_book().unwrap().source.clone());
            }
        }
    }

    debug!("Source: {source:?}");

    let Some(source) = source else {
        return Ok(());
    };

    if let Some(mut ret) = get_metadata_by_source(&source).await? {
        let (main_author, author_ids) = ret.add_or_ignore_authors_into_database(db).await?;

        let MetadataReturned {
            meta, publisher, ..
        } = ret;

        // TODO: Make BookModel creation more readable.

        let mut book_model: NewBookModel = meta.into();

        book_model.library_id = library_id;
        book_model.type_of = BookType::ComicBook;

        // TODO: Store Publisher inside Database
        book_model.cached = book_model
            .cached
            .publisher_optional(publisher)
            .author_optional(main_author);

        // Either find the main book, or create it.
        let main_book_id = match BookModel::find_one_by_source(&source, db).await? {
            Some(book) => book.id,
            None => book_model.clone().insert_or_increment(db).await?.id,
        };

        debug!("Main Book ID: {main_book_id:?}");

        let Some(volume_type) = extract_comic_volume(&file.file_name) else {
            // TODO: How to handle this?
            // We don't know what volume this is.

            error!(
                "Unable to extract volume from file name: {:?}",
                file.file_name
            );

            return Ok(());
        };

        // Either find the section book, or create it.
        let (sec_book_id, book_index, is_prologue) = match volume_type {
            VolumeType::Prologue(i) => {
                match BookModel::find_one_by_parent_id_and_index(main_book_id, 0, db).await? {
                    Some(v) => (v.id, i, true),
                    None => (
                        NewBookModel::new_section(true, library_id, main_book_id, source)
                            .insert(db)
                            .await?
                            .id,
                        i,
                        true,
                    ),
                }
            }

            VolumeType::Volume(i) => {
                match BookModel::find_one_by_parent_id_and_index(main_book_id, 1, db).await? {
                    Some(v) => (v.id, i, false),
                    None => (
                        NewBookModel::new_section(false, library_id, main_book_id, source)
                            .insert(db)
                            .await?
                            .id,
                        i,
                        false,
                    ),
                }
            }

            VolumeType::Unknown(_) => todo!(),
        };

        debug!("Section Book ID: {sec_book_id:?} - Index: {book_index}");

        // Multiple index by 10 so we can define .5 chapters.
        let book_index = book_index as i64 * 10;

        // Now we can officially find or create the sub book.
        let sub_book_model =
            match BookModel::find_one_by_parent_id_and_index(sec_book_id, book_index, db).await? {
                Some(v) => v,
                None => {
                    book_model.library_id = library_id;
                    book_model.type_of = BookType::ComicBookChapter;
                    book_model.parent_id = Some(sec_book_id);
                    book_model.index = Some(book_index);
                    book_model.title = Some(format!(
                        "{} - {}",
                        if is_prologue { "Prologue" } else { "Chapter" },
                        book_index / 10
                    ));
                    book_model.original_title = book_model.title.clone();

                    book_model.insert(db).await?
                }
            };

        debug!("Update File Id");

        FileModel::update_book_id(file.id, sub_book_model.id, db).await?;

        debug!("Updated File Id");

        if let Some(thumb_path) = sub_book_model.thumb_url.as_value() {
            if let Some(image) = UploadedImageModel::get_by_path(thumb_path, db).await? {
                ImageLinkModel::new_book(image.id, sub_book_model.id)
                    .insert(db)
                    .await?;
            }
        }

        for person_id in author_ids {
            BookPersonModel {
                book_id: sub_book_model.id,
                person_id,
            }
            .insert_or_ignore(db)
            .await?;
        }
    }

    Ok(())
}
