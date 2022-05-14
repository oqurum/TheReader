use std::sync::Mutex;

use books_common::setup::SetupConfig;
use lazy_static::lazy_static;
use serde::{Serialize, Deserialize};

use crate::Result;


// <?xml version="1.0" encoding="utf-8"?>
// <Preferences
// 	MachineIdentifier="c80610b4-1c7e-42a0-a345-ad762bd7a9a9"
// 	ProcessedMachineIdentifier="221a00fbe4d1d9771af241aeb7e3a06d695d8443"
// 	TranscoderTempDirectory="/transcode"
// 	OldestPreviousVersion="legacy"
// 	AnonymousMachineIdentifier="30b001d0-4dfe-4e2b-ba59-3b994c2e8ba1"
// 	MetricsEpoch="1"
// 	AcceptedEULA="1"
// 	FriendlyName="Media 2"
// 	PublishServerOnPlexOnlineKey="1"
// 	PlexOnlineToken="TR5RE7xBMs2zWj4RVys1"
// 	PlexOnlineUsername="Timmayy"
// 	PlexOnlineMail="timjfernandes@gmail.com"
// 	PlexOnlineHome="1"
// 	DlnaEnabled="0"
// 	DvrIncrementalEpgLoader="0"
// 	CertificateUUID="bf3439507c5b413cabb13e0fc4cc41dd"
// 	PubSubServer="184.105.148.113"
// 	PubSubServerRegion="sjc"
// 	PubSubServerPing="31"
// 	CertificateVersion="3"
// 	LastAutomaticMappedPort="0"
// 	ManualPortMappingMode="1"
// 	LanguageInCloud="1"
// />

pub static CONFIG_PATH: &str = "../app/config.json";


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