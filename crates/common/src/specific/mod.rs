use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};

#[cfg(feature = "backend")]
use sqlx::{
    encode::IsNull,
    error::BoxDynError,
    sqlite::{SqliteArgumentValue, SqliteValueRef},
    Decode, Encode, Sqlite, Type,
};

mod edit;
pub mod filter;
mod id;
mod perms;
mod preferences;
pub mod setup;

pub use edit::*;
pub use id::*;
pub use perms::*;
pub use preferences::*;

// TODO: Place this into own file.

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum MemberAuthType {
    Invite = 0,
    External = 1,
    Passwordless = 2,
    Password = 3,
    Guest = 4,
}

impl MemberAuthType {
    pub fn is_invited(self) -> bool {
        matches!(self, Self::Invite)
    }

    pub fn is_guest(self) -> bool {
        matches!(self, Self::Guest)
    }
}

// Used for DB
#[cfg(feature = "backend")]
impl From<i64> for MemberAuthType {
    fn from(value: i64) -> Self {
        Self::try_from(value as u8).unwrap()
    }
}

#[cfg(feature = "backend")]
impl<'q> Encode<'q, Sqlite> for MemberAuthType {
    fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'q>>) -> IsNull {
        args.push(SqliteArgumentValue::Int(u8::from(*self) as i32));

        IsNull::No
    }
}

#[cfg(feature = "backend")]
impl<'r> Decode<'r, Sqlite> for MemberAuthType {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        Ok(Self::try_from(u8::decode(value)?).unwrap())
    }
}

#[cfg(feature = "backend")]
impl Type<Sqlite> for MemberAuthType {
    fn type_info() -> <Sqlite as sqlx::Database>::TypeInfo {
        <i32 as Type<Sqlite>>::type_info()
    }
}
