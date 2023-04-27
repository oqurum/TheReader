use crate::Result;
use sqlx::{SqlitePool, Sqlite, migrate::MigrateDatabase, Connection, Executor};

const DATABASE_PATH: &str = "./app/database.db";

pub type SqlPool = sqlx::Pool<sqlx::Sqlite>;
pub type SqlConnection = sqlx::pool::PoolConnection<sqlx::Sqlite>;


pub async fn init() -> Result<SqlPool> {
    let does_db_exist = Sqlite::database_exists(DATABASE_PATH).await.unwrap_or(false);

    if !does_db_exist {
        tracing::debug!("Creating database {DATABASE_PATH}");

        Sqlite::create_database(DATABASE_PATH).await?;
    } else {
        tracing::debug!("Database already exists");
    }

    let pool = SqlitePool::connect(DATABASE_PATH).await?;

    if !does_db_exist {
        create_tables(&pool).await?;
    }

    match sqlx::migrate!("./migrations").run(&pool).await {
        Ok(_) => tracing::debug!("Migration success"),
        Err(error) => panic!("Migration Error: {error}"),
    }

    Ok(pool)
}

pub async fn create_tables(database: &SqlPool) -> Result<()> {
    let mut conn = database.acquire().await?;

    conn.transaction(|conn| Box::pin(async move {
        // Migrations
        conn.execute(
            r#"CREATE TABLE migration (
                id          INT NOT NULL,

                title       TEXT NOT NULL,
                duration    INT NOT NULL,
                notes       TEXT NOT NULL,

                created_at  DATETIME NOT NULL
            );"#,
        ).await?;

        // Library
        conn.execute(
            r#"CREATE TABLE "library" (
                "id"                 INTEGER NOT NULL UNIQUE,

                "name"               TEXT NOT NULL UNIQUE,
                "type_of"            INT NOT NULL,

                "is_public"          BOOLEAN NOT NULL,
                "settings"           TEXT,

                "scanned_at"         DATETIME NOT NULL,
                "created_at"         DATETIME NOT NULL,
                "updated_at"         DATETIME NOT NULL,

                PRIMARY KEY("id" AUTOINCREMENT)
            );"#,
        ).await?;

        // Directory
        conn.execute(
            r#"CREATE TABLE "directory" (
                "library_id"    INTEGER NOT NULL,
                "path"          TEXT NOT NULL UNIQUE,

                FOREIGN KEY("library_id") REFERENCES library("id") ON DELETE CASCADE
            );"#,
        ).await?;

        // File
        conn.execute(
            r#"CREATE TABLE "file" (
                "id"               INTEGER NOT NULL UNIQUE,

                "path"             TEXT NOT NULL UNIQUE,
                "file_name"        TEXT NOT NULL,
                "file_type"        TEXT NOT NULL,
                "file_size"        INTEGER NOT NULL,

                "library_id"       INTEGER NOT NULL,
                "book_id"          INTEGER,
                "chapter_count"    INTEGER NOT NULL,

                "identifier"       TEXT,
                "hash"             TEXT NOT NULL UNIQUE,

                "modified_at"      DATETIME NOT NULL,
                "accessed_at"      DATETIME NOT NULL,
                "created_at"       DATETIME NOT NULL,
                "deleted_at"       DATETIME,

                PRIMARY KEY("id" AUTOINCREMENT),

                FOREIGN KEY("book_id") REFERENCES book("id") ON DELETE CASCADE
            );"#,
        ).await?;

        // Book Item
        conn.execute(
            r#"CREATE TABLE "book" (
                "id"                  INTEGER NOT NULL,

                "library_id"          INTEGER NOT NULL,

                "type_of"             INT NOT NULL,

                "parent_id"           INTEGER REFERENCES book("id") ON DELETE CASCADE,

                "source"              TEXT NOT NULL,
                "file_item_count"     INTEGER NOT NULL,
                "title"               TEXT,
                "original_title"      TEXT,
                "description"         TEXT,
                "rating"              FLOAT NOT NULL,
                "thumb_url"           TEXT,

                "cached"              TEXT NOT NULL,
                "index"               INTEGER,

                "available_at"        DATETIME,
                "year"                INTEGER,

                "refreshed_at"        DATETIME NOT NULL,
                "created_at"          DATETIME NOT NULL,
                "updated_at"          DATETIME NOT NULL,
                "deleted_at"          DATETIME,

                PRIMARY KEY("id" AUTOINCREMENT),

                FOREIGN KEY("library_id") REFERENCES library("id") ON DELETE CASCADE
            );"#,
        ).await?;

        // Book People
        conn.execute(
            r#"CREATE TABLE "book_person" (
                "book_id"   INTEGER NOT NULL,
                "person_id" INTEGER NOT NULL,

                FOREIGN KEY("book_id") REFERENCES book("id") ON DELETE CASCADE,
                FOREIGN KEY("person_id") REFERENCES tag_person("id") ON DELETE CASCADE,

                UNIQUE(book_id, person_id)
            );"#,
        ).await?;

        // TODO: Versionize Notes. Keep last 20 versions for X one month. Auto delete old versions.
        // File Note
        conn.execute(
            r#"CREATE TABLE "file_note" (
                "file_id"       INTEGER NOT NULL,
                "user_id"       INTEGER NOT NULL,

                "data"          TEXT NOT NULL,
                "data_size"     INTEGER NOT NULL,

                "updated_at"    DATETIME NOT NULL,
                "created_at"    DATETIME NOT NULL,

                FOREIGN KEY("user_id") REFERENCES members("id") ON DELETE CASCADE,
                FOREIGN KEY("file_id") REFERENCES file("id") ON DELETE CASCADE,

                UNIQUE(file_id, user_id)
            );"#,
        ).await?;

        // File Progression
        conn.execute(
            r#"CREATE TABLE "file_progression" (
                "book_id"       INTEGER NOT NULL,
                "file_id"       INTEGER NOT NULL,
                "user_id"       INTEGER NOT NULL,

                "type_of"       INTEGER NOT NULL,

                "chapter"       INTEGER,
                "page"          INTEGER,
                "char_pos"      INTEGER,
                "seek_pos"      INTEGER,

                "updated_at"    DATETIME NOT NULL,
                "created_at"    DATETIME NOT NULL,

                FOREIGN KEY("user_id") REFERENCES members("id") ON DELETE CASCADE,
                FOREIGN KEY("file_id") REFERENCES file("id") ON DELETE CASCADE,
                FOREIGN KEY("book_id") REFERENCES book("id") ON DELETE CASCADE,

                UNIQUE(book_id, user_id)
            );"#,
        ).await?;

        // File Notation
        conn.execute(
            r#"CREATE TABLE "file_notation" (
                "file_id"       INTEGER NOT NULL,
                "user_id"       INTEGER NOT NULL,

                "data"          TEXT NOT NULL,
                "data_size"     INTEGER NOT NULL,
                "version"       INTEGER NOT NULL,

                "updated_at"    DATETIME NOT NULL,
                "created_at"    DATETIME NOT NULL,

                FOREIGN KEY("user_id") REFERENCES members("id") ON DELETE CASCADE,
                FOREIGN KEY("file_id") REFERENCES file("id") ON DELETE CASCADE,

                UNIQUE(file_id, user_id)
            );"#,
        ).await?;

        // Tags People
        conn.execute(
            r#"CREATE TABLE "tag_person" (
                "id"             INTEGER NOT NULL,

                "source"         TEXT NOT NULL,

                "name"           TEXT NOT NULL COLLATE NOCASE,
                "description"    TEXT,
                "birth_date"     TEXT,

                "thumb_url"      TEXT,

                "updated_at"     DATETIME NOT NULL,
                "created_at"     DATETIME NOT NULL,

                PRIMARY KEY("id" AUTOINCREMENT)
            );"#,
        ).await?;

        // People Alt names
        conn.execute(
            r#"CREATE TABLE "tag_person_alt" (
                "person_id"    INTEGER NOT NULL,

                "name"         TEXT NOT NULL COLLATE NOCASE,

                FOREIGN KEY("person_id") REFERENCES tag_person("id") ON DELETE CASCADE,

                UNIQUE(person_id, name)
            );"#,
        ).await?;

        // Members
        conn.execute(
            r#"CREATE TABLE "members" (
                "id"             INTEGER NOT NULL,

                "name"           TEXT NOT NULL COLLATE NOCASE,
                "email"          TEXT NOT NULL COLLATE NOCASE,
                "password"       TEXT,

                "type_of"        INTEGER NOT NULL,

                "permissions"    INTEGER NOT NULL,

                "library_access" TEXT,

                "created_at"     DATETIME NOT NULL,
                "updated_at"     DATETIME NOT NULL,

                UNIQUE(email),
                PRIMARY KEY("id" AUTOINCREMENT)
            );"#,
        ).await?;

        // Auth
        conn.execute(
            r#"CREATE TABLE "auth" (
                "oauth_token"           TEXT UNIQUE,
                "oauth_token_secret"    TEXT NOT NULL UNIQUE,

                "member_id"             INTEGER,

                "created_at"            DATETIME NOT NULL,
                "updated_at"            DATETIME NOT NULL,

                FOREIGN KEY("member_id") REFERENCES members("id") ON DELETE CASCADE
            );"#,
        ).await?;

        // Client
        conn.execute(
            r#"CREATE TABLE client (
                id          INTEGER NOT NULL,

                oauth       INTEGER NOT NULL,

                identifier  TEXT NOT NULL UNIQUE,

                client      TEXT NOT NULL,
                device      TEXT NOT NULL,
                platform    TEXT,

                created_at  DATETIME NOT NULL,
                updated_at  DATETIME NOT NULL,

                FOREIGN KEY("oauth") REFERENCES auth("oauth_token_secret") ON DELETE CASCADE,
                PRIMARY KEY("id" AUTOINCREMENT)
            );"#,
        ).await?;

        // Uploaded Images
        conn.execute(
            r#"CREATE TABLE "uploaded_images" (
                "id"            INTEGER NOT NULL,

                "path"          TEXT NOT NULL,
                "created_at"    DATETIME NOT NULL,

                UNIQUE(path),
                PRIMARY KEY("id" AUTOINCREMENT)
            );"#,
        ).await?;

        // Image Link
        conn.execute(
            r#"CREATE TABLE "image_link" (
                "image_id"    INTEGER NOT NULL,

                "link_id"     INTEGER NOT NULL,
                "type_of"     INTEGER NOT NULL,

                FOREIGN KEY("image_id") REFERENCES uploaded_images("id") ON DELETE CASCADE,

                UNIQUE(image_id, link_id, type_of)
            );"#,
        ).await?;

        // Collection
        conn.execute(
            r#"CREATE TABLE "collection" (
                "id"             INTEGER NOT NULL UNIQUE,

                "member_id"      INTEGER NOT NULL,

                "name"           TEXT NOT NULL,
                "description"    TEXT,

                "thumb_url"      TEXT,

                "created_at"     DATETIME NOT NULL,
                "updated_at"     DATETIME NOT NULL,

                FOREIGN KEY("member_id") REFERENCES members("id") ON DELETE CASCADE,

                PRIMARY KEY("id" AUTOINCREMENT)
            );"#,
        ).await?;

        // Collection Item
        conn.execute(
            r#"CREATE TABLE "collection_item" (
                "collection_id"   INTEGER NOT NULL,
                "book_id"         INTEGER NOT NULL,

                FOREIGN KEY("collection_id") REFERENCES collection("id") ON DELETE CASCADE,
                FOREIGN KEY("book_id") REFERENCES book("id") ON DELETE CASCADE,

                UNIQUE(collection_id, book_id)
            );"#,
        ).await
    })).await?;

    Ok(())
}