use std::{
    fmt::{self, Display},
    num::ParseIntError,
    ops::Deref,
    str::FromStr,
};

#[cfg(feature = "backend")]
use rusqlite::{
    types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Result,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use common::create_single_id;

create_single_id!(FileId);
create_single_id!(LibraryId);
create_single_id!(CollectionId);
