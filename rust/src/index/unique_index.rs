use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{DbError, DbResult};
use crate::index::hash_index::value_to_index_key;
use crate::storage::codec;

/// A unique index enforces that no two documents share the same value for a field.
/// Maps field values (as strings) to a single document position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueIndex {
    pub field: String,
    pub entries: HashMap<String, usize>,
}

impl UniqueIndex {
    pub fn new(field: &str) -> Self {
        UniqueIndex {
            field: field.to_string(),
            entries: HashMap::new(),
        }
    }

    /// Rebuild the index from scratch. Returns an error if duplicates are found.
    pub fn rebuild(&mut self, docs: &[Value]) -> DbResult<()> {
        self.entries.clear();
        for (i, doc) in docs.iter().enumerate() {
            if let Some(val) = doc.get(&self.field) {
                let key = value_to_index_key(val);
                if self.entries.contains_key(&key) {
                    return Err(DbError::DuplicateKey(self.field.clone(), key));
                }
                self.entries.insert(key, i);
            }
        }
        Ok(())
    }

    /// Check if inserting a value would violate uniqueness.
    pub fn check_unique(&self, value: &Value) -> DbResult<()> {
        let key = value_to_index_key(value);
        if self.entries.contains_key(&key) {
            return Err(DbError::DuplicateKey(self.field.clone(), key));
        }
        Ok(())
    }

    /// Check uniqueness allowing a specific position (for updates).
    pub fn check_unique_except(&self, value: &Value, except_pos: usize) -> DbResult<()> {
        let key = value_to_index_key(value);
        if let Some(&pos) = self.entries.get(&key) {
            if pos != except_pos {
                return Err(DbError::DuplicateKey(self.field.clone(), key));
            }
        }
        Ok(())
    }

    /// Look up a document position by exact field value.
    pub fn lookup(&self, value: &Value) -> Option<usize> {
        let key = value_to_index_key(value);
        self.entries.get(&key).copied()
    }

    /// Persist the index to disk (compressed, optionally encrypted).
    pub fn save(&self, data_path: &str, collection: &str, key: Option<&[u8; 32]>) -> DbResult<()> {
        crate::storage::file_storage::ensure_indexes_dir(data_path)?;
        let path = index_path(data_path, collection, &self.field);
        let json = serde_json::to_string(self)?;
        let encoded = codec::encode_raw(json.as_bytes(), key)?;
        fs::write(path, encoded)?;
        Ok(())
    }

    /// Load the index from disk. Returns None if file doesn't exist.
    pub fn load(data_path: &str, collection: &str, field: &str, key: Option<&[u8; 32]>) -> DbResult<Option<Self>> {
        let path = index_path(data_path, collection, field);
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read(&path)?;
        let decoded = codec::decode_raw(&data, key)?;
        let idx: UniqueIndex = serde_json::from_slice(&decoded)?;
        Ok(Some(idx))
    }

    /// Delete the index file from disk.
    pub fn delete_file(data_path: &str, collection: &str, field: &str) -> DbResult<()> {
        let path = index_path(data_path, collection, field);
        if path.exists() {
            fs::remove_file(path)?;
        }
        // Also clean up legacy .idx.json
        let legacy = Path::new(data_path)
            .join("indexes")
            .join(format!("{}_{}.idx.json", collection, field));
        if legacy.exists() {
            let _ = fs::remove_file(legacy);
        }
        Ok(())
    }
}

fn index_path(data_path: &str, collection: &str, field: &str) -> std::path::PathBuf {
    Path::new(data_path)
        .join("indexes")
        .join(format!("{}_{}.idx.anvil", collection, field))
}
