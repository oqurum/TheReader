mod auth;

pub use auth::{login::{LoginPage, PasswordLogin, PasswordlessLogin}, logout::LogoutPage};

pub mod home;
pub mod setup;
pub mod library;
pub mod options;
pub mod reading;
pub mod media_view;
pub mod list_authors;
pub mod author_view;

pub use home::HomePage;
pub use setup::SetupPage;
pub use library::LibraryPage;
pub use options::OptionsPage;
pub use reading::ReadingBook;
pub use media_view::MediaView;
pub use list_authors::AuthorListPage;
pub use author_view::AuthorView;