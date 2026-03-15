use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::DbResult;
use crate::storage::file_lock;

/// Return the path to a collection's JSON file.
pub fn collection_path(data_path: &str, name: &str) -> PathBuf {
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

/// Read documents from a collection file. Returns an empty vec if file doesn't exist.
pub fn read_collection(data_path: &str, name: &str) -> DbResult<Vec<Value>> {
    let path = collection_path(data_path, name);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut file = file_lock::lock_shared(&path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    if contents.trim().is_empty() {
        return Ok(Vec::new());
    }

    let docs: Vec<Value> = serde_json::from_str(&contents)?;
    Ok(docs)
}

/// Write documents to a collection file using atomic write (temp file + rename)
/// protected by an exclusive file lock.
pub fn write_collection(data_path: &str, name: &str, docs: &[Value]) -> DbResult<()> {
    ensure_collections_dir(data_path)?;

    let path = collection_path(data_path, name);
    let dir = path.parent().unwrap();
    let tmp_path = dir.join(format!(".{}.{}.tmp", name, std::process::id()));

    // Acquire exclusive lock on a dedicated lock file
    let lock_path = lock_file_path(data_path, name);
    let _lock = file_lock::lock_exclusive(&lock_path)?;

    // Write to temp file
    let json = serde_json::to_string_pretty(docs)?;
    fs::write(&tmp_path, json.as_bytes())?;

    // Atomic rename
    fs::rename(&tmp_path, &path)?;
    // _lock is dropped here, releasing the exclusive lock
    Ok(())
}

/// Read and modify a collection atomically.
/// Acquires an exclusive lock, reads current state from disk, applies the
/// modifier function, and writes back. This ensures multi-process safety.
pub fn read_modify_write<F>(data_path: &str, name: &str, modify: F) -> DbResult<Vec<Value>>
where
    F: FnOnce(&mut Vec<Value>) -> DbResult<()>,
{
    ensure_collections_dir(data_path)?;

    let path = collection_path(data_path, name);
    let dir = path.parent().unwrap();
    let tmp_path = dir.join(format!(".{}.{}.tmp", name, std::process::id()));

    // Acquire exclusive lock
    let lock_path = lock_file_path(data_path, name);
    let _lock = file_lock::lock_exclusive(&lock_path)?;

    // Read current state under lock
    let mut docs = if path.exists() {
        let contents = fs::read_to_string(&path)?;
        if contents.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&contents)?
        }
    } else {
        Vec::new()
    };

    // Apply modification
    modify(&mut docs)?;

    // Write back atomically
    let json = serde_json::to_string_pretty(&docs)?;
    fs::write(&tmp_path, json.as_bytes())?;
    fs::rename(&tmp_path, &path)?;

    // _lock released here
    Ok(docs)
}

/// Delete a collection file.
pub fn delete_collection_file(data_path: &str, name: &str) -> DbResult<()> {
    let path = collection_path(data_path, name);
    if path.exists() {
        fs::remove_file(&path)?;
    }
    // Also remove lock file
    let lock = lock_file_path(data_path, name);
    if lock.exists() {
        let _ = fs::remove_file(&lock);
    }
    Ok(())
}

/// List collection names by scanning the collections directory.
pub fn list_collection_files(data_path: &str) -> DbResult<Vec<String>> {
    let dir = Path::new(data_path).join("collections");
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut names = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}
