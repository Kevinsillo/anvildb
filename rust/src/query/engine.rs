use serde_json::Value;

use crate::error::DbResult;
use crate::query::builder::{Filter, OrderBy, QuerySpec};

/// Execute a query against a set of documents.
/// Returns the filtered, sorted, and paginated results.
pub fn execute_query(docs: &[Value], spec: &QuerySpec) -> DbResult<Vec<Value>> {
    // Filter
    let mut results: Vec<Value> = docs
        .iter()
        .filter(|doc| matches_filters(doc, &spec.filters))
        .cloned()
        .collect();

    // Sort
    if let Some(ref order_by) = spec.order_by {
        sort_documents(&mut results, order_by);
    }

    // Offset
    let offset = spec.offset.unwrap_or(0);
    if offset > 0 && offset < results.len() {
        results = results[offset..].to_vec();
    } else if offset >= results.len() && !results.is_empty() {
        results.clear();
    }

    // Limit
    if let Some(limit) = spec.limit {
        results.truncate(limit);
    }

    Ok(results)
}

/// Count documents matching the given filters.
pub fn count_matching(docs: &[Value], filters: &[Filter]) -> usize {
    docs.iter()
        .filter(|doc| matches_filters(doc, filters))
        .count()
}

/// Check if a document matches all filters.
fn matches_filters(doc: &Value, filters: &[Filter]) -> bool {
    filters.iter().all(|f| matches_filter(doc, f))
}

/// Check if a document matches a single filter.
fn matches_filter(doc: &Value, filter: &Filter) -> bool {
    let field_val = doc.get(&filter.field);

    match filter.op.as_str() {
        "=" | "==" => match field_val {
            Some(v) => values_equal(v, &filter.value),
            None => filter.value.is_null(),
        },
        "!=" | "<>" => match field_val {
            Some(v) => !values_equal(v, &filter.value),
            None => !filter.value.is_null(),
        },
        ">" => match field_val {
            Some(v) => compare_values(v, &filter.value) == Some(std::cmp::Ordering::Greater),
            None => false,
        },
        "<" => match field_val {
            Some(v) => compare_values(v, &filter.value) == Some(std::cmp::Ordering::Less),
            None => false,
        },
        ">=" => match field_val {
            Some(v) => matches!(
                compare_values(v, &filter.value),
                Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
            ),
            None => false,
        },
        "<=" => match field_val {
            Some(v) => matches!(
                compare_values(v, &filter.value),
                Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
            ),
            None => false,
        },
        "contains" => match (field_val, &filter.value) {
            (Some(Value::String(haystack)), Value::String(needle)) => {
                haystack.contains(needle.as_str())
            }
            (Some(Value::Array(arr)), needle) => arr.iter().any(|v| values_equal(v, needle)),
            _ => false,
        },
        _ => false,
    }
}

/// Compare two JSON values for equality.
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(na), Value::Number(nb)) => {
            // Compare numerically
            na.as_f64() == nb.as_f64()
        }
        _ => a == b,
    }
}

/// Compare two JSON values for ordering.
fn compare_values(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Number(na), Value::Number(nb)) => {
            na.as_f64()?.partial_cmp(&nb.as_f64()?)
        }
        (Value::String(sa), Value::String(sb)) => Some(sa.cmp(sb)),
        _ => None,
    }
}

/// Sort documents by a field in the specified direction.
fn sort_documents(docs: &mut [Value], order_by: &OrderBy) {
    let ascending = order_by.dir.to_lowercase() != "desc";

    docs.sort_by(|a, b| {
        let va = a.get(&order_by.field);
        let vb = b.get(&order_by.field);

        let ord = match (va, vb) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(va), Some(vb)) => compare_values(va, vb).unwrap_or(std::cmp::Ordering::Equal),
        };

        if ascending {
            ord
        } else {
            ord.reverse()
        }
    });
}
