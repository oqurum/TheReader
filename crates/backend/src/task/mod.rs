use std::{sync::Mutex, thread, time::{Duration, Instant}, collections::VecDeque};

use actix_web::web;
use anyhow::Result;
use async_trait::async_trait;
use lazy_static::lazy_static;
use tokio::{runtime::Runtime, time::sleep};

use crate::database::Database;


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



#[derive(Clone, Copy)]
pub enum UpdatingMetadata {
	Invalid,
	Single(i64)
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
		match self.state {
			UpdatingMetadata::Invalid => {
				for file in db.get_files_of_no_metadata()? {
					// TODO: Ensure we ALWAYS creates some type of metadata for the file.
					if file.metadata_id.map(|v| v == 0).unwrap_or(true) {
						match crate::metadata::get_metadata(&file, None).await {
							Ok(meta) => {
								if let Some(meta) = meta {
									let meta = db.add_or_increment_metadata(&meta)?;
									db.update_file_metadata_id(file.id, meta.id)?;
								}
							}

							Err(e) => {
								eprintln!("metadata::get_metadata: {:?}", e);
							}
						}
					}
				}
			}

			// TODO: Check if we're refreshing metadata.
			UpdatingMetadata::Single(file_id) => {
				println!("Finding Metadata for File ID: {}", file_id);

				let fm = db.find_file_by_id_with_metadata(file_id)?.unwrap();

				match crate::metadata::get_metadata(&fm.file, fm.meta.as_ref()).await {
					Ok(Some(mut meta)) => {
						if let Some(fm_meta) = fm.meta {
							meta.id = fm_meta.id;
							meta.rating = fm_meta.rating;
							meta.deleted_at = fm_meta.deleted_at;
							meta.file_item_count = fm_meta.file_item_count;

							if fm_meta.title != fm_meta.original_title {
								meta.title = fm_meta.title;
							}

							// TODO: Only if metadata exists and IS the same source.
							meta.created_at = fm_meta.created_at;
						}

						db.update_metadata(&meta)?;

						println!("{:#?}", meta);
					}

					Ok(None) => eprintln!("Metadata Grabber Error: UNABLE TO FIND"),
					Err(e) => eprintln!("Metadata Grabber Error: {}", e)
				}
			}
		}


		Ok(())
	}

	fn name(&self) ->  &'static str {
		"Updating Metadata"
	}
}