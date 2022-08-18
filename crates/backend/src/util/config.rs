use std::sync::Mutex;

use common_local::setup::Config;
use lazy_static::lazy_static;

use crate::Result;


pub static CONFIG_PATH: &str = "./app/config.toml";


lazy_static! {
    pub static ref CONFIG_FILE: Mutex<Option<Config>> = {
        if let Ok(data) = std::fs::read(CONFIG_PATH) {
            #[allow(clippy::expect_used)]
            Mutex::new(toml_edit::de::from_slice(&data).expect("Loading Config File"))
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

pub fn update_config<F: FnOnce(&mut Config) -> Result<()>>(value: F) -> Result<()> {
    let mut config = get_config();

    value(&mut config)?;

    *CONFIG_FILE.lock().unwrap() = Some(config);

    Ok(())
}



pub async fn save_config() -> Result<()> {
    let config = get_config();

    tokio::fs::write(
        CONFIG_PATH,
        toml_edit::ser::to_string_pretty(&config)?,
    ).await?;

    *CONFIG_FILE.lock().unwrap() = Some(config);

    Ok(())
}