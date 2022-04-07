use std::{sync::Mutex, thread, time::{Duration, Instant}, collections::VecDeque};

use actix_web::web;
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use lazy_static::lazy_static;
use tokio::{runtime::Runtime, time::sleep};

use crate::{database::{Database, table}, ThumbnailLocation, metadata::{MetadataReturned, get_metadata, get_metadata_by_source, get_person_by_source}, ThumbnailType};


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
	AutoMatchMetaId(i64),
	SpecificMatchSingleMetaId {
		meta_id: i64,
		source: String,
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
						match get_metadata(&file, None, db).await {
							Ok(meta) => {
								if let Some(mut ret) = meta {
									let (main_author, author_ids) = ret.add_or_ignore_authors_into_database(db).await?;

									let MetadataReturned { mut meta, publisher, .. } = ret;

									// TODO: Store Publisher inside Database
									meta.cached = meta.cached.publisher_optional(publisher).author_optional(main_author);

									let meta = db.add_or_increment_metadata(&meta)?;
									db.update_file_metadata_id(file.id, meta.id)?;

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

			// TODO: Check how long it has been since we've refreshed metadata if auto-ran.
			UpdatingMetadata::AutoMatchMetaId(meta_id) => {
				println!("Finding Metadata for current Metadata ID: {}", meta_id);

				// TODO: Remove file aspect of get_metadata or add Vec<File> to get_metadata

				// let fm = db.find_file_by_id_with_metadata(meta_id)?.unwrap();

				// match get_metadata(&fm.file, fm.meta.as_ref(), db).await {
				// 	Ok(Some(mut ret)) => {
				// 		let (main_author, author_ids) = ret.add_or_ignore_authors_into_database(db).await?;

				// 		let MetadataReturned { mut meta, publisher, .. } = ret;

				// 		// TODO: Store Publisher inside Database
				// 		meta.cached = meta.cached.publisher_optional(publisher).author_optional(main_author);

				// 		if let Some(fm_meta) = fm.meta {
				// 			meta.id = fm_meta.id;
				// 			meta.library_id = fm_meta.library_id;
				// 			meta.rating = fm_meta.rating;
				// 			meta.deleted_at = fm_meta.deleted_at;
				// 			meta.file_item_count = fm_meta.file_item_count;
				// 			meta.cached.overwrite_with(fm_meta.cached);

				// 			if fm_meta.title != fm_meta.original_title {
				// 				meta.title = fm_meta.title;
				// 			}

				// 			match (meta.thumb_path.is_some(), fm_meta.thumb_path.is_some()) {
				// 				// No new thumb, but we have an old one. Set old one as new one.
				// 				(false, true) => {
				// 					meta.thumb_path = fm_meta.thumb_path;
				// 				}

				// 				// Both have a poster and they're both different.
				// 				(true, true) if meta.thumb_path != fm_meta.thumb_path => {
				// 					// Remove old poster.
				// 					let loc = ThumbnailLocation::from(fm_meta.thumb_path.unwrap());
				// 					let path = crate::image::prefixhash_to_path(loc.as_type(), loc.as_value());
				// 					tokio::fs::remove_file(path).await?;
				// 				}

				// 				_ => ()
				// 			}

				// 			// TODO: Only if metadata exists and IS the same source.
				// 			meta.created_at = fm_meta.created_at;
				// 		}

				// 		db.update_metadata(&meta)?;

				// 		for person_id in author_ids {
				// 			db.add_meta_person(&MetadataPerson {
				// 				metadata_id: meta.id,
				// 				person_id,
				// 			})?;
				// 		}
				// 	}

				// 	Ok(None) => eprintln!("Metadata Grabber Error: UNABLE TO FIND"),
				// 	Err(e) => eprintln!("Metadata Grabber Error: {}", e)
				// }
			}

			UpdatingMetadata::SpecificMatchSingleMetaId { meta_id: old_meta_id, source } => {
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
							println!("Current File Metadata is equal to New File Metadata");
						}
					}

					// No metadata source. Lets scrape it and update our current one with the new one.
					None => {
						let (source, value) = source.split_once(':').unwrap();

						if let Some(mut new_meta) = get_metadata_by_source(source, value).await? {
							println!("Grabbed New Metadata from Source \"{}:{}\", updating old Metadata ({}) with it.", source, value, old_meta_id);

							let old_meta = db.get_metadata_by_id(old_meta_id)?.unwrap();

							let (main_author, author_ids) = new_meta.add_or_ignore_authors_into_database(db).await?;

							let MetadataReturned { mut meta, publisher, .. } = new_meta;

							// TODO: Store Publisher inside Database
							meta.cached = meta.cached.publisher_optional(publisher).author_optional(main_author);

							meta.id = old_meta.id;
							meta.library_id = old_meta.library_id;
							meta.file_item_count = old_meta.file_item_count;
							meta.rating = old_meta.rating;

							if old_meta.title != old_meta.original_title {
								meta.title = old_meta.title;
							}

							match (meta.thumb_path.is_some(), old_meta.thumb_path.is_some()) {
								// No new thumb, but we have an old one. Set old one as new one.
								(false, true) => {
									meta.thumb_path = old_meta.thumb_path;
								}

								// Both have a poster and they're both different.
								(true, true) if meta.thumb_path != old_meta.thumb_path => {
									// Remove old poster.
									let loc = ThumbnailLocation::from(old_meta.thumb_path.unwrap());
									let path = crate::image::prefixhash_to_path(loc.as_type(), loc.as_value());
									tokio::fs::remove_file(path).await?;
								}

								_ => ()
							}

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
							println!("Unable to get metadata from source \"{}:{}\"", source, value);
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



// People

#[derive(Clone)]
pub enum UpdatingPeople {
	AutoUpdateById(i64),
	UpdatePersonWithSource {
		person_id: i64,
		source: String,
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
				let mut old_person = db.get_person_by_id(person_id)?.unwrap();

				let (source, value) = old_person.source.split_once(':').unwrap();

				if let Some(new_person) = get_person_by_source(source, value).await? {
					// TODO: Need to make sure it doesn't conflict with alt names or normal names if different.
					if old_person.name != new_person.name {
						println!("TODO: Old Name {:?} != New Name {:?}", old_person.name, new_person.name);
					}

					// Download thumb url and store it.
					if let Some(url) = new_person.cover_image_url {
						let resp = reqwest::get(url).await?;

						if resp.status().is_success() {
							let bytes = resp.bytes().await?;

							match crate::store_image(ThumbnailType::Metadata, bytes.to_vec()).await {
								Ok(path) => old_person.thumb_url = Some(ThumbnailType::Metadata.prefix_text(&path)),
								Err(e) => {
									eprintln!("UpdatingPeople::AutoUpdateById (store_image) Error: {}", e);
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
								person_id,
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

					db.update_person(&old_person).unwrap();
				} else {
					println!("[TASK] Unable to find person to auto-update");
				}
			}

			UpdatingPeople::UpdatePersonWithSource { person_id, source } => {
				unimplemented!("UpdatingPeople::UpdatePersonWithSource: {:?}, {:?}", person_id, source);
			}
		}

		Ok(())
	}

	fn name(&self) ->  &'static str {
		"Updating Person"
	}
}