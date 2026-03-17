use std::collections::HashMap;

use serde_json::Value;

use crate::error::{DbError, DbResult};
use crate::query::builder::{Aggregation, Filter, GroupBy, JoinClause, OrderBy, QuerySpec};

/// Execute a query against a set of documents.
/// Returns the filtered, sorted, and paginated results.
/// If aggregations are present, returns aggregation results instead.
pub fn execute_query(docs: &[Value], spec: &QuerySpec) -> DbResult<Vec<Value>> {
    // Filter
    let filtered: Vec<Value> = docs
        .iter()
        .filter(|doc| matches_filters(doc, &spec.filters))
        .cloned()
        .collect();

    // Aggregations
    if let Some(ref group_by) = spec.group_by {
        return execute_group_by(&filtered, group_by);
    }
    if !spec.aggregate.is_empty() {
        let result = execute_aggregations(&filtered, &spec.aggregate);
        return Ok(vec![result]);
    }

    let mut results = filtered;

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
        "between" => match (field_val, &filter.value) {
            (Some(v), Value::Array(range)) if range.len() == 2 => {
                matches!(
                    compare_values(v, &range[0]),
                    Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
                ) && matches!(
                    compare_values(v, &range[1]),
                    Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
                )
            }
            _ => false,
        },
        "in" => match (field_val, &filter.value) {
            (Some(v), Value::Array(list)) => list.iter().any(|item| values_equal(v, item)),
            _ => false,
        },
        "not_in" => match (field_val, &filter.value) {
            (Some(v), Value::Array(list)) => !list.iter().any(|item| values_equal(v, item)),
            (None, Value::Array(_)) => true,
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

/// Execute a query that includes joins across multiple collections.
pub fn execute_join_query(
    primary_docs: &[Value],
    joins: &[JoinClause],
    collections: &HashMap<&str, &[Value]>,
    spec: &QuerySpec,
) -> DbResult<Vec<Value>> {
    // 1. Start with all primary docs (unfiltered — filters apply after joins)
    let mut results: Vec<Value> = primary_docs.to_vec();

    // 2. Apply each join sequentially
    for join in joins {
        let right_docs = collections
            .get(join.collection.as_str())
            .ok_or_else(|| DbError::CollectionNotFound(join.collection.clone()))?;

        let prefix = join
            .prefix
            .clone()
            .unwrap_or_else(|| format!("{}_", join.collection));

        results = hash_join(
            &results,
            right_docs,
            &join.left_field,
            &join.right_field,
            &prefix,
            &join.join_type,
        );
    }

    // 3. Apply filters AFTER all joins (filters can reference joined fields)
    results.retain(|doc| matches_filters(doc, &spec.filters));

    // Aggregations
    if let Some(ref group_by) = spec.group_by {
        return execute_group_by(&results, group_by);
    }
    if !spec.aggregate.is_empty() {
        let result = execute_aggregations(&results, &spec.aggregate);
        return Ok(vec![result]);
    }

    // 4. Sort
    if let Some(ref order_by) = spec.order_by {
        sort_documents(&mut results, order_by);
    }

    // 5. Offset + Limit
    let offset = spec.offset.unwrap_or(0);
    if offset > 0 && offset < results.len() {
        results = results[offset..].to_vec();
    } else if offset >= results.len() && !results.is_empty() {
        results.clear();
    }

    if let Some(limit) = spec.limit {
        results.truncate(limit);
    }

    Ok(results)
}

/// Perform a hash join between left and right document sets.
fn hash_join(
    left: &[Value],
    right: &[Value],
    left_field: &str,
    right_field: &str,
    prefix: &str,
    join_type: &str,
) -> Vec<Value> {
    // Build hash index on the right side: right_field_value -> Vec<&Value>
    let mut index: HashMap<String, Vec<&Value>> = HashMap::new();
    for doc in right {
        if let Some(val) = doc.get(right_field) {
            let key = value_to_hash_key(val);
            index.entry(key).or_default().push(doc);
        }
    }

    let is_left_join = join_type == "left";
    let mut output = Vec::new();

    for left_doc in left {
        let key = left_doc.get(left_field).map(value_to_hash_key);
        let matches = key.as_ref().and_then(|k| index.get(k));

        match matches {
            Some(right_docs) => {
                for right_doc in right_docs {
                    output.push(merge_documents(left_doc, right_doc, prefix));
                }
            }
            None if is_left_join => {
                // LEFT JOIN: include left row without right fields
                output.push(left_doc.clone());
            }
            None => {
                // INNER JOIN: skip unmatched rows
            }
        }
    }

    output
}

/// Convert a JSON value to a string key for hash-based lookups.
fn value_to_hash_key(val: &Value) -> String {
    match val {
        Value::String(s) => format!("s:{}", s),
        Value::Number(n) => format!("n:{}", n),
        Value::Bool(b) => format!("b:{}", b),
        Value::Null => "null".to_string(),
        other => format!("j:{}", other),
    }
}

/// Merge two documents: left fields stay as-is, right fields get the prefix.
fn merge_documents(left: &Value, right: &Value, prefix: &str) -> Value {
    let mut merged = left.clone();
    if let (Some(left_obj), Some(right_obj)) = (merged.as_object_mut(), right.as_object()) {
        for (key, value) in right_obj {
            let prefixed_key = format!("{}{}", prefix, key);
            left_obj.insert(prefixed_key, value.clone());
        }
    }
    merged
}

/// Execute aggregations on a set of documents (no grouping).
/// Returns a single JSON object with the aggregation results.
fn execute_aggregations(docs: &[Value], aggregations: &[Aggregation]) -> Value {
    let mut result = serde_json::Map::new();

    for agg in aggregations {
        let alias = agg
            .alias
            .clone()
            .unwrap_or_else(|| format!("{}_{}", agg.function, agg.field.as_deref().unwrap_or("*")));

        let value = compute_aggregation(docs, &agg.function, agg.field.as_deref());
        result.insert(alias, value);
    }

    Value::Object(result)
}

/// Execute group_by with aggregations.
/// Returns one JSON object per group.
fn execute_group_by(docs: &[Value], group_by: &GroupBy) -> DbResult<Vec<Value>> {
    // Group documents by the group_by fields
    let mut groups: HashMap<String, Vec<&Value>> = HashMap::new();

    for doc in docs {
        let key = group_by
            .fields
            .iter()
            .map(|f| {
                doc.get(f)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "null".to_string())
            })
            .collect::<Vec<_>>()
            .join("|");

        groups.entry(key).or_default().push(doc);
    }

    let mut results = Vec::new();

    for (_key, group_docs) in &groups {
        let mut row = serde_json::Map::new();

        // Add group_by field values from the first doc in the group
        if let Some(first) = group_docs.first() {
            for field in &group_by.fields {
                if let Some(val) = first.get(field) {
                    row.insert(field.clone(), val.clone());
                }
            }
        }

        // Compute aggregations for this group
        let owned_docs: Vec<Value> = group_docs.iter().map(|d| (*d).clone()).collect();
        for agg in &group_by.aggregations {
            let alias = agg.alias.clone().unwrap_or_else(|| {
                format!("{}_{}", agg.function, agg.field.as_deref().unwrap_or("*"))
            });
            let value = compute_aggregation(&owned_docs, &agg.function, agg.field.as_deref());
            row.insert(alias, value);
        }

        results.push(Value::Object(row));
    }

    Ok(results)
}

/// Compute a single aggregation function over a set of documents.
fn compute_aggregation(docs: &[Value], function: &str, field: Option<&str>) -> Value {
    match function {
        "count" => Value::Number(serde_json::Number::from(docs.len())),
        "sum" => {
            let field = field.unwrap_or("");
            let sum: f64 = docs
                .iter()
                .filter_map(|d| d.get(field)?.as_f64())
                .sum();
            serde_json::Number::from_f64(sum)
                .map(Value::Number)
                .unwrap_or(Value::Null)
        }
        "avg" => {
            let field = field.unwrap_or("");
            let values: Vec<f64> = docs
                .iter()
                .filter_map(|d| d.get(field)?.as_f64())
                .collect();
            if values.is_empty() {
                Value::Null
            } else {
                let avg = values.iter().sum::<f64>() / values.len() as f64;
                serde_json::Number::from_f64(avg)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            }
        }
        "min" => {
            let field = field.unwrap_or("");
            docs.iter()
                .filter_map(|d| d.get(field))
                .min_by(|a, b| compare_values(a, b).unwrap_or(std::cmp::Ordering::Equal))
                .cloned()
                .unwrap_or(Value::Null)
        }
        "max" => {
            let field = field.unwrap_or("");
            docs.iter()
                .filter_map(|d| d.get(field))
                .max_by(|a, b| compare_values(a, b).unwrap_or(std::cmp::Ordering::Equal))
                .cloned()
                .unwrap_or(Value::Null)
        }
        _ => Value::Null,
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
