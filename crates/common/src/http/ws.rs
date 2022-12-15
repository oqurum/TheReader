use std::{
    fmt,
    sync::atomic::{AtomicUsize, Ordering},
};

use common::BookId;
use serde::{Deserialize, Serialize};

pub static UNIQUE_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(usize);

impl TaskId {
    pub fn new() -> Self {
        Self(UNIQUE_ID.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        usize::fmt(&self.0, f)
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub name: String,

    pub current: Option<TaskType>,
}


#[derive(Debug, Serialize, Deserialize)]
pub enum WebsocketResponse {
    Ping,
    Pong,

    Notification(WebsocketNotification),
}

impl WebsocketResponse {
    pub fn is_ping(&self) -> bool {
        matches!(self, Self::Ping)
    }

    pub fn is_pong(&self) -> bool {
        matches!(self, Self::Pong)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebsocketNotification {
    TaskStart {
        id: TaskId,
        name: String,
    },

    TaskUpdate {
        id: TaskId,
        type_of: TaskType,
        inserting: bool,
    },

    TaskEnd(TaskId),
}

impl WebsocketNotification {
    pub fn new_task(id: TaskId, name: String) -> Self {
        Self::TaskStart { id, name }
    }

    pub fn update_task(id: TaskId, type_of: TaskType, inserting: bool) -> Self {
        Self::TaskUpdate { id, type_of, inserting }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    UpdatingBook {
        id: BookId,
        subtitle: Option<String>,
    },

    LibraryScan(String),
}
