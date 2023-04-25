use rusqlite::{types::FromSql, Row};

mod auth;
mod book;
mod book_person;
mod collection;
mod collection_item;
mod directory;
mod file;
mod image;
mod library;
mod member;
mod person;
mod person_alt;
mod progress;
mod client;

pub use auth::*;
pub use book::*;
pub use book_person::*;
pub use collection::*;
pub use collection_item::*;
pub use directory::*;
pub use file::*;
pub use self::image::*;
pub use library::*;
pub use member::{MemberModel, NewMemberModel};
pub use person::*;
pub use person_alt::*;
pub use progress::*;
pub use client::*;

pub trait TableRow<'a>
where
    Self: Sized,
{
    fn create(row: &mut AdvRow<'a>) -> rusqlite::Result<Self>;

    fn from_row(value: &'a Row<'a>) -> rusqlite::Result<Self> {
        Self::create(&mut AdvRow::from(value))
    }
}

pub struct AdvRow<'a> {
    index: usize,
    row: &'a Row<'a>,
}

impl<'a> AdvRow<'a> {
    #[allow(clippy::should_implement_trait)]
    pub fn next<T: FromSql>(&mut self) -> rusqlite::Result<T> {
        self.index += 1;

        self.row.get(self.index - 1)
    }

    pub fn next_opt<T: FromSql>(&mut self) -> rusqlite::Result<Option<T>> {
        self.next()
    }

    pub fn has_next(&self) -> rusqlite::Result<bool> {
        self.row
            .get::<_, Option<String>>(self.index)
            .map(|v| v.is_some())
    }
}

impl<'a> From<&'a Row<'a>> for AdvRow<'a> {
    fn from(row: &'a Row<'a>) -> Self {
        AdvRow { index: 0, row }
    }
}
