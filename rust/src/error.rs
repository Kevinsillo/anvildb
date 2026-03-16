use std::fmt;
use std::io;

#[derive(Debug)]
pub enum DbError {
    Io(io::Error),
    Json(serde_json::Error),
    CollectionNotFound(String),
    CollectionAlreadyExists(String),
    DocumentNotFound(String),
    DuplicateKey(String, String),
    ValidationError(String),
    InvalidQuery(String),
    InvalidArgument(String),
    LockError(String),
    EncryptionRequired,
    EncryptionError(String),
    DecryptionFailed(String),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::Io(e) => write!(f, "IO error: {}", e),
            DbError::Json(e) => write!(f, "JSON error: {}", e),
            DbError::CollectionNotFound(name) => {
                write!(f, "Collection not found: {}", name)
            }
            DbError::CollectionAlreadyExists(name) => {
                write!(f, "Collection already exists: {}", name)
            }
            DbError::DocumentNotFound(id) => {
                write!(f, "Document not found: {}", id)
            }
            DbError::DuplicateKey(field, value) => {
                write!(f, "Duplicate key on field '{}': {}", field, value)
            }
            DbError::ValidationError(msg) => {
                write!(f, "Validation error: {}", msg)
            }
            DbError::InvalidQuery(msg) => {
                write!(f, "Invalid query: {}", msg)
            }
            DbError::InvalidArgument(msg) => {
                write!(f, "Invalid argument: {}", msg)
            }
            DbError::LockError(msg) => {
                write!(f, "Lock error: {}", msg)
            }
            DbError::EncryptionRequired => {
                write!(f, "Database is encrypted — encryption key required")
            }
            DbError::EncryptionError(msg) => {
                write!(f, "Encryption error: {}", msg)
            }
            DbError::DecryptionFailed(msg) => {
                write!(f, "Decryption failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for DbError {}

impl From<io::Error> for DbError {
    fn from(e: io::Error) -> Self {
        DbError::Io(e)
    }
}

impl From<serde_json::Error> for DbError {
    fn from(e: serde_json::Error) -> Self {
        DbError::Json(e)
    }
}

pub type DbResult<T> = Result<T, DbError>;
