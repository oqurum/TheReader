use std::ops::Deref;

use anyhow::Result;
use sqlx::{sqlite::{SqlitePool, SqliteArguments}, Arguments};

pub async fn init() -> Result<Database> {
	let conn = SqlitePool::connect("sqlite::memory:").await?;

	sqlx::query(r#"
		CREATE TABLE "files" (
			"path"  	TEXT NOT NULL UNIQUE,
			"file_type"	TEXT,
			"file_name"	TEXT NOT NULL,
			"file_size"	NUMBER NOT NULL,
			"modified_at"	NUMBER NOT NULL,
			"accessed_at"	NUMBER NOT NULL,
			"created_at"	NUMBER NOT NULL
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
	pub async fn add_file(&self, file: &File) -> Result<()> {
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

	pub async fn get_file_count(&self) -> Result<i64> {
		Ok(sqlx::query_scalar(r#"SELECT COUNT(*) FROM files"#).fetch_one(self.deref()).await?)
	}
}


pub struct File {
	pub path: String,
	pub file_type: String,
	pub file_name: String,
	pub file_size: i64,
	pub modified_at: i64,
	pub accessed_at: i64,
	pub created_at: i64,
}