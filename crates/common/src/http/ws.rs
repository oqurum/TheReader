use serde::{Serialize, Deserialize};



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
	Task(TaskNotif)
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskNotif {
	UpdatingMetadata(usize),
}