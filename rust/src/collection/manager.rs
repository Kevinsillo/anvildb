use serde_json::Value;

use crate::collection::collection::Collection;
use crate::error::{DbError, DbResult};
use crate::storage::file_storage;

/// Load a collection from disk into memory.
pub fn load_collection(data_path: &str, name: &str) -> DbResult<Collection> {
    let docs = file_storage::read_collection(data_path, name)?;
    let mut col = Collection::new(name);
    col.load(docs);
    Ok(col)
}

/// Persist a collection to disk.
pub fn save_collection(data_path: &str, col: &Collection) -> DbResult<()> {
    file_storage::write_collection(data_path, &col.name, &col.documents)
}

/// Atomically insert documents into a collection on disk.
/// Uses read-modify-write with exclusive file lock for multi-process safety.
/// Returns the final documents list after insertion.
pub fn atomic_insert(data_path: &str, name: &str, new_docs: Vec<Value>) -> DbResult<Vec<Value>> {
    file_storage::read_modify_write(data_path, name, |docs| {
        for doc in &new_docs {
            docs.push(doc.clone());
        }
        Ok(())
    })
}

/// Atomically update a document by id on disk.
pub fn atomic_update(data_path: &str, name: &str, id: &str, new_doc: Value) -> DbResult<()> {
    file_storage::read_modify_write(data_path, name, |docs| {
        let pos = docs
            .iter()
            .position(|d| d.get("id").and_then(|v| v.as_str()) == Some(id))
            .ok_or_else(|| DbError::DocumentNotFound(id.to_string()))?;
        docs[pos] = new_doc.clone();
        Ok(())
    })?;
    Ok(())
}

/// Atomically delete a document by id on disk.
pub fn atomic_delete(data_path: &str, name: &str, id: &str) -> DbResult<()> {
    file_storage::read_modify_write(data_path, name, |docs| {
        let pos = docs
            .iter()
            .position(|d| d.get("id").and_then(|v| v.as_str()) == Some(id))
            .ok_or_else(|| DbError::DocumentNotFound(id.to_string()))?;
        docs.remove(pos);
        Ok(())
    })?;
    Ok(())
}

/// Create a new empty collection on disk.
pub fn create_collection(data_path: &str, name: &str) -> DbResult<Collection> {
    file_storage::ensure_collections_dir(data_path)?;
    let path = file_storage::collection_path(data_path, name);
    if path.exists() {
        return Err(DbError::CollectionAlreadyExists(name.to_string()));
    }
    let col = Collection::new(name);
    file_storage::write_collection(data_path, name, &col.documents)?;
    Ok(col)
}

/// Drop a collection from disk.
pub fn drop_collection(data_path: &str, name: &str) -> DbResult<()> {
    let path = file_storage::collection_path(data_path, name);
    if !path.exists() {
        return Err(DbError::CollectionNotFound(name.to_string()));
    }
    file_storage::delete_collection_file(data_path, name)?;
    Ok(())
}
