use std::collections::HashMap;
use std::sync::{Mutex, RwLock};

use serde_json::Value;

use crate::cache::lru_cache::LruCache;
use crate::collection::collection::Collection;
use crate::collection::manager;
use crate::error::{DbError, DbResult};
use crate::index::hash_index::HashIndex;
use crate::index::unique_index::UniqueIndex;
use crate::query::builder::{self, QuerySpec};
use crate::query::engine as qe;
use crate::storage::file_storage;
use crate::validation::schema::Schema;

/// Top-level engine holding all database state.
pub struct Engine {
    pub data_path: String,
    pub collections: RwLock<HashMap<String, Collection>>,
    pub cache: Mutex<LruCache>,
    pub last_error: Mutex<Option<String>>,
}

impl Engine {
    /// Open (or create) a database at the given path.
    pub fn open(data_path: &str) -> DbResult<Self> {
        file_storage::ensure_collections_dir(data_path)?;
        file_storage::ensure_indexes_dir(data_path)?;

        let mut collections = HashMap::new();

        // Load existing collections from disk
        let names = file_storage::list_collection_files(data_path)?;
        for name in names {
            let col = manager::load_collection(data_path, &name)?;
            collections.insert(name, col);
        }

        Ok(Engine {
            data_path: data_path.to_string(),
            collections: RwLock::new(collections),
            cache: Mutex::new(LruCache::new()),
            last_error: Mutex::new(None),
        })
    }

    /// Set the last error message.
    pub fn set_error(&self, msg: String) {
        if let Ok(mut err) = self.last_error.lock() {
            *err = Some(msg);
        }
    }

    /// Get and clear the last error message.
    pub fn take_error(&self) -> Option<String> {
        if let Ok(mut err) = self.last_error.lock() {
            err.take()
        } else {
            None
        }
    }

    /// Invalidate cache entries for a collection.
    fn invalidate_cache(&self, collection: &str) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.invalidate_prefix(collection);
        }
    }

    /// Create a new collection.
    pub fn create_collection(&self, name: &str) -> DbResult<()> {
        let col = manager::create_collection(&self.data_path, name)?;
        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;
        cols.insert(name.to_string(), col);
        Ok(())
    }

    /// Drop a collection.
    pub fn drop_collection(&self, name: &str) -> DbResult<()> {
        manager::drop_collection(&self.data_path, name)?;
        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;
        cols.remove(name);
        self.invalidate_cache(name);
        Ok(())
    }

    /// List all collection names.
    pub fn list_collections(&self) -> DbResult<Vec<String>> {
        let cols = self
            .collections
            .read()
            .map_err(|e| DbError::LockError(e.to_string()))?;
        let mut names: Vec<String> = cols.keys().cloned().collect();
        names.sort();
        Ok(names)
    }

    /// Insert a document into a collection. Auto-generates "id" if missing.
    /// Returns the inserted document (with id).
    pub fn insert(&self, collection: &str, doc_json: &str) -> DbResult<Value> {
        let mut doc: Value = serde_json::from_str(doc_json)?;

        // Auto-generate id if missing
        if doc.get("id").is_none() || doc["id"].is_null() {
            let id = uuid::Uuid::new_v4().to_string();
            doc["id"] = Value::String(id);
        }

        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get_mut(collection)
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        col.insert(doc.clone())?;

        // Append to NDJSON file — O(1), no re-read needed
        file_storage::append_documents(&self.data_path, collection, &[doc.clone()])?;
        self.invalidate_cache(collection);

        Ok(doc)
    }

    /// Bulk insert multiple documents. Returns the inserted documents.
    pub fn bulk_insert(&self, collection: &str, docs_json: &str) -> DbResult<Vec<Value>> {
        let mut docs: Vec<Value> = serde_json::from_str(docs_json)?;

        // Auto-generate ids
        for doc in &mut docs {
            if doc.get("id").is_none() || doc["id"].is_null() {
                let id = uuid::Uuid::new_v4().to_string();
                doc["id"] = Value::String(id);
            }
        }

        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get_mut(collection)
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        for doc in &docs {
            col.insert(doc.clone())?;
        }

        // Append all docs to NDJSON file — O(k) for k new docs
        file_storage::append_documents(&self.data_path, collection, &docs)?;
        self.invalidate_cache(collection);

        Ok(docs)
    }

    /// Find a document by id.
    pub fn find_by_id(&self, collection: &str, id: &str) -> DbResult<Value> {
        let cols = self
            .collections
            .read()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get(collection)
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        let (_, doc) = col
            .find_by_id(id)
            .ok_or_else(|| DbError::DocumentNotFound(id.to_string()))?;

        Ok(doc.clone())
    }

    /// Update a document by id.
    pub fn update(&self, collection: &str, id: &str, doc_json: &str) -> DbResult<()> {
        let mut new_doc: Value = serde_json::from_str(doc_json)?;

        // Ensure the id field stays consistent
        new_doc["id"] = Value::String(id.to_string());

        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get_mut(collection)
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        let (pos, _) = col
            .find_by_id(id)
            .ok_or_else(|| DbError::DocumentNotFound(id.to_string()))?;

        col.update(pos, new_doc)?;
        // Rewrite entire NDJSON file (updates require full rewrite)
        file_storage::rewrite_collection(&self.data_path, collection, &col.documents)?;
        self.invalidate_cache(collection);

        Ok(())
    }

    /// Delete a document by id.
    pub fn delete(&self, collection: &str, id: &str) -> DbResult<()> {
        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get_mut(collection)
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        let (pos, _) = col
            .find_by_id(id)
            .ok_or_else(|| DbError::DocumentNotFound(id.to_string()))?;

        col.delete(pos)?;
        // Rewrite entire NDJSON file (deletes require full rewrite)
        file_storage::rewrite_collection(&self.data_path, collection, &col.documents)?;
        self.invalidate_cache(collection);

        Ok(())
    }

    /// Execute a query.
    pub fn query(&self, query_json: &str) -> DbResult<Vec<Value>> {
        let spec = QuerySpec::from_json(query_json)?;

        // Check cache
        let cache_key = format!("{}:{}", spec.collection, query_json);
        if let Ok(mut cache) = self.cache.lock() {
            if let Some(cached) = cache.get(&cache_key) {
                let results: Vec<Value> = serde_json::from_str(cached)?;
                return Ok(results);
            }
        }

        let cols = self
            .collections
            .read()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get(&spec.collection)
            .ok_or_else(|| DbError::CollectionNotFound(spec.collection.clone()))?;

        let results = qe::execute_query(&col.documents, &spec)?;

        // Store in cache
        if let Ok(result_json) = serde_json::to_string(&results) {
            if let Ok(mut cache) = self.cache.lock() {
                cache.put(cache_key, result_json);
            }
        }

        Ok(results)
    }

    /// Count documents matching filters in a collection.
    pub fn count(&self, collection: &str, filter_json: &str) -> DbResult<i64> {
        let filters = if filter_json.is_empty() || filter_json == "null" {
            Vec::new()
        } else {
            builder::parse_filters(filter_json)?
        };

        let cols = self
            .collections
            .read()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get(collection)
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        let count = if filters.is_empty() {
            col.documents.len()
        } else {
            qe::count_matching(&col.documents, &filters)
        };

        Ok(count as i64)
    }

    /// Create an index on a field.
    pub fn create_index(
        &self,
        collection: &str,
        field: &str,
        index_type: &str,
    ) -> DbResult<()> {
        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get_mut(collection)
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        match index_type {
            "unique" => {
                col.add_unique_index(field)?;
                // Persist
                if let Some(idx) = col.unique_indexes.get(field) {
                    idx.save(&self.data_path, collection)?;
                }
            }
            "hash" | _ => {
                col.add_hash_index(field);
                // Persist
                if let Some(idx) = col.hash_indexes.get(field) {
                    idx.save(&self.data_path, collection)?;
                }
            }
        }

        Ok(())
    }

    /// Drop an index on a field.
    pub fn drop_index(&self, collection: &str, field: &str) -> DbResult<()> {
        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get_mut(collection)
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        col.drop_index(field);
        HashIndex::delete_file(&self.data_path, collection, field)?;
        UniqueIndex::delete_file(&self.data_path, collection, field)?;

        Ok(())
    }

    /// Set a schema on a collection.
    pub fn set_schema(&self, collection: &str, schema_json: &str) -> DbResult<()> {
        let val: Value = serde_json::from_str(schema_json)?;
        let schema = Schema::from_value(&val)?;

        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get_mut(collection)
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        col.set_schema(schema);
        Ok(())
    }

    /// Clear the query cache.
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
    }
}
