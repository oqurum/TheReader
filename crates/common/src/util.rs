use chrono::{DateTime, Utc, TimeZone};
use serde::{Serializer, Deserializer, Deserialize};



pub fn serialize_datetime<S>(value: &DateTime<Utc>, s: S) -> std::result::Result<S::Ok, S::Error> where S: Serializer {
	s.serialize_i64(value.timestamp_millis())
}

pub fn serialize_datetime_opt<S>(value: &Option<DateTime<Utc>>, s: S) -> std::result::Result<S::Ok, S::Error> where S: Serializer {
	match value {
		Some(v) => s.serialize_i64(v.timestamp_millis()),
		None => s.serialize_none()
	}
}


pub fn deserialize_datetime<'de, D>(value: D) -> std::result::Result<DateTime<Utc>, D::Error> where D: Deserializer<'de> {
	Ok(Utc.timestamp_millis(i64::deserialize(value)?))
}

pub fn deserialize_datetime_opt<'de, D>(value: D) -> std::result::Result<Option<DateTime<Utc>>, D::Error> where D: Deserializer<'de> {
	if let Some(v) = Option::<i64>::deserialize(value)? {
		Ok(Some(Utc.timestamp_millis(v)))
	} else {
		Ok(None)
	}
}