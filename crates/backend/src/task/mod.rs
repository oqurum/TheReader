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