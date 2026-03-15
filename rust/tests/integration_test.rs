use std::fs;

use anvildb::engine::Engine;

/// Create a temporary directory for test data and return its path.
fn temp_dir(test_name: &str) -> String {
    let dir = std::env::temp_dir()
        .join(format!("anvildb_test_{}_{}", test_name, std::process::id()));
    // Clean up any leftover from a previous run
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir.to_string_lossy().to_string()
}

/// Remove the temporary directory after a test.
fn cleanup(path: &str) {
    let _ = fs::remove_dir_all(path);
}

// ---------------------------------------------------------------------------
// 1. Engine lifecycle
// ---------------------------------------------------------------------------

#[test]
fn test_engine_open_and_close() {
    let path = temp_dir("lifecycle");
    {
        let engine = Engine::open(&path).expect("Engine::open should succeed");
        // Engine exists and can list (empty) collections
        let cols = engine.list_collections().unwrap();
        assert!(cols.is_empty());
    }
    // Opening again should reload state
    {
        let engine = Engine::open(&path).expect("Engine::open again should succeed");
        let cols = engine.list_collections().unwrap();
        assert!(cols.is_empty());
    }
    cleanup(&path);
}

// ---------------------------------------------------------------------------
// 2. Collection CRUD
// ---------------------------------------------------------------------------

#[test]
fn test_create_and_drop_collection() {
    let path = temp_dir("col_crud");
    let engine = Engine::open(&path).unwrap();

    engine.create_collection("users").unwrap();
    let cols = engine.list_collections().unwrap();
    assert_eq!(cols, vec!["users"]);

    engine.drop_collection("users").unwrap();
    let cols = engine.list_collections().unwrap();
    assert!(cols.is_empty());

    cleanup(&path);
}

#[test]
fn test_insert_find_update_delete() {
    let path = temp_dir("doc_crud");
    let engine = Engine::open(&path).unwrap();
    engine.create_collection("items").unwrap();

    // Insert
    let doc = engine
        .insert("items", r#"{"name":"Widget","price":9.99}"#)
        .unwrap();
    let id = doc["id"].as_str().expect("inserted doc should have an id");

    // Find by id
    let found = engine.find_by_id("items", id).unwrap();
    assert_eq!(found["name"], "Widget");
    assert_eq!(found["price"], 9.99);

    // Update
    engine
        .update("items", id, r#"{"name":"Gadget","price":19.99}"#)
        .unwrap();
    let updated = engine.find_by_id("items", id).unwrap();
    assert_eq!(updated["name"], "Gadget");
    assert_eq!(updated["price"], 19.99);

    // Delete
    engine.delete("items", id).unwrap();
    let result = engine.find_by_id("items", id);
    assert!(result.is_err(), "finding deleted doc should fail");

    cleanup(&path);
}

// ---------------------------------------------------------------------------
// 3. Query engine (filters, sort, limit, offset)
// ---------------------------------------------------------------------------

#[test]
fn test_query_filters() {
    let path = temp_dir("query_filters");
    let engine = Engine::open(&path).unwrap();
    engine.create_collection("products").unwrap();

    engine.insert("products", r#"{"id":"1","name":"Apple","price":1.5,"tags":["fruit","red"]}"#).unwrap();
    engine.insert("products", r#"{"id":"2","name":"Banana","price":0.75,"tags":["fruit","yellow"]}"#).unwrap();
    engine.insert("products", r#"{"id":"3","name":"Carrot","price":2.0,"tags":["vegetable","orange"]}"#).unwrap();
    engine.insert("products", r#"{"id":"4","name":"Dragonfruit","price":5.0,"tags":["fruit","pink"]}"#).unwrap();
    engine.insert("products", r#"{"id":"5","name":"Eggplant","price":3.0,"tags":["vegetable","purple"]}"#).unwrap();

    // = filter
    let results = engine.query(r#"{"collection":"products","filters":[{"field":"name","op":"=","value":"Apple"}]}"#).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["name"], "Apple");

    // != filter
    let results = engine.query(r#"{"collection":"products","filters":[{"field":"name","op":"!=","value":"Apple"}]}"#).unwrap();
    assert_eq!(results.len(), 4);

    // > filter
    let results = engine.query(r#"{"collection":"products","filters":[{"field":"price","op":">","value":2.0}]}"#).unwrap();
    assert_eq!(results.len(), 2); // Dragonfruit (5.0) and Eggplant (3.0)

    // < filter
    let results = engine.query(r#"{"collection":"products","filters":[{"field":"price","op":"<","value":2.0}]}"#).unwrap();
    assert_eq!(results.len(), 2); // Apple (1.5) and Banana (0.75)

    // >= filter
    let results = engine.query(r#"{"collection":"products","filters":[{"field":"price","op":">=","value":2.0}]}"#).unwrap();
    assert_eq!(results.len(), 3); // Carrot, Dragonfruit, Eggplant

    // <= filter
    let results = engine.query(r#"{"collection":"products","filters":[{"field":"price","op":"<=","value":2.0}]}"#).unwrap();
    assert_eq!(results.len(), 3); // Apple, Banana, Carrot

    // contains (string)
    let results = engine.query(r#"{"collection":"products","filters":[{"field":"name","op":"contains","value":"an"}]}"#).unwrap();
    assert_eq!(results.len(), 2); // Banana, Eggplant

    // contains (array)
    let results = engine.query(r#"{"collection":"products","filters":[{"field":"tags","op":"contains","value":"vegetable"}]}"#).unwrap();
    assert_eq!(results.len(), 2); // Carrot, Eggplant

    cleanup(&path);
}

#[test]
fn test_query_sort_limit_offset() {
    let path = temp_dir("query_paging");
    let engine = Engine::open(&path).unwrap();
    engine.create_collection("nums").unwrap();

    for i in 1..=5 {
        engine
            .insert("nums", &format!(r#"{{"id":"{}","val":{}}}"#, i, i))
            .unwrap();
    }

    // Sort ascending
    let results = engine
        .query(r#"{"collection":"nums","order_by":{"field":"val","dir":"asc"}}"#)
        .unwrap();
    let vals: Vec<i64> = results.iter().map(|d| d["val"].as_i64().unwrap()).collect();
    assert_eq!(vals, vec![1, 2, 3, 4, 5]);

    // Sort descending
    let results = engine
        .query(r#"{"collection":"nums","order_by":{"field":"val","dir":"desc"}}"#)
        .unwrap();
    let vals: Vec<i64> = results.iter().map(|d| d["val"].as_i64().unwrap()).collect();
    assert_eq!(vals, vec![5, 4, 3, 2, 1]);

    // Limit
    let results = engine
        .query(r#"{"collection":"nums","order_by":{"field":"val","dir":"asc"},"limit":3}"#)
        .unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0]["val"], 1);

    // Offset + limit
    let results = engine
        .query(r#"{"collection":"nums","order_by":{"field":"val","dir":"asc"},"limit":2,"offset":2}"#)
        .unwrap();
    assert_eq!(results.len(), 2);
    let vals: Vec<i64> = results.iter().map(|d| d["val"].as_i64().unwrap()).collect();
    assert_eq!(vals, vec![3, 4]);

    cleanup(&path);
}

// ---------------------------------------------------------------------------
// 4. Index operations
// ---------------------------------------------------------------------------

#[test]
fn test_hash_index() {
    let path = temp_dir("hash_index");
    let engine = Engine::open(&path).unwrap();
    engine.create_collection("idx_test").unwrap();

    engine
        .insert("idx_test", r#"{"id":"1","email":"a@b.com"}"#)
        .unwrap();
    engine
        .insert("idx_test", r#"{"id":"2","email":"c@d.com"}"#)
        .unwrap();

    // Create hash index on email
    engine
        .create_index("idx_test", "email", "hash")
        .unwrap();

    // Queries should still work with the index in place
    let results = engine
        .query(r#"{"collection":"idx_test","filters":[{"field":"email","op":"=","value":"a@b.com"}]}"#)
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["email"], "a@b.com");

    cleanup(&path);
}

#[test]
fn test_unique_index() {
    let path = temp_dir("unique_index");
    let engine = Engine::open(&path).unwrap();
    engine.create_collection("uniq_test").unwrap();

    engine
        .insert("uniq_test", r#"{"id":"1","email":"a@b.com"}"#)
        .unwrap();

    // Create unique index on email
    engine
        .create_index("uniq_test", "email", "unique")
        .unwrap();

    // Inserting a duplicate email should fail
    let result = engine.insert("uniq_test", r#"{"id":"2","email":"a@b.com"}"#);
    assert!(result.is_err(), "duplicate unique key should fail");

    // Inserting a different email should succeed
    engine
        .insert("uniq_test", r#"{"id":"3","email":"x@y.com"}"#)
        .unwrap();

    // Find by id still works after index is created
    let doc = engine.find_by_id("uniq_test", "1").unwrap();
    assert_eq!(doc["email"], "a@b.com");

    cleanup(&path);
}

#[test]
fn test_query_with_index() {
    let path = temp_dir("query_with_idx");
    let engine = Engine::open(&path).unwrap();
    engine.create_collection("indexed").unwrap();

    // Insert docs first, then create index
    engine
        .insert("indexed", r#"{"id":"1","category":"A","val":10}"#)
        .unwrap();
    engine
        .insert("indexed", r#"{"id":"2","category":"B","val":20}"#)
        .unwrap();
    engine
        .insert("indexed", r#"{"id":"3","category":"A","val":30}"#)
        .unwrap();

    engine.create_index("indexed", "category", "hash").unwrap();

    // Query filtering on the indexed field
    let results = engine
        .query(r#"{"collection":"indexed","filters":[{"field":"category","op":"=","value":"A"}]}"#)
        .unwrap();
    assert_eq!(results.len(), 2);

    cleanup(&path);
}

// ---------------------------------------------------------------------------
// 5. Schema validation
// ---------------------------------------------------------------------------

#[test]
fn test_schema_valid_insert() {
    let path = temp_dir("schema_valid");
    let engine = Engine::open(&path).unwrap();
    engine.create_collection("typed").unwrap();

    engine
        .set_schema("typed", r#"{"name":"string","age":"int"}"#)
        .unwrap();

    // Valid document
    let doc = engine
        .insert("typed", r#"{"name":"Alice","age":30}"#)
        .unwrap();
    assert_eq!(doc["name"], "Alice");
    assert_eq!(doc["age"], 30);

    cleanup(&path);
}

#[test]
fn test_schema_invalid_insert() {
    let path = temp_dir("schema_invalid");
    let engine = Engine::open(&path).unwrap();
    engine.create_collection("typed2").unwrap();

    engine
        .set_schema("typed2", r#"{"name":"string","age":"int"}"#)
        .unwrap();

    // Invalid document: age is a string instead of int
    let result = engine.insert("typed2", r#"{"name":"Bob","age":"not_a_number"}"#);
    assert!(result.is_err(), "inserting invalid doc should fail");

    cleanup(&path);
}

// ---------------------------------------------------------------------------
// 6. Bulk insert
// ---------------------------------------------------------------------------

#[test]
fn test_bulk_insert() {
    let path = temp_dir("bulk");
    let engine = Engine::open(&path).unwrap();
    engine.create_collection("bulk_col").unwrap();

    let docs = engine
        .bulk_insert(
            "bulk_col",
            r#"[{"name":"A"},{"name":"B"},{"name":"C"}]"#,
        )
        .unwrap();
    assert_eq!(docs.len(), 3);

    // All docs should have auto-generated ids
    for doc in &docs {
        assert!(doc.get("id").is_some());
        assert!(doc["id"].is_string());
    }

    // Count should reflect all inserted docs
    let count = engine.count("bulk_col", "").unwrap();
    assert_eq!(count, 3);

    cleanup(&path);
}

// ---------------------------------------------------------------------------
// 7. List collections
// ---------------------------------------------------------------------------

#[test]
fn test_list_collections() {
    let path = temp_dir("list_cols");
    let engine = Engine::open(&path).unwrap();

    engine.create_collection("alpha").unwrap();
    engine.create_collection("beta").unwrap();
    engine.create_collection("gamma").unwrap();

    let cols = engine.list_collections().unwrap();
    // list_collections sorts the names
    assert_eq!(cols, vec!["alpha", "beta", "gamma"]);

    cleanup(&path);
}

// ---------------------------------------------------------------------------
// 8. Count
// ---------------------------------------------------------------------------

#[test]
fn test_count_all() {
    let path = temp_dir("count_all");
    let engine = Engine::open(&path).unwrap();
    engine.create_collection("counter").unwrap();

    assert_eq!(engine.count("counter", "").unwrap(), 0);

    engine.insert("counter", r#"{"val":1}"#).unwrap();
    engine.insert("counter", r#"{"val":2}"#).unwrap();
    engine.insert("counter", r#"{"val":3}"#).unwrap();

    assert_eq!(engine.count("counter", "").unwrap(), 3);

    cleanup(&path);
}

#[test]
fn test_count_with_filter() {
    let path = temp_dir("count_filter");
    let engine = Engine::open(&path).unwrap();
    engine.create_collection("filtered").unwrap();

    engine.insert("filtered", r#"{"status":"active"}"#).unwrap();
    engine.insert("filtered", r#"{"status":"active"}"#).unwrap();
    engine
        .insert("filtered", r#"{"status":"inactive"}"#)
        .unwrap();

    let count = engine
        .count(
            "filtered",
            r#"[{"field":"status","op":"=","value":"active"}]"#,
        )
        .unwrap();
    assert_eq!(count, 2);

    cleanup(&path);
}
