use async_trait::async_trait;
use chrono::Utc;
use common::{BookId, Source};
use common_local::{LibraryId, ws::{WebsocketNotification, TaskType, TaskId}, SearchFor, SearchForBooksBy, filter::FilterContainer};
use sqlx::SqliteConnection;
use tracing::info;

use crate::{Result, metadata::{ActiveAgents, get_metadata_by_source, search_and_return_first_valid_agent, SearchItem, MetadataReturned, get_metadata_from_files, search_all_agents}, model::{BookModel, NewBookModel, FileModel, BookPersonModel, UploadedImageModel, ImageLinkModel}, http::send_message_to_clients, sort_by_similarity, Task, SqlPool};


#[derive(Clone)]
pub enum UpdatingBook {
    /// Refresh the books Metadata.
    Refresh(BookId),
    /// Update book by file info
    AutoUpdateBookIdByFiles(BookId),
    /// Update book w/ all steps. weither by source, files, or agent id.
    ///
    /// Steps:
    /// 1. Re-check the agents above our current one.
    /// 2.
    // TODO: Expand upon
    AutoUpdateBookId(BookId),
    /// Update book by source.
    ///
    /// If input (old) Book ID is different than Sources' Book ID (current) we replace and join the old one into the current one.
    ///
    /// If they're equal we update based off the external metadata agents data we receive.
    UpdateBookWithSource { book_id: BookId, source: Source },
    /// Updates all books with specified agent by files.
    UpdateAllWithAgent {
        library_id: LibraryId,
        agent: String,
    },
    /// Updates all books with specified agent by files.
    UnMatch(BookId),
}

pub struct TaskUpdateInvalidBook {
    state: UpdatingBook,
}

impl TaskUpdateInvalidBook {
    pub fn new(state: UpdatingBook) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Task for TaskUpdateInvalidBook {
    async fn run(&mut self, task_id: TaskId, pool: &SqlPool) -> Result<()> {
        let db = &mut *pool.acquire().await?;

        match self.state.clone() {
            UpdatingBook::UnMatch(book_id) => {
                info!(id = ?book_id, "Unmatching Book By Id");

                let fm_book = BookModel::find_one_by_id(book_id, db).await?.unwrap();

                Self::update_book_by_files(
                    fm_book,
                    &ActiveAgents {
                        local: true,
                        google: false,
                        libby: false,
                        openlib: false,
                    },
                    db,
                )
                .await?;
            }

            UpdatingBook::AutoUpdateBookIdByFiles(book_id) => {
                info!(id = ?book_id, "Auto Update Metadata ID by Files");

                send_message_to_clients(WebsocketNotification::update_task(
                    task_id,
                    TaskType::UpdatingBook {
                        id: book_id,
                        subtitle: None,
                    },
                    true,
                ));

                let fm_book = BookModel::find_one_by_id(book_id, db).await?.unwrap();

                Self::update_book_by_files(fm_book, &ActiveAgents::default(), db).await?;
            }

            UpdatingBook::Refresh(book_id) => {
                info!(id = ?book_id, "Refresh Book");

                send_message_to_clients(WebsocketNotification::update_task(
                    task_id,
                    TaskType::UpdatingBook {
                        id: book_id,
                        subtitle: None,
                    },
                    true,
                ));

                let book_model = BookModel::find_one_by_id(book_id, db).await?.unwrap();

                if let Some(metadata) = get_metadata_by_source(&book_model.source).await? {
                    overwrite_book_with_new_metadata(book_model, metadata, db).await?;
                }
            }

            UpdatingBook::AutoUpdateBookId(book_id) => {
                info!(id = ?book_id, "Auto Update By Title");

                send_message_to_clients(WebsocketNotification::update_task(
                    task_id,
                    TaskType::UpdatingBook {
                        id: book_id,
                        subtitle: None,
                    },
                    true,
                ));

                let book_model = BookModel::find_one_by_id(book_id, db).await?.unwrap();

                // Step 1
                let search_query = book_model
                    .title
                    .as_deref()
                    .or(book_model.original_title.as_deref());

                if let Some(search_query) = search_query {
                    let found = search_and_return_first_valid_agent(
                        search_query,
                        SearchFor::Book(SearchForBooksBy::Query),
                        &ActiveAgents::default(),
                    )
                    .await?;

                    if !found.is_empty() {
                        let found_item = sort_by_similarity(search_query, found, |v| match v {
                            SearchItem::Book(v) => v.title.as_deref(),
                            SearchItem::Author(v) => Some(&v.name),
                        })
                        .into_iter()
                        .find(|&(score, ref item)| match item {
                            SearchItem::Book(book) => {
                                // info!("Score: {score} | {:?} | {}", book.title, book.source);
                                score > 0.75 && !book.thumb_locations.is_empty()
                            }
                            _ => false,
                        })
                        .map(|(_, item)| item);

                        // Now we need to do a search for found item and return it.
                        if let Some(item) = found_item.and_then(|v| v.into_book()) {
                            match get_metadata_by_source(&item.source).await? {
                                Some(metadata) => {
                                    overwrite_book_with_new_metadata(book_model, metadata, db)
                                        .await?;

                                    return Ok(());
                                }

                                None => info!("Unable to find by source"),
                            }
                        }
                    }
                }

                // Step 2
                // Etc..
            }

            UpdatingBook::UpdateBookWithSource {
                book_id: old_book_id,
                source,
            } => {
                info!(
                    id = ?old_book_id,
                    ?source,
                    "Update Book By Source",
                );

                send_message_to_clients(WebsocketNotification::update_task(
                    task_id,
                    TaskType::UpdatingBook {
                        id: old_book_id,
                        subtitle: None,
                    },
                    true,
                ));

                match BookModel::find_one_by_source(&source, db).await? {
                    // If the metadata already exists we move the old metadata files to the new one and completely remove old metadata.
                    Some(book_item) => {
                        if book_item.id != old_book_id {
                            info!(
                                ?old_book_id,
                                new_book_id = ?book_item.id,
                                "Converting File Metadata to New File Metadata",
                            );

                            // Change file metas'from old to new meta
                            let changed_files =
                                FileModel::transfer_book_id(old_book_id, book_item.id, db).await?;

                            // Update new meta file count
                            BookModel::set_file_count(
                                book_item.id,
                                book_item.file_item_count + changed_files as i64,
                                db,
                            )
                            .await?;

                            // Remove old meta persons
                            BookPersonModel::delete_by_book_id(old_book_id, db).await?;

                            // TODO: Change to "deleted" instead of delting from database. We will delete from database every 24 hours.

                            // Remove old Metadata
                            BookModel::delete_by_id(old_book_id, db).await?;
                        } else {
                            // Update existing metadata.
                            // TODO: Check how long it has been since we've refreshed meta: new_meta if auto-ran.

                            info!("Updating existing File Metadata.");

                            if let Some(mut new_meta) = get_metadata_by_source(&source).await? {
                                let mut current_book =
                                    BookModel::find_one_by_id(old_book_id, db).await?.unwrap();

                                let (main_author, author_ids) =
                                    new_meta.add_or_ignore_authors_into_database(db).await?;

                                let MetadataReturned {
                                    mut meta,
                                    publisher,
                                    ..
                                } = new_meta;

                                if let Some(item) =
                                    meta.thumb_locations.iter_mut().find(|v| v.is_url())
                                {
                                    item.download(db).await?;
                                }

                                let mut new_book: NewBookModel = meta.into();

                                // TODO: Store Publisher inside Database
                                new_book.cached = new_book
                                    .cached
                                    .publisher_optional(publisher)
                                    .author_optional(main_author);

                                if current_book.rating == 0.0 {
                                    current_book.rating = new_book.rating;
                                }

                                // If we didn't update the original title
                                if current_book.title == current_book.original_title {
                                    current_book.title = new_book.title;
                                }

                                if new_book.description.is_some() {
                                    current_book.description = new_book.description;
                                }

                                current_book.original_title = new_book.original_title;
                                current_book.refreshed_at = Utc::now().naive_utc();
                                current_book.updated_at = Utc::now().naive_utc();

                                // No old thumb, but we have an new one. Set new one as old one.
                                if current_book.thumb_url.is_none()
                                    && new_book.thumb_url.is_some()
                                {
                                    current_book.thumb_url = new_book.thumb_url;
                                }

                                if let Some(thumb_path) = current_book.thumb_url.as_value() {
                                    if let Some(image) =
                                        UploadedImageModel::get_by_path(thumb_path, db).await?
                                    {
                                        ImageLinkModel::new_book(image.id, current_book.id)
                                            .insert(db)
                                            .await?;
                                    }
                                }

                                current_book.update(db).await?;

                                for person_id in author_ids {
                                    BookPersonModel {
                                        book_id: current_book.id,
                                        person_id,
                                    }
                                    .insert_or_ignore(db)
                                    .await?;
                                }
                            } else {
                                info!(?source, "Unable to find metadata");
                                // TODO: Error since this shouldn't have happened.
                            }
                        }
                    }

                    // No metadata source. Lets scrape it and update our current one with the new one.
                    None => {
                        if let Some(mut new_meta) = get_metadata_by_source(&source).await? {
                            info!(
                                ?source,
                                ?old_book_id,
                                "Grabbed New Book from Source, updating old Book with it."
                            );

                            let old_book =
                                BookModel::find_one_by_id(old_book_id, db).await?.unwrap();

                            let (main_author, author_ids) =
                                new_meta.add_or_ignore_authors_into_database(db).await?;

                            let MetadataReturned {
                                mut meta,
                                publisher,
                                ..
                            } = new_meta;

                            if let Some(item) = meta.thumb_locations.iter_mut().find(|v| v.is_url())
                            {
                                item.download(db).await?;
                            }

                            let book: NewBookModel = meta.into();
                            let mut book = book.set_id(old_book.id);

                            // TODO: Store Publisher inside Database
                            book.cached = book
                                .cached
                                .publisher_optional(publisher)
                                .author_optional(main_author);

                            book.library_id = old_book.library_id;
                            book.file_item_count = old_book.file_item_count;
                            book.rating = old_book.rating;

                            if old_book.title != old_book.original_title {
                                book.title = old_book.title;
                            }

                            // No new thumb, but we have an old one. Set old one as new one.
                            if book.thumb_url.is_none() && old_book.thumb_url.is_some() {
                                book.thumb_url = old_book.thumb_url;
                            }

                            if book.description.is_none() {
                                book.description = old_book.description;
                            }

                            if let Some(thumb_path) = book.thumb_url.as_value() {
                                if let Some(image) =
                                    UploadedImageModel::get_by_path(thumb_path, db).await?
                                {
                                    ImageLinkModel::new_book(image.id, book.id)
                                        .insert(db)
                                        .await?;
                                }
                            }

                            book.update(db).await?;

                            // TODO: Should I start with a clean slate like this?
                            BookPersonModel::delete_by_book_id(old_book_id, db).await?;

                            for person_id in author_ids {
                                BookPersonModel {
                                    book_id: book.id,
                                    person_id,
                                }
                                .insert_or_ignore(db)
                                .await?;
                            }
                        } else {
                            info!(?source, "Unable to get metadata");
                            // TODO: Error since this shouldn't have happened.
                        }
                    }
                }
            }

            UpdatingBook::UpdateAllWithAgent {
                library_id,
                agent: _a,
            } => {
                // TODO: Use agent
                let active_agent = ActiveAgents {
                    google: false,
                    libby: true,
                    local: false,
                    openlib: false,
                };

                const LIMIT: i64 = 100;

                let amount =
                    BookModel::count_search_by(&FilterContainer::default(), Some(library_id), db)
                        .await? as i64;
                let mut offset = 0;

                while offset < amount {
                    let books =
                        BookModel::find_by(Some(library_id), offset, LIMIT, None, db).await?;

                    for book in books {
                        if Utc::now()
                            .signed_duration_since(book.refreshed_at.and_local_timezone(Utc).unwrap())
                            .num_days()
                            > 7
                        {
                            let book_id = book.id;

                            send_message_to_clients(WebsocketNotification::update_task(
                                task_id,
                                TaskType::UpdatingBook {
                                    id: book_id,
                                    subtitle: None,
                                },
                                true,
                            ));

                            Self::update_book_by_files(book, &active_agent, db).await?;

                            send_message_to_clients(WebsocketNotification::update_task(
                                task_id,
                                TaskType::UpdatingBook {
                                    id: book_id,
                                    subtitle: None,
                                },
                                true,
                            ));
                        }
                    }

                    offset += LIMIT;
                }
            }
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "Updating Book"
    }
}

impl TaskUpdateInvalidBook {
    async fn return_found_metadata_by_files(
        book_model: &BookModel,
        agent: &ActiveAgents,
        db: &mut SqliteConnection,
    ) -> Result<Option<MetadataReturned>> {
        let files = FileModel::find_by_book_id(book_model.id, db).await?;

        Ok(match get_metadata_from_files(&files, agent).await? {
            None => {
                if let Some(title) = book_model.title.as_deref() {
                    // TODO: Separate
                    // Check by "title - author" secondly.
                    let search = format!(
                        "{} {}",
                        title,
                        book_model.cached.author.as_deref().unwrap_or_default()
                    );

                    // Search for query.
                    let results = search_all_agents(
                        search.trim(),
                        SearchFor::Book(SearchForBooksBy::Query),
                        agent,
                    )
                    .await?;

                    // Find the SearchedItem by similarity.
                    let found_item = results
                        .sort_items_by_similarity(title)
                        .into_iter()
                        .find(|&(score, ref item)| match item {
                            SearchItem::Book(book) => {
                                // info!("Score: {score} | {:?} | {}", book.title, book.source);
                                score > 0.75 && !book.thumb_locations.is_empty()
                            }
                            _ => false,
                        })
                        .map(|(_, item)| item);

                    // Now we need to do a search for found item and return it.
                    if let Some(item) = found_item.and_then(|v| v.into_book()) {
                        get_metadata_by_source(&item.source).await?
                    } else {
                        None
                    }
                } else {
                    None
                }
            }

            v => v,
        })
    }

    async fn update_book_by_files(
        curr_book_model: BookModel,
        agent: &ActiveAgents,
        db: &mut SqliteConnection,
    ) -> Result<()> {
        // Check Files first.
        let found_meta = Self::return_found_metadata_by_files(&curr_book_model, agent, db).await?;

        match found_meta {
            Some(metadata) => {
                overwrite_book_with_new_metadata(curr_book_model, metadata, db).await?
            }
            None => info!("Unable to find by files"),
        }

        Ok(())
    }
}

async fn overwrite_book_with_new_metadata(
    mut curr_book_model: BookModel,
    mut metadata: MetadataReturned,
    db: &mut SqliteConnection,
) -> Result<()> {
    let (main_author, author_ids) = metadata.add_or_ignore_authors_into_database(db).await?;

    let MetadataReturned {
        mut meta,
        publisher,
        ..
    } = metadata;

    // If we have no local files we'll download the first one.
    if !meta.thumb_locations.iter().any(|v| v.is_local()) {
        if let Some(item) = meta.thumb_locations.first_mut() {
            item.download(db).await?;
        }
    }

    let new_book_model: NewBookModel = meta.into();
    let mut new_book_model = new_book_model.set_id(curr_book_model.id);

    // TODO: Store Publisher inside Database
    new_book_model.cached = new_book_model
        .cached
        .publisher_optional(publisher)
        .author_optional(main_author);

    // Update New Book with old one
    new_book_model.library_id = curr_book_model.library_id;
    new_book_model.deleted_at = curr_book_model.deleted_at;
    new_book_model.file_item_count = curr_book_model.file_item_count;

    // If we're not replacing the metadata with local then we'll make sure everything is filled in.
    if new_book_model.source.agent.as_ref() != "local" {
        new_book_model.rating = curr_book_model.rating;

        // Overwrite prev with new and replace new with prev.
        curr_book_model.cached.overwrite_with(new_book_model.cached);
        new_book_model.cached = curr_book_model.cached;

        if curr_book_model.title != curr_book_model.original_title {
            new_book_model.title = curr_book_model.title;
        }

        // No new thumb, but we have an old one. Set old one as new one.
        if new_book_model.thumb_url.is_none() && curr_book_model.thumb_url.is_some() {
            new_book_model.thumb_url = curr_book_model.thumb_url;
        }

        if new_book_model.description.is_none() {
            new_book_model.description = curr_book_model.description;
        }
    }

    // TODO: Only if book exists and IS the same source.
    new_book_model.created_at = curr_book_model.created_at;

    if let Some(thumb_path) = new_book_model.thumb_url.as_value() {
        if let Some(image) = UploadedImageModel::get_by_path(thumb_path, db).await? {
            ImageLinkModel::new_book(image.id, new_book_model.id)
                .insert(db)
                .await?;
        }
    }

    new_book_model.refreshed_at = Utc::now().naive_utc();

    new_book_model.update(db).await?;

    BookPersonModel::delete_by_book_id(new_book_model.id, db).await?;

    for person_id in author_ids {
        BookPersonModel {
            book_id: new_book_model.id,
            person_id,
        }
        .insert_or_ignore(db)
        .await?;
    }

    Ok(())
}