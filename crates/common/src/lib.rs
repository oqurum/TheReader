use serde::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaItem {
	pub id: usize,

	pub title: String,
	pub author: String,
	pub icon: String,

	pub chapter_count: usize
}

impl PartialEq for MediaItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}





#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Progression {
	Ebook {
		stopped: i32,
		total: i32
	},

	AudioBook {
		stopped: i32,
		total: i32
	}
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Chapter {
	pub value: usize,
	pub html: String
}