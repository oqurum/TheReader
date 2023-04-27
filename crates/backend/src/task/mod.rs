use std::{
    collections::VecDeque,
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

use actix_web::web;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use common_local::ws::{TaskId, WebsocketNotification};
use lazy_static::lazy_static;
use tokio::{runtime::Runtime, time::sleep};
use tracing::{error, info};

use crate::{http::send_message_to_clients, Result, SqlPool};

mod book_update;
mod library_scan;
mod update_people;

pub use book_update::*;
pub use library_scan::*;
pub use update_people::*;

pub(self) static MAX_CONCURRENT_RUNS: usize = 2;

// TODO: Unused Image Deletion task.
// TODO: A should stop boolean

lazy_static! {
    /// The tasks which are currently queued.
    pub static ref TASKS_QUEUED: Mutex<VecDeque<Box<dyn Task>>> = Mutex::new(VecDeque::new());

    /// The tasks which run in intervals.
    static ref TASK_INTERVALS: Mutex<Vec<TaskInterval>> = Mutex::new(vec![]);

    /// Currently running Tasks
    static ref TASKS_RUNNING: Mutex<Vec<TaskRunning>> = Mutex::new(Vec::new());
}

struct TaskRunning {
    id: TaskId,
    name: &'static str,
    started: DateTime<Utc>,
}

struct TaskInterval {
    pub last_ran: Option<DateTime<Utc>>,
    pub interval: Duration,
    pub task: fn() -> Box<dyn Task>,
}

// TODO: Implement for Concurrent task running.
// Only 1 task can run for each category.
// enum TaskCategory {
//     LibraryScan(LibraryId),
// }

#[async_trait]
pub trait Task: Send {
    async fn run(&mut self, task_id: TaskId, pool: &SqlPool) -> Result<()>;

    fn name(&self) -> &'static str;
}

pub fn queue_task<T: Task + 'static>(task: T) {
    TASKS_QUEUED.lock().unwrap().push_back(Box::new(task));
}

pub fn queue_task_priority<T: Task + 'static>(task: T) {
    TASKS_QUEUED.lock().unwrap().push_front(Box::new(task));
}

pub fn start_task_manager(db: web::Data<SqlPool>) {
    thread::spawn(move || {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            loop {
                sleep(Duration::from_secs(1)).await;

                // TODO: Should I check intervals first or manually queued first?

                // Used to prevent holding lock past await.
                let task = {
                    let now = Utc::now();

                    // Get the next task in interval.
                    let mut v = TASK_INTERVALS.lock().unwrap();

                    let interval = v.iter_mut().find_map(|v| {
                        match v.last_ran {
                            None => {
                                // TODO: Update last_ran AFTER we've ran the task.
                                v.last_ran = Some(Utc::now());

                                Some((v.task)())
                            }

                            Some(d)
                                if now.signed_duration_since(d).to_std().unwrap() >= v.interval =>
                            {
                                // TODO: Update last_ran AFTER we've ran the task.
                                v.last_ran = Some(Utc::now());

                                Some((v.task)())
                            }

                            _ => None,
                        }
                    });

                    interval.or_else(|| TASKS_QUEUED.lock().unwrap().pop_front())
                };

                // Run the found task.
                if let Some(mut task) = task {
                    let start_time = Instant::now();

                    let task_id = TaskId::default();

                    {
                        let mut tasks = TASKS_RUNNING.lock().unwrap();
                        tasks.push(TaskRunning {
                            id: task_id,
                            name: task.name(),
                            started: Utc::now(),
                        });
                    }

                    info!(id = ?task_id, name = task.name(), "Task Started");

                    send_message_to_clients(WebsocketNotification::new_task(
                        task_id,
                        task.name().to_string(),
                    ));

                    match task.run(task_id, &db).await {
                        Ok(_) => info!(
                            name = task.name(),
                            elapsed = ?start_time.elapsed(),
                            "Task Finished Successfully.",
                        ),
                        Err(e) => error!(task = task.name(), ?e),
                    }

                    send_message_to_clients(WebsocketNotification::TaskEnd(task_id));

                    {
                        let mut tasks = TASKS_RUNNING.lock().unwrap();
                        if let Some(index) = tasks.iter().position(|v| v.id == task_id) {
                            tasks.remove(index);
                        }
                    }
                }
            }
        });
    });
}
