use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::DbResult;
use crate::storage::file_lock;

/// Return the path to a collection's NDJSON file.
pub fn collection_path(data_path: &str, name: &str) -> PathBuf {
    Path::new(data_path)
        .join("collections")
        .join(format!("{}.ndjson", name))
}

/// Return the path to a legacy JSON array file.
fn legacy_collection_path(data_path: &str, name: &str) -> PathBuf {
    Path::new(data_path)
        .join("collections")
        .join(format!("{}.json", name))
}

/// Return the path to a collection's lock file.
fn lock_file_path(data_path: &str, name: &str) -> PathBuf {
    Path::new(data_path)
        .join("collections")
        .join(format!(".{}.lock", name))
}

/// Ensure the collections directory exists.
pub fn ensure_collections_dir(data_path: &str) -> DbResult<()> {
    let dir = Path::new(data_path).join("collections");
    fs::create_dir_all(&dir)?;
    Ok(())
}

/// Ensure the indexes directory exists.
pub fn ensure_indexes_dir(data_path: &str) -> DbResult<()> {
    let dir = Path::new(data_path).join("indexes");
    fs::create_dir_all(&dir)?;
    Ok(())
}

/// Migrate a legacy JSON array file to NDJSON format if it exists.
fn migrate_if_legacy(data_path: &str, name: &str) -> DbResult<()> {
    let legacy = legacy_collection_path(data_path, name);
    if !legacy.exists() {
        return Ok(());
    }

    let ndjson = collection_path(data_path, name);
    if ndjson.exists() {
        // Both exist — legacy is stale, remove it
        let _ = fs::remove_file(&legacy);
        return Ok(());
    }

    // Read legacy JSON array
    let contents = fs::read_to_string(&legacy)?;
    let trimmed = contents.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        // Empty collection — create empty NDJSON and remove legacy
        File::create(&ndjson)?;
        fs::remove_file(&legacy)?;
        return Ok(());
    }

    let docs: Vec<Value> = serde_json::from_str(trimmed)?;

    // Write as NDJSON
    let mut file = File::create(&ndjson)?;
    for doc in &docs {
        let line = serde_json::to_string(doc)?;
        writeln!(file, "{}", line)?;
    }
    file.flush()?;

    // Remove legacy file
    fs::remove_file(&legacy)?;

    Ok(())
}

/// Read all documents from a collection's NDJSON file.
/// Handles migration from legacy JSON array format automatically.
pub fn read_collection(data_path: &str, name: &str) -> DbResult<Vec<Value>> {
    migrate_if_legacy(data_path, name)?;

    let path = collection_path(data_path, name);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = file_lock::lock_shared(&path)?;
    let reader = BufReader::new(file);
    let mut docs = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let doc: Value = serde_json::from_str(trimmed)?;
        docs.push(doc);
    }

    Ok(docs)
}

/// Append documents to a collection's NDJSON file — O(1) per document.
/// Uses exclusive file lock for multi-process safety.
pub fn append_documents(data_path: &str, name: &str, docs: &[Value]) -> DbResult<()> {
    ensure_collections_dir(data_path)?;

    let lock_path = lock_file_path(data_path, name);
    let _lock = file_lock::lock_exclusive(&lock_path)?;

    let path = collection_path(data_path, name);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;

    for doc in docs {
        let line = serde_json::to_string(doc)?;
        writeln!(file, "{}", line)?;
    }
    file.flush()?;

    Ok(())
}

/// Rewrite a collection's NDJSON file with the given documents.
/// Used for updates and deletes (which cannot be appended).
/// Uses exclusive file lock + atomic temp file + rename.
pub fn rewrite_collection(data_path: &str, name: &str, docs: &[Value]) -> DbResult<()> {
    ensure_collections_dir(data_path)?;

    let lock_path = lock_file_path(data_path, name);
    let _lock = file_lock::lock_exclusive(&lock_path)?;

    let path = collection_path(data_path, name);
    let dir = path.parent().unwrap();
    let tmp_path = dir.join(format!(".{}.{}.tmp", name, std::process::id()));

    let mut file = File::create(&tmp_path)?;
    for doc in docs {
        let line = serde_json::to_string(doc)?;
        writeln!(file, "{}", line)?;
    }
    file.flush()?;

    fs::rename(&tmp_path, &path)?;
    Ok(())
}

/// Write documents to a collection file (NDJSON format).
/// Used by create_collection to initialize an empty file.
pub fn write_collection(data_path: &str, name: &str, docs: &[Value]) -> DbResult<()> {
    rewrite_collection(data_path, name, docs)
}

/// Delete a collection file.
pub fn delete_collection_file(data_path: &str, name: &str) -> DbResult<()> {
    // Remove NDJSON file
    let path = collection_path(data_path, name);
    if path.exists() {
        fs::remove_file(&path)?;
    }
    // Remove legacy JSON file if still present
    let legacy = legacy_collection_path(data_path, name);
    if legacy.exists() {
        let _ = fs::remove_file(&legacy);
    }
    // Remove lock file
    let lock = lock_file_path(data_path, name);
    if lock.exists() {
        let _ = fs::remove_file(&lock);
    }
    Ok(())
}

/// List collection names by scanning the collections directory.
/// Detects both .ndjson and legacy .json files.
pub fn list_collection_files(data_path: &str) -> DbResult<Vec<String>> {
    let dir = Path::new(data_path).join("collections");
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut names = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str());
        if ext == Some("ndjson") || ext == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                // Skip temp/lock files
                if stem.starts_with('.') {
                    continue;
                }
                if !names.contains(&stem.to_string()) {
                    names.push(stem.to_string());
                }
            }
        }
    }
    names.sort();
    Ok(names)
}
