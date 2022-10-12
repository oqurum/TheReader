use rusqlite::{types::FromSql, Row};

pub mod auth;
pub mod book;
pub mod book_person;
pub mod directory;
pub mod file;
pub mod image;
pub mod library;
pub mod member;
pub mod note;
pub mod person;
pub mod person_alt;
pub mod progress;

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
