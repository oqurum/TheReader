use crate::{database::Database, Result};

pub async fn init(database: &Database) -> Result<()> {
    let conn = database.write().await;

    // TODO: Migrations https://github.com/rusqlite/rusqlite/discussions/1117

    // Migrations
    conn.execute(
        r#"CREATE TABLE migration (
            id          INT NOT NULL,

            title       TEXT NOT NULL,
            duration    INT NOT NULL,
            notes       TEXT NOT NULL,

            created_at  TEXT NOT NULL
        );"#,
        [],
    )?;

    // Library
    conn.execute(
        r#"CREATE TABLE "library" (
            "id"                 INTEGER NOT NULL UNIQUE,

            "name"               TEXT UNIQUE,
            "type_of"            INT NOT NULL,

            "scanned_at"         TEXT NOT NULL,
            "created_at"         TEXT NOT NULL,
            "updated_at"         TEXT NOT NULL,

            PRIMARY KEY("id" AUTOINCREMENT)
        );"#,
        [],
    )?;

    // Directory
    conn.execute(
        r#"CREATE TABLE "directory" (
            "library_id"    INTEGER NOT NULL,
            "path"          TEXT NOT NULL UNIQUE,

            FOREIGN KEY("library_id") REFERENCES library("id") ON DELETE CASCADE
        );"#,
        [],
    )?;

    // File
    conn.execute(
        r#"CREATE TABLE "file" (
            "id"               INTEGER NOT NULL UNIQUE,

            "path"             TEXT NOT NULL UNIQUE,
            "file_name"        TEXT NOT NULL,
            "file_type"        TEXT,
            "file_size"        INTEGER NOT NULL,

            "library_id"       INTEGER,
            "book_id"          INTEGER,
            "chapter_count"    INTEGER,

            "identifier"       TEXT,
            "hash"             TEXT NOT NULL UNIQUE,

            "modified_at"      TEXT NOT NULL,
            "accessed_at"      TEXT NOT NULL,
            "created_at"       TEXT NOT NULL,
            "deleted_at"       TEXT,

            PRIMARY KEY("id" AUTOINCREMENT),

            FOREIGN KEY("book_id") REFERENCES book("id") ON DELETE CASCADE
        );"#,
        [],
    )?;

    // Book Item
    conn.execute(
        r#"CREATE TABLE "book" (
            "id"                  INTEGER NOT NULL,

            "library_id"          INTEGER,

            "type_of"             INT NOT NULL,

            "parent_id"           INTEGER REFERENCES book("id") ON DELETE CASCADE,

            "source"              TEXT,
            "file_item_count"     INTEGER,
            "title"               TEXT,
            "original_title"      TEXT,
            "description"         TEXT,
            "rating"              FLOAT,
            "thumb_url"           TEXT,

            "cached"              TEXT,
            "index"               INTEGER,

            "available_at"        TEXT,
            "year"                INTEGER,

            "refreshed_at"        TEXT,
            "created_at"          TEXT,
            "updated_at"          TEXT,
            "deleted_at"          TEXT,

            PRIMARY KEY("id" AUTOINCREMENT),

            FOREIGN KEY("library_id") REFERENCES library("id") ON DELETE CASCADE
        );"#,
        [],
    )?;

    // Book People
    conn.execute(
        r#"CREATE TABLE "book_person" (
            "book_id"    INTEGER NOT NULL,
            "person_id"      INTEGER NOT NULL,

            FOREIGN KEY("book_id") REFERENCES book("id") ON DELETE CASCADE,
        	FOREIGN KEY("person_id") REFERENCES tag_person("id") ON DELETE CASCADE,

            UNIQUE(book_id, person_id)
        );"#,
        [],
    )?;

    // TODO: Versionize Notes. Keep last 20 versions for X one month. Auto delete old versions.
    // File Note
    conn.execute(
        r#"CREATE TABLE "file_note" (
            "file_id"       INTEGER NOT NULL,
            "user_id"       INTEGER NOT NULL,

            "data"          TEXT NOT NULL,
            "data_size"     INTEGER NOT NULL,

            "updated_at"    TEXT NOT NULL,
            "created_at"    TEXT NOT NULL,

            FOREIGN KEY("user_id") REFERENCES members("id") ON DELETE CASCADE,
        	FOREIGN KEY("file_id") REFERENCES file("id") ON DELETE CASCADE,

            UNIQUE(file_id, user_id)
        );"#,
        [],
    )?;

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

            "updated_at"    TEXT NOT NULL,
            "created_at"    TEXT NOT NULL,

            FOREIGN KEY("user_id") REFERENCES members("id") ON DELETE CASCADE,
        	FOREIGN KEY("file_id") REFERENCES file("id") ON DELETE CASCADE,
        	FOREIGN KEY("book_id") REFERENCES book("id") ON DELETE CASCADE,

            UNIQUE(book_id, user_id)
        );"#,
        [],
    )?;

    // File Notation
    conn.execute(
        r#"CREATE TABLE "file_notation" (
            "file_id"       INTEGER NOT NULL,
            "user_id"       INTEGER NOT NULL,

            "data"          TEXT NOT NULL,
            "data_size"     INTEGER NOT NULL,
            "version"       INTEGER NOT NULL,

            "updated_at"    TEXT NOT NULL,
            "created_at"    TEXT NOT NULL,

            FOREIGN KEY("user_id") REFERENCES members("id") ON DELETE CASCADE,
        	FOREIGN KEY("file_id") REFERENCES file("id") ON DELETE CASCADE,

            UNIQUE(file_id, user_id)
        );"#,
        [],
    )?;

    // Tags People
    conn.execute(
        r#"CREATE TABLE "tag_person" (
            "id"             INTEGER NOT NULL,

            "source"         TEXT NOT NULL,

            "name"           TEXT NOT NULL COLLATE NOCASE,
            "description"    TEXT,
            "birth_date"     TEXT,

            "thumb_url"      TEXT,

            "updated_at"     TEXT NOT NULL,
            "created_at"     TEXT NOT NULL,

            PRIMARY KEY("id" AUTOINCREMENT)
        );"#,
        [],
    )?;

    // People Alt names
    conn.execute(
        r#"CREATE TABLE "tag_person_alt" (
            "person_id"    INTEGER NOT NULL,

            "name"         TEXT NOT NULL COLLATE NOCASE,

            FOREIGN KEY("person_id") REFERENCES tag_person("id") ON DELETE CASCADE,

            UNIQUE(person_id, name)
        );"#,
        [],
    )?;

    // Members
    conn.execute(
        r#"CREATE TABLE "members" (
            "id"             INTEGER NOT NULL,

            "name"           TEXT NOT NULL COLLATE NOCASE,
            "email"          TEXT COLLATE NOCASE,
            "password"       TEXT,

            "type_of"        INTEGER NOT NULL,

            "permissions"    TEXT NOT NULL,
            "preferences"    TEXT,

            "created_at"     TEXT NOT NULL,
            "updated_at"     TEXT NOT NULL,

            UNIQUE(email),
            PRIMARY KEY("id" AUTOINCREMENT)
        );"#,
        [],
    )?;

    // Auth
    conn.execute(
        r#"CREATE TABLE "auth" (
            "oauth_token"           TEXT UNIQUE,
            "oauth_token_secret"    TEXT NOT NULL UNIQUE,

            "member_id"             INTEGER,

            "created_at"            TEXT NOT NULL,
            "updated_at"            TEXT NOT NULL,

            FOREIGN KEY("member_id") REFERENCES members("id") ON DELETE CASCADE
        );"#,
        [],
    )?;

    // Uploaded Images
    conn.execute(
        r#"CREATE TABLE "uploaded_images" (
            "id"            INTEGER NOT NULL,

            "path"          TEXT NOT NULL,

            "created_at"    TEXT NOT NULL,

            UNIQUE(path),
            PRIMARY KEY("id" AUTOINCREMENT)
        );"#,
        [],
    )?;

    // Image Link
    conn.execute(
        r#"CREATE TABLE "image_link" (
            "image_id"    INTEGER NOT NULL,

            "link_id"     INTEGER NOT NULL,
            "type_of"     INTEGER NOT NULL,

            FOREIGN KEY("image_id") REFERENCES uploaded_images("id") ON DELETE CASCADE,

            UNIQUE(image_id, link_id, type_of)
        );"#,
        [],
    )?;

    // Collection
    conn.execute(
        r#"CREATE TABLE "collection" (
            "id"             INTEGER NOT NULL UNIQUE,

            "member_id"      INTEGER NOT NULL,

            "name"           TEXT NOT NULL,
            "description"    TEXT,

            "thumb_url"      TEXT,

            "created_at"     TEXT NOT NULL,
            "updated_at"     TEXT NOT NULL,

            FOREIGN KEY("member_id") REFERENCES members("id") ON DELETE CASCADE,

            PRIMARY KEY("id" AUTOINCREMENT)
        );"#,
        [],
    )?;

    // Collection Item
    conn.execute(
        r#"CREATE TABLE "collection_item" (
            "collection_id"   INTEGER NOT NULL,
            "book_id"         INTEGER NOT NULL,

            FOREIGN KEY("collection_id") REFERENCES collection("id") ON DELETE CASCADE,
        	FOREIGN KEY("book_id") REFERENCES book("id") ON DELETE CASCADE,

            UNIQUE(collection_id, book_id)
        );"#,
        [],
    )?;

    Ok(())
}
