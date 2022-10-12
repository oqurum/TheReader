use std::sync::Mutex;

use common_local::setup::Config;
use lazy_static::lazy_static;

use crate::Result;

pub static CONFIG_PATH: &str = "./app/config.toml";

pub static IS_SETUP: Mutex<bool> = Mutex::new(false);

lazy_static! {
    pub static ref CONFIG_FILE: Mutex<Config> = {
        if let Ok(data) = std::fs::read(CONFIG_PATH) {
            #[allow(clippy::expect_used)]
            let config: Config = toml_edit::de::from_slice(&data).expect("Loading Config File");

            *IS_SETUP.lock().unwrap() = config.is_fully_setup();

            Mutex::new(config)
        } else {
            Mutex::default()
        }
    };
}

pub fn is_setup() -> bool {
    *IS_SETUP.lock().unwrap()
}

pub fn get_config() -> Config {
    CONFIG_FILE.lock().unwrap().clone()
}

pub fn update_config<F: FnOnce(&mut Config) -> Result<()>>(value: F) -> Result<()> {
    let mut config = get_config();

    value(&mut config)?;

    *CONFIG_FILE.lock().unwrap() = config;

    Ok(())
}

pub async fn save_config() -> Result<()> {
    let config = get_config();

    tokio::fs::write(CONFIG_PATH, toml_edit::ser::to_string_pretty(&config)?).await?;

    *CONFIG_FILE.lock().unwrap() = config;

    Ok(())
}
