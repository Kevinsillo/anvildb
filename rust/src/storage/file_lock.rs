use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::path::Path;

use crate::error::{DbError, DbResult};

/// Acquire a shared (read) lock on the given file path.
/// Returns the locked `File` handle; the lock is released when the file is dropped.
pub fn lock_shared(path: &Path) -> DbResult<File> {
    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|e| DbError::LockError(format!("Cannot open for shared lock: {}", e)))?;
    FileExt::lock_shared(&file)
        .map_err(|e| DbError::LockError(format!("Shared lock failed: {}", e)))?;
    Ok(file)
}

/// Acquire an exclusive (write) lock on the given file path.
/// Creates the file if it does not exist.
/// Returns the locked `File` handle; the lock is released when the file is dropped.
pub fn lock_exclusive(path: &Path) -> DbResult<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| DbError::LockError(format!("Cannot open for exclusive lock: {}", e)))?;
    FileExt::lock_exclusive(&file)
        .map_err(|e| DbError::LockError(format!("Exclusive lock failed: {}", e)))?;
    Ok(file)
}
