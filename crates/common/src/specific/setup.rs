use serde::{Serialize, Deserialize};



#[derive(Default, Clone, Serialize, Deserialize)]
pub struct SetupConfig {
	pub name: Option<String>,
	pub directories: Vec<String>,
}