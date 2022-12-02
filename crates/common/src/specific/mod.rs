use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};

#[cfg(feature = "backend")]
use rusqlite::{
    types::{FromSql, FromSqlResult, ToSqlOutput, ValueRef},
    Result, ToSql,
};

mod edit;
pub mod filter;
mod id;
mod perms;
pub mod setup;

pub use edit::*;
pub use id::*;
pub use perms::*;

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
}

#[cfg(feature = "backend")]
impl FromSql for MemberAuthType {
    #[inline]
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(Self::try_from(u8::column_result(value)?).unwrap())
    }
}

#[cfg(feature = "backend")]
impl ToSql for MemberAuthType {
    #[inline]
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(u8::from(*self)))
    }
}
