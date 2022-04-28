use std::{sync::Mutex, thread, time::{Duration, Instant}, collections::VecDeque};

use actix_web::web;
use crate::{Result, metadata::{search_all_agents, SearchItem}};
use async_trait::async_trait;
use books_common::{Source, ThumbnailStoreType, SearchFor, SearchForBooksBy};
use chrono::Utc;
use lazy_static::lazy_static;
use tokio::{runtime::Runtime, time::sleep};

use crate::{database::{Database, table::{self, MetadataPerson, MetadataItem}}, metadata::{MetadataReturned, get_metadata_from_files, get_metadata_by_source, get_person_by_source}};


// TODO: A should stop boolean
// TODO: Store what's currently running

lazy_static! {
	pub static ref TASKS_QUEUED: Mutex<VecDeque<Box<dyn Task>>> = Mutex::new(VecDeque::new());
}


#[async_trait]
pub trait Task: Send {
	async fn run(&mut self, db: &Database) -> Result<()>;

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

				if let Some(mut task) = TASKS_QUEUED.lock().unwrap().pop_front() {
					let start_time = Instant::now();

					match task.run(&db).await {
						Ok(_) => println!("Task {:?} Finished Successfully. Took: {:?}", task.name(), start_time.elapsed()),
						Err(e) => eprintln!("Task {:?} Error: {e}", task.name()),
					}
				}
			}
		});
	});
}



// TODO: Better name.

pub struct TaskLibraryScan;

#[async_trait]
impl Task for TaskLibraryScan {
	async fn run(&mut self, db: &Database) -> Result<()> {
		for library in db.list_all_libraries()? {
			let directories = db.get_directories(library.id)?;

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
pub enum UpdatingMetadata {
	AutoMatchInvalid,
	AutoUpdateMetaIdBySource(usize),
	AutoUpdateMetaIdByFiles(usize),
	UpdateMetadataWithSource {
		meta_id: usize,
		source: Source,
	}
}

pub struct TaskUpdateInvalidMetadata {
	state: UpdatingMetadata
}

impl TaskUpdateInvalidMetadata {
	pub fn new(state: UpdatingMetadata) -> Self {
		Self {
			state
		}
	}
}

#[async_trait]
impl Task for TaskUpdateInvalidMetadata {
	async fn run(&mut self, db: &Database) -> Result<()> {
		match self.state.clone() {
			UpdatingMetadata::AutoMatchInvalid => {
				for file in db.get_files_of_no_metadata()? {
					// TODO: Ensure we ALWAYS creates some type of metadata for the file.
					if file.metadata_id.map(|v| v == 0).unwrap_or(true) {
						let file_id = file.id;

						match get_metadata_from_files(&[file]).await {
							Ok(meta) => {
								if let Some(mut ret) = meta {
									let (main_author, author_ids) = ret.add_or_ignore_authors_into_database(db).await?;

									let MetadataReturned { mut meta, publisher, .. } = ret;

									// TODO: This is For Local File Data. Need specify.
									if let Some(item) = meta.thumb_locations.iter_mut().find(|v| v.is_file_data()) {
										item.download().await?;
									}

									let mut meta: MetadataItem = meta.into();

									// TODO: Store Publisher inside Database
									meta.cached = meta.cached.publisher_optional(publisher).author_optional(main_author);

									let meta = db.add_or_increment_metadata(&meta)?;
									db.update_file_metadata_id(file_id, meta.id)?;

									db.add_poster(&table::NewPoster {
										link_id: meta.id,
										path: meta.thumb_path.clone(),
										created_at: Utc::now(),
									})?;

									for person_id in author_ids {
										db.add_meta_person(&table::MetadataPerson {
											metadata_id: meta.id,
											person_id,
										})?;
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

			UpdatingMetadata::AutoUpdateMetaIdByFiles(meta_id) => {
				println!("Auto Update Metadata ID by Files: {}", meta_id);

				let fm_meta = db.get_metadata_by_id(meta_id)?.unwrap();

				Self::search_meta_by_files(meta_id, fm_meta, db).await?;
			}

			// TODO: Check how long it has been since we've refreshed meta: metnew_metaa if auto-ran.
			UpdatingMetadata::AutoUpdateMetaIdBySource(meta_id) => {
				println!("Auto Update Metadata ID by Source: {}", meta_id);

				let fm_meta = db.get_metadata_by_id(meta_id)?.unwrap();

				// TODO: Attempt to update source first.
				if let Some(mut new_meta) = get_metadata_by_source(&fm_meta.source).await? {
					println!("Updating by source");

					let mut current_meta = fm_meta.clone();

					let (main_author, author_ids) = new_meta.add_or_ignore_authors_into_database(db).await?;

					let MetadataReturned { meta, publisher, .. } = new_meta;

					let mut new_meta: MetadataItem = meta.into();

					// TODO: Utilize EditManager which is currently in frontend util.rs

					// TODO: Store Publisher inside Database
					new_meta.cached = new_meta.cached.publisher_optional(publisher).author_optional(main_author);

					if current_meta.rating == 0.0 {
						current_meta.rating = new_meta.rating;
					}

					// If we didn't update the original title
					if current_meta.title == current_meta.original_title {
						current_meta.title = new_meta.title;
					}

					current_meta.original_title = new_meta.original_title;
					current_meta.refreshed_at = Utc::now();
					current_meta.updated_at = Utc::now();

					// No new thumb, but we have an old one. Set old one as new one.
					if current_meta.thumb_path.is_none() && new_meta.thumb_path.is_some() {
						current_meta.thumb_path = new_meta.thumb_path;
					}

					if new_meta.description.is_some() {
						current_meta.description = new_meta.description;
					}

					db.add_poster(&table::NewPoster {
						link_id: current_meta.id,
						path: current_meta.thumb_path.clone(),
						created_at: Utc::now(),
					})?;

					db.update_metadata(&current_meta)?;

					for person_id in author_ids {
						db.add_meta_person(&table::MetadataPerson {
							metadata_id: new_meta.id,
							person_id,
						})?;
					}

					return Ok(());
				}

				println!("Updating by file check");

				Self::search_meta_by_files(meta_id, fm_meta, db).await?;
			}

			UpdatingMetadata::UpdateMetadataWithSource { meta_id: old_meta_id, source } => {
				println!("UpdatingMetadata::SpecificMatchSingleMetaId {{ meta_id: {:?}, source: {:?} }}", old_meta_id, source);

				match db.get_metadata_by_source(&source)? {
					// If the metadata already exists we move the old metadata files to the new one and completely remove old metadata.
					Some(meta_item) => {
						if meta_item.id != old_meta_id {
							println!("Changing Current File Metadata ({}) to New File Metadata ({})", old_meta_id, meta_item.id);

							// Change file metas'from old to new meta
							let changed_files = db.change_files_metadata_id(old_meta_id, meta_item.id)?;

							// Update new meta file count
							db.set_metadata_file_count(meta_item.id, meta_item.file_item_count as usize + changed_files)?;

							// Remove old meta persons
							db.remove_persons_by_meta_id(old_meta_id)?;

							// TODO: Change to "deleted" instead of delting from database. We will delete from database every 24 hours.

							// Remove old Metadata
							db.remove_metadata_by_id(old_meta_id)?;
						} else {
							// Update existing metadata.

							println!("Updating File Metadata.");

							if let Some(mut new_meta) = get_metadata_by_source(&source).await? {
								let mut current_meta = db.get_metadata_by_id(old_meta_id)?.unwrap();

								let (main_author, author_ids) = new_meta.add_or_ignore_authors_into_database(db).await?;

								let MetadataReturned { mut meta, publisher, .. } = new_meta;

								if let Some(item) = meta.thumb_locations.iter_mut().find(|v| v.is_url()) {
									item.download().await?;
								}

								let mut new_meta: MetadataItem = meta.into();

								// TODO: Store Publisher inside Database
								new_meta.cached = new_meta.cached.publisher_optional(publisher).author_optional(main_author);

								if current_meta.rating == 0.0 {
									current_meta.rating = new_meta.rating;
								}

								// If we didn't update the original title
								if current_meta.title == current_meta.original_title {
									current_meta.title = new_meta.title;
								}

								if new_meta.description.is_some() {
									current_meta.description = new_meta.description;
								}

								current_meta.original_title = new_meta.original_title;
								current_meta.refreshed_at = Utc::now();
								current_meta.updated_at = Utc::now();

								// No old thumb, but we have an new one. Set new one as old one.
								if current_meta.thumb_path.is_none() && new_meta.thumb_path.is_some() {
									current_meta.thumb_path = new_meta.thumb_path;
								}

								db.add_poster(&table::NewPoster {
									link_id: current_meta.id,
									path: current_meta.thumb_path.clone(),
									created_at: Utc::now(),
								})?;

								db.update_metadata(&current_meta)?;

								for person_id in author_ids {
									db.add_meta_person(&table::MetadataPerson {
										metadata_id: current_meta.id,
										person_id,
									})?;
								}
							} else {
								println!("Unable to get metadata from source {:?}", source);
								// TODO: Error since this shouldn't have happened.
							}
						}
					}

					// No metadata source. Lets scrape it and update our current one with the new one.
					None => {
						if let Some(mut new_meta) = get_metadata_by_source(&source).await? {
							println!("Grabbed New Metadata from Source {:?}, updating old Metadata ({}) with it.", source, old_meta_id);

							let old_meta = db.get_metadata_by_id(old_meta_id)?.unwrap();

							let (main_author, author_ids) = new_meta.add_or_ignore_authors_into_database(db).await?;

							let MetadataReturned { mut meta, publisher, .. } = new_meta;

							if let Some(item) = meta.thumb_locations.iter_mut().find(|v| v.is_url()) {
								item.download().await?;
							}

							let mut meta: MetadataItem = meta.into();

							// TODO: Store Publisher inside Database
							meta.cached = meta.cached.publisher_optional(publisher).author_optional(main_author);

							meta.id = old_meta.id;
							meta.library_id = old_meta.library_id;
							meta.file_item_count = old_meta.file_item_count;
							meta.rating = old_meta.rating;

							if old_meta.title != old_meta.original_title {
								meta.title = old_meta.title;
							}

							// No new thumb, but we have an old one. Set old one as new one.
							if meta.thumb_path.is_none() && old_meta.thumb_path.is_some() {
								meta.thumb_path = old_meta.thumb_path;
							}

							if meta.description.is_none() {
								meta.description = old_meta.description;
							}

							db.add_poster(&table::NewPoster {
								link_id: meta.id,
								path: meta.thumb_path.clone(),
								created_at: Utc::now(),
							})?;

							db.update_metadata(&meta)?;

							// TODO: Should I start with a clean slate like this?
							db.remove_persons_by_meta_id(old_meta_id)?;

							for person_id in author_ids {
								db.add_meta_person(&table::MetadataPerson {
									metadata_id: meta.id,
									person_id,
								})?;
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
		"Updating Metadata"
	}
}

impl TaskUpdateInvalidMetadata {
	async fn search_meta_by_files(meta_id: usize, mut fm_meta: MetadataItem, db: &Database) -> Result<()> {
		// Check Files first.
		let files = db.get_files_by_metadata_id(meta_id)?;

		let found_meta = match get_metadata_from_files(&files).await? {
			None => if let Some(title) = fm_meta.title.as_deref() { // TODO: Place into own function
				// Check by "title - author" secondly.
				let search = format!(
					"{} {}",
					title,
					fm_meta.cached.author.as_deref().unwrap_or_default()
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
						item.download().await?;
					}
				}

				let mut meta: MetadataItem = meta.into();

				// TODO: Store Publisher inside Database
				meta.cached = meta.cached.publisher_optional(publisher).author_optional(main_author);

				// Update New Metadata with old one
				meta.id = fm_meta.id;
				meta.library_id = fm_meta.library_id;
				meta.rating = fm_meta.rating;
				meta.deleted_at = fm_meta.deleted_at;
				meta.file_item_count = fm_meta.file_item_count;

				// Overwrite prev with new and replace new with prev.
				fm_meta.cached.overwrite_with(meta.cached);
				meta.cached = fm_meta.cached;

				if fm_meta.title != fm_meta.original_title {
					meta.title = fm_meta.title;
				}

				// No new thumb, but we have an old one. Set old one as new one.
				if meta.thumb_path.is_none() && fm_meta.thumb_path.is_some() {
					meta.thumb_path = fm_meta.thumb_path;
				}

				if fm_meta.description.is_some() {
					meta.description = fm_meta.description;
				}

				// TODO: Only if metadata exists and IS the same source.
				meta.created_at = fm_meta.created_at;

				db.add_poster(&table::NewPoster {
					link_id: meta.id,
					path: meta.thumb_path.clone(),
					created_at: Utc::now(),
				})?;

				db.update_metadata(&meta)?;

				for person_id in author_ids {
					db.add_meta_person(&MetadataPerson {
						metadata_id: meta.id,
						person_id,
					})?;
				}
			}

			None => eprintln!("Metadata Grabber Error: UNABLE TO FIND"),
		}

		Ok(())
	}
}



// People

#[derive(Clone)]
pub enum UpdatingPeople {
	AutoUpdateById(usize),
	UpdatePersonWithSource {
		person_id: usize,
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
	async fn run(&mut self, db: &Database) -> Result<()> {
		match self.state.clone() {
			UpdatingPeople::AutoUpdateById(person_id) => {
				let old_person = db.get_person_by_id(person_id)?.unwrap();
				let source = old_person.source.clone();

				Self::overwrite_person_with_source(old_person, &source, db).await
			}

			UpdatingPeople::UpdatePersonWithSource { person_id, source } => {
				let old_person = db.get_person_by_id(person_id)?.unwrap();

				Self::overwrite_person_with_source(old_person, &source, db).await
			}
		}
	}

	fn name(&self) ->  &'static str {
		"Updating Person"
	}
}

impl TaskUpdatePeople {
	pub async fn overwrite_person_with_source(mut old_person: table::TagPerson, source: &Source, db: &Database) -> Result<()> {
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

						match crate::store_image(ThumbnailStoreType::Metadata, bytes.to_vec()).await {
							Ok(path) => old_person.thumb_url = path,
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
					if let Err(e) = db.add_person_alt(&table::TagPersonAlt {
						person_id: old_person.id,
						name,
					}) {
						eprintln!("[TASK]: Add Alt Name Error: {e}");
					}
				}
			}

			old_person.birth_date = new_person.birth_date;
			old_person.description = new_person.description;
			old_person.source = new_person.source;
			old_person.updated_at = Utc::now();

			db.update_person(&old_person)?;

			// TODO: Update Metadata cache
		} else {
			println!("[TASK] Unable to find person to auto-update");
		}

		Ok(())
	}
}