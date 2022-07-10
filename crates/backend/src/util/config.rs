use std::sync::Mutex;

use books_common::setup::SetupConfig;
use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};

use crate::Result;


pub static CONFIG_PATH: &str = "./app/config.json";


lazy_static! {
	static ref CONFIG_FILE: Mutex<Option<Config>> = {
		if let Ok(data) = std::fs::read(CONFIG_PATH) {
			#[allow(clippy::expect_used)]
			Mutex::new(serde_json::from_slice(&data).expect("Loading Config File"))
		} else {
			Mutex::default()
		}
	};
}

pub fn does_config_exist() -> bool {
	CONFIG_FILE.lock().unwrap().is_some()
}

pub fn get_config() -> Config {
	CONFIG_FILE.lock().unwrap().clone().unwrap()
}


pub async fn save_config(value: SetupConfig) -> Result<()> {
	let config = Config {
		server_name: value.name.unwrap_or_else(|| String::from("Unnamed Server")),
		libby: None,
	};

	tokio::fs::write(
		CONFIG_PATH,
		serde_json::to_string_pretty(&config)?,
	).await?;

	*CONFIG_FILE.lock().unwrap() = Some(config);

	Ok(())
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	pub server_name: String,
	pub libby: Option<LibraryConnection>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConnection {
	pub token: String,
	pub username: String,
	pub email: String,
}