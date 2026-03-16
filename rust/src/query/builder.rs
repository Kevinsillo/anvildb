use serde::Deserialize;
use serde_json::Value;

use crate::error::{DbError, DbResult};

/// A join clause describing how to merge two collections.
#[derive(Debug, Clone, Deserialize)]
pub struct JoinClause {
    /// The collection to join against.
    pub collection: String,
    /// "inner" or "left".
    #[serde(default = "default_join_type")]
    pub join_type: String,
    /// Field on the primary (left) side, e.g. "user_id".
    pub left_field: String,
    /// Field on the joined (right) side, e.g. "id".
    pub right_field: String,
    /// Prefix for fields from the joined collection (e.g. "user_").
    /// Defaults to "{collection}_" if omitted.
    pub prefix: Option<String>,
}

fn default_join_type() -> String {
    "inner".to_string()
}

/// An aggregation operation.
#[derive(Debug, Clone, Deserialize)]
pub struct Aggregation {
    /// The aggregation function: "sum", "avg", "min", "max", "count".
    pub function: String,
    /// The field to aggregate on (not needed for "count").
    pub field: Option<String>,
    /// Optional alias for the result key.
    pub alias: Option<String>,
}

/// A group_by clause with aggregations.
#[derive(Debug, Clone, Deserialize)]
pub struct GroupBy {
    /// The field(s) to group by.
    pub fields: Vec<String>,
    /// Aggregation operations to apply per group.
    #[serde(default)]
    pub aggregations: Vec<Aggregation>,
}

/// A complete query specification deserialized from JSON.
#[derive(Debug, Clone, Deserialize)]
pub struct QuerySpec {
    pub collection: String,
    #[serde(default)]
    pub filters: Vec<Filter>,
    pub order_by: Option<OrderBy>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    #[serde(default)]
    pub joins: Vec<JoinClause>,
    /// Aggregations without grouping (returns a single result object).
    #[serde(default)]
    pub aggregate: Vec<Aggregation>,
    /// Group by with aggregations (returns one result per group).
    pub group_by: Option<GroupBy>,
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
