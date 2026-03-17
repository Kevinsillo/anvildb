use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

/// Configuration for the write buffer.
pub struct WriteBufferConfig {
    /// Max pending inserts per collection before auto-flush (default 100).
    pub max_docs: usize,
    /// Background flush interval in seconds (default 5).
    pub flush_interval_secs: u64,
}

impl Default for WriteBufferConfig {
    fn default() -> Self {
        WriteBufferConfig {
            max_docs: 100,
            flush_interval_secs: 5,
        }
    }
}

/// Tracks which collections have pending (unflushed) writes.
/// The actual documents live in the in-memory `Collection.documents` —
/// the buffer only marks which collections need to be written to disk.
pub struct WriteBuffer {
    /// collection_name → number of pending inserts since last flush.
    dirty: Mutex<HashMap<String, usize>>,
    config: Mutex<WriteBufferConfig>,
}

impl WriteBuffer {
    pub fn new(config: WriteBufferConfig) -> Self {
        WriteBuffer {
            dirty: Mutex::new(HashMap::new()),
            config: Mutex::new(config),
        }
    }

    /// Mark a collection as dirty (has pending writes).
    /// Returns `true` if the threshold is reached and the caller should flush.
    pub fn mark_dirty(&self, collection: &str, count: usize) -> bool {
        let mut dirty = self.dirty.lock().unwrap();
        let entry = dirty.entry(collection.to_string()).or_insert(0);
        *entry += count;

        let max_docs = self.config.lock().unwrap().max_docs;
        *entry >= max_docs
    }

    /// Take the set of dirty collections and reset.
    pub fn take_dirty(&self) -> HashSet<String> {
        let mut dirty = self.dirty.lock().unwrap();
        let names: HashSet<String> = dirty.keys().cloned().collect();
        dirty.clear();
        names
    }

    /// Remove a collection from the dirty set (used on drop_collection).
    pub fn remove_collection(&self, collection: &str) {
        let mut dirty = self.dirty.lock().unwrap();
        dirty.remove(collection);
    }

    /// Get the current flush interval in seconds.
    pub fn flush_interval_secs(&self) -> u64 {
        self.config.lock().unwrap().flush_interval_secs
    }

    /// Reconfigure buffer thresholds.
    pub fn configure(&self, max_docs: usize, flush_interval_secs: u64) {
        let mut config = self.config.lock().unwrap();
        config.max_docs = max_docs;
        config.flush_interval_secs = flush_interval_secs;
    }
}
