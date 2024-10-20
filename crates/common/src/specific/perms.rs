use bitflags::bitflags;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "backend")]
use sqlx::{
    encode::IsNull,
    error::BoxDynError,
    sqlite::{SqliteArgumentValue, SqliteValueRef},
    Decode, Encode, Sqlite, Type,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GroupPermissions(i64);

bitflags! {
    impl GroupPermissions: i64 {
        const OWNER = 1 << 0;
        const BASIC = 1 << 1;
        const GUEST = 1 << 2;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Permissions {
    pub group: GroupPermissions,
    // TODO: Specifics.
}

impl Permissions {
    pub fn empty() -> Self {
        Self {
            group: GroupPermissions::empty(),
        }
    }

    pub fn guest() -> Self {
        Self {
            group: GroupPermissions::GUEST,
        }
    }

    pub fn basic() -> Self {
        Self {
            group: GroupPermissions::BASIC,
        }
    }

    pub fn owner() -> Self {
        Self {
            group: GroupPermissions::OWNER,
        }
    }

    /// Returns true if all of the flags in other are contained within self.
    pub fn contains_group(self, value: GroupPermissions) -> bool {
        self.group.contains(value)
    }

    /// Returns true if there are flags common to both self and other.
    pub fn intersects_group(self, value: GroupPermissions) -> bool {
        self.group.intersects(value)
    }

    // Custom

    pub fn is_owner(self) -> bool {
        self.contains_group(GroupPermissions::OWNER)
    }

    pub fn is_basic(self) -> bool {
        self.contains_group(GroupPermissions::BASIC)
    }
}

#[cfg(feature = "backend")]
impl From<i64> for Permissions {
    fn from(value: i64) -> Self {
        Self {
            group: GroupPermissions(value),
        }
    }
}

#[cfg(feature = "backend")]
impl<'q> Encode<'q, Sqlite> for Permissions {
    fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'q>>) -> IsNull {
        args.push(SqliteArgumentValue::Int64(self.group.0));

        IsNull::No
    }
}

#[cfg(feature = "backend")]
impl<'r> Decode<'r, Sqlite> for Permissions {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let val = <i64 as Decode<'r, Sqlite>>::decode(value)?;

        Ok(Self {
            group: GroupPermissions(val),
        })
    }
}

#[cfg(feature = "backend")]
impl Type<Sqlite> for Permissions {
    fn type_info() -> <Sqlite as sqlx::Database>::TypeInfo {
        // TODO: Why does it require i32 instead of i64
        <i32 as Type<Sqlite>>::type_info()
    }
}

impl<'de> Deserialize<'de> for Permissions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self {
            group: GroupPermissions(i64::deserialize(deserializer)?),
        })
    }
}

impl Serialize for Permissions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.group.0.serialize(serializer)
    }
}
