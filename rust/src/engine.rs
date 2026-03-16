use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::{self, JoinHandle};

use serde_json::Value;

use crate::buffer::{WriteBuffer, WriteBufferConfig};
use crate::cache::lru_cache::LruCache;
use crate::collection::collection::LazyCollection;
use crate::collection::manager;
use crate::error::{DbError, DbResult};
use crate::index::hash_index::HashIndex;
use crate::index::range_index::RangeIndex;
use crate::index::unique_index::UniqueIndex;
use crate::query::builder::{self, QuerySpec};
use crate::query::engine as qe;
use crate::storage::file_storage;
use crate::storage::metadata::DbMetadata;
use crate::validation::schema::Schema;

/// Top-level engine holding all database state.
pub struct Engine {
    pub data_path: String,
    pub collections: Arc<RwLock<HashMap<String, LazyCollection>>>,
    pub cache: Mutex<LruCache>,
    pub last_error: Mutex<Option<String>>,
    pub last_error_code: Mutex<i32>,
    pub last_warning: Mutex<Option<String>>,
    encryption_key: RwLock<Option<[u8; 32]>>,
    buffer: Arc<WriteBuffer>,
    shutdown: Arc<(Mutex<bool>, Condvar)>,
    flush_thread: Mutex<Option<JoinHandle<()>>>,
}

impl Engine {
    /// Open (or create) a database at the given path.
    /// Pass an encryption key to open an encrypted database.
    pub fn open(data_path: &str, encryption_key: Option<&[u8; 32]>) -> DbResult<Self> {
        file_storage::ensure_collections_dir(data_path)?;
        file_storage::ensure_indexes_dir(data_path)?;

        // Load or create metadata
        let (mut metadata, existed) = DbMetadata::load(data_path)?;

        if existed && metadata.encrypted && encryption_key.is_none() {
            return Err(DbError::EncryptionRequired);
        }

        let open_warning = if existed && !metadata.encrypted && encryption_key.is_some() {
            Some("Encryption key provided but database is not encrypted".to_string())
        } else {
            None
        };

        // New database with key → mark as encrypted from the start
        if !existed && encryption_key.is_some() {
            metadata.encrypted = true;
        }

        metadata.save(data_path)?;

        let mut collections = HashMap::new();

        // Discover existing collections but don't load them yet (lazy loading)
        let names = file_storage::list_collection_files(data_path)?;
        for name in names {
            collections.insert(name, LazyCollection::Unloaded);
        }

        let collections = Arc::new(RwLock::new(collections));
        let buffer = Arc::new(WriteBuffer::new(WriteBufferConfig::default()));
        let shutdown = Arc::new((Mutex::new(false), Condvar::new()));
        let key_copy = encryption_key.copied();

        // Spawn background flush thread
        let flush_thread = {
            let buffer = Arc::clone(&buffer);
            let shutdown = Arc::clone(&shutdown);
            let collections = Arc::clone(&collections);
            let data_path = data_path.to_string();

            thread::spawn(move || {
                Self::flush_loop(&data_path, &buffer, &shutdown, &collections, key_copy.as_ref());
            })
        };

        Ok(Engine {
            data_path: data_path.to_string(),
            collections,
            cache: Mutex::new(LruCache::new()),
            last_error: Mutex::new(None),
            last_error_code: Mutex::new(0),
            last_warning: Mutex::new(open_warning),
            encryption_key: RwLock::new(encryption_key.copied()),
            buffer,
            shutdown,
            flush_thread: Mutex::new(Some(flush_thread)),
        })
    }

    /// Get the current encryption key (if set).
    fn get_key(&self) -> Option<[u8; 32]> {
        self.encryption_key.read().ok().and_then(|k| *k)
    }

    /// Ensure a collection is loaded into memory.
    fn ensure_loaded(&self, name: &str) -> DbResult<()> {
        // Fast path: check with read lock
        {
            let cols = self
                .collections
                .read()
                .map_err(|e| DbError::LockError(e.to_string()))?;

            match cols.get(name) {
                Some(LazyCollection::Loaded(_)) => return Ok(()),
                Some(LazyCollection::Unloaded) => {}
                None => return Err(DbError::CollectionNotFound(name.to_string())),
            }
        }

        // Slow path: acquire write lock and load from disk
        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        // Double-check
        match cols.get(name) {
            Some(LazyCollection::Loaded(_)) => return Ok(()),
            Some(LazyCollection::Unloaded) => {}
            None => return Err(DbError::CollectionNotFound(name.to_string())),
        }

        let key = self.get_key();
        let col = manager::load_collection(&self.data_path, name, key.as_ref())?;
        cols.insert(name.to_string(), LazyCollection::Loaded(col));

        Ok(())
    }

    /// Background loop: flush dirty collections every N seconds.
    fn flush_loop(
        data_path: &str,
        buffer: &WriteBuffer,
        shutdown: &(Mutex<bool>, Condvar),
        collections: &RwLock<HashMap<String, LazyCollection>>,
        key: Option<&[u8; 32]>,
    ) {
        let (lock, cvar) = shutdown;

        loop {
            let interval = std::time::Duration::from_secs(buffer.flush_interval_secs());

            let mut stop = lock.lock().unwrap();
            let result = cvar.wait_timeout(stop, interval).unwrap();
            stop = result.0;

            // Flush dirty collections
            let dirty = buffer.take_dirty();
            if !dirty.is_empty() {
                if let Ok(cols) = collections.read() {
                    for name in &dirty {
                        if let Some(LazyCollection::Loaded(col)) = cols.get(name) {
                            if let Err(e) = file_storage::rewrite_collection(
                                data_path,
                                name,
                                &col.documents,
                                key,
                            ) {
                                log::error!("background flush failed for '{}': {}", name, e);
                                // Re-mark as dirty so it gets retried
                                buffer.mark_dirty(name, 0);
                            }
                        }
                    }
                }
            }

            if *stop {
                break;
            }
        }
    }

    /// Set the last error message (with a generic error code of -1).
    pub fn set_error(&self, msg: String) {
        if let Ok(mut err) = self.last_error.lock() {
            *err = Some(msg);
        }
        if let Ok(mut code) = self.last_error_code.lock() {
            *code = -1;
        }
    }

    /// Set the last error from a `DbError`, capturing both the message and
    /// the numeric error code.
    pub fn set_error_from(&self, error: &crate::error::DbError) {
        if let Ok(mut err) = self.last_error.lock() {
            *err = Some(error.to_string());
        }
        if let Ok(mut code) = self.last_error_code.lock() {
            *code = error.code();
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

    /// Get and clear the last error code. Returns 0 when there is no error.
    pub fn take_error_code(&self) -> i32 {
        if let Ok(mut code) = self.last_error_code.lock() {
            let c = *code;
            *code = 0;
            c
        } else {
            0
        }
    }

    /// Get and clear the last warning message.
    pub fn take_warning(&self) -> Option<String> {
        if let Ok(mut warn) = self.last_warning.lock() {
            warn.take()
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

    /// Flush all dirty collections to disk.
    pub fn flush(&self) -> DbResult<()> {
        let dirty = self.buffer.take_dirty();
        if dirty.is_empty() {
            return Ok(());
        }

        let key = self.get_key();
        let cols = self
            .collections
            .read()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        for name in &dirty {
            if let Some(LazyCollection::Loaded(col)) = cols.get(name) {
                file_storage::rewrite_collection(&self.data_path, name, &col.documents, key.as_ref())?;
            }
        }
        Ok(())
    }

    /// Flush a single collection to disk.
    pub fn flush_collection(&self, collection: &str) -> DbResult<()> {
        self.ensure_loaded(collection)?;

        let key = self.get_key();
        let cols = self
            .collections
            .read()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        if let Some(LazyCollection::Loaded(col)) = cols.get(collection) {
            file_storage::rewrite_collection(&self.data_path, collection, &col.documents, key.as_ref())?;
        }

        // Clear dirty flag
        self.buffer.remove_collection(collection);
        Ok(())
    }

    /// Reconfigure buffer thresholds.
    pub fn configure_buffer(&self, max_docs: usize, flush_interval_secs: u64) {
        self.buffer.configure(max_docs, flush_interval_secs);
        let (_, cvar) = &*self.shutdown;
        cvar.notify_one();
    }

    /// Encrypt an unencrypted database. Rewrites all files with encryption.
    pub fn encrypt(&self, key: &[u8; 32]) -> DbResult<()> {
        // Flush pending writes first (unencrypted)
        self.flush()?;

        let (metadata, _) = DbMetadata::load(&self.data_path)?;
        if metadata.encrypted {
            return Err(DbError::EncryptionError("Database is already encrypted".into()));
        }

        // Load all collections
        let cols = self
            .collections
            .read()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let names: Vec<String> = cols.keys().cloned().collect();
        drop(cols);

        for name in &names {
            self.ensure_loaded(name)?;
        }

        // Rewrite all collections with encryption
        let cols = self
            .collections
            .read()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        for name in &names {
            if let Some(LazyCollection::Loaded(col)) = cols.get(name) {
                file_storage::rewrite_collection(&self.data_path, name, &col.documents, Some(key))?;

                // Rewrite indexes
                for (_, idx) in &col.hash_indexes {
                    idx.save(&self.data_path, name, Some(key))?;
                }
                for (_, idx) in &col.unique_indexes {
                    idx.save(&self.data_path, name, Some(key))?;
                }
                for (_, idx) in &col.range_indexes {
                    idx.save(&self.data_path, name, Some(key))?;
                }
            }
        }

        // Update metadata and key
        let (mut metadata, _) = DbMetadata::load(&self.data_path)?;
        metadata.encrypted = true;
        metadata.save(&self.data_path)?;

        if let Ok(mut k) = self.encryption_key.write() {
            *k = Some(*key);
        }

        Ok(())
    }

    /// Decrypt an encrypted database. Rewrites all files without encryption.
    pub fn decrypt(&self, _key: &[u8; 32]) -> DbResult<()> {
        // Flush pending writes first (encrypted)
        self.flush()?;

        let (metadata, _) = DbMetadata::load(&self.data_path)?;
        if !metadata.encrypted {
            return Err(DbError::EncryptionError("Database is not encrypted".into()));
        }

        // Load all collections (using the provided key)
        let cols = self
            .collections
            .read()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let names: Vec<String> = cols.keys().cloned().collect();
        drop(cols);

        for name in &names {
            self.ensure_loaded(name)?;
        }

        // Rewrite all collections without encryption
        let cols = self
            .collections
            .read()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        for name in &names {
            if let Some(LazyCollection::Loaded(col)) = cols.get(name) {
                file_storage::rewrite_collection(&self.data_path, name, &col.documents, None)?;

                // Rewrite indexes
                for (_, idx) in &col.hash_indexes {
                    idx.save(&self.data_path, name, None)?;
                }
                for (_, idx) in &col.unique_indexes {
                    idx.save(&self.data_path, name, None)?;
                }
                for (_, idx) in &col.range_indexes {
                    idx.save(&self.data_path, name, None)?;
                }
            }
        }

        // Update metadata and key
        let (mut metadata, _) = DbMetadata::load(&self.data_path)?;
        metadata.encrypted = false;
        metadata.save(&self.data_path)?;

        if let Ok(mut k) = self.encryption_key.write() {
            *k = None;
        }

        Ok(())
    }

    /// Create a new collection.
    pub fn create_collection(&self, name: &str) -> DbResult<()> {
        let key = self.get_key();
        let col = manager::create_collection(&self.data_path, name, key.as_ref())?;
        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;
        cols.insert(name.to_string(), LazyCollection::Loaded(col));
        Ok(())
    }

    /// Drop a collection.
    pub fn drop_collection(&self, name: &str) -> DbResult<()> {
        self.buffer.remove_collection(name);
        manager::drop_collection(&self.data_path, name)?;
        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;
        cols.remove(name);
        self.invalidate_cache(name);
        Ok(())
    }

    /// List all collection names (without loading them).
    pub fn list_collections(&self) -> DbResult<Vec<String>> {
        let cols = self
            .collections
            .read()
            .map_err(|e| DbError::LockError(e.to_string()))?;
        let mut names: Vec<String> = cols.keys().cloned().collect();
        names.sort();
        Ok(names)
    }

    /// Insert a document into a collection.
    pub fn insert(&self, collection: &str, doc_json: &str) -> DbResult<Value> {
        self.ensure_loaded(collection)?;

        let mut doc: Value = serde_json::from_str(doc_json)?;

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
            .and_then(|lc| lc.as_loaded_mut())
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        col.insert(doc.clone())?;

        // Mark dirty — if threshold reached, caller should flush
        let should_flush = self.buffer.mark_dirty(collection, 1);
        if should_flush {
            let key = self.get_key();
            file_storage::rewrite_collection(&self.data_path, collection, &col.documents, key.as_ref())?;
            self.buffer.remove_collection(collection);
        }

        self.invalidate_cache(collection);

        Ok(doc)
    }

    /// Bulk insert multiple documents.
    pub fn bulk_insert(&self, collection: &str, docs_json: &str) -> DbResult<Vec<Value>> {
        self.ensure_loaded(collection)?;

        let mut docs: Vec<Value> = serde_json::from_str(docs_json)?;

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
            .and_then(|lc| lc.as_loaded_mut())
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        for doc in &docs {
            col.insert(doc.clone())?;
        }

        let should_flush = self.buffer.mark_dirty(collection, docs.len());
        if should_flush {
            let key = self.get_key();
            file_storage::rewrite_collection(&self.data_path, collection, &col.documents, key.as_ref())?;
            self.buffer.remove_collection(collection);
        }

        self.invalidate_cache(collection);

        Ok(docs)
    }

    /// Find a document by id.
    pub fn find_by_id(&self, collection: &str, id: &str) -> DbResult<Value> {
        self.ensure_loaded(collection)?;

        let cols = self
            .collections
            .read()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get(collection)
            .and_then(|lc| lc.as_loaded())
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        let (_, doc) = col
            .find_by_id(id)
            .ok_or_else(|| DbError::DocumentNotFound(id.to_string()))?;

        Ok(doc.clone())
    }

    /// Update a document by id.
    pub fn update(&self, collection: &str, id: &str, doc_json: &str) -> DbResult<()> {
        self.ensure_loaded(collection)?;

        let mut new_doc: Value = serde_json::from_str(doc_json)?;
        new_doc["id"] = Value::String(id.to_string());

        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get_mut(collection)
            .and_then(|lc| lc.as_loaded_mut())
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        let (pos, _) = col
            .find_by_id(id)
            .ok_or_else(|| DbError::DocumentNotFound(id.to_string()))?;

        col.update(pos, new_doc)?;
        self.buffer.mark_dirty(collection, 1);
        self.invalidate_cache(collection);

        Ok(())
    }

    /// Delete a document by id.
    pub fn delete(&self, collection: &str, id: &str) -> DbResult<()> {
        self.ensure_loaded(collection)?;

        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get_mut(collection)
            .and_then(|lc| lc.as_loaded_mut())
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        let (pos, _) = col
            .find_by_id(id)
            .ok_or_else(|| DbError::DocumentNotFound(id.to_string()))?;

        col.delete(pos)?;
        self.buffer.mark_dirty(collection, 1);
        self.invalidate_cache(collection);

        Ok(())
    }

    /// Execute a query (with optional joins).
    pub fn query(&self, query_json: &str) -> DbResult<Vec<Value>> {
        let spec = QuerySpec::from_json(query_json)?;

        self.ensure_loaded(&spec.collection)?;
        for join in &spec.joins {
            self.ensure_loaded(&join.collection)?;
        }

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

        let primary_col = cols
            .get(&spec.collection)
            .and_then(|lc| lc.as_loaded())
            .ok_or_else(|| DbError::CollectionNotFound(spec.collection.clone()))?;

        let results = if spec.joins.is_empty() {
            qe::execute_query(&primary_col.documents, &spec)?
        } else {
            let mut col_map: HashMap<&str, &[Value]> = HashMap::new();
            for join in &spec.joins {
                let jcol = cols
                    .get(&join.collection)
                    .and_then(|lc| lc.as_loaded())
                    .ok_or_else(|| DbError::CollectionNotFound(join.collection.clone()))?;
                col_map.insert(&join.collection, &jcol.documents);
            }
            qe::execute_join_query(&primary_col.documents, &spec.joins, &col_map, &spec)?
        };

        if let Ok(result_json) = serde_json::to_string(&results) {
            if let Ok(mut cache) = self.cache.lock() {
                cache.put(cache_key, result_json);
            }
        }

        Ok(results)
    }

    /// Count documents matching filters.
    pub fn count(&self, collection: &str, filter_json: &str) -> DbResult<i64> {
        self.ensure_loaded(collection)?;

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
            .and_then(|lc| lc.as_loaded())
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
        self.ensure_loaded(collection)?;

        let key = self.get_key();
        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get_mut(collection)
            .and_then(|lc| lc.as_loaded_mut())
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        match index_type {
            "unique" => {
                col.add_unique_index(field)?;
                if let Some(idx) = col.unique_indexes.get(field) {
                    idx.save(&self.data_path, collection, key.as_ref())?;
                }
            }
            "range" => {
                col.add_range_index(field);
                if let Some(idx) = col.range_indexes.get(field) {
                    idx.save(&self.data_path, collection, key.as_ref())?;
                }
            }
            "hash" | _ => {
                col.add_hash_index(field);
                if let Some(idx) = col.hash_indexes.get(field) {
                    idx.save(&self.data_path, collection, key.as_ref())?;
                }
            }
        }

        Ok(())
    }

    /// Drop an index on a field.
    pub fn drop_index(&self, collection: &str, field: &str) -> DbResult<()> {
        self.ensure_loaded(collection)?;

        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get_mut(collection)
            .and_then(|lc| lc.as_loaded_mut())
            .ok_or_else(|| DbError::CollectionNotFound(collection.to_string()))?;

        col.drop_index(field);
        HashIndex::delete_file(&self.data_path, collection, field)?;
        UniqueIndex::delete_file(&self.data_path, collection, field)?;
        RangeIndex::delete_file(&self.data_path, collection, field)?;

        Ok(())
    }

    /// Set a schema on a collection.
    pub fn set_schema(&self, collection: &str, schema_json: &str) -> DbResult<()> {
        self.ensure_loaded(collection)?;

        let val: Value = serde_json::from_str(schema_json)?;
        let schema = Schema::from_value(&val)?;

        let mut cols = self
            .collections
            .write()
            .map_err(|e| DbError::LockError(e.to_string()))?;

        let col = cols
            .get_mut(collection)
            .and_then(|lc| lc.as_loaded_mut())
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

impl Drop for Engine {
    fn drop(&mut self) {
        // Signal the flush thread to stop
        {
            let (lock, cvar) = &*self.shutdown;
            let mut stop = lock.lock().unwrap();
            *stop = true;
            cvar.notify_one();
        }

        // Join the flush thread
        if let Ok(mut handle) = self.flush_thread.lock() {
            if let Some(h) = handle.take() {
                let _ = h.join();
            }
        }

        // Final flush
        if let Err(e) = self.flush() {
            log::error!("flush on drop failed: {}", e);
        }
    }
}
