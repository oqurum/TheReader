use chrono::{Utc, NaiveDateTime};
use common::{BookId, ImageId, ImageType, PersonId, ThumbnailStore};
use serde::Serialize;
use sqlx::{FromRow, SqliteConnection};

use crate::{InternalError, Result};

#[derive(Debug, Serialize, FromRow)]
pub struct ImageLinkModel {
    pub image_id: ImageId,

    pub link_id: i64,
    pub type_of: ImageType,
}

#[derive(Serialize)]
pub struct NewUploadedImageModel {
    pub path: ThumbnailStore,

    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct UploadedImageModel {
    pub id: ImageId,

    pub path: ThumbnailStore,

    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ImageWithLink {
    pub image_id: ImageId,

    pub link_id: i64,
    pub type_of: ImageType,

    pub path: ThumbnailStore,

    pub created_at: NaiveDateTime,
}

impl NewUploadedImageModel {
    pub fn new(path: ThumbnailStore) -> Self {
        Self {
            path,
            created_at: Utc::now().naive_utc(),
        }
    }

    pub async fn get_or_insert(self, db: &mut SqliteConnection) -> Result<UploadedImageModel> {
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

    pub async fn insert(self, db: &mut SqliteConnection) -> Result<UploadedImageModel> {
        let Some(path) = self.path.into_value() else {
            return Err(InternalError::InvalidModel.into());
        };

        let res = sqlx::query(
            "INSERT OR IGNORE INTO uploaded_images (path, created_at) VALUES ($1, $2)"
        )
        .bind(&path)
        .bind(self.created_at)
        .execute(db).await?;

        Ok(UploadedImageModel {
            id: ImageId::from(res.last_insert_rowid()),
            path: ThumbnailStore::Path(path),
            created_at: self.created_at,
        })
    }

    pub async fn path_exists(path: &str, db: &mut SqliteConnection) -> Result<bool> {
        Ok(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM uploaded_images WHERE path = $1").bind(path).fetch_one(db).await? != 0)
    }
}

impl UploadedImageModel {
    pub async fn get_by_path(value: &str, db: &mut SqliteConnection) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT * FROM uploaded_images WHERE path = $1"
        ).bind(value).fetch_optional(db).await?)
    }

    pub async fn get_by_id(id: ImageId, db: &mut SqliteConnection) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT * FROM uploaded_images WHERE id = $1"
        ).bind(id).fetch_optional(db).await?)
    }

    // pub async fn remove(
    //     link_id: BookId,
    //     path: ThumbnailStore,
    //     db: &mut SqliteConnection,
    // ) -> Result<usize> {
    //     // TODO: Check for currently set images
    //     // TODO: Remove image links.
    //     if let Some(path) = path.into_value() {
    //         let res = sqlx::query!(
    //             "DELETE FROM uploaded_images WHERE link_id = $1 AND path = $2",
    //             link_id, path
    //         ).execute(db).await?;

    //         Ok(res.rows_affected())
    //     } else {
    //         Ok(0)
    //     }
    // }
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

    pub async fn insert(&self, db: &mut SqliteConnection) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO image_link (image_id, link_id, type_of) VALUES ($1, $2, $3)",
        )
        .bind(self.image_id)
        .bind(self.link_id)
        .bind(self.type_of)
        .execute(db).await?;

        Ok(())
    }

    pub async fn delete(self, db: &mut SqliteConnection) -> Result<()> {
        sqlx::query(
            "DELETE FROM image_link WHERE image_id = $1 AND link_id = $2 AND type_of = $3"
        )
        .bind(self.image_id)
        .bind(self.link_id)
        .bind(self.type_of)
        .execute(db).await?;

        Ok(())
    }

    // TODO: Place into ImageWithLink struct?
    pub async fn find_with_link_by_link_id(
        id: i64,
        type_of: ImageType,
        db: &mut SqliteConnection,
    ) -> Result<Vec<ImageWithLink>> {
        Ok(sqlx::query_as(
            r#"SELECT image_link.*, uploaded_images.path, uploaded_images.created_at
                FROM image_link
                INNER JOIN uploaded_images
                    ON uploaded_images.id = image_link.image_id
                WHERE link_id = $1 AND type_of = $2
            "#
        ).bind(id).bind(type_of).fetch_all(db).await?)
    }
}
