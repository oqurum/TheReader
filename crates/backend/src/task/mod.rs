use std::{sync::Mutex, thread, time::{Duration, Instant}, collections::VecDeque};

use actix_web::web;
use async_trait::async_trait;
use common_local::{SearchFor, SearchForBooksBy, ws::{UniqueId, WebsocketNotification, TaskType}};
use chrono::Utc;
use common::{BookId, PersonId, Source};
use lazy_static::lazy_static;
use tokio::{runtime::Runtime, time::sleep};

use crate::{
    Result,
    database::Database,
    metadata::{MetadataReturned, get_metadata_from_files, get_metadata_by_source, get_person_by_source, search_all_agents, SearchItem}, http::send_message_to_clients, model::{image::{ImageLinkModel, UploadedImageModel}, library::LibraryModel, directory::DirectoryModel, book::BookModel, file::FileModel, book_person::BookPersonModel, person::PersonModel, person_alt::PersonAltModel}
};


// TODO: A should stop boolean
// TODO: Store what's currently running

lazy_static! {
    pub static ref TASKS_QUEUED: Mutex<VecDeque<Box<dyn Task>>> = Mutex::new(VecDeque::new());
}


#[async_trait]
pub trait Task: Send {
    async fn run(&mut self, task_id: UniqueId, db: &Database) -> Result<()>;

    fn name(&self) -> &'static str;
}



pub fn queue_task<T: Task + 'static>(task: T) {
    TASKS_QUEUED.lock().unwrap().push_back(Box::new(task));
}

pub fn queue_task_priority<T: Task + 'static>(task: T) {
    TASKS_QUEUED.lock().unwrap().push_front(Box::new(task));
}



pub fn start_task_manager(db: web::Data<Database>) {
    thread::spawn(move || {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            loop {
                sleep(Duration::from_secs(1)).await;

                // Used to prevent holding lock past await.
                let task = {
                    let mut v = TASKS_QUEUED.lock().unwrap();
                    v.pop_front()
                };

                if let Some(mut task) = task {
                    let start_time = Instant::now();

                    let task_id = UniqueId::default();

                    match task.run(task_id, &db).await {
                        Ok(_) => println!("Task {:?} Finished Successfully. Took: {:?}", task.name(), start_time.elapsed()),
                        Err(e) => eprintln!("Task {:?} Error: {e}", task.name()),
                    }

                    send_message_to_clients(WebsocketNotification::TaskEnd(task_id));
                }
            }
        });
    });
}



// TODO: Better name.

pub struct TaskLibraryScan;

#[async_trait]
impl Task for TaskLibraryScan {
    async fn run(&mut self, _task_id: UniqueId, db: &Database) -> Result<()> {
        for library in LibraryModel::get_all(db).await? {
            let directories = DirectoryModel::find_directories_by_library_id(library.id, db).await?;

            crate::scanner::library_scan(&library, directories, db).await?;
        }

        Ok(())
    }

    fn name(&self) ->  &'static str {
        "Library Scan"
    }
}



// Metadata

#[derive(Clone)]
pub enum UpdatingBook {
    AutoMatchInvalid,
    AutoUpdateBookIdBySource(BookId),
    AutoUpdateBookIdByFiles(BookId),
    UpdateBookWithSource {
        book_id: BookId,
        source: Source,
    }
}

pub struct TaskUpdateInvalidBook {
    state: UpdatingBook
}

impl TaskUpdateInvalidBook {
    pub fn new(state: UpdatingBook) -> Self {
        Self {
            state
        }
    }
}

#[async_trait]
impl Task for TaskUpdateInvalidBook {
    async fn run(&mut self, task_id: UniqueId, db: &Database) -> Result<()> {
        match self.state.clone() {
            // TODO: Remove at some point. Currently inside of scanner.
            UpdatingBook::AutoMatchInvalid => {
                for file in FileModel::find_by_missing_book(db).await? {
                    // TODO: Ensure we ALWAYS creates some type of metadata for the file.
                    if file.book_id.map(|v| v == 0).unwrap_or(true) {
                        let file_id = file.id;

                        match get_metadata_from_files(&[file]).await {
                            Ok(meta) => {
                                if let Some(mut ret) = meta {
                                    let (main_author, author_ids) = ret.add_or_ignore_authors_into_database(db).await?;

                                    let MetadataReturned { mut meta, publisher, .. } = ret;

                                    // TODO: This is For Local File Data. Need specify.
                                    if let Some(item) = meta.thumb_locations.iter_mut().find(|v| v.is_file_data()) {
                                        item.download(db).await?;
                                    }

                                    let mut book_model: BookModel = meta.into();

                                    // TODO: Store Publisher inside Database
                                    book_model.cached = book_model.cached.publisher_optional(publisher).author_optional(main_author);

                                    let book_model = book_model.insert_or_increment(db).await?;
                                    FileModel::update_book_id(file_id, book_model.id, db).await?;

                                    if let Some(thumb_path) = book_model.thumb_path.as_value() {
                                        if let Some(image) = UploadedImageModel::get_by_path(thumb_path, db).await? {
                                            ImageLinkModel::new_book(image.id, book_model.id).insert(db).await?;
                                        }
                                    }

                                    for person_id in author_ids {
                                        BookPersonModel {
                                            book_id: book_model.id,
                                            person_id,
                                        }.insert_or_ignore(db).await?;
                                    }
                                }
                            }

                            Err(e) => {
                                eprintln!("metadata::get_metadata: {:?}", e);
                            }
                        }
                    }
                }
            }

            UpdatingBook::AutoUpdateBookIdByFiles(book_id) => {
                println!("Auto Update Metadata ID by Files: {}", book_id);
                send_message_to_clients(WebsocketNotification::new_task(task_id, TaskType::UpdatingBook(book_id)));

                let fm_book = BookModel::find_one_by_id(book_id, db).await?.unwrap();

                Self::search_book_by_files(book_id, fm_book, db).await?;
            }

            // TODO: Check how long it has been since we've refreshed meta: new_meta if auto-ran.
            UpdatingBook::AutoUpdateBookIdBySource(book_id) => {
                println!("Auto Update Metadata ID by Source: {}", book_id);
                send_message_to_clients(WebsocketNotification::new_task(task_id, TaskType::UpdatingBook(book_id)));

                let fm_book = BookModel::find_one_by_id(book_id, db).await?.unwrap();

                // TODO: Attempt to update source first.
                if let Some(mut new_meta) = get_metadata_by_source(&fm_book.source).await? {
                    println!("Updating by source");

                    let mut current_book = fm_book.clone();

                    let (main_author, author_ids) = new_meta.add_or_ignore_authors_into_database(db).await?;

                    let MetadataReturned { meta, publisher, .. } = new_meta;

                    let mut new_book: BookModel = meta.into();

                    // TODO: Utilize EditManager which is currently in frontend util.rs

                    // TODO: Store Publisher inside Database
                    new_book.cached = new_book.cached.publisher_optional(publisher).author_optional(main_author);

                    if current_book.rating == 0.0 {
                        current_book.rating = new_book.rating;
                    }

                    // If we didn't update the original title
                    if current_book.title == current_book.original_title {
                        current_book.title = new_book.title;
                    }

                    current_book.original_title = new_book.original_title;
                    current_book.refreshed_at = Utc::now();
                    current_book.updated_at = Utc::now();

                    // No new thumb, but we have an old one. Set old one as new one.
                    if current_book.thumb_path.is_none() && new_book.thumb_path.is_some() {
                        current_book.thumb_path = new_book.thumb_path;
                    }

                    if new_book.description.is_some() {
                        current_book.description = new_book.description;
                    }

                    if let Some(thumb_path) = current_book.thumb_path.as_value() {
                        if let Some(image) = UploadedImageModel::get_by_path(thumb_path, db).await? {
                            ImageLinkModel::new_book(image.id, current_book.id).insert(db).await?;
                        }
                    }


                    current_book.update(db).await?;

                    for person_id in author_ids {
                        BookPersonModel {
                            book_id: new_book.id,
                            person_id,
                        }.insert_or_ignore(db).await?;
                    }

                    return Ok(());
                }

                println!("Updating by file check");

                Self::search_book_by_files(book_id, fm_book, db).await?;
            }

            UpdatingBook::UpdateBookWithSource { book_id: old_book_id, source } => {
                println!("UpdatingMetadata::SpecificMatchSingleMetaId {{ book_id: {:?}, source: {:?} }}", old_book_id, source);
                send_message_to_clients(WebsocketNotification::new_task(task_id, TaskType::UpdatingBook(old_book_id)));

                match BookModel::find_one_by_source(&source, db).await? {
                    // If the metadata already exists we move the old metadata files to the new one and completely remove old metadata.
                    Some(book_item) => {
                        if book_item.id != old_book_id {
                            println!("Changing Current File Metadata ({}) to New File Metadata ({})", old_book_id, book_item.id);

                            // Change file metas'from old to new meta
                            let changed_files = FileModel::transfer_book_id(old_book_id, book_item.id, db).await?;

                            // Update new meta file count
                            BookModel::set_file_count(book_item.id, book_item.file_item_count as usize + changed_files, db).await?;

                            // Remove old meta persons
                            BookPersonModel::delete_by_book_id(old_book_id, db).await?;

                            // TODO: Change to "deleted" instead of delting from database. We will delete from database every 24 hours.

                            // Remove old Metadata
                            BookModel::delete_by_id(old_book_id, db).await?;
                        } else {
                            // Update existing metadata.

                            println!("Updating File Metadata.");

                            if let Some(mut new_meta) = get_metadata_by_source(&source).await? {
                                let mut current_book = BookModel::find_one_by_id(old_book_id, db).await?.unwrap();

                                let (main_author, author_ids) = new_meta.add_or_ignore_authors_into_database(db).await?;

                                let MetadataReturned { mut meta, publisher, .. } = new_meta;

                                if let Some(item) = meta.thumb_locations.iter_mut().find(|v| v.is_url()) {
                                    item.download(db).await?;
                                }

                                let mut new_book: BookModel = meta.into();

                                // TODO: Store Publisher inside Database
                                new_book.cached = new_book.cached.publisher_optional(publisher).author_optional(main_author);

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
                                current_book.refreshed_at = Utc::now();
                                current_book.updated_at = Utc::now();

                                // No old thumb, but we have an new one. Set new one as old one.
                                if current_book.thumb_path.is_none() && new_book.thumb_path.is_some() {
                                    current_book.thumb_path = new_book.thumb_path;
                                }

                                if let Some(thumb_path) = current_book.thumb_path.as_value() {
                                    if let Some(image) = UploadedImageModel::get_by_path(thumb_path, db).await? {
                                        ImageLinkModel::new_book(image.id, current_book.id).insert(db).await?;
                                    }
                                }


                                current_book.update(db).await?;

                                for person_id in author_ids {
                                    BookPersonModel {
                                        book_id: current_book.id,
                                        person_id,
                                    }.insert_or_ignore(db).await?;
                                }
                            } else {
                                println!("Unable to get book metadata from source {:?}", source);
                                // TODO: Error since this shouldn't have happened.
                            }
                        }
                    }

                    // No metadata source. Lets scrape it and update our current one with the new one.
                    None => {
                        if let Some(mut new_meta) = get_metadata_by_source(&source).await? {
                            println!("Grabbed New Book from Source {:?}, updating old Book ({}) with it.", source, old_book_id);

                            let old_book = BookModel::find_one_by_id(old_book_id, db).await?.unwrap();

                            let (main_author, author_ids) = new_meta.add_or_ignore_authors_into_database(db).await?;

                            let MetadataReturned { mut meta, publisher, .. } = new_meta;

                            if let Some(item) = meta.thumb_locations.iter_mut().find(|v| v.is_url()) {
                                item.download(db).await?;
                            }

                            let mut book: BookModel = meta.into();

                            // TODO: Store Publisher inside Database
                            book.cached = book.cached.publisher_optional(publisher).author_optional(main_author);

                            book.id = old_book.id;
                            book.library_id = old_book.library_id;
                            book.file_item_count = old_book.file_item_count;
                            book.rating = old_book.rating;

                            if old_book.title != old_book.original_title {
                                book.title = old_book.title;
                            }

                            // No new thumb, but we have an old one. Set old one as new one.
                            if book.thumb_path.is_none() && old_book.thumb_path.is_some() {
                                book.thumb_path = old_book.thumb_path;
                            }

                            if book.description.is_none() {
                                book.description = old_book.description;
                            }

                            if let Some(thumb_path) = book.thumb_path.as_value() {
                                if let Some(image) = UploadedImageModel::get_by_path(thumb_path, db).await? {
                                    ImageLinkModel::new_book(image.id, book.id).insert(db).await?;
                                }
                            }

                            book.update(db).await?;

                            // TODO: Should I start with a clean slate like this?
                            BookPersonModel::delete_by_book_id(old_book_id, db).await?;

                            for person_id in author_ids {
                                BookPersonModel {
                                    book_id: book.id,
                                    person_id,
                                }.insert_or_ignore(db).await?;
                            }
                        } else {
                            println!("Unable to get metadata from source {:?}", source);
                            // TODO: Error since this shouldn't have happened.
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn name(&self) ->  &'static str {
        "Updating Book"
    }
}

impl TaskUpdateInvalidBook {
    async fn search_book_by_files(book_id: BookId, mut fm_book: BookModel, db: &Database) -> Result<()> {
        // Check Files first.
        let files = FileModel::find_by_book_id(book_id, db).await?;

        let found_meta = match get_metadata_from_files(&files).await? {
            None => if let Some(title) = fm_book.title.as_deref() { // TODO: Place into own function
                // Check by "title - author" secondly.
                let search = format!(
                    "{} {}",
                    title,
                    fm_book.cached.author.as_deref().unwrap_or_default()
                );

                // Search for query.
                let results = search_all_agents(search.as_str(), SearchFor::Book(SearchForBooksBy::Query)).await?;

                // Find the SearchedItem by similarity.
                let found_item = results.sort_items_by_similarity(title)
                    .into_iter()
                    .find(|&(score, ref item)| match item {
                        SearchItem::Book(book) => {
                            println!("Score: {score} | {:?} | {}", book.title, book.source);
                            score > 0.75 && !book.thumb_locations.is_empty()
                        }
                        _ => false
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

            v => v
        };


        match found_meta {
            Some(mut ret) => {
                let (main_author, author_ids) = ret.add_or_ignore_authors_into_database(db).await?;

                let MetadataReturned { mut meta, publisher, .. } = ret;

                // If we have no local files we'll download the first one.
                if !meta.thumb_locations.iter().any(|v| v.is_local()) {
                    if let Some(item) = meta.thumb_locations.first_mut() {
                        item.download(db).await?;
                    }
                }

                let mut book: BookModel = meta.into();

                // TODO: Store Publisher inside Database
                book.cached = book.cached.publisher_optional(publisher).author_optional(main_author);

                // Update New Book with old one
                book.id = fm_book.id;
                book.library_id = fm_book.library_id;
                book.rating = fm_book.rating;
                book.deleted_at = fm_book.deleted_at;
                book.file_item_count = fm_book.file_item_count;

                // Overwrite prev with new and replace new with prev.
                fm_book.cached.overwrite_with(book.cached);
                book.cached = fm_book.cached;

                if fm_book.title != fm_book.original_title {
                    book.title = fm_book.title;
                }

                // No new thumb, but we have an old one. Set old one as new one.
                if book.thumb_path.is_none() && fm_book.thumb_path.is_some() {
                    book.thumb_path = fm_book.thumb_path;
                }

                if fm_book.description.is_some() {
                    book.description = fm_book.description;
                }

                // TODO: Only if book exists and IS the same source.
                book.created_at = fm_book.created_at;

                if let Some(thumb_path) = book.thumb_path.as_value() {
                    if let Some(image) = UploadedImageModel::get_by_path(thumb_path, db).await? {
                        ImageLinkModel::new_book(image.id, book.id).insert(db).await?;
                    }
                }

                book.update(db).await?;

                for person_id in author_ids {
                    BookPersonModel {
                        book_id: book.id,
                        person_id,
                    }.insert_or_ignore(db).await?;
                }
            }

            None => eprintln!("Book Grabber Error: UNABLE TO FIND"),
        }

        Ok(())
    }
}



// People

#[derive(Clone)]
pub enum UpdatingPeople {
    AutoUpdateById(PersonId),
    UpdatePersonWithSource {
        person_id: PersonId,
        source: Source,
    }
}


pub struct TaskUpdatePeople {
    state: UpdatingPeople
}

impl TaskUpdatePeople {
    pub fn new(state: UpdatingPeople) -> Self {
        Self {
            state
        }
    }
}


#[async_trait]
impl Task for TaskUpdatePeople {
    async fn run(&mut self, _task_id: UniqueId, db: &Database) -> Result<()> {
        match self.state.clone() {
            UpdatingPeople::AutoUpdateById(person_id) => {
                let old_person = PersonModel::find_one_by_id(person_id, db).await?.unwrap();
                let source = old_person.source.clone();

                Self::overwrite_person_with_source(old_person, &source, db).await
            }

            UpdatingPeople::UpdatePersonWithSource { person_id, source } => {
                let old_person = PersonModel::find_one_by_id(person_id, db).await?.unwrap();

                Self::overwrite_person_with_source(old_person, &source, db).await
            }
        }
    }

    fn name(&self) ->  &'static str {
        "Updating Person"
    }
}

impl TaskUpdatePeople {
    pub async fn overwrite_person_with_source(mut old_person: PersonModel, source: &Source, db: &Database) -> Result<()> {
        if let Some(new_person) = get_person_by_source(source).await? {
            // TODO: Need to make sure it doesn't conflict with alt names or normal names if different.
            if old_person.name != new_person.name {
                println!("TODO: Old Name {:?} != New Name {:?}", old_person.name, new_person.name);
            }

            // Download thumb url and store it.
            if let Some(url) = new_person.cover_image_url {
                let resp = reqwest::get(&url).await?;

                if resp.status().is_success() {
                    let bytes = resp.bytes().await?;

                    // TODO: Used for Open Library. We don't check to see if we actually have an image yet.
                    if bytes.len() > 1000 {
                        println!("Cover URL: {}", url);

                        match crate::store_image(bytes.to_vec(), db).await {
                            Ok(model) => old_person.thumb_url = model.path,
                            Err(e) => {
                                eprintln!("UpdatingPeople::AutoUpdateById (store_image) Error: {}", e);
                            }
                        }
                    }
                } else {
                    let text = resp.text().await;
                    eprintln!("UpdatingPeople::AutoUpdateById (image request) Error: {:?}", text);
                }
            }

            if let Some(alts) = new_person.other_names {
                for name in alts {
                    // Ignore errors. Errors should just be UNIQUE constraint failed
                    if let Err(e) = (PersonAltModel {
                        person_id: old_person.id,
                        name,
                    }).insert(db).await {
                        eprintln!("[TASK]: Add Alt Name Error: {e}");
                    }
                }
            }

            old_person.birth_date = new_person.birth_date;
            old_person.description = new_person.description;
            old_person.source = new_person.source;
            old_person.updated_at = Utc::now();

            old_person.update(db).await?;

            // TODO: Update Book cache
        } else {
            println!("[TASK] Unable to find person to auto-update");
        }

        Ok(())
    }
}