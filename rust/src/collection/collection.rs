use std::collections::HashMap;

use serde_json::Value;

use crate::error::{DbError, DbResult};
use crate::index::hash_index::HashIndex;
use crate::index::unique_index::UniqueIndex;
use crate::validation::schema::Schema;

/// Represents an in-memory collection of JSON documents.
#[derive(Debug)]
pub struct Collection {
    pub name: String,
    pub documents: Vec<Value>,
    pub hash_indexes: HashMap<String, HashIndex>,
    pub unique_indexes: HashMap<String, UniqueIndex>,
    pub schema: Option<Schema>,
}

impl Collection {
    pub fn new(name: &str) -> Self {
        Collection {
            name: name.to_string(),
            documents: Vec::new(),
            hash_indexes: HashMap::new(),
            unique_indexes: HashMap::new(),
            schema: None,
        }
    }

    /// Load documents into this collection (from storage).
    pub fn load(&mut self, docs: Vec<Value>) {
        self.documents = docs;
    }

    /// Insert a document. Validates against schema and indexes.
    /// Returns the position of the inserted document.
    pub fn insert(&mut self, doc: Value) -> DbResult<usize> {
        // Validate schema
        if let Some(ref schema) = self.schema {
            schema.validate(&doc)?;
        }

        let pos = self.documents.len();

        // Check unique indexes
        for (_, idx) in &self.unique_indexes {
            if let Some(val) = doc.get(&idx.field) {
                idx.check_unique(val)?;
            }
        }

        self.documents.push(doc.clone());

        // Update indexes
        self.update_indexes_for_insert(pos, &doc);

        Ok(pos)
    }

    /// Find a document by its "id" field. Returns a reference and its position.
    pub fn find_by_id(&self, id: &str) -> Option<(usize, &Value)> {
        // Check if we have a hash or unique index on "id"
        let id_val = Value::String(id.to_string());

        if let Some(idx) = self.unique_indexes.get("id") {
            if let Some(pos) = idx.lookup(&id_val) {
                return self.documents.get(pos).map(|d| (pos, d));
            }
            return None;
        }

        if let Some(idx) = self.hash_indexes.get("id") {
            let positions = idx.lookup(&id_val);
            if let Some(&pos) = positions.first() {
                return self.documents.get(pos).map(|d| (pos, d));
            }
            return None;
        }

        // Linear scan fallback
        for (i, doc) in self.documents.iter().enumerate() {
            if let Some(Value::String(doc_id)) = doc.get("id") {
                if doc_id == id {
                    return Some((i, doc));
                }
            }
        }
        None
    }

    /// Update a document at a given position. Validates against schema and indexes.
    pub fn update(&mut self, pos: usize, new_doc: Value) -> DbResult<()> {
        if pos >= self.documents.len() {
            return Err(DbError::DocumentNotFound(format!("position {}", pos)));
        }

        // Validate schema
        if let Some(ref schema) = self.schema {
            schema.validate(&new_doc)?;
        }

        // Check unique indexes (allowing same position)
        for (_, idx) in &self.unique_indexes {
            if let Some(val) = new_doc.get(&idx.field) {
                idx.check_unique_except(val, pos)?;
            }
        }

        self.documents[pos] = new_doc;

        // Rebuild all indexes (simpler than incremental update)
        self.rebuild_all_indexes()?;

        Ok(())
    }

    /// Delete a document at a given position.
    pub fn delete(&mut self, pos: usize) -> DbResult<Value> {
        if pos >= self.documents.len() {
            return Err(DbError::DocumentNotFound(format!("position {}", pos)));
        }

        let removed = self.documents.remove(pos);

        // Rebuild indexes since positions shifted
        self.rebuild_all_indexes()?;

        Ok(removed)
    }

    /// Add a hash index on a field.
    pub fn add_hash_index(&mut self, field: &str) {
        let mut idx = HashIndex::new(field);
        idx.rebuild(&self.documents);
        self.hash_indexes.insert(field.to_string(), idx);
    }

    /// Add a unique index on a field.
    pub fn add_unique_index(&mut self, field: &str) -> DbResult<()> {
        let mut idx = UniqueIndex::new(field);
        idx.rebuild(&self.documents)?;
        self.unique_indexes.insert(field.to_string(), idx);
        Ok(())
    }

    /// Drop an index on a field (hash or unique).
    pub fn drop_index(&mut self, field: &str) -> bool {
        let a = self.hash_indexes.remove(field).is_some();
        let b = self.unique_indexes.remove(field).is_some();
        a || b
    }

    /// Set a schema for this collection.
    pub fn set_schema(&mut self, schema: Schema) {
        self.schema = Some(schema);
    }

    /// Rebuild all indexes from current documents.
    fn rebuild_all_indexes(&mut self) -> DbResult<()> {
        for (_, idx) in &mut self.hash_indexes {
            idx.rebuild(&self.documents);
        }
        for (_, idx) in &mut self.unique_indexes {
            idx.rebuild(&self.documents)?;
        }
        Ok(())
    }

    /// Update indexes after inserting a document at position `pos`.
    fn update_indexes_for_insert(&mut self, pos: usize, doc: &Value) {
        for (_, idx) in &mut self.hash_indexes {
            if let Some(val) = doc.get(&idx.field) {
                let key = crate::index::hash_index::value_to_index_key(val);
                idx.entries.entry(key).or_default().push(pos);
            }
        }
        for (_, idx) in &mut self.unique_indexes {
            if let Some(val) = doc.get(&idx.field) {
                let key = crate::index::hash_index::value_to_index_key(val);
                idx.entries.insert(key, pos);
            }
        }
    }
}
