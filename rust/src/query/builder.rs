use serde::Deserialize;
use serde_json::Value;

use crate::error::{DbError, DbResult};

/// A complete query specification deserialized from JSON.
#[derive(Debug, Clone, Deserialize)]
pub struct QuerySpec {
    pub collection: String,
    #[serde(default)]
    pub filters: Vec<Filter>,
    pub order_by: Option<OrderBy>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Filter {
    pub field: String,
    pub op: String,
    pub value: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrderBy {
    pub field: String,
    #[serde(default = "default_dir")]
    pub dir: String,
}

fn default_dir() -> String {
    "asc".to_string()
}

impl QuerySpec {
    /// Parse a query spec from a JSON string.
    pub fn from_json(json: &str) -> DbResult<Self> {
        serde_json::from_str(json)
            .map_err(|e| DbError::InvalidQuery(format!("Failed to parse query: {}", e)))
    }
}

/// Parse a filter-only JSON (used by anvildb_count).
/// Expected format: `[{"field":"x","op":"=","value":"y"}]` or `{"field":"x","op":"=","value":"y"}`
pub fn parse_filters(json: &str) -> DbResult<Vec<Filter>> {
    if json.is_empty() || json == "null" {
        return Ok(Vec::new());
    }

    let val: Value = serde_json::from_str(json)
        .map_err(|e| DbError::InvalidQuery(format!("Failed to parse filters: {}", e)))?;

    if val.is_array() {
        let filters: Vec<Filter> = serde_json::from_value(val)
            .map_err(|e| DbError::InvalidQuery(format!("Invalid filter array: {}", e)))?;
        Ok(filters)
    } else if val.is_object() {
        let filter: Filter = serde_json::from_value(val)
            .map_err(|e| DbError::InvalidQuery(format!("Invalid filter object: {}", e)))?;
        Ok(vec![filter])
    } else {
        Err(DbError::InvalidQuery(
            "Filters must be an array or object".into(),
        ))
    }
}
