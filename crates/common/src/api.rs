use serde::{Serialize, Deserialize};

use crate::{MediaItem, Progression};



#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetBookIdResponse {
	pub media: MediaItem,
	pub progress: Option<Progression>
}