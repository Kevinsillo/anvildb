use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::DbResult;

const METADATA_FILE: &str = "metadata.json";
const CURRENT_VERSION: u32 = 3;

/// Database metadata stored as plain JSON (never encrypted).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMetadata {
    pub version: u32,
    pub encrypted: bool,
}

impl Default for DbMetadata {
    fn default() -> Self {
        DbMetadata {
            version: CURRENT_VERSION,
            encrypted: false,
        }
    }
}

impl DbMetadata {
    /// Load metadata from the DB directory. Returns `(metadata, existed)`.
    /// If the file doesn't exist, returns the default metadata with `existed = false`.
    pub fn load(data_path: &str) -> DbResult<(Self, bool)> {
        let path = Path::new(data_path).join(METADATA_FILE);
        if !path.exists() {
            return Ok((Self::default(), false));
        }
        let contents = fs::read_to_string(&path)?;
        let meta: DbMetadata = serde_json::from_str(&contents)?;
        Ok((meta, true))
    }

    /// Save metadata to the DB directory.
    pub fn save(&self, data_path: &str) -> DbResult<()> {
        let path = Path::new(data_path).join(METADATA_FILE);
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}
