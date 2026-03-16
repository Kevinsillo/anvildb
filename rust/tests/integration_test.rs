use std::fs;

use anvildb::engine::Engine;
use serde_json::Value;

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
        let engine = Engine::open(&path, None).expect("Engine::open should succeed");
        // Engine exists and can list (empty) collections
        let cols = engine.list_collections().unwrap();
        assert!(cols.is_empty());
    }
    // Opening again should reload state
    {
        let engine = Engine::open(&path, None).expect("Engine::open again should succeed");
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
    let engine = Engine::open(&path, None).unwrap();

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
    let engine = Engine::open(&path, None).unwrap();
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
    let engine = Engine::open(&path, None).unwrap();
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
    let engine = Engine::open(&path, None).unwrap();
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
    let engine = Engine::open(&path, None).unwrap();
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
    let engine = Engine::open(&path, None).unwrap();
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
    let engine = Engine::open(&path, None).unwrap();
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
    let engine = Engine::open(&path, None).unwrap();
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
    let engine = Engine::open(&path, None).unwrap();
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
    let engine = Engine::open(&path, None).unwrap();
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
    let engine = Engine::open(&path, None).unwrap();

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
    let engine = Engine::open(&path, None).unwrap();
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
    let engine = Engine::open(&path, None).unwrap();
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

// ---------------------------------------------------------------------------
// 9. Write buffering
// ---------------------------------------------------------------------------

#[test]
fn test_buffered_insert_visible_before_flush() {
    let path = temp_dir("buf_visible");
    let engine = Engine::open(&path, None).unwrap();
    engine.create_collection("items").unwrap();

    // Insert without explicit flush — doc should be visible in memory
    let doc = engine.insert("items", r#"{"name":"Buffered"}"#).unwrap();
    let id = doc["id"].as_str().unwrap();

    let found = engine.find_by_id("items", id).unwrap();
    assert_eq!(found["name"], "Buffered");

    cleanup(&path);
}

#[test]
fn test_flush_persists_to_disk() {
    let path = temp_dir("buf_flush");
    {
        let engine = Engine::open(&path, None).unwrap();
        engine.create_collection("persist").unwrap();

        engine.insert("persist", r#"{"id":"1","name":"Alpha"}"#).unwrap();
        engine.insert("persist", r#"{"id":"2","name":"Beta"}"#).unwrap();

        // Explicit flush
        engine.flush().unwrap();
    }

    // Reopen engine and verify data survived
    {
        let engine = Engine::open(&path, None).unwrap();
        let doc = engine.find_by_id("persist", "1").unwrap();
        assert_eq!(doc["name"], "Alpha");
        let doc = engine.find_by_id("persist", "2").unwrap();
        assert_eq!(doc["name"], "Beta");
    }

    cleanup(&path);
}

#[test]
fn test_flush_collection_persists() {
    let path = temp_dir("buf_flush_col");
    {
        let engine = Engine::open(&path, None).unwrap();
        engine.create_collection("col_a").unwrap();
        engine.create_collection("col_b").unwrap();

        engine.insert("col_a", r#"{"id":"1","v":"a"}"#).unwrap();
        engine.insert("col_b", r#"{"id":"1","v":"b"}"#).unwrap();

        // Only flush col_a
        engine.flush_collection("col_a").unwrap();
    }

    // Reopen — col_a should have data, col_b may not (was only in buffer)
    {
        let engine = Engine::open(&path, None).unwrap();
        let doc = engine.find_by_id("col_a", "1").unwrap();
        assert_eq!(doc["v"], "a");
    }

    cleanup(&path);
}

#[test]
fn test_threshold_auto_flush() {
    let path = temp_dir("buf_threshold");
    {
        let engine = Engine::open(&path, None).unwrap();
        engine.create_collection("thresh").unwrap();

        // Set threshold to 3 docs
        engine.configure_buffer(3, 60);

        // Insert 4 docs — first 3 should trigger auto-flush, 4th stays in buffer
        engine.insert("thresh", r#"{"id":"1","v":1}"#).unwrap();
        engine.insert("thresh", r#"{"id":"2","v":2}"#).unwrap();
        engine.insert("thresh", r#"{"id":"3","v":3}"#).unwrap();
        // At this point, threshold was reached and 3 docs flushed to disk
        engine.insert("thresh", r#"{"id":"4","v":4}"#).unwrap();
        // 4th doc is still in buffer — flush it
        engine.flush().unwrap();
    }

    // Reopen and verify all 4 docs are persisted
    {
        let engine = Engine::open(&path, None).unwrap();
        assert_eq!(engine.count("thresh", "").unwrap(), 4);
    }

    cleanup(&path);
}

#[test]
fn test_shutdown_flushes_buffer() {
    let path = temp_dir("buf_shutdown");
    {
        let engine = Engine::open(&path, None).unwrap();
        engine.create_collection("shut").unwrap();

        engine.insert("shut", r#"{"id":"1","name":"Persisted"}"#).unwrap();
        // Drop triggers shutdown → flush
    }

    // Reopen and verify
    {
        let engine = Engine::open(&path, None).unwrap();
        let doc = engine.find_by_id("shut", "1").unwrap();
        assert_eq!(doc["name"], "Persisted");
    }

    cleanup(&path);
}

#[test]
fn test_update_after_buffered_insert() {
    let path = temp_dir("buf_update");
    {
        let engine = Engine::open(&path, None).unwrap();
        engine.create_collection("upd").unwrap();

        let doc = engine.insert("upd", r#"{"name":"Original"}"#).unwrap();
        let id = doc["id"].as_str().unwrap();

        // Update while insert is still buffered — triggers rewrite which persists everything
        engine.update("upd", id, r#"{"name":"Updated"}"#).unwrap();
    }

    // Reopen and verify
    {
        let engine = Engine::open(&path, None).unwrap();
        let count = engine.count("upd", "").unwrap();
        assert_eq!(count, 1);
        let results = engine.query(r#"{"collection":"upd"}"#).unwrap();
        assert_eq!(results[0]["name"], "Updated");
    }

    cleanup(&path);
}

#[test]
fn test_delete_after_buffered_insert() {
    let path = temp_dir("buf_delete");
    {
        let engine = Engine::open(&path, None).unwrap();
        engine.create_collection("del").unwrap();

        let doc = engine.insert("del", r#"{"name":"ToDelete"}"#).unwrap();
        let id = doc["id"].as_str().unwrap();

        // Delete while insert is still buffered
        engine.delete("del", id).unwrap();
    }

    // Reopen and verify — should be empty
    {
        let engine = Engine::open(&path, None).unwrap();
        let count = engine.count("del", "").unwrap();
        assert_eq!(count, 0);
    }

    cleanup(&path);
}

#[test]
fn test_drop_collection_clears_buffer() {
    let path = temp_dir("buf_drop_col");
    let engine = Engine::open(&path, None).unwrap();
    engine.create_collection("temp").unwrap();

    engine.insert("temp", r#"{"id":"1","v":1}"#).unwrap();

    // Drop the collection — buffer should be cleared, no orphaned writes
    engine.drop_collection("temp").unwrap();

    let cols = engine.list_collections().unwrap();
    assert!(!cols.contains(&"temp".to_string()));

    cleanup(&path);
}

// ---------------------------------------------------------------------------
// 10. Joins
// ---------------------------------------------------------------------------

/// Helper: set up users + orders for join tests.
fn setup_join_data(engine: &Engine) {
    engine.create_collection("users").unwrap();
    engine.create_collection("orders").unwrap();

    engine.insert("users", r#"{"id":"u1","name":"Alice","status":"active"}"#).unwrap();
    engine.insert("users", r#"{"id":"u2","name":"Bob","status":"inactive"}"#).unwrap();
    engine.insert("users", r#"{"id":"u3","name":"Charlie","status":"active"}"#).unwrap();

    engine.insert("orders", r#"{"id":"o1","user_id":"u1","product":"Laptop","total":999}"#).unwrap();
    engine.insert("orders", r#"{"id":"o2","user_id":"u1","product":"Mouse","total":25}"#).unwrap();
    engine.insert("orders", r#"{"id":"o3","user_id":"u2","product":"Keyboard","total":75}"#).unwrap();
    // Note: u3 (Charlie) has no orders
}

#[test]
fn test_inner_join() {
    let path = temp_dir("join_inner");
    let engine = Engine::open(&path, None).unwrap();
    setup_join_data(&engine);

    let results = engine.query(r#"{
        "collection": "orders",
        "joins": [{"collection": "users", "left_field": "user_id", "right_field": "id"}]
    }"#).unwrap();

    // 3 orders, all have matching users → 3 results
    assert_eq!(results.len(), 3);

    // Verify joined fields are prefixed with "users_"
    assert_eq!(results[0]["users_name"], "Alice");
    assert_eq!(results[0]["product"], "Laptop");
    assert_eq!(results[2]["users_name"], "Bob");

    cleanup(&path);
}

#[test]
fn test_left_join() {
    let path = temp_dir("join_left");
    let engine = Engine::open(&path, None).unwrap();
    setup_join_data(&engine);

    // LEFT JOIN: users left join orders — Charlie (u3) has no orders but should appear
    let results = engine.query(r#"{
        "collection": "users",
        "joins": [{"collection": "orders", "join_type": "left", "left_field": "id", "right_field": "user_id"}]
    }"#).unwrap();

    // Alice has 2 orders, Bob has 1, Charlie has 0 → 2 + 1 + 1 = 4 rows
    assert_eq!(results.len(), 4);

    // Charlie should be in the results without order fields
    let charlie_rows: Vec<&Value> = results.iter()
        .filter(|r| r["name"] == "Charlie")
        .collect();
    assert_eq!(charlie_rows.len(), 1);
    assert!(charlie_rows[0].get("orders_product").is_none());

    cleanup(&path);
}

#[test]
fn test_join_with_filter_on_joined_field() {
    let path = temp_dir("join_filter");
    let engine = Engine::open(&path, None).unwrap();
    setup_join_data(&engine);

    // Join orders with users, then filter by users_name
    let results = engine.query(r#"{
        "collection": "orders",
        "joins": [{"collection": "users", "left_field": "user_id", "right_field": "id"}],
        "filters": [{"field": "users_name", "op": "=", "value": "Alice"}]
    }"#).unwrap();

    assert_eq!(results.len(), 2); // Alice has 2 orders
    assert_eq!(results[0]["users_name"], "Alice");
    assert_eq!(results[1]["users_name"], "Alice");

    cleanup(&path);
}

#[test]
fn test_join_with_sort() {
    let path = temp_dir("join_sort");
    let engine = Engine::open(&path, None).unwrap();
    setup_join_data(&engine);

    let results = engine.query(r#"{
        "collection": "orders",
        "joins": [{"collection": "users", "left_field": "user_id", "right_field": "id"}],
        "order_by": {"field": "total", "dir": "desc"}
    }"#).unwrap();

    assert_eq!(results.len(), 3);
    assert_eq!(results[0]["total"], 999);  // Laptop
    assert_eq!(results[1]["total"], 75);   // Keyboard
    assert_eq!(results[2]["total"], 25);   // Mouse

    cleanup(&path);
}

#[test]
fn test_join_with_limit_offset() {
    let path = temp_dir("join_paging");
    let engine = Engine::open(&path, None).unwrap();
    setup_join_data(&engine);

    let results = engine.query(r#"{
        "collection": "orders",
        "joins": [{"collection": "users", "left_field": "user_id", "right_field": "id"}],
        "order_by": {"field": "total", "dir": "asc"},
        "limit": 2,
        "offset": 1
    }"#).unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["total"], 75);   // Keyboard (offset skipped Mouse at 25)
    assert_eq!(results[1]["total"], 999);  // Laptop

    cleanup(&path);
}

#[test]
fn test_join_custom_prefix() {
    let path = temp_dir("join_prefix");
    let engine = Engine::open(&path, None).unwrap();
    setup_join_data(&engine);

    let results = engine.query(r#"{
        "collection": "orders",
        "joins": [{"collection": "users", "left_field": "user_id", "right_field": "id", "prefix": "u_"}]
    }"#).unwrap();

    assert_eq!(results.len(), 3);
    // Fields should use "u_" prefix instead of "users_"
    assert_eq!(results[0]["u_name"], "Alice");
    assert!(results[0].get("users_name").is_none());

    cleanup(&path);
}

#[test]
fn test_multiple_joins() {
    let path = temp_dir("join_multi");
    let engine = Engine::open(&path, None).unwrap();

    engine.create_collection("order_items").unwrap();
    engine.create_collection("orders").unwrap();
    engine.create_collection("products").unwrap();

    engine.insert("products", r#"{"id":"p1","name":"Laptop","price":999}"#).unwrap();
    engine.insert("products", r#"{"id":"p2","name":"Mouse","price":25}"#).unwrap();

    engine.insert("orders", r#"{"id":"o1","customer":"Alice"}"#).unwrap();

    engine.insert("order_items", r#"{"id":"oi1","order_id":"o1","product_id":"p1","qty":1}"#).unwrap();
    engine.insert("order_items", r#"{"id":"oi2","order_id":"o1","product_id":"p2","qty":3}"#).unwrap();

    // Three-way join: order_items → orders + products
    let results = engine.query(r#"{
        "collection": "order_items",
        "joins": [
            {"collection": "orders", "left_field": "order_id", "right_field": "id", "prefix": "order_"},
            {"collection": "products", "left_field": "product_id", "right_field": "id", "prefix": "product_"}
        ]
    }"#).unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["order_customer"], "Alice");
    assert_eq!(results[0]["product_name"], "Laptop");
    assert_eq!(results[1]["product_name"], "Mouse");

    cleanup(&path);
}

#[test]
fn test_inner_join_no_matches() {
    let path = temp_dir("join_empty");
    let engine = Engine::open(&path, None).unwrap();

    engine.create_collection("a").unwrap();
    engine.create_collection("b").unwrap();

    engine.insert("a", r#"{"id":"1","ref":"x"}"#).unwrap();
    engine.insert("b", r#"{"id":"1","ref":"y"}"#).unwrap();

    // Inner join with no matching keys → empty result
    let results = engine.query(r#"{
        "collection": "a",
        "joins": [{"collection": "b", "left_field": "ref", "right_field": "ref"}]
    }"#).unwrap();

    assert_eq!(results.len(), 0);

    cleanup(&path);
}

#[test]
fn test_join_collection_not_found() {
    let path = temp_dir("join_not_found");
    let engine = Engine::open(&path, None).unwrap();

    engine.create_collection("exists").unwrap();
    engine.insert("exists", r#"{"id":"1","ref":"x"}"#).unwrap();

    let result = engine.query(r#"{
        "collection": "exists",
        "joins": [{"collection": "nonexistent", "left_field": "ref", "right_field": "id"}]
    }"#);

    assert!(result.is_err());

    cleanup(&path);
}

// ---------------------------------------------------------------------------
// 11. Lazy loading
// ---------------------------------------------------------------------------

#[test]
fn test_lazy_loading_list_without_loading() {
    let path = temp_dir("lazy_list");

    // Create collections and insert data, then close
    {
        let engine = Engine::open(&path, None).unwrap();
        engine.create_collection("users").unwrap();
        engine.create_collection("orders").unwrap();
        engine.insert("users", r#"{"id":"1","name":"Alice"}"#).unwrap();
        engine.insert("orders", r#"{"id":"1","total":100}"#).unwrap();
        engine.flush().unwrap();
    }

    // Reopen — collections should be discovered but not loaded
    {
        let engine = Engine::open(&path, None).unwrap();

        // list_collections should return names without loading data
        let cols = engine.list_collections().unwrap();
        assert_eq!(cols, vec!["orders", "users"]);

        // Now access one — should lazy-load it
        let doc = engine.find_by_id("users", "1").unwrap();
        assert_eq!(doc["name"], "Alice");
    }

    cleanup(&path);
}

#[test]
fn test_lazy_loading_only_loads_accessed_collections() {
    let path = temp_dir("lazy_partial");

    // Create 3 collections
    {
        let engine = Engine::open(&path, None).unwrap();
        engine.create_collection("a").unwrap();
        engine.create_collection("b").unwrap();
        engine.create_collection("c").unwrap();
        engine.insert("a", r#"{"id":"1","v":"a"}"#).unwrap();
        engine.insert("b", r#"{"id":"1","v":"b"}"#).unwrap();
        engine.insert("c", r#"{"id":"1","v":"c"}"#).unwrap();
        engine.flush().unwrap();
    }

    // Reopen and only access collection "b"
    {
        let engine = Engine::open(&path, None).unwrap();

        // All 3 should be listed
        assert_eq!(engine.list_collections().unwrap().len(), 3);

        // Only access "b"
        let doc = engine.find_by_id("b", "1").unwrap();
        assert_eq!(doc["v"], "b");

        // "a" and "c" should still be unloaded in the internal map
        let cols = engine.collections.read().unwrap();
        assert!(cols.get("a").unwrap().as_loaded().is_none());
        assert!(cols.get("b").unwrap().as_loaded().is_some());
        assert!(cols.get("c").unwrap().as_loaded().is_none());
    }

    cleanup(&path);
}

#[test]
fn test_lazy_loading_with_join() {
    let path = temp_dir("lazy_join");

    // Create and populate
    {
        let engine = Engine::open(&path, None).unwrap();
        engine.create_collection("users").unwrap();
        engine.create_collection("orders").unwrap();
        engine.insert("users", r#"{"id":"u1","name":"Alice"}"#).unwrap();
        engine.insert("orders", r#"{"id":"o1","user_id":"u1","total":42}"#).unwrap();
        engine.flush().unwrap();
    }

    // Reopen — both collections should lazy-load when the join executes
    {
        let engine = Engine::open(&path, None).unwrap();

        let results = engine.query(r#"{
            "collection": "orders",
            "joins": [{"collection": "users", "left_field": "user_id", "right_field": "id", "prefix": "u_"}]
        }"#).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["u_name"], "Alice");
        assert_eq!(results[0]["total"], 42);
    }

    cleanup(&path);
}

// ---------------------------------------------------------------------------
// 12. Compression + Encryption
// ---------------------------------------------------------------------------

const TEST_KEY: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
    0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
    0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];

#[test]
fn test_compression_transparent() {
    // Data should be compressed on disk (smaller than plain NDJSON)
    let path = temp_dir("compress");
    {
        let engine = Engine::open(&path, None).unwrap();
        engine.create_collection("data").unwrap();
        // Insert repetitive data that compresses well
        for i in 0..100 {
            engine.insert("data", &format!(r#"{{"id":"{}","name":"User {}","status":"active","role":"admin"}}"#, i, i)).unwrap();
        }
        engine.flush().unwrap();
    }

    // Verify the .anvil file exists and is smaller than equivalent NDJSON would be
    let anvil_path = std::path::Path::new(&path).join("collections").join("data.anvil");
    assert!(anvil_path.exists());
    let file_size = fs::metadata(&anvil_path).unwrap().len();
    // 100 docs * ~70 bytes each = ~7000 bytes as NDJSON. Compressed should be much less.
    assert!(file_size < 5000, "Compressed file should be smaller than plain NDJSON, got {} bytes", file_size);

    // Reopen and verify data is intact
    {
        let engine = Engine::open(&path, None).unwrap();
        assert_eq!(engine.count("data", "").unwrap(), 100);
        let doc = engine.find_by_id("data", "42").unwrap();
        assert_eq!(doc["name"], "User 42");
    }

    cleanup(&path);
}

#[test]
fn test_encrypted_database() {
    let path = temp_dir("encrypted");

    // Create encrypted DB
    {
        let engine = Engine::open(&path, Some(&TEST_KEY)).unwrap();
        engine.create_collection("secrets").unwrap();
        engine.insert("secrets", r#"{"id":"1","data":"classified"}"#).unwrap();
        engine.flush().unwrap();
    }

    // Reopen with correct key — should work
    {
        let engine = Engine::open(&path, Some(&TEST_KEY)).unwrap();
        let doc = engine.find_by_id("secrets", "1").unwrap();
        assert_eq!(doc["data"], "classified");
    }

    cleanup(&path);
}

#[test]
fn test_encrypted_db_requires_key() {
    let path = temp_dir("enc_required");

    // Create encrypted DB
    {
        let engine = Engine::open(&path, None).unwrap();
        engine.create_collection("data").unwrap();
        engine.insert("data", r#"{"id":"1","v":"test"}"#).unwrap();
        engine.flush().unwrap();
        engine.encrypt(&TEST_KEY).unwrap();
    }

    // Reopen without key — should fail
    let result = Engine::open(&path, None);
    assert!(result.is_err());

    cleanup(&path);
}

#[test]
fn test_encrypt_existing_db() {
    let path = temp_dir("enc_existing");

    // Create unencrypted DB with data
    {
        let engine = Engine::open(&path, None).unwrap();
        engine.create_collection("users").unwrap();
        engine.insert("users", r#"{"id":"1","name":"Alice"}"#).unwrap();
        engine.flush().unwrap();

        // Encrypt it
        engine.encrypt(&TEST_KEY).unwrap();
    }

    // Reopen with key — data should be intact
    {
        let engine = Engine::open(&path, Some(&TEST_KEY)).unwrap();
        let doc = engine.find_by_id("users", "1").unwrap();
        assert_eq!(doc["name"], "Alice");
    }

    cleanup(&path);
}

#[test]
fn test_decrypt_existing_db() {
    let path = temp_dir("dec_existing");

    // Create encrypted DB
    {
        let engine = Engine::open(&path, None).unwrap();
        engine.create_collection("data").unwrap();
        engine.insert("data", r#"{"id":"1","v":"secret"}"#).unwrap();
        engine.flush().unwrap();
        engine.encrypt(&TEST_KEY).unwrap();
    }

    // Reopen with key and decrypt
    {
        let engine = Engine::open(&path, Some(&TEST_KEY)).unwrap();
        engine.decrypt(&TEST_KEY).unwrap();
    }

    // Reopen without key — should work now
    {
        let engine = Engine::open(&path, None).unwrap();
        let doc = engine.find_by_id("data", "1").unwrap();
        assert_eq!(doc["v"], "secret");
    }

    cleanup(&path);
}

#[test]
fn test_wrong_key_fails() {
    let path = temp_dir("wrong_key");
    let wrong_key: [u8; 32] = [0xff; 32];

    // Create encrypted DB
    {
        let engine = Engine::open(&path, Some(&TEST_KEY)).unwrap();
        engine.create_collection("data").unwrap();
        engine.insert("data", r#"{"id":"1","v":"test"}"#).unwrap();
        engine.flush().unwrap();
    }

    // Reopen with wrong key — should fail when trying to read data
    {
        let engine = Engine::open(&path, Some(&wrong_key)).unwrap();
        let result = engine.find_by_id("data", "1");
        assert!(result.is_err());
    }

    cleanup(&path);
}

#[test]
fn test_anvil_file_format() {
    let path = temp_dir("anvil_format");
    {
        let engine = Engine::open(&path, None).unwrap();
        engine.create_collection("test").unwrap();
        engine.insert("test", r#"{"id":"1","v":"hello"}"#).unwrap();
        engine.flush().unwrap();
    }

    // Verify .anvil file exists (not .ndjson)
    let anvil = std::path::Path::new(&path).join("collections").join("test.anvil");
    let ndjson = std::path::Path::new(&path).join("collections").join("test.ndjson");
    assert!(anvil.exists(), ".anvil file should exist");
    assert!(!ndjson.exists(), ".ndjson file should not exist");

    // Verify metadata.json exists
    let meta = std::path::Path::new(&path).join("metadata.json");
    assert!(meta.exists(), "metadata.json should exist");

    cleanup(&path);
}
