//! Wire encoding for complete serialized node records.

use serde_json::Value;

pub(crate) const FORMAT: &str = "rows-v1";
pub(crate) const COLUMNS: [&str; 6] = ["id", "parentId", "sortKey", "kind", "props", "extra"];

pub(crate) fn pack(node: Value) -> Value {
    let Value::Object(mut fields) = node else {
        unreachable!("serialized node must be an object");
    };
    let mut row = Vec::with_capacity(COLUMNS.len());
    for column in &COLUMNS[..5] {
        row.push(fields.remove(*column).expect("serialized node core field"));
    }
    if !fields.is_empty() {
        row.push(Value::Object(fields));
    }
    Value::Array(row)
}
