CREATE TABLE library (
    id INTEGER NOT NULL UNIQUE,

    name TEXT NOT NULL UNIQUE,
    type_of INT NOT NULL,

    is_public BOOLEAN NOT NULL,
    settings TEXT,

    scanned_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,

    PRIMARY KEY("id" AUTOINCREMENT)
);

CREATE TABLE directory
(
    library_id INTEGER NOT NULL REFERENCES library(id) ON DELETE CASCADE,
    path TEXT NOT NULL UNIQUE
);

CREATE TABLE file (
    id INTEGER NOT NULL UNIQUE,

    path TEXT NOT NULL UNIQUE,
    file_name TEXT NOT NULL,
    file_type TEXT NOT NULL,
    file_size INTEGER NOT NULL,

    library_id INTEGER NOT NULL,
    book_id INTEGER REFERENCES book(id) ON DELETE CASCADE,
    chapter_count INTEGER NOT NULL,

    identifier TEXT,
    hash TEXT NOT NULL UNIQUE,

    modified_at DATETIME NOT NULL,
    accessed_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL,
    deleted_at DATETIME,

    PRIMARY KEY("id" AUTOINCREMENT)
);

CREATE TABLE book (
    id INTEGER NOT NULL,

    library_id INTEGER NOT NULL REFERENCES library(id) ON DELETE CASCADE,

    type_of INT NOT NULL,

    parent_id INTEGER REFERENCES book(id) ON DELETE CASCADE,

    source TEXT NOT NULL,
    file_item_count INTEGER NOT NULL,
    title TEXT,
    original_title TEXT,
    description TEXT,
    rating FLOAT NOT NULL,
    thumb_url TEXT,

    cached TEXT NOT NULL,
    "index" INTEGER,

    available_at DATETIME,
    year INTEGER,

    refreshed_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    deleted_at DATETIME,

    PRIMARY KEY("id" AUTOINCREMENT)
);

CREATE TABLE book_person
(
    book_id INTEGER NOT NULL REFERENCES book(id) ON DELETE CASCADE,
    person_id INTEGER NOT NULL REFERENCES tag_person(id) ON DELETE CASCADE,

    UNIQUE(book_id, person_id)
);

CREATE TABLE file_note
(
    file_id INTEGER NOT NULL REFERENCES file(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES members(id) ON DELETE CASCADE,

    data TEXT NOT NULL,
    data_size INTEGER NOT NULL,

    updated_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL,

    UNIQUE(file_id, user_id)
);

CREATE TABLE file_progression
(
    book_id INTEGER NOT NULL REFERENCES book(id) ON DELETE CASCADE,
    file_id INTEGER NOT NULL REFERENCES file(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES members(id) ON DELETE CASCADE,

    type_of INTEGER NOT NULL,

    chapter INTEGER,
    page INTEGER,
    char_pos INTEGER,
    seek_pos INTEGER,

    updated_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL,

    UNIQUE(book_id, user_id)
);

CREATE TABLE file_notation
(
    file_id INTEGER NOT NULL REFERENCES file(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES members(id) ON DELETE CASCADE,

    data TEXT NOT NULL,
    data_size INTEGER NOT NULL,
    version INTEGER NOT NULL,

    updated_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL,

    UNIQUE(file_id, user_id)
);

CREATE TABLE tag_person
(
    id INTEGER NOT NULL,

    source TEXT NOT NULL,

    name TEXT NOT NULL COLLATE NOCASE,
    description TEXT,
    birth_date TEXT,

    thumb_url TEXT,

    updated_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL,

    PRIMARY KEY("id" AUTOINCREMENT)
);

CREATE TABLE tag_person_alt
(
    person_id INTEGER NOT NULL REFERENCES tag_person(id) ON DELETE CASCADE,

    name TEXT NOT NULL
    COLLATE NOCASE,

    UNIQUE(person_id, name)
);

CREATE TABLE members
(
    id INTEGER NOT NULL,

    name TEXT NOT NULL COLLATE NOCASE,
    email TEXT NOT NULL COLLATE NOCASE,
    password TEXT,

    type_of INTEGER NOT NULL,

    permissions INTEGER NOT NULL,

    library_access TEXT,

    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,

    UNIQUE(email),
    PRIMARY KEY("id" AUTOINCREMENT)
);

CREATE TABLE auth
(
    oauth_token TEXT UNIQUE,
    oauth_token_secret TEXT NOT NULL UNIQUE,

    member_id INTEGER REFERENCES members(id) ON DELETE CASCADE,

    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

CREATE TABLE client (
    id INTEGER NOT NULL,

    oauth INTEGER NOT NULL REFERENCES auth(oauth_token_secret) ON DELETE CASCADE,

    identifier TEXT NOT NULL UNIQUE,

    client TEXT NOT NULL,
    device TEXT NOT NULL,
    platform TEXT,

    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,

    PRIMARY KEY("id" AUTOINCREMENT)
);

CREATE TABLE uploaded_images (
    id INTEGER NOT NULL,

    path TEXT NOT NULL,
    created_at DATETIME NOT NULL,

    UNIQUE(path),
    PRIMARY KEY("id" AUTOINCREMENT)
);

CREATE TABLE image_link
(
    image_id INTEGER NOT NULL REFERENCES uploaded_images(id) ON DELETE CASCADE,

    link_id INTEGER NOT NULL,
    type_of INTEGER NOT NULL,

    UNIQUE(image_id, link_id, type_of)
);

CREATE TABLE collection (
    id INTEGER NOT NULL UNIQUE,

    member_id INTEGER NOT NULL REFERENCES members(id) ON DELETE CASCADE,

    name TEXT NOT NULL,
    description TEXT,

    thumb_url TEXT,

    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,

    PRIMARY KEY("id" AUTOINCREMENT)
);

CREATE TABLE collection_item
(
    collection_id INTEGER NOT NULL REFERENCES collection(id) ON DELETE CASCADE,
    book_id INTEGER NOT NULL REFERENCES book(id) ON DELETE CASCADE,

    UNIQUE(collection_id, book_id)
);