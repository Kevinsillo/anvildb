mod cache;
mod collection;
pub mod engine;
pub mod error;
mod ffi;
mod index;
mod query;
mod storage;
mod validation;

use std::os::raw::c_char;

use engine::Engine;
use ffi::{cstr_to_str, free_c_string, string_to_c};

/// Opaque handle to the engine.
pub type AnvilDbHandle = *mut Engine;

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn anvildb_open(data_path: *const c_char) -> AnvilDbHandle {
    let _ = env_logger::try_init();

    let path = match cstr_to_str(data_path) {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };

    match Engine::open(path) {
        Ok(eng) => Box::into_raw(Box::new(eng)),
        Err(e) => {
            log::error!("anvildb_open failed: {}", e);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn anvildb_close(handle: AnvilDbHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

#[no_mangle]
pub unsafe extern "C" fn anvildb_shutdown(handle: AnvilDbHandle) {
    anvildb_close(handle);
}

// ---------------------------------------------------------------------------
// Collections
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn anvildb_create_collection(
    handle: AnvilDbHandle,
    name: *const c_char,
) -> i32 {
    let eng = match handle.as_ref() {
        Some(e) => e,
        None => return -1,
    };
    let name = match cstr_to_str(name) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid collection name".into());
            return -1;
        }
    };

    match eng.create_collection(name) {
        Ok(()) => 0,
        Err(e) => {
            eng.set_error(e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn anvildb_drop_collection(
    handle: AnvilDbHandle,
    name: *const c_char,
) -> i32 {
    let eng = match handle.as_ref() {
        Some(e) => e,
        None => return -1,
    };
    let name = match cstr_to_str(name) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid collection name".into());
            return -1;
        }
    };

    match eng.drop_collection(name) {
        Ok(()) => 0,
        Err(e) => {
            eng.set_error(e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn anvildb_list_collections(
    handle: AnvilDbHandle,
) -> *const c_char {
    let eng = match handle.as_ref() {
        Some(e) => e,
        None => return std::ptr::null(),
    };

    match eng.list_collections() {
        Ok(names) => {
            let json = serde_json::to_string(&names).unwrap_or_else(|_| "[]".into());
            string_to_c(json)
        }
        Err(e) => {
            eng.set_error(e.to_string());
            std::ptr::null()
        }
    }
}

// ---------------------------------------------------------------------------
// CRUD
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn anvildb_insert(
    handle: AnvilDbHandle,
    collection: *const c_char,
    json_doc: *const c_char,
) -> *const c_char {
    let eng = match handle.as_ref() {
        Some(e) => e,
        None => return std::ptr::null(),
    };
    let collection = match cstr_to_str(collection) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid collection name".into());
            return std::ptr::null();
        }
    };
    let json_doc = match cstr_to_str(json_doc) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid JSON document".into());
            return std::ptr::null();
        }
    };

    match eng.insert(collection, json_doc) {
        Ok(doc) => {
            let json = serde_json::to_string(&doc).unwrap_or_else(|_| "{}".into());
            string_to_c(json)
        }
        Err(e) => {
            eng.set_error(e.to_string());
            std::ptr::null()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn anvildb_find_by_id(
    handle: AnvilDbHandle,
    collection: *const c_char,
    id: *const c_char,
) -> *const c_char {
    let eng = match handle.as_ref() {
        Some(e) => e,
        None => return std::ptr::null(),
    };
    let collection = match cstr_to_str(collection) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid collection name".into());
            return std::ptr::null();
        }
    };
    let id = match cstr_to_str(id) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid id".into());
            return std::ptr::null();
        }
    };

    match eng.find_by_id(collection, id) {
        Ok(doc) => {
            let json = serde_json::to_string(&doc).unwrap_or_else(|_| "{}".into());
            string_to_c(json)
        }
        Err(e) => {
            eng.set_error(e.to_string());
            std::ptr::null()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn anvildb_update(
    handle: AnvilDbHandle,
    collection: *const c_char,
    id: *const c_char,
    json_doc: *const c_char,
) -> i32 {
    let eng = match handle.as_ref() {
        Some(e) => e,
        None => return -1,
    };
    let collection = match cstr_to_str(collection) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid collection name".into());
            return -1;
        }
    };
    let id = match cstr_to_str(id) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid id".into());
            return -1;
        }
    };
    let json_doc = match cstr_to_str(json_doc) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid JSON document".into());
            return -1;
        }
    };

    match eng.update(collection, id, json_doc) {
        Ok(()) => 0,
        Err(e) => {
            eng.set_error(e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn anvildb_delete(
    handle: AnvilDbHandle,
    collection: *const c_char,
    id: *const c_char,
) -> i32 {
    let eng = match handle.as_ref() {
        Some(e) => e,
        None => return -1,
    };
    let collection = match cstr_to_str(collection) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid collection name".into());
            return -1;
        }
    };
    let id = match cstr_to_str(id) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid id".into());
            return -1;
        }
    };

    match eng.delete(collection, id) {
        Ok(()) => 0,
        Err(e) => {
            eng.set_error(e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn anvildb_bulk_insert(
    handle: AnvilDbHandle,
    collection: *const c_char,
    json_docs: *const c_char,
) -> *const c_char {
    let eng = match handle.as_ref() {
        Some(e) => e,
        None => return std::ptr::null(),
    };
    let collection = match cstr_to_str(collection) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid collection name".into());
            return std::ptr::null();
        }
    };
    let json_docs = match cstr_to_str(json_docs) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid JSON documents".into());
            return std::ptr::null();
        }
    };

    match eng.bulk_insert(collection, json_docs) {
        Ok(docs) => {
            let json = serde_json::to_string(&docs).unwrap_or_else(|_| "[]".into());
            string_to_c(json)
        }
        Err(e) => {
            eng.set_error(e.to_string());
            std::ptr::null()
        }
    }
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn anvildb_query(
    handle: AnvilDbHandle,
    json_query_spec: *const c_char,
) -> *const c_char {
    let eng = match handle.as_ref() {
        Some(e) => e,
        None => return std::ptr::null(),
    };
    let spec = match cstr_to_str(json_query_spec) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid query spec".into());
            return std::ptr::null();
        }
    };

    match eng.query(spec) {
        Ok(results) => {
            let json = serde_json::to_string(&results).unwrap_or_else(|_| "[]".into());
            string_to_c(json)
        }
        Err(e) => {
            eng.set_error(e.to_string());
            std::ptr::null()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn anvildb_count(
    handle: AnvilDbHandle,
    collection: *const c_char,
    json_filter: *const c_char,
) -> i64 {
    let eng = match handle.as_ref() {
        Some(e) => e,
        None => return -1,
    };
    let collection = match cstr_to_str(collection) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid collection name".into());
            return -1;
        }
    };
    let filter = cstr_to_str(json_filter).unwrap_or("");

    match eng.count(collection, filter) {
        Ok(n) => n,
        Err(e) => {
            eng.set_error(e.to_string());
            -1
        }
    }
}

// ---------------------------------------------------------------------------
// Indexes
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn anvildb_create_index(
    handle: AnvilDbHandle,
    collection: *const c_char,
    field: *const c_char,
    index_type: *const c_char,
) -> i32 {
    let eng = match handle.as_ref() {
        Some(e) => e,
        None => return -1,
    };
    let collection = match cstr_to_str(collection) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid collection name".into());
            return -1;
        }
    };
    let field = match cstr_to_str(field) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid field name".into());
            return -1;
        }
    };
    let idx_type = cstr_to_str(index_type).unwrap_or("hash");

    match eng.create_index(collection, field, idx_type) {
        Ok(()) => 0,
        Err(e) => {
            eng.set_error(e.to_string());
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn anvildb_drop_index(
    handle: AnvilDbHandle,
    collection: *const c_char,
    field: *const c_char,
) -> i32 {
    let eng = match handle.as_ref() {
        Some(e) => e,
        None => return -1,
    };
    let collection = match cstr_to_str(collection) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid collection name".into());
            return -1;
        }
    };
    let field = match cstr_to_str(field) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid field name".into());
            return -1;
        }
    };

    match eng.drop_index(collection, field) {
        Ok(()) => 0,
        Err(e) => {
            eng.set_error(e.to_string());
            -1
        }
    }
}

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn anvildb_set_schema(
    handle: AnvilDbHandle,
    collection: *const c_char,
    json_schema: *const c_char,
) -> i32 {
    let eng = match handle.as_ref() {
        Some(e) => e,
        None => return -1,
    };
    let collection = match cstr_to_str(collection) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid collection name".into());
            return -1;
        }
    };
    let schema = match cstr_to_str(json_schema) {
        Some(s) => s,
        None => {
            eng.set_error("Invalid schema JSON".into());
            return -1;
        }
    };

    match eng.set_schema(collection, schema) {
        Ok(()) => 0,
        Err(e) => {
            eng.set_error(e.to_string());
            -1
        }
    }
}

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn anvildb_clear_cache(handle: AnvilDbHandle) {
    if let Some(eng) = handle.as_ref() {
        eng.clear_cache();
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn anvildb_last_error(handle: AnvilDbHandle) -> *const c_char {
    let eng = match handle.as_ref() {
        Some(e) => e,
        None => return std::ptr::null(),
    };

    match eng.take_error() {
        Some(msg) => string_to_c(msg),
        None => std::ptr::null(),
    }
}

// ---------------------------------------------------------------------------
// Memory management
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn anvildb_free_string(ptr: *const c_char) {
    free_c_string(ptr);
}
