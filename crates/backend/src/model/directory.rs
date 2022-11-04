use rusqlite::params;

use crate::{DatabaseAccess, Result};
use common_local::LibraryId;

use super::{AdvRow, TableRow};

pub struct DirectoryModel {
    pub library_id: LibraryId,
    pub path: String,
}

impl TableRow<'_> for DirectoryModel {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            library_id: row.next()?,
            path: row.next()?,
        })
    }
}

impl DirectoryModel {
    pub async fn insert(&self, db: &dyn DatabaseAccess) -> Result<()> {
        db.write().await.execute(
            r#"INSERT INTO directory (library_id, path) VALUES (?1, ?2)"#,
            params![&self.library_id, &self.path],
        )?;

        Ok(())
    }

    pub async fn remove_by_path(path: &str, db: &dyn DatabaseAccess) -> Result<usize> {
        Ok(db
            .write()
            .await
            .execute(r#"DELETE FROM directory WHERE path = ?1"#, [path])?)
    }

    pub async fn delete_by_library_id(id: LibraryId, db: &dyn DatabaseAccess) -> Result<usize> {
        Ok(db
            .write()
            .await
            .execute(r#"DELETE FROM directory WHERE library_id = ?1"#, [id])?)
    }

    pub async fn find_directories_by_library_id(
        library_id: LibraryId,
        db: &dyn DatabaseAccess,
    ) -> Result<Vec<DirectoryModel>> {
        let this = db.read().await;

        let mut conn = this.prepare("SELECT * FROM directory WHERE library_id = ?1")?;

        let map = conn.query_map([library_id], |v| DirectoryModel::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn get_all(db: &dyn DatabaseAccess) -> Result<Vec<DirectoryModel>> {
        let this = db.read().await;

        let mut conn = this.prepare("SELECT * FROM directory")?;

        let map = conn.query_map([], |v| DirectoryModel::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }
}
