use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::DbResult;
use crate::storage::codec;

/// A hash index maps field values (as strings) to lists of document positions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashIndex {
    pub field: String,
    pub entries: HashMap<String, Vec<usize>>,
}

impl HashIndex {
    pub fn new(field: &str) -> Self {
        HashIndex {
            field: field.to_string(),
            entries: HashMap::new(),
        }
    }

    /// Rebuild the index from scratch given a set of documents.
    pub fn rebuild(&mut self, docs: &[Value]) {
        self.entries.clear();
        for (i, doc) in docs.iter().enumerate() {
            if let Some(val) = doc.get(&self.field) {
                let key = value_to_index_key(val);
                self.entries.entry(key).or_default().push(i);
            }
        }
    }

    /// Look up document positions by exact field value.
    pub fn lookup(&self, value: &Value) -> Vec<usize> {
        let key = value_to_index_key(value);
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
        let idx: HashIndex = serde_json::from_slice(&decoded)?;
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

/// Convert a serde_json::Value to a string key for indexing.
pub fn value_to_index_key(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}
