use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Default, Clone, Serialize, Deserialize, Validate)]
pub struct SetupConfig {
    #[validate]
    pub server: ConfigServer,
    pub directories: Vec<String>,
    #[validate]
    pub email: Option<ConfigEmail>,
    pub authenticators: Authenticators,
    pub libby: Option<LibraryConnection>,
}

impl SetupConfig {
    pub fn get_email_mut(&mut self) -> &mut ConfigEmail {
        // TODO: Use Option::get_or_insert_default once stable
        self.email.get_or_insert_with(Default::default)
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ConfigServer,
    #[serde(default)]
    pub libby: LibraryConnection,
    pub email: Option<ConfigEmail>,
    pub authenticators: Authenticators,

    pub has_admin_account: bool,
    pub is_public_access: bool,
}

impl Config {
    pub fn is_fully_setup(&self) -> bool {
        self.has_admin_account &&
        // TODO: Remove. Used for second setup location.
        !self.server.name.trim().is_empty()
        // TODO: self.authenticators.main_server
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ConfigServer {
    #[validate(length(
        min = 3,
        max = 32,
        message = "Must be at least 3 long and less than 32 long."
    ))]
    pub name: String,
    pub is_secure: bool,
    pub auth_key: Vec<u8>,
}

impl Default for ConfigServer {
    fn default() -> Self {
        use rand::RngCore;

        let mut rng = rand::thread_rng();
        let mut key = [0; 64];

        rng.try_fill_bytes(&mut key)
            .expect("Unable to fill buffer for Auth Key");

        Self {
            name: Default::default(),
            is_secure: Default::default(),
            auth_key: key.to_vec(),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, Validate)]
pub struct ConfigEmail {
    #[validate(length(min = 1, message = "Cannot be empty"))]
    pub display_name: String,
    #[validate(length(min = 1, message = "Cannot be empty"))]
    pub sending_email: String,
    #[validate(length(min = 1, message = "Cannot be empty"))]
    pub contact_email: String,

    #[validate(length(min = 1, message = "Cannot be empty"))]
    pub subject_line: String,

    #[validate(length(min = 1, message = "Cannot be empty"))]
    pub smtp_username: String,
    #[validate(length(min = 1, message = "Cannot be empty"))]
    pub smtp_password: String,
    #[validate(length(min = 1, message = "Cannot be empty"))]
    pub smtp_relay: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Authenticators {
    pub email_pass: bool,
    pub email_no_pass: bool,
    pub main_server: bool,
}

impl Default for Authenticators {
    fn default() -> Self {
        Self {
            email_pass: true,
            email_no_pass: false,
            main_server: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryConnection {
    pub pubid: Option<String>,
    pub token: Option<String>,
    pub url: String,

    pub public_only: bool,
}

impl Default for LibraryConnection {
    fn default() -> Self {
        Self {
            pubid: None,
            token: None,
            url: String::from("https://oqurum.io"),
            public_only: true,
        }
    }
}
