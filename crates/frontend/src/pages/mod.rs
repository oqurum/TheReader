mod auth;

pub use auth::{
    login::{LoginPage, PasswordLogin, PasswordlessLogin},
    logout::LogoutPage,
};

pub mod author_view;
pub mod collection;
pub mod home;
pub mod library;
pub mod list_authors;
pub mod list_collections;
pub mod book;
pub mod reading;
pub mod settings;
pub mod setup;

pub use author_view::AuthorView;
pub use collection::CollectionItemPage;
pub use home::HomePage;
pub use library::LibraryPage;
pub use list_authors::AuthorListPage;
pub use list_collections::CollectionListPage;
pub use book::BookPage;
pub use reading::ReadingBook;
pub use setup::SetupPage;
