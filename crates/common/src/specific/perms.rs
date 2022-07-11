use bitflags::bitflags;
use serde::{Serialize, Deserialize, Deserializer, Serializer};

#[cfg(feature = "backend")]
use rusqlite::{Result, types::{FromSql, FromSqlResult, ValueRef, ToSql, ToSqlOutput}};


bitflags! {
	#[derive(Serialize, Deserialize)]
	pub struct GroupPermissions: u64 {
		const OWNER 			= 1 << 0;
		const BASIC 			= 1 << 1;
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
impl FromSql for Permissions {
	#[inline]
	fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
		let val = String::column_result(value)?;

		Ok(Self {
			group: GroupPermissions { bits: val.parse().unwrap() },
		})
	}
}

#[cfg(feature = "backend")]
impl ToSql for Permissions {
	#[inline]
	fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
		Ok((self.group.bits as i64).into())
	}
}


impl<'de> Deserialize<'de> for Permissions {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
		Ok(Self {
			group: GroupPermissions { bits: u64::deserialize(deserializer)? },
		})
	}
}

impl Serialize for Permissions {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
		self.group.bits.serialize(serializer)
	}
}