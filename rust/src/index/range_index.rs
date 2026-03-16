use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::DbResult;
use crate::storage::codec;

/// A range index using BTreeMap for ordered lookups (>, <, >=, <=, between).
/// Maps field values (as sortable strings) to lists of document positions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeIndex {
    pub field: String,
    pub entries: BTreeMap<String, Vec<usize>>,
}

impl RangeIndex {
    pub fn new(field: &str) -> Self {
        RangeIndex {
            field: field.to_string(),
            entries: BTreeMap::new(),
        }
    }

    /// Rebuild the index from scratch given a set of documents.
    pub fn rebuild(&mut self, docs: &[Value]) {
        self.entries.clear();
        for (i, doc) in docs.iter().enumerate() {
            if let Some(val) = doc.get(&self.field) {
                let key = value_to_sortable_key(val);
                self.entries.entry(key).or_default().push(i);
            }
        }
    }

    /// Look up document positions where field > value.
    pub fn greater_than(&self, value: &Value) -> Vec<usize> {
        let key = value_to_sortable_key(value);
        self.entries
            .range::<String, _>((std::ops::Bound::Excluded(&key), std::ops::Bound::Unbounded))
            .flat_map(|(_, positions)| positions.iter().copied())
            .collect()
    }

    /// Look up document positions where field >= value.
    pub fn greater_than_or_equal(&self, value: &Value) -> Vec<usize> {
        let key = value_to_sortable_key(value);
        self.entries
            .range::<String, _>((std::ops::Bound::Included(&key), std::ops::Bound::Unbounded))
            .flat_map(|(_, positions)| positions.iter().copied())
            .collect()
    }

    /// Look up document positions where field < value.
    pub fn less_than(&self, value: &Value) -> Vec<usize> {
        let key = value_to_sortable_key(value);
        self.entries
            .range::<String, _>((std::ops::Bound::Unbounded, std::ops::Bound::Excluded(&key)))
            .flat_map(|(_, positions)| positions.iter().copied())
            .collect()
    }

    /// Look up document positions where field <= value.
    pub fn less_than_or_equal(&self, value: &Value) -> Vec<usize> {
        let key = value_to_sortable_key(value);
        self.entries
            .range::<String, _>((std::ops::Bound::Unbounded, std::ops::Bound::Included(&key)))
            .flat_map(|(_, positions)| positions.iter().copied())
            .collect()
    }

    /// Look up document positions where field is between min and max (inclusive).
    pub fn between(&self, min: &Value, max: &Value) -> Vec<usize> {
        let min_key = value_to_sortable_key(min);
        let max_key = value_to_sortable_key(max);
        self.entries
            .range::<String, _>((
                std::ops::Bound::Included(&min_key),
                std::ops::Bound::Included(&max_key),
            ))
            .flat_map(|(_, positions)| positions.iter().copied())
            .collect()
    }

    /// Look up document positions by exact field value (equality).
    pub fn lookup(&self, value: &Value) -> Vec<usize> {
        let key = value_to_sortable_key(value);
        self.entries.get(&key).cloned().unwrap_or_default()
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
        let idx: RangeIndex = serde_json::from_slice(&decoded)?;
        Ok(Some(idx))
    }

    /// Delete the index file from disk.
    pub fn delete_file(data_path: &str, collection: &str, field: &str) -> DbResult<()> {
        let path = index_path(data_path, collection, field);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

fn index_path(data_path: &str, collection: &str, field: &str) -> std::path::PathBuf {
    Path::new(data_path)
        .join("indexes")
        .join(format!("{}_{}.ridx.anvil", collection, field))
}

/// Public accessor for `value_to_sortable_key` (used by Collection for incremental index updates).
pub fn value_to_sortable_key_pub(val: &Value) -> String {
    value_to_sortable_key(val)
}

/// Convert a JSON value to a sortable string key.
/// Numbers are zero-padded to ensure correct lexicographic ordering.
fn value_to_sortable_key(val: &Value) -> String {
    match val {
        Value::Number(n) => {
            let f = n.as_f64().unwrap_or(0.0);
            // Encode as sortable string: shift by large offset to handle negatives
            // Format: sign + zero-padded integer part + decimal part
            if f >= 0.0 {
                format!("n:1:{:020.10}", f)
            } else {
                // For negatives, invert so that -100 < -1 in lexicographic order
                format!("n:0:{:020.10}", f + 10_000_000_000.0)
            }
        }
        Value::String(s) => format!("s:{}", s),
        Value::Bool(b) => format!("b:{}", *b as u8),
        Value::Null => "null".to_string(),
        other => format!("j:{}", other),
    }
}
