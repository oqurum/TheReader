use std::sync::Mutex;

use books_common::setup::{SetupConfig, Authenticators, ConfigEmail, ConfigServer};
use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};

use crate::Result;


pub static CONFIG_PATH: &str = "./app/config.toml";


lazy_static! {
	static ref CONFIG_FILE: Mutex<Option<Config>> = {
		if let Ok(data) = std::fs::read(CONFIG_PATH) {
			#[allow(clippy::expect_used)]
			Mutex::new(toml::from_slice(&data).expect("Loading Config File"))
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
		server: value.server,
		libby: None,
		email: value.email,
		authenticators: value.authenticators,
	};

	tokio::fs::write(
		CONFIG_PATH,
		toml::to_string_pretty(&config)?,
	).await?;

	*CONFIG_FILE.lock().unwrap() = Some(config);

	Ok(())
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	pub server: ConfigServer,
	pub libby: Option<LibraryConnection>,
	pub email: Option<ConfigEmail>,
	pub authenticators: Authenticators,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConnection {
	pub token: String,
	pub username: String,
	pub email: String,
}