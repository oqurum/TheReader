use chrono::{DateTime, TimeZone, Utc};
use common::{BookId, ImageId, ImageType, PersonId, ThumbnailStore};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use crate::{database::Database, InternalError, Result};
use common_local::util::serialize_datetime;

use super::{AdvRow, TableRow};

#[derive(Debug, Serialize)]
pub struct ImageLinkModel {
    pub image_id: ImageId,

    pub link_id: usize,
    pub type_of: ImageType,
}

#[derive(Serialize)]
pub struct NewUploadedImageModel {
    pub path: ThumbnailStore,

    #[serde(serialize_with = "serialize_datetime")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct UploadedImageModel {
    pub id: ImageId,

    pub path: ThumbnailStore,

    #[serde(serialize_with = "serialize_datetime")]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ImageWithLink {
    pub image_id: ImageId,

    pub link_id: usize,
    pub type_of: ImageType,

    pub path: ThumbnailStore,

    #[serde(serialize_with = "serialize_datetime")]
    pub created_at: DateTime<Utc>,
}

impl TableRow<'_> for UploadedImageModel {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.next()?,
            path: ThumbnailStore::from(row.next::<String>()?),
            created_at: Utc.timestamp_millis(row.next()?),
        })
    }
}

impl TableRow<'_> for ImageLinkModel {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            image_id: row.next()?,
            link_id: row.next()?,
            type_of: ImageType::from_number(row.next()?).unwrap(),
        })
    }
}

impl NewUploadedImageModel {
    pub fn new(path: ThumbnailStore) -> Self {
        Self {
            path,
            created_at: Utc::now(),
        }
    }

    pub async fn get_or_insert(self, db: &Database) -> Result<UploadedImageModel> {
        if let Some(path) = self.path.as_value() {
            if let Some(value) = UploadedImageModel::get_by_path(path, db).await? {
                Ok(value)
            } else {
                self.insert(db).await
            }
        } else {
            Err(InternalError::InvalidModel.into())
        }
    }

    pub async fn insert(self, db: &Database) -> Result<UploadedImageModel> {
        let path = match self.path.into_value() {
            Some(v) => v,
            None => return Err(InternalError::InvalidModel.into()),
        };

        let conn = db.write().await;

        conn.execute(
            r#"
            INSERT OR IGNORE INTO uploaded_images (path, created_at)
            VALUES (?1, ?2)
        "#,
            params![path.as_str(), self.created_at.timestamp_millis()],
        )?;

        Ok(UploadedImageModel {
            id: ImageId::from(conn.last_insert_rowid() as usize),
            path: ThumbnailStore::Path(path),
            created_at: self.created_at,
        })
    }

    pub async fn path_exists(path: &str, db: &Database) -> Result<bool> {
        Ok(db.read().await.query_row(
            "SELECT COUNT(*) FROM uploaded_images WHERE path = ?1",
            [path],
            |v| Ok(v.get::<_, usize>(0)? != 0),
        )?)
    }
}

impl UploadedImageModel {
    pub async fn get_by_path(value: &str, db: &Database) -> Result<Option<Self>> {
        Ok(db
            .read()
            .await
            .query_row(
                r#"SELECT * FROM uploaded_images WHERE path = ?1"#,
                [value],
                |v| Self::from_row(v),
            )
            .optional()?)
    }

    pub async fn get_by_id(value: ImageId, db: &Database) -> Result<Option<Self>> {
        Ok(db
            .read()
            .await
            .query_row(
                r#"SELECT * FROM uploaded_images WHERE id = ?1"#,
                [value],
                |v| Self::from_row(v),
            )
            .optional()?)
    }

    pub async fn remove(link_id: BookId, path: ThumbnailStore, db: &Database) -> Result<usize> {
        // TODO: Check for currently set images
        // TODO: Remove image links.
        if let Some(path) = path.into_value() {
            Ok(db.write().await.execute(
                r#"DELETE FROM uploaded_images WHERE link_id = ?1 AND path = ?2"#,
                params![link_id, path,],
            )?)
        } else {
            Ok(0)
        }
    }
}

impl ImageLinkModel {
    pub fn new_book(image_id: ImageId, link_id: BookId) -> Self {
        Self {
            image_id,
            link_id: *link_id,
            type_of: ImageType::Book,
        }
    }

    pub fn new_person(image_id: ImageId, link_id: PersonId) -> Self {
        Self {
            image_id,
            link_id: *link_id,
            type_of: ImageType::Person,
        }
    }

    pub async fn insert(&self, db: &Database) -> Result<()> {
        let conn = db.write().await;

        conn.execute(
            r#"
            INSERT OR IGNORE INTO image_link (image_id, link_id, type_of)
            VALUES (?1, ?2, ?3)
        "#,
            params![
                self.image_id.to_string(),
                self.link_id.to_string(),
                self.type_of.as_num()
            ],
        )?;

        Ok(())
    }

    pub async fn delete(self, db: &Database) -> Result<()> {
        db.write().await.execute(
            r#"DELETE FROM image_link WHERE image_id = ?1 AND link_id = ?2 AND type_of = ?3"#,
            params![self.image_id, self.link_id, self.type_of.as_num(),],
        )?;

        Ok(())
    }

    // TODO: Place into ImageWithLink struct?
    pub async fn find_with_link_by_link_id(
        id: usize,
        type_of: ImageType,
        db: &Database,
    ) -> Result<Vec<ImageWithLink>> {
        let this = db.read().await;

        let mut conn = this.prepare(
            r#"
            SELECT image_link.*, uploaded_images.path, uploaded_images.created_at
            FROM image_link
            INNER JOIN uploaded_images
                ON uploaded_images.id = image_link.image_id
            WHERE link_id = ?1 AND type_of = ?2
        "#,
        )?;

        let map = conn.query_map(params![id, type_of.as_num()], |row| {
            Ok(ImageWithLink {
                image_id: row.get(0)?,
                link_id: row.get(1)?,
                type_of: ImageType::from_number(row.get(2)?).unwrap(),
                path: ThumbnailStore::from(row.get::<_, String>(3)?),
                created_at: Utc.timestamp_millis(row.get(4)?),
            })
        })?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }
}
