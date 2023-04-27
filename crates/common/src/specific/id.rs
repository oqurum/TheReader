use std::{
    fmt::{self, Display},
    num::ParseIntError,
    ops::Deref,
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use common::create_single_id;

create_single_id!(FileId);
create_single_id!(LibraryId);
create_single_id!(CollectionId);
