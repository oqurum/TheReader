use async_trait::async_trait;
use chrono::Utc;
use common::{PersonId, Source};
use common_local::ws::TaskId;
use sqlx::SqliteConnection;
use tracing::{debug, error, info};

use crate::{
    metadata::{get_person_by_source, FoundImageLocation},
    model::{PersonAltModel, PersonModel},
    Result, SqlPool, Task,
};

#[derive(Clone)]
pub enum UpdatingPeople {
    AutoUpdateById(PersonId),
    UpdatePersonWithSource { person_id: PersonId, source: Source },
}

pub struct TaskUpdatePeople {
    state: UpdatingPeople,
}

impl TaskUpdatePeople {
    pub fn new(state: UpdatingPeople) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Task for TaskUpdatePeople {
    async fn run(&mut self, _task_id: TaskId, pool: &SqlPool) -> Result<()> {
        let db = &mut *pool.acquire().await?;

        match self.state.clone() {
            UpdatingPeople::AutoUpdateById(person_id) => {
                let old_person = PersonModel::find_one_by_id(person_id, db).await?.unwrap();
                let source = old_person.source.clone();

                Self::overwrite_person_with_source(old_person, &source, db).await
            }

            UpdatingPeople::UpdatePersonWithSource { person_id, source } => {
                let old_person = PersonModel::find_one_by_id(person_id, db).await?.unwrap();

                Self::overwrite_person_with_source(old_person, &source, db).await
            }
        }
    }

    fn name(&self) -> &'static str {
        "Updating Person"
    }
}

impl TaskUpdatePeople {
    pub async fn overwrite_person_with_source(
        mut old_person: PersonModel,
        source: &Source,
        db: &mut SqliteConnection,
    ) -> Result<()> {
        if let Some(new_person) = get_person_by_source(source).await? {
            // TODO: Need to make sure it doesn't conflict with alt names or normal names if different.
            if old_person.name != new_person.name {
                debug!(
                    "TODO: Old Name {:?} != New Name {:?}",
                    old_person.name, new_person.name
                );
            }

            // Download thumb url and store it.
            if let Some(mut url) = new_person.cover_image_url {
                url.download(db).await?;

                if let FoundImageLocation::Local(path) = url {
                    old_person.thumb_url = path;
                }
            }

            if let Some(alts) = new_person.other_names {
                for name in alts {
                    // Ignore errors. Errors should just be UNIQUE constraint failed
                    if let Err(error) = (PersonAltModel {
                        person_id: old_person.id,
                        name,
                    })
                    .insert(db)
                    .await
                    {
                        error!(?error, "Adding Alt Name");
                    }
                }
            }

            old_person.birth_date = new_person.birth_date;
            old_person.description = new_person.description;
            old_person.source = new_person.source;
            old_person.updated_at = Utc::now().naive_utc();

            old_person.update(db).await?;

            // TODO: Update Book cache
        } else {
            info!("Unable to find person to update");
        }

        Ok(())
    }
}
