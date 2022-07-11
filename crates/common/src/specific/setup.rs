use serde::{Serialize, Deserialize};



#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SetupConfig {
	pub name: Option<String>,
	pub directories: Vec<String>,
	pub email: Option<ConfigEmail>,
	pub authenticators: Authenticators,
}

impl SetupConfig {
	pub fn get_email_mut(&mut self) -> &mut ConfigEmail {
		// TODO: Use Option::get_or_insert_default once stable
		self.email.get_or_insert_with(Default::default)
	}
}


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ConfigEmail {
	pub display_name: String,
	pub sending_email: String,
	pub contact_email: String,

	pub subject_line: String,

	pub smtp_username: String,
	pub smtp_password: String,
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
			email_no_pass: true,
			main_server: true,
		}
	}
}