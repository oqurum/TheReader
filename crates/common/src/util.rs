use chrono::{DateTime, Utc, TimeZone};
use serde::{Serializer, Deserializer, Deserialize};


pub const FILE_SIZE_IDENTIFIERS: [&str; 4] = ["B", "KB", "MB", "GB"];





pub fn file_size_bytes_to_readable_string(value: i64) -> String {
    let mut size = value as f64;

    // 1024

    let mut index = 0;
    while size > 1024.0 && index != 3 {
        size /= 1024.0;
        index += 1;
    }

    if index + 1 == FILE_SIZE_IDENTIFIERS.len() {
        format!("{}{}", (size * 100.0).floor() / 100.0, FILE_SIZE_IDENTIFIERS[index])
    } else {
        format!("{}{}", size.floor(), FILE_SIZE_IDENTIFIERS[index])
    }

}


pub fn take_from_and_swap<V, P: Fn(&V) -> bool>(array: &mut Vec<V>, predicate: P) -> Vec<V> {
    let mut ret = Vec::new();

    for i in (0..array.len()).rev() {
        if predicate(&array[i]) {
            ret.push(array.swap_remove(i));
        }
    }

    ret.reverse();

    ret
}




// Serde

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