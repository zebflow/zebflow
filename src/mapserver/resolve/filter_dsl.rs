//! Attribute filter DSL for MapServer (WMS + WFS).
//!
//! Compact URL-friendly notation for per-feature attribute filtering:
//!   `?filter=category:residential;value>100;status~active,pending`
//!
//! Operators:
//!   `:` equals, `!` not-equals, `>` gt, `<` lt, `>=` gte, `<=` lte,
//!   `~` IN (comma list), `!~` NOT IN.
//!
//! Semicolons are AND. Number suffixes reuse `parse_number_with_suffix` from style_dsl.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::style_dsl::parse_number_with_suffix;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum FilterOp {
    Eq,    // :
    Ne,    // !
    Gt,    // >
    Lt,    // <
    Gte,   // >=
    Lte,   // <=
    In,    // ~
    NotIn, // !~
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterValue {
    Str(String),
    Num(f64),
    List(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterCondition {
    pub field: String,
    pub op: FilterOp,
    pub value: FilterValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterExpr {
    pub conditions: Vec<FilterCondition>,
}

// ── Parser ───────────────────────────────────────────────────────────────────

/// Parse a filter DSL string into a `FilterExpr`.
///
/// Input: `"category:residential;value>100;status~active,pending"`
pub fn parse_filter(input: &str) -> Result<FilterExpr, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("empty filter expression".into());
    }

    let mut conditions = Vec::new();
    for part in input.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        conditions.push(parse_condition(part)?);
    }

    if conditions.is_empty() {
        return Err("no valid filter conditions found".into());
    }

    Ok(FilterExpr { conditions })
}

fn parse_condition(s: &str) -> Result<FilterCondition, String> {
    // Order matters: check multi-char operators first, then single-char.
    // >=, <=, !~ must be checked before >, <, !, ~

    if let Some((field, value)) = s.split_once(">=") {
        return make_condition(field, FilterOp::Gte, value);
    }
    if let Some((field, value)) = s.split_once("<=") {
        return make_condition(field, FilterOp::Lte, value);
    }
    if let Some((field, value)) = s.split_once("!~") {
        return make_list_condition(field, FilterOp::NotIn, value);
    }

    // Single-char operators: scan left-to-right for first operator char.
    // We need to find the operator position carefully.
    // `>` and `<` are unambiguous single-char (>= and <= already handled above).
    if let Some((field, value)) = s.split_once('>') {
        return make_condition(field, FilterOp::Gt, value);
    }
    if let Some((field, value)) = s.split_once('<') {
        return make_condition(field, FilterOp::Lt, value);
    }

    // `!` — but NOT `!~` (already handled above)
    if let Some((field, value)) = s.split_once('!') {
        return make_condition(field, FilterOp::Ne, value);
    }

    // `~` — IN operator
    if let Some((field, value)) = s.split_once('~') {
        return make_list_condition(field, FilterOp::In, value);
    }

    // `:` — equals
    if let Some((field, value)) = s.split_once(':') {
        return make_condition(field, FilterOp::Eq, value);
    }

    Err(format!(
        "no operator found in filter condition: '{s}' (expected one of : ! > < >= <= ~ !~)"
    ))
}

fn make_condition(field: &str, op: FilterOp, value: &str) -> Result<FilterCondition, String> {
    let field = field.trim().to_string();
    if field.is_empty() {
        return Err("filter condition has empty field name".into());
    }
    let value = value.trim();

    let filter_value = if let Ok(num) = parse_number_with_suffix(value) {
        FilterValue::Num(num)
    } else {
        FilterValue::Str(value.to_string())
    };

    Ok(FilterCondition {
        field,
        op,
        value: filter_value,
    })
}

fn make_list_condition(
    field: &str,
    op: FilterOp,
    value: &str,
) -> Result<FilterCondition, String> {
    let field = field.trim().to_string();
    if field.is_empty() {
        return Err("filter condition has empty field name".into());
    }
    let items: Vec<String> = value.split(',').map(|s| s.trim().to_string()).collect();
    Ok(FilterCondition {
        field,
        op,
        value: FilterValue::List(items),
    })
}

// ── GeoJSON evaluator ────────────────────────────────────────────────────────

/// Evaluate filter against a GeoJSON feature (`serde_json::Value`).
pub fn matches_geojson_feature(filter: &FilterExpr, feature: &serde_json::Value) -> bool {
    let props = feature.get("properties");
    filter.conditions.iter().all(|cond| {
        let field_val = props.and_then(|p| p.get(&cond.field));
        eval_condition(cond, field_val)
    })
}

fn eval_condition(cond: &FilterCondition, field_val: Option<&serde_json::Value>) -> bool {
    match (&cond.op, &cond.value) {
        (FilterOp::Eq, FilterValue::Str(expected)) => {
            json_str_eq(field_val, expected)
        }
        (FilterOp::Eq, FilterValue::Num(expected)) => {
            json_num_cmp(field_val, *expected) == Some(std::cmp::Ordering::Equal)
        }
        (FilterOp::Ne, FilterValue::Str(expected)) => {
            !json_str_eq(field_val, expected)
        }
        (FilterOp::Ne, FilterValue::Num(expected)) => {
            json_num_cmp(field_val, *expected) != Some(std::cmp::Ordering::Equal)
        }
        (FilterOp::Gt, FilterValue::Num(expected)) => {
            json_num_cmp(field_val, *expected) == Some(std::cmp::Ordering::Greater)
        }
        (FilterOp::Lt, FilterValue::Num(expected)) => {
            json_num_cmp(field_val, *expected) == Some(std::cmp::Ordering::Less)
        }
        (FilterOp::Gte, FilterValue::Num(expected)) => {
            matches!(
                json_num_cmp(field_val, *expected),
                Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
            )
        }
        (FilterOp::Lte, FilterValue::Num(expected)) => {
            matches!(
                json_num_cmp(field_val, *expected),
                Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
            )
        }
        (FilterOp::Gt, FilterValue::Str(expected))
        | (FilterOp::Lt, FilterValue::Str(expected))
        | (FilterOp::Gte, FilterValue::Str(expected))
        | (FilterOp::Lte, FilterValue::Str(expected)) => {
            // Try numeric comparison on string values
            if let Ok(num) = expected.parse::<f64>() {
                let ord = json_num_cmp(field_val, num);
                match cond.op {
                    FilterOp::Gt => ord == Some(std::cmp::Ordering::Greater),
                    FilterOp::Lt => ord == Some(std::cmp::Ordering::Less),
                    FilterOp::Gte => matches!(ord, Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)),
                    FilterOp::Lte => matches!(ord, Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)),
                    _ => false,
                }
            } else {
                false
            }
        }
        (FilterOp::In, FilterValue::List(items)) => {
            let s = json_as_string(field_val);
            s.map(|v| items.iter().any(|item| item == &v)).unwrap_or(false)
        }
        (FilterOp::NotIn, FilterValue::List(items)) => {
            let s = json_as_string(field_val);
            s.map(|v| !items.iter().any(|item| item == &v)).unwrap_or(true)
        }
        _ => false,
    }
}

fn json_str_eq(val: Option<&serde_json::Value>, expected: &str) -> bool {
    match val {
        Some(serde_json::Value::String(s)) => s == expected,
        Some(serde_json::Value::Number(n)) => n.to_string() == expected,
        Some(serde_json::Value::Bool(b)) => {
            (expected == "true" && *b) || (expected == "false" && !*b)
        }
        _ => false,
    }
}

fn json_num_cmp(val: Option<&serde_json::Value>, expected: f64) -> Option<std::cmp::Ordering> {
    let v = match val {
        Some(serde_json::Value::Number(n)) => n.as_f64()?,
        Some(serde_json::Value::String(s)) => s.parse::<f64>().ok()?,
        _ => return None,
    };
    v.partial_cmp(&expected)
}

fn json_as_string(val: Option<&serde_json::Value>) -> Option<String> {
    match val {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(serde_json::Value::Number(n)) => Some(n.to_string()),
        Some(serde_json::Value::Bool(b)) => Some(b.to_string()),
        _ => None,
    }
}

// ── Arrow batch evaluator ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub enum ArrowColType {
    Utf8,
    LargeUtf8,
    Int32,
    Int64,
    Float32,
    Float64,
    Other,
}

pub struct FilterColumnCache {
    /// For each condition: (column_index, column_type) or None if column not found.
    pub entries: Vec<Option<(usize, ArrowColType)>>,
}

/// Pre-resolve column indices from batch schema (call once per batch).
pub fn build_filter_column_cache(
    filter: &FilterExpr,
    schema: &datafusion::arrow::datatypes::SchemaRef,
) -> FilterColumnCache {
    use datafusion::arrow::datatypes::DataType;
    let entries = filter
        .conditions
        .iter()
        .map(|cond| {
            schema.index_of(&cond.field).ok().map(|idx| {
                let dt = schema.field(idx).data_type();
                let col_type = match dt {
                    DataType::Utf8 => ArrowColType::Utf8,
                    DataType::LargeUtf8 => ArrowColType::LargeUtf8,
                    DataType::Int32 => ArrowColType::Int32,
                    DataType::Int64 => ArrowColType::Int64,
                    DataType::Float32 => ArrowColType::Float32,
                    DataType::Float64 => ArrowColType::Float64,
                    _ => ArrowColType::Other,
                };
                (idx, col_type)
            })
        })
        .collect();
    FilterColumnCache { entries }
}

/// Evaluate filter against an Arrow batch row.
pub fn matches_arrow_row(
    filter: &FilterExpr,
    batch: &datafusion::arrow::record_batch::RecordBatch,
    row: usize,
    col_cache: &FilterColumnCache,
) -> bool {
    use datafusion::arrow::array::*;

    filter
        .conditions
        .iter()
        .zip(col_cache.entries.iter())
        .all(|(cond, cache_entry)| {
            let Some((col_idx, col_type)) = cache_entry else {
                // Column not in batch — condition fails for Eq/Gt/etc, passes for Ne/NotIn
                return matches!(cond.op, FilterOp::Ne | FilterOp::NotIn);
            };

            let col = batch.column(*col_idx);
            if col.is_null(row) {
                return matches!(cond.op, FilterOp::Ne | FilterOp::NotIn);
            }

            // Extract the field value as string and/or f64 for comparison
            match col_type {
                ArrowColType::Utf8 => {
                    let arr = col.as_any().downcast_ref::<StringArray>().unwrap();
                    let val = arr.value(row);
                    eval_arrow_str(cond, val)
                }
                ArrowColType::LargeUtf8 => {
                    let arr = col.as_any().downcast_ref::<LargeStringArray>().unwrap();
                    let val = arr.value(row);
                    eval_arrow_str(cond, val)
                }
                ArrowColType::Int32 => {
                    let arr = col.as_any().downcast_ref::<Int32Array>().unwrap();
                    eval_arrow_num(cond, arr.value(row) as f64, &arr.value(row).to_string())
                }
                ArrowColType::Int64 => {
                    let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
                    eval_arrow_num(cond, arr.value(row) as f64, &arr.value(row).to_string())
                }
                ArrowColType::Float32 => {
                    let arr = col.as_any().downcast_ref::<Float32Array>().unwrap();
                    let v = arr.value(row) as f64;
                    let s = if v == (v as i64) as f64 {
                        format!("{}", v as i64)
                    } else {
                        format!("{v}")
                    };
                    eval_arrow_num(cond, v, &s)
                }
                ArrowColType::Float64 => {
                    let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
                    let v = arr.value(row);
                    let s = if v == (v as i64) as f64 {
                        format!("{}", v as i64)
                    } else {
                        format!("{v}")
                    };
                    eval_arrow_num(cond, v, &s)
                }
                ArrowColType::Other => {
                    // Unsupported column type — fail the condition conservatively
                    matches!(cond.op, FilterOp::Ne | FilterOp::NotIn)
                }
            }
        })
}

fn eval_arrow_str(cond: &FilterCondition, val: &str) -> bool {
    match (&cond.op, &cond.value) {
        (FilterOp::Eq, FilterValue::Str(expected)) => val == expected,
        (FilterOp::Eq, FilterValue::Num(expected)) => {
            val.parse::<f64>().ok().map(|v| v == *expected).unwrap_or(false)
        }
        (FilterOp::Ne, FilterValue::Str(expected)) => val != expected,
        (FilterOp::Ne, FilterValue::Num(expected)) => {
            val.parse::<f64>().ok().map(|v| v != *expected).unwrap_or(true)
        }
        (FilterOp::Gt, FilterValue::Num(expected)) => {
            val.parse::<f64>().ok().map(|v| v > *expected).unwrap_or(false)
        }
        (FilterOp::Lt, FilterValue::Num(expected)) => {
            val.parse::<f64>().ok().map(|v| v < *expected).unwrap_or(false)
        }
        (FilterOp::Gte, FilterValue::Num(expected)) => {
            val.parse::<f64>().ok().map(|v| v >= *expected).unwrap_or(false)
        }
        (FilterOp::Lte, FilterValue::Num(expected)) => {
            val.parse::<f64>().ok().map(|v| v <= *expected).unwrap_or(false)
        }
        (FilterOp::In, FilterValue::List(items)) => items.iter().any(|item| item == val),
        (FilterOp::NotIn, FilterValue::List(items)) => !items.iter().any(|item| item == val),
        _ => false,
    }
}

fn eval_arrow_num(cond: &FilterCondition, val: f64, val_str: &str) -> bool {
    match (&cond.op, &cond.value) {
        (FilterOp::Eq, FilterValue::Num(expected)) => val == *expected,
        (FilterOp::Eq, FilterValue::Str(expected)) => val_str == expected,
        (FilterOp::Ne, FilterValue::Num(expected)) => val != *expected,
        (FilterOp::Ne, FilterValue::Str(expected)) => val_str != expected,
        (FilterOp::Gt, FilterValue::Num(expected)) => val > *expected,
        (FilterOp::Lt, FilterValue::Num(expected)) => val < *expected,
        (FilterOp::Gte, FilterValue::Num(expected)) => val >= *expected,
        (FilterOp::Lte, FilterValue::Num(expected)) => val <= *expected,
        (FilterOp::In, FilterValue::List(items)) => items.iter().any(|item| item == val_str),
        (FilterOp::NotIn, FilterValue::List(items)) => !items.iter().any(|item| item == val_str),
        _ => false,
    }
}

// ── SQL generation (for DataFusion) ──────────────────────────────────────────

/// Generate a SQL WHERE fragment for DataFusion.
pub fn filter_to_sql(filter: &FilterExpr) -> String {
    filter
        .conditions
        .iter()
        .map(condition_to_sql)
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn condition_to_sql(cond: &FilterCondition) -> String {
    let quoted_field = format!("\"{}\"", cond.field.replace('"', "\"\""));

    match (&cond.op, &cond.value) {
        (FilterOp::Eq, FilterValue::Num(n)) => format!("{quoted_field} = {n}"),
        (FilterOp::Eq, FilterValue::Str(s)) => {
            format!("{quoted_field} = '{}'", sql_escape_str(s))
        }
        (FilterOp::Ne, FilterValue::Num(n)) => format!("{quoted_field} != {n}"),
        (FilterOp::Ne, FilterValue::Str(s)) => {
            format!("{quoted_field} != '{}'", sql_escape_str(s))
        }
        (FilterOp::Gt, FilterValue::Num(n)) => format!("{quoted_field} > {n}"),
        (FilterOp::Gt, FilterValue::Str(s)) => {
            format!("{quoted_field} > '{}'", sql_escape_str(s))
        }
        (FilterOp::Lt, FilterValue::Num(n)) => format!("{quoted_field} < {n}"),
        (FilterOp::Lt, FilterValue::Str(s)) => {
            format!("{quoted_field} < '{}'", sql_escape_str(s))
        }
        (FilterOp::Gte, FilterValue::Num(n)) => format!("{quoted_field} >= {n}"),
        (FilterOp::Gte, FilterValue::Str(s)) => {
            format!("{quoted_field} >= '{}'", sql_escape_str(s))
        }
        (FilterOp::Lte, FilterValue::Num(n)) => format!("{quoted_field} <= {n}"),
        (FilterOp::Lte, FilterValue::Str(s)) => {
            format!("{quoted_field} <= '{}'", sql_escape_str(s))
        }
        (FilterOp::In, FilterValue::List(items)) => {
            let vals: Vec<String> = items.iter().map(|i| format!("'{}'", sql_escape_str(i))).collect();
            format!("{quoted_field} IN ({})", vals.join(", "))
        }
        (FilterOp::NotIn, FilterValue::List(items)) => {
            let vals: Vec<String> = items.iter().map(|i| format!("'{}'", sql_escape_str(i))).collect();
            format!("{quoted_field} NOT IN ({})", vals.join(", "))
        }
        // Fallback: shouldn't happen but handle gracefully
        (FilterOp::In | FilterOp::NotIn, _) => "1=1".to_string(),
        (_, FilterValue::List(_)) => "1=1".to_string(),
    }
}

fn sql_escape_str(s: &str) -> String {
    s.replace('\'', "''")
}

// ── Cache support ────────────────────────────────────────────────────────────

/// Compute a stable hash of a filter string for cache keys.
pub fn filter_hash(input: &str) -> u64 {
    let mut h = DefaultHasher::new();
    input.hash(&mut h);
    h.finish()
}

// ── Field name extraction ────────────────────────────────────────────────────

/// Get list of field names referenced by a filter (for projection masks).
pub fn filter_field_names(filter: &FilterExpr) -> Vec<&str> {
    filter.conditions.iter().map(|c| c.field.as_str()).collect()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_simple_eq() {
        let f = parse_filter("category:residential").unwrap();
        assert_eq!(f.conditions.len(), 1);
        assert_eq!(f.conditions[0].field, "category");
        assert_eq!(f.conditions[0].op, FilterOp::Eq);
        assert_eq!(f.conditions[0].value, FilterValue::Str("residential".into()));
    }

    #[test]
    fn parse_numeric_gt() {
        let f = parse_filter("value>100").unwrap();
        assert_eq!(f.conditions[0].op, FilterOp::Gt);
        assert_eq!(f.conditions[0].value, FilterValue::Num(100.0));
    }

    #[test]
    fn parse_gte_lte_suffix() {
        let f = parse_filter("pop>=1k").unwrap();
        assert_eq!(f.conditions[0].op, FilterOp::Gte);
        assert_eq!(f.conditions[0].value, FilterValue::Num(1000.0));

        let f = parse_filter("pop<=5.5m").unwrap();
        assert_eq!(f.conditions[0].op, FilterOp::Lte);
        assert_eq!(f.conditions[0].value, FilterValue::Num(5_500_000.0));
    }

    #[test]
    fn parse_in_list() {
        let f = parse_filter("type~highway,local,trunk").unwrap();
        assert_eq!(f.conditions[0].op, FilterOp::In);
        assert_eq!(
            f.conditions[0].value,
            FilterValue::List(vec!["highway".into(), "local".into(), "trunk".into()])
        );
    }

    #[test]
    fn parse_not_in() {
        let f = parse_filter("status!~deleted,draft").unwrap();
        assert_eq!(f.conditions[0].op, FilterOp::NotIn);
        assert_eq!(
            f.conditions[0].value,
            FilterValue::List(vec!["deleted".into(), "draft".into()])
        );
    }

    #[test]
    fn parse_ne() {
        let f = parse_filter("status!deleted").unwrap();
        assert_eq!(f.conditions[0].op, FilterOp::Ne);
        assert_eq!(f.conditions[0].value, FilterValue::Str("deleted".into()));
    }

    #[test]
    fn parse_multi_condition() {
        let f = parse_filter("category:residential;value>100").unwrap();
        assert_eq!(f.conditions.len(), 2);
        assert_eq!(f.conditions[0].field, "category");
        assert_eq!(f.conditions[0].op, FilterOp::Eq);
        assert_eq!(f.conditions[1].field, "value");
        assert_eq!(f.conditions[1].op, FilterOp::Gt);
        assert_eq!(f.conditions[1].value, FilterValue::Num(100.0));
    }

    #[test]
    fn parse_lt() {
        let f = parse_filter("value<50").unwrap();
        assert_eq!(f.conditions[0].op, FilterOp::Lt);
        assert_eq!(f.conditions[0].value, FilterValue::Num(50.0));
    }

    #[test]
    fn matches_geojson_eq() {
        let f = parse_filter("category:A").unwrap();
        let feature = json!({"type":"Feature","properties":{"category":"A"},"geometry":null});
        assert!(matches_geojson_feature(&f, &feature));

        let feature2 = json!({"type":"Feature","properties":{"category":"B"},"geometry":null});
        assert!(!matches_geojson_feature(&f, &feature2));
    }

    #[test]
    fn matches_geojson_numeric_gt() {
        let f = parse_filter("value>100").unwrap();
        let feature = json!({"type":"Feature","properties":{"value":150},"geometry":null});
        assert!(matches_geojson_feature(&f, &feature));

        let feature2 = json!({"type":"Feature","properties":{"value":50},"geometry":null});
        assert!(!matches_geojson_feature(&f, &feature2));

        let feature3 = json!({"type":"Feature","properties":{"value":100},"geometry":null});
        assert!(!matches_geojson_feature(&f, &feature3));
    }

    #[test]
    fn matches_geojson_in() {
        let f = parse_filter("type~highway,local").unwrap();
        let feature = json!({"type":"Feature","properties":{"type":"highway"},"geometry":null});
        assert!(matches_geojson_feature(&f, &feature));

        let feature2 = json!({"type":"Feature","properties":{"type":"residential"},"geometry":null});
        assert!(!matches_geojson_feature(&f, &feature2));
    }

    #[test]
    fn matches_geojson_not_in() {
        let f = parse_filter("status!~deleted,draft").unwrap();
        let feature = json!({"type":"Feature","properties":{"status":"active"},"geometry":null});
        assert!(matches_geojson_feature(&f, &feature));

        let feature2 = json!({"type":"Feature","properties":{"status":"deleted"},"geometry":null});
        assert!(!matches_geojson_feature(&f, &feature2));
    }

    #[test]
    fn matches_geojson_ne() {
        let f = parse_filter("status!deleted").unwrap();
        let feature = json!({"type":"Feature","properties":{"status":"active"},"geometry":null});
        assert!(matches_geojson_feature(&f, &feature));

        let feature2 = json!({"type":"Feature","properties":{"status":"deleted"},"geometry":null});
        assert!(!matches_geojson_feature(&f, &feature2));
    }

    #[test]
    fn matches_geojson_multi_condition() {
        let f = parse_filter("category:A;value>50").unwrap();
        let hit = json!({"type":"Feature","properties":{"category":"A","value":100},"geometry":null});
        assert!(matches_geojson_feature(&f, &hit));

        let miss_cat = json!({"type":"Feature","properties":{"category":"B","value":100},"geometry":null});
        assert!(!matches_geojson_feature(&f, &miss_cat));

        let miss_val = json!({"type":"Feature","properties":{"category":"A","value":10},"geometry":null});
        assert!(!matches_geojson_feature(&f, &miss_val));
    }

    #[test]
    fn matches_geojson_gte_lte() {
        let f = parse_filter("pop>=100;pop<=200").unwrap();
        let hit = json!({"type":"Feature","properties":{"pop":150},"geometry":null});
        assert!(matches_geojson_feature(&f, &hit));

        let edge_low = json!({"type":"Feature","properties":{"pop":100},"geometry":null});
        assert!(matches_geojson_feature(&f, &edge_low));

        let edge_high = json!({"type":"Feature","properties":{"pop":200},"geometry":null});
        assert!(matches_geojson_feature(&f, &edge_high));

        let below = json!({"type":"Feature","properties":{"pop":50},"geometry":null});
        assert!(!matches_geojson_feature(&f, &below));
    }

    #[test]
    fn filter_to_sql_produces_valid_fragments() {
        let f = parse_filter("category:residential;value>100").unwrap();
        let sql = filter_to_sql(&f);
        assert_eq!(sql, "\"category\" = 'residential' AND \"value\" > 100");
    }

    #[test]
    fn filter_to_sql_in_list() {
        let f = parse_filter("type~highway,local,trunk").unwrap();
        let sql = filter_to_sql(&f);
        assert_eq!(sql, "\"type\" IN ('highway', 'local', 'trunk')");
    }

    #[test]
    fn filter_to_sql_not_in() {
        let f = parse_filter("status!~deleted,draft").unwrap();
        let sql = filter_to_sql(&f);
        assert_eq!(sql, "\"status\" NOT IN ('deleted', 'draft')");
    }

    #[test]
    fn filter_to_sql_escapes_quotes() {
        let f = parse_filter("name:O'Brien").unwrap();
        let sql = filter_to_sql(&f);
        assert_eq!(sql, "\"name\" = 'O''Brien'");
    }

    #[test]
    fn filter_hash_stable() {
        let h1 = filter_hash("category:A;value>100");
        let h2 = filter_hash("category:A;value>100");
        assert_eq!(h1, h2);

        let h3 = filter_hash("category:B");
        assert_ne!(h1, h3);
    }

    #[test]
    fn filter_field_names_extraction() {
        let f = parse_filter("category:A;value>100;status~active,pending").unwrap();
        let names = filter_field_names(&f);
        assert_eq!(names, vec!["category", "value", "status"]);
    }

    #[test]
    fn parse_error_empty() {
        assert!(parse_filter("").is_err());
    }

    #[test]
    fn parse_error_no_operator() {
        assert!(parse_filter("justafieldname").is_err());
    }

    #[test]
    fn parse_numeric_eq_via_colon() {
        let f = parse_filter("count:42").unwrap();
        assert_eq!(f.conditions[0].op, FilterOp::Eq);
        assert_eq!(f.conditions[0].value, FilterValue::Num(42.0));
    }

    #[test]
    fn matches_geojson_missing_field() {
        let f = parse_filter("nonexistent:value").unwrap();
        let feature = json!({"type":"Feature","properties":{"other":"x"},"geometry":null});
        assert!(!matches_geojson_feature(&f, &feature));
    }

    #[test]
    fn matches_geojson_ne_missing_field() {
        let f = parse_filter("nonexistent!value").unwrap();
        let feature = json!({"type":"Feature","properties":{"other":"x"},"geometry":null});
        // Missing field should pass Ne (it's not equal to "value")
        assert!(matches_geojson_feature(&f, &feature));
    }
}
