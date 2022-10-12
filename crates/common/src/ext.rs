use std::borrow::Cow;

use common::{ThumbnailStore, MISSING_THUMB_PATH};

pub trait ThumbnailStoreExt {
    fn get_book_http_path(&self) -> Cow<str>;
}

impl ThumbnailStoreExt for ThumbnailStore {
    fn get_book_http_path(&self) -> Cow<str> {
        match self {
            ThumbnailStore::Path(path) => Cow::Owned(format!("/api/image/{path}")),
            ThumbnailStore::None => Cow::Borrowed(MISSING_THUMB_PATH),
        }
    }
}
