//! An In Memory Database used for caching Auths.

use std::collections::{HashMap, hash_map::Entry};

use chrono::{Utc, DateTime, Duration};
use lazy_static::lazy_static;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::RwLock;

use crate::Result;

lazy_static! {
    pub static ref IN_MEM_DB: ImD = ImD::default();
}

#[derive(Default)]
pub struct ImD {
    store: RwLock<HashMap<String, ImdItem>>
}

impl ImD {
    pub async fn read_value<V: DeserializeOwned>(&self, name: &str) -> Result<Option<V>> {
        let read = self.store.read().await;

        let Some(item) = read.get(name) else {
            return Ok(None);
        };

        let Some(value) = item.value.as_deref() else {
            return Ok(None);
        };

        Ok(Some(serde_json::from_str(value)?))
    }

    /// Returns a boolean on whether or not we wrote a NEW value.
    pub async fn write_value<S: Serialize>(&self, name: String, value: S) -> Result<bool> {
        self._write_entry(name, Some(value), None).await
    }

    /// Returns a boolean on whether or not we wrote a NEW value.
    pub async fn write_value_duration<S: Serialize>(&self, name: String, value: S, valid_for: Duration) -> Result<bool> {
        self._write_entry(name, Some(value), Some(Utc::now().checked_add_signed(valid_for).unwrap())).await
    }

    /// Returns a boolean on whether or not we wrote a NEW value.
    pub async fn write_item(&self, name: String) -> Result<bool> {
        self._write_entry(name, Option::<()>::None, None).await
    }

    /// Returns a boolean on whether or not we wrote a NEW value.
    pub async fn write_item_duration(&self, name: String, valid_for: Duration) -> Result<bool> {
        self._write_entry(name, Option::<()>::None, Some(Utc::now().checked_add_signed(valid_for).unwrap())).await
    }

    pub async fn contains(&self, name: &str) -> bool {
        self.store.read().await.contains_key(name)
    }

    pub async fn delete(&self, name: &str) -> bool {
        self.store.write().await.remove(name).is_some()
    }

    /// Returns a boolean on whether or not we wrote a NEW value.
    async fn _write_entry(&self, name: String, value: Option<impl Serialize>, delete_after: Option<DateTime<Utc>>) -> Result<bool> {
        let mut write = self.store.write().await;

        match write.entry(name) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().value = Some(serde_json::to_string(&value)?);
                entry.get_mut().delete_after = delete_after;

                Ok(false)
            }

            Entry::Vacant(entry) => {
                entry.insert(ImdItem {
                    value: Some(serde_json::to_string(&value)?),
                    delete_after,
                });

                Ok(true)
            }
        }
    }
}

struct ImdItem {
    value: Option<String>,
    delete_after: Option<DateTime<Utc>>,
}