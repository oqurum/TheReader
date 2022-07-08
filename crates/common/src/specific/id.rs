use std::{ops::Deref, fmt::{Display, self}, num::ParseIntError, str::FromStr};

#[cfg(feature = "backend")]
use rusqlite::{Result, types::{FromSql, FromSqlResult, ValueRef, ToSql, ToSqlOutput}};

use serde::{Serialize, Deserialize, Deserializer, Serializer};

use common::create_single_id;

create_single_id!(FileId);
create_single_id!(LibraryId);
create_single_id!(MetadataId);