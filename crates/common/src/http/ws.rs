use serde::{Serialize, Deserialize};



#[derive(Debug, Serialize, Deserialize)]
pub enum WebsocketResponse {
	Notification(WebsocketNotification)
}


#[derive(Debug, Serialize, Deserialize)]
pub enum WebsocketNotification {
	//
}