use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Convert a `*const c_char` to a `&str`. Returns `None` if the pointer is null
/// or the data is not valid UTF-8.
pub unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}

/// Convert a Rust `String` into a `*const c_char` that the caller must free
/// with `anvildb_free_string`. The memory is leaked intentionally.
pub fn string_to_c(s: String) -> *const c_char {
    match CString::new(s) {
        Ok(cs) => cs.into_raw() as *const c_char,
        Err(_) => std::ptr::null(),
    }
}

/// Reclaim a C string previously returned by `string_to_c`.
pub unsafe fn free_c_string(ptr: *const c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr as *mut c_char));
    }
}
