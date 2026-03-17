use std::collections::{HashMap, VecDeque};

const MAX_ENTRIES: usize = 1000;

/// A simple LRU cache with a maximum number of entries.
/// Keys are query hashes (strings), values are cached JSON result strings.
#[derive(Debug)]
pub struct LruCache {
    map: HashMap<String, String>,
    order: VecDeque<String>,
    max_size: usize,
}

impl LruCache {
    pub fn new() -> Self {
        LruCache {
            map: HashMap::new(),
            order: VecDeque::new(),
            max_size: MAX_ENTRIES,
        }
    }

    /// Get a cached value. Moves the key to the front (most recently used).
    pub fn get(&mut self, key: &str) -> Option<&str> {
        if self.map.contains_key(key) {
            // Move to front
            self.order.retain(|k| k != key);
            self.order.push_front(key.to_string());
            self.map.get(key).map(|s| s.as_str())
        } else {
            None
        }
    }

    /// Insert a key-value pair. Evicts the least recently used entry if full.
    pub fn put(&mut self, key: String, value: String) {
        if self.map.contains_key(&key) {
            self.order.retain(|k| k != &key);
        } else if self.map.len() >= self.max_size {
            // Evict the least recently used
            if let Some(old_key) = self.order.pop_back() {
                self.map.remove(&old_key);
            }
        }
        self.order.push_front(key.clone());
        self.map.insert(key, value);
    }

    /// Invalidate all entries whose key starts with the given prefix.
    /// Used to invalidate cache when a collection is modified.
    pub fn invalidate_prefix(&mut self, prefix: &str) {
        let keys_to_remove: Vec<String> = self
            .map
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        for key in &keys_to_remove {
            self.map.remove(key);
        }
        self.order.retain(|k| !k.starts_with(prefix));
    }

    /// Clear all cached entries.
    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}

impl Default for LruCache {
    fn default() -> Self {
        Self::new()
    }
}
