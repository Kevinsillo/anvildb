use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::DbResult;
use crate::storage::codec;

/// Return the path to a collection's compressed file.
pub fn collection_path(data_path: &str, name: &str) -> PathBuf {
    Path::new(data_path)
        .join("collections")
        .join(format!("{}.anvil", name))
}

/// Return the path to a legacy NDJSON file.
fn legacy_ndjson_path(data_path: &str, name: &str) -> PathBuf {
    Path::new(data_path)
        .join("collections")
        .join(format!("{}.ndjson", name))
}

/// Return the path to a legacy JSON array file.
fn legacy_json_path(data_path: &str, name: &str) -> PathBuf {
    Path::new(data_path)
        .join("collections")
        .join(format!("{}.json", name))
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

/// Migrate legacy formats (.json, .ndjson) to .anvil if they exist.
fn migrate_if_legacy(data_path: &str, name: &str, key: Option<&[u8; 32]>) -> DbResult<()> {
    let anvil = collection_path(data_path, name);
    if anvil.exists() {
        // Already migrated — clean up legacy files
        let _ = fs::remove_file(legacy_ndjson_path(data_path, name));
        let _ = fs::remove_file(legacy_json_path(data_path, name));
        return Ok(());
    }

    // Try NDJSON first
    let ndjson = legacy_ndjson_path(data_path, name);
    if ndjson.exists() {
        let contents = fs::read_to_string(&ndjson)?;
        let docs = parse_ndjson(&contents);
        let encoded = codec::encode_docs(&docs, key)?;
        fs::write(&anvil, &encoded)?;
        fs::remove_file(&ndjson)?;
        return Ok(());
    }

    // Try legacy JSON array
    let json = legacy_json_path(data_path, name);
    if json.exists() {
        let contents = fs::read_to_string(&json)?;
        let trimmed = contents.trim();
        let docs: Vec<Value> = if trimmed.is_empty() || trimmed == "[]" {
            Vec::new()
        } else {
            serde_json::from_str(trimmed)?
        };
        let encoded = codec::encode_docs(&docs, key)?;
        fs::write(&anvil, &encoded)?;
        fs::remove_file(&json)?;
        return Ok(());
    }

    Ok(())
}

/// Parse NDJSON content into documents.
fn parse_ndjson(content: &str) -> Vec<Value> {
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// Read all documents from a collection's compressed file.
pub fn read_collection(
    data_path: &str,
    name: &str,
    key: Option<&[u8; 32]>,
) -> DbResult<Vec<Value>> {
    migrate_if_legacy(data_path, name, key)?;

    let path = collection_path(data_path, name);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let data = fs::read(&path)?;
    if data.is_empty() {
        return Ok(Vec::new());
    }

    codec::decode_docs(&data, key)
}

/// Rewrite a collection's file with the given documents.
/// Uses atomic temp file + rename.
pub fn rewrite_collection(
    data_path: &str,
    name: &str,
    docs: &[Value],
    key: Option<&[u8; 32]>,
) -> DbResult<()> {
    ensure_collections_dir(data_path)?;

    let encoded = codec::encode_docs(docs, key)?;

    let path = collection_path(data_path, name);
    let dir = path.parent().unwrap();
    let tmp_path = dir.join(format!(".{}.{}.tmp", name, std::process::id()));

    fs::write(&tmp_path, &encoded)?;
    fs::rename(&tmp_path, &path)?;

    Ok(())
}

/// Write documents to a collection file (used by create_collection).
pub fn write_collection(
    data_path: &str,
    name: &str,
    docs: &[Value],
    key: Option<&[u8; 32]>,
) -> DbResult<()> {
    rewrite_collection(data_path, name, docs, key)
}

/// Delete a collection file.
pub fn delete_collection_file(data_path: &str, name: &str) -> DbResult<()> {
    // Remove .anvil file
    let path = collection_path(data_path, name);
    if path.exists() {
        fs::remove_file(&path)?;
    }
    // Remove legacy files if still present
    let ndjson = legacy_ndjson_path(data_path, name);
    if ndjson.exists() {
        let _ = fs::remove_file(&ndjson);
    }
    let json = legacy_json_path(data_path, name);
    if json.exists() {
        let _ = fs::remove_file(&json);
    }
    Ok(())
}

/// List collection names by scanning the collections directory.
/// Detects .anvil, .ndjson and legacy .json files.
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
        if ext == Some("anvil") || ext == Some("ndjson") || ext == Some("json") {
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
