use chrono::{Utc, DateTime, TimeZone};
use common::{Source, ThumbnailStore};
use rusqlite::{params, OptionalExtension};

use books_common::{LibraryId, MetadataId, MetadataItemCached, DisplayMetaItem, util::{serialize_datetime, serialize_datetime_opt}};
use serde::Serialize;
use crate::{Result, database::Database};

use super::{TableRow, AdvRow};




#[derive(Debug, Clone, Serialize)]
pub struct MetadataModel {
	pub id: MetadataId,

	pub library_id: LibraryId,

	pub source: Source,
	pub file_item_count: i64,
	pub title: Option<String>,
	pub original_title: Option<String>,
	pub description: Option<String>,
	pub rating: f64,

	pub thumb_path: ThumbnailStore,
	pub all_thumb_urls: Vec<String>,

	// TODO: Make table for all tags. Include publisher in it. Remove country.
	pub cached: MetadataItemCached,

	#[serde(serialize_with = "serialize_datetime")]
	pub refreshed_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub created_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub updated_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime_opt")]
	pub deleted_at: Option<DateTime<Utc>>,

	pub available_at: Option<i64>,
	pub year: Option<i64>,

	pub hash: String
}


impl From<MetadataModel> for DisplayMetaItem {
	fn from(val: MetadataModel) -> Self {
		DisplayMetaItem {
			id: val.id,
			library_id: val.library_id,
			source: val.source,
			file_item_count: val.file_item_count,
			title: val.title,
			original_title: val.original_title,
			description: val.description,
			rating: val.rating,
			thumb_path: val.thumb_path,
			cached: val.cached,
			refreshed_at: val.refreshed_at,
			created_at: val.created_at,
			updated_at: val.updated_at,
			deleted_at: val.deleted_at,
			available_at: val.available_at,
			year: val.year,
			hash: val.hash,
		}
	}
}


impl TableRow<'_> for MetadataModel {
	fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
		Ok(Self {
			id: row.next()?,
			library_id: row.next()?,
			source: Source::try_from(row.next::<String>()?).unwrap(),
			file_item_count: row.next()?,
			title: row.next()?,
			original_title: row.next()?,
			description: row.next()?,
			rating: row.next()?,
			thumb_path: ThumbnailStore::from(row.next_opt::<String>()?),
			all_thumb_urls: Vec::new(),
			cached: row.next_opt::<String>()?
				.map(|v| MetadataItemCached::from_string(&v))
				.unwrap_or_default(),
			available_at: row.next()?,
			year: row.next()?,
			refreshed_at: Utc.timestamp_millis(row.next()?),
			created_at: Utc.timestamp_millis(row.next()?),
			updated_at: Utc.timestamp_millis(row.next()?),
			deleted_at: row.next_opt()?.map(|v| Utc.timestamp_millis(v)),
			hash: row.next()?
		})
	}
}


impl MetadataModel {
	pub fn add_or_increment(&self, db: &Database) -> Result<MetadataModel> {
		let table_meta = if self.id != 0 {
			Self::get_by_id(self.id, db)?
		} else {
			Self::get_by_source(&self.source, db)?
		};

		if table_meta.is_none() {
			db.write()?
			.execute(r#"
				INSERT INTO metadata_item (
					library_id, source, file_item_count,
					title, original_title, description, rating, thumb_url,
					cached,
					available_at, year,
					refreshed_at, created_at, updated_at, deleted_at,
					hash
				)
				VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"#,
				params![
					self.library_id, self.source.to_string(), &self.file_item_count,
					&self.title, &self.original_title, &self.description, &self.rating, self.thumb_path.to_optional_string(),
					&self.cached.as_string_optional(),
					&self.available_at, &self.year,
					&self.refreshed_at.timestamp_millis(), &self.created_at.timestamp_millis(), &self.updated_at.timestamp_millis(),
					self.deleted_at.as_ref().map(|v| v.timestamp_millis()),
					&self.hash
				]
			)?;

			return Ok(Self::get_by_source(&self.source, db)?.unwrap());
		} else if self.id != 0 {
			db.write()?
			.execute(r#"UPDATE metadata_item SET file_item_count = file_item_count + 1 WHERE id = ?1"#,
				params![self.id]
			)?;
		} else {
			db.write()?
			.execute(r#"UPDATE metadata_item SET file_item_count = file_item_count + 1 WHERE source = ?1"#,
				params![self.source.to_string()]
			)?;
		}

		Ok(table_meta.unwrap())
	}

	pub fn update(&self, db: &Database) -> Result<()> {
		db.write()?
		.execute(r#"
			UPDATE metadata_item SET
				library_id = ?2, source = ?3, file_item_count = ?4,
				title = ?5, original_title = ?6, description = ?7, rating = ?8, thumb_url = ?9,
				cached = ?10,
				available_at = ?11, year = ?12,
				refreshed_at = ?13, created_at = ?14, updated_at = ?15, deleted_at = ?16,
				hash = ?17
			WHERE id = ?1"#,
			params![
				self.id,
				self.library_id, self.source.to_string(), &self.file_item_count,
				&self.title, &self.original_title, &self.description, &self.rating, self.thumb_path.to_optional_string(),
				&self.cached.as_string_optional(),
				&self.available_at, &self.year,
				&self.refreshed_at.timestamp_millis(), &self.created_at.timestamp_millis(), &self.updated_at.timestamp_millis(),
				self.deleted_at.as_ref().map(|v| v.timestamp_millis()),
				&self.hash
			]
		)?;

		Ok(())
	}

	pub fn decrement_or_remove(id: MetadataId, db: &Database) -> Result<()> {
		if let Some(meta) = Self::get_by_id(id, db)? {
			if meta.file_item_count < 1 {
				db.write()?
				.execute(
					r#"UPDATE metadata_item SET file_item_count = file_item_count - 1 WHERE id = ?1"#,
					params![id]
				)?;
			} else {
				db.write()?
				.execute(
					r#"DELETE FROM metadata_item WHERE id = ?1"#,
					params![id]
				)?;
			}
		}

		Ok(())
	}

	pub fn decrement(id: MetadataId, db: &Database) -> Result<()> {
		if let Some(meta) = Self::get_by_id(id, db)? {
			if meta.file_item_count > 0 {
				db.write()?
				.execute(
					r#"UPDATE metadata_item SET file_item_count = file_item_count - 1 WHERE id = ?1"#,
					params![id]
				)?;
			}
		}

		Ok(())
	}

	pub fn set_file_count(id: MetadataId, file_count: usize, db: &Database) -> Result<()> {
		db.write()?
		.execute(
			r#"UPDATE metadata_item SET file_item_count = ?2 WHERE id = ?1"#,
			params![id, file_count]
		)?;

		Ok(())
	}

	// TODO: Change to get_metadata_by_hash. We shouldn't get metadata by source. Local metadata could be different with the same source id.
	pub fn get_by_source(source: &Source, db: &Database) -> Result<Option<MetadataModel>> {
		Ok(db.read()?.query_row(
			r#"SELECT * FROM metadata_item WHERE source = ?1 LIMIT 1"#,
			params![source.to_string()],
			|v| MetadataModel::from_row(v)
		).optional()?)
	}

	pub fn get_by_id(id: MetadataId, db: &Database) -> Result<Option<MetadataModel>> {
		Ok(db.read()?.query_row(
			r#"SELECT * FROM metadata_item WHERE id = ?1 LIMIT 1"#,
			params![id],
			|v| MetadataModel::from_row(v)
		).optional()?)
	}

	pub fn remove_by_id(id: MetadataId, db: &Database) -> Result<usize> {
		Ok(db.write()?.execute(
			r#"DELETE FROM metadata_item WHERE id = ?1"#,
			params![id]
		)?)
	}

	pub fn get_list_by(library: Option<LibraryId>, offset: usize, limit: usize, db: &Database) -> Result<Vec<MetadataModel>> {
		let this = db.read()?;

		let lib_where = library.map(|v| format!("WHERE library_id={v}")).unwrap_or_default();

		let mut conn = this.prepare(&format!(r#"SELECT * FROM metadata_item {} LIMIT ?1 OFFSET ?2"#, lib_where))?;

		let map = conn.query_map([limit, offset], |v| MetadataModel::from_row(v))?;

		Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
	}
}