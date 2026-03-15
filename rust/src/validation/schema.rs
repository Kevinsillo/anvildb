use serde_json::Value;

use crate::error::{DbError, DbResult};

/// A schema is a JSON object mapping field names to type strings.
/// Supported types: "string", "int", "float", "bool", "array", "object"
#[derive(Debug, Clone)]
pub struct Schema {
    pub fields: Vec<(String, String)>,
}

impl Schema {
    /// Parse a schema from a JSON value.
    /// Expected format: `{"field_name": "type", ...}`
    pub fn from_value(val: &Value) -> DbResult<Self> {
        let obj = val
            .as_object()
            .ok_or_else(|| DbError::ValidationError("Schema must be a JSON object".into()))?;

        let mut fields = Vec::new();
        for (key, type_val) in obj {
            let type_str = type_val.as_str().ok_or_else(|| {
                DbError::ValidationError(format!(
                    "Schema type for field '{}' must be a string",
                    key
                ))
            })?;

            match type_str {
                "string" | "int" | "float" | "bool" | "array" | "object" => {}
                other => {
                    return Err(DbError::ValidationError(format!(
                        "Unknown type '{}' for field '{}'",
                        other, key
                    )));
                }
            }

            fields.push((key.clone(), type_str.to_string()));
        }

        Ok(Schema { fields })
    }

    /// Validate a document against this schema.
    /// Only checks fields that are present in the document and defined in the schema.
    pub fn validate(&self, doc: &Value) -> DbResult<()> {
        let obj = doc
            .as_object()
            .ok_or_else(|| DbError::ValidationError("Document must be a JSON object".into()))?;

        for (field, expected_type) in &self.fields {
            if let Some(val) = obj.get(field) {
                if val.is_null() {
                    // Allow null values for any field
                    continue;
                }
                let ok = match expected_type.as_str() {
                    "string" => val.is_string(),
                    "int" => val.is_i64() || val.is_u64(),
                    "float" => val.is_f64(),
                    "bool" => val.is_boolean(),
                    "array" => val.is_array(),
                    "object" => val.is_object(),
                    _ => false,
                };
                if !ok {
                    return Err(DbError::ValidationError(format!(
                        "Field '{}' expected type '{}', got {:?}",
                        field, expected_type, val
                    )));
                }
            }
        }

        Ok(())
    }
}
