use std::ops::Deref;

use anyhow::Result;
use books_common::StrippedMediaItem;
use serde::Serialize;
use sqlx::{sqlite::{SqlitePool, SqliteArguments, SqliteRow}, Arguments, Row};

pub async fn init() -> Result<Database> {
	let conn = SqlitePool::connect("sqlite::memory:").await?;

	sqlx::query(r#"
		CREATE TABLE "files" (
			"id" INTEGER NOT NULL UNIQUE,

			"path" TEXT NOT NULL UNIQUE,
			"file_type"	TEXT,
			"file_name"	TEXT NOT NULL,
			"file_size"	INTEGER NOT NULL,

			"modified_at" INTEGER NOT NULL,
			"accessed_at" INTEGER NOT NULL,
			"created_at" INTEGER NOT NULL,

			PRIMARY KEY("id" AUTOINCREMENT)
		);
	"#).execute(&conn).await?;

	sqlx::query(r#"
		CREATE TABLE "notes" (
			"id" INTEGER NOT NULL UNIQUE,
			"file_id" TEXT NOT NULL,
			"user_id" TEXT NOT NULL,

			"data" TEXT NOT NULL,
			"data_size" INTEGER NOT NULL,

			"updated_at" INTEGER NOT NULL,
			"created_at" INTEGER NOT NULL,

			PRIMARY KEY("id" AUTOINCREMENT)
		);
	"#).execute(&conn).await?;

	Ok(Database(conn))
}


#[derive(Clone)]
pub struct Database(SqlitePool);

impl Deref for Database {
    type Target = SqlitePool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Database {
	pub async fn add_file(&self, file: &NewFile) -> Result<()> {
		let mut args = SqliteArguments::default();

		args.add(&file.path);
		args.add(&file.file_type);
		args.add(&file.file_name);
		args.add(file.file_size);
		args.add(file.modified_at);
		args.add(file.accessed_at);
		args.add(file.created_at);

		sqlx::query_with(r#"
			INSERT INTO files (path, file_type, file_name, file_size, modified_at, accessed_at, created_at)
			VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
		"#, args)
		.execute(self.deref())
		.await?;

		Ok(())
	}

	pub async fn list_all_files(&self) -> Result<Vec<File>> {
		Ok(sqlx::query_with(r#"SELECT * FROM files"#, SqliteArguments::default())
			.fetch_all(self.deref())
			.await?
			.into_iter()
			.map(|v| v.into())
			.collect())
	}

	pub async fn find_file_by_id(&self, id: i64) -> Result<Option<File>> {
		let mut args = SqliteArguments::default();

		args.add(id);

		Ok(sqlx::query_with(r#"SELECT * FROM files WHERE id=?1 LIMIT 1"#, args)
			.fetch_optional(self.deref())
			.await?
			.map(|v| v.into()))
	}

	pub async fn get_file_count(&self) -> Result<i64> {
		Ok(sqlx::query_scalar(r#"SELECT COUNT(*) FROM files"#).fetch_one(self.deref()).await?)
	}
}


pub struct NewFile {
	pub path: String,

	pub file_name: String,
	pub file_type: String,
	pub file_size: i64,

	pub modified_at: i64,
	pub accessed_at: i64,
	pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct File {
	pub id: i64,

	pub path: String,

	pub file_name: String,
	pub file_type: String,
	pub file_size: i64,

	pub modified_at: i64,
	pub accessed_at: i64,
	pub created_at: i64,
}

impl From<SqliteRow> for File {
	fn from(value: SqliteRow) -> Self {
		Self {
			id: value.get(0),
			path: value.get(1),
			file_type: value.get(2),
			file_name: value.get(3),
			file_size: value.get(4),
			modified_at: value.get(5),
			accessed_at: value.get(6),
			created_at: value.get(7),
		}
	}
}


impl From<File> for StrippedMediaItem {
	fn from(val: File) -> Self {
		StrippedMediaItem {
			id: val.id,
			file_name: val.file_name,
			file_type: val.file_type,
			modified_at: val.modified_at,
			created_at: val.created_at,
		}
	}
}