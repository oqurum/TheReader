use async_trait::async_trait;
use common_local::{ws::TaskId, LibraryId};

use crate::{
    model::{DirectoryModel, LibraryModel},
    Result, SqlPool, Task,
};

pub struct TaskLibraryScan {
    pub library_id: LibraryId,
}

#[async_trait]
impl Task for TaskLibraryScan {
    async fn run(&mut self, task_id: TaskId, pool: &SqlPool) -> Result<()> {
        let db = &mut *pool.acquire().await?;

        let library = LibraryModel::find_one_by_id(self.library_id, db)
            .await?
            .unwrap();

        // TODO: Return groups of Directories.
        let directories =
            DirectoryModel::find_directories_by_library_id(self.library_id, db).await?;

        crate::scanner::library_scan(&library, directories, task_id, db).await?;

        Ok(())
    }

    fn name(&self) -> &'static str {
        "Library Scan"
    }
}
