use std::sync::atomic::{AtomicUsize, Ordering};

use common::BookId;
use serde::{Serialize, Deserialize};

pub static UNIQUE_ID: AtomicUsize = AtomicUsize::new(0);


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UniqueId(usize);

impl UniqueId {
    pub fn new() -> Self {
        Self(UNIQUE_ID.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for UniqueId {
    fn default() -> Self {
        Self::new()
    }
}



#[derive(Debug, Serialize, Deserialize)]
pub enum WebsocketResponse {
    Ping,
    Pong,

    Notification(WebsocketNotification)
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
        id: UniqueId,
        type_of: TaskType
    },
    TaskEnd(UniqueId),
}

impl WebsocketNotification {
    pub fn new_task(id: UniqueId, type_of: TaskType) -> Self {
        Self::TaskStart { id, type_of }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    UpdatingBook(BookId),

    TempRustWarningFix,
}