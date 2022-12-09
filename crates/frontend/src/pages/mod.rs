mod auth;

pub use auth::{
    login::{LoginPage, PasswordLogin, PasswordlessLogin},
    logout::LogoutPage,
};

pub mod author_view;
pub mod home;
pub mod library;
pub mod list_authors;
pub mod media_view;
pub mod reading;
pub mod settings;
pub mod setup;
pub mod list_collections;
pub mod collection;

pub use author_view::AuthorView;
pub use home::HomePage;
pub use library::LibraryPage;
pub use list_authors::AuthorListPage;
pub use media_view::MediaView;
pub use reading::ReadingBook;
pub use setup::SetupPage;
pub use list_collections::CollectionListPage;