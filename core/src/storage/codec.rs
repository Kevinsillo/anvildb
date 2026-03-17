use std::io::{BufRead, BufReader};

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Nonce};
use miniz_oxide::deflate::compress_to_vec;
use miniz_oxide::inflate::decompress_to_vec;
use serde_json::Value;

use crate::error::{DbError, DbResult};

/// AES-256-GCM nonce size in bytes.
const NONCE_SIZE: usize = 12;

/// Deflate compression level (6 = good balance of speed and ratio).
const COMPRESSION_LEVEL: u8 = 6;

/// Encode documents for writing to disk.
/// Pipeline: docs → NDJSON bytes → deflate compress → (optional) AES-256-GCM encrypt.
pub fn encode_docs(docs: &[Value], key: Option<&[u8; 32]>) -> DbResult<Vec<u8>> {
    // Serialize to NDJSON
    let mut ndjson = Vec::new();
    for doc in docs {
        serde_json::to_writer(&mut ndjson, doc)?;
        ndjson.push(b'\n');
    }

    encode_raw(&ndjson, key)
}

/// Decode documents from disk bytes.
/// Pipeline: (optional) decrypt → inflate decompress → parse NDJSON → Vec<Value>.
pub fn decode_docs(data: &[u8], key: Option<&[u8; 32]>) -> DbResult<Vec<Value>> {
    if data.is_empty() {
        return Ok(Vec::new());
    }

    let ndjson = decode_raw(data, key)?;

    let reader = BufReader::new(ndjson.as_slice());
    let mut docs = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| DbError::Io(e))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let doc: Value = serde_json::from_str(trimmed)?;
        docs.push(doc);
    }

    Ok(docs)
}

/// Encode raw bytes: compress → (optional) encrypt.
pub fn encode_raw(data: &[u8], key: Option<&[u8; 32]>) -> DbResult<Vec<u8>> {
    let compressed = compress_to_vec(data, COMPRESSION_LEVEL);

    match key {
        Some(k) => encrypt(&compressed, k),
        None => Ok(compressed),
    }
}

/// Decode raw bytes: (optional) decrypt → decompress.
pub fn decode_raw(data: &[u8], key: Option<&[u8; 32]>) -> DbResult<Vec<u8>> {
    let compressed = match key {
        Some(k) => decrypt(data, k)?,
        None => data.to_vec(),
    };

    decompress_to_vec(&compressed).map_err(|e| {
        DbError::DecryptionFailed(format!("Decompression failed: {:?}", e))
    })
}

/// Encrypt data with AES-256-GCM. Prepends the 12-byte nonce to the output.
fn encrypt(data: &[u8], key: &[u8; 32]) -> DbResult<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| DbError::EncryptionError(format!("Invalid key: {}", e)))?;

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, data)
        .map_err(|e| DbError::EncryptionError(format!("Encryption failed: {}", e)))?;

    let mut output = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

/// Decrypt data with AES-256-GCM. Expects the first 12 bytes to be the nonce.
fn decrypt(data: &[u8], key: &[u8; 32]) -> DbResult<Vec<u8>> {
    if data.len() < NONCE_SIZE {
        return Err(DbError::DecryptionFailed(
            "Data too short to contain nonce".into(),
        ));
    }

    let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| DbError::DecryptionFailed(format!("Invalid key: {}", e)))?;

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| DbError::DecryptionFailed("Decryption failed — wrong key?".into()))
}
