use crate::collection::collection::Collection;
use crate::error::{DbError, DbResult};
use crate::storage::file_storage;

/// Load a collection from disk into memory.
pub fn load_collection(data_path: &str, name: &str, key: Option<&[u8; 32]>) -> DbResult<Collection> {
    let docs = file_storage::read_collection(data_path, name, key)?;
    let mut col = Collection::new(name);
    col.load(docs);
    Ok(col)
}

/// Create a new empty collection on disk.
pub fn create_collection(data_path: &str, name: &str, key: Option<&[u8; 32]>) -> DbResult<Collection> {
    file_storage::ensure_collections_dir(data_path)?;
    let path = file_storage::collection_path(data_path, name);
    if path.exists() {
        return Err(DbError::CollectionAlreadyExists(name.to_string()));
    }
    let col = Collection::new(name);
    file_storage::write_collection(data_path, name, &col.documents, key)?;
    Ok(col)
}

/// Drop a collection from disk.
pub fn drop_collection(data_path: &str, name: &str) -> DbResult<()> {
    let path = file_storage::collection_path(data_path, name);
    if !path.exists() {
        // Check legacy paths before erroring
        let legacy_ndjson = std::path::Path::new(data_path)
            .join("collections")
            .join(format!("{}.ndjson", name));
        let legacy_json = std::path::Path::new(data_path)
            .join("collections")
            .join(format!("{}.json", name));
        if !legacy_ndjson.exists() && !legacy_json.exists() {
            return Err(DbError::CollectionNotFound(name.to_string()));
        }
    }
    file_storage::delete_collection_file(data_path, name)?;
    Ok(())
}
