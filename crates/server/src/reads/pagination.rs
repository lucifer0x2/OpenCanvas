use super::ReadRequest;
use crate::error::{ApiError, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use opencanvas_storage::Snapshot;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub fn scope(s: &Snapshot, tool: &str, req: &ReadRequest) -> Result<(String, usize)> {
    if req.revision.is_some_and(|r| r != s.revision) {
        return Err(ApiError::new(
            "REVISION_UNAVAILABLE",
            "Requested read revision is unavailable",
        ));
    }
    let mut query = serde_json::to_value(req)?;
    query.as_object_mut().unwrap().remove("cursor");
    let scope = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&json!([
            s.project_id,
            s.revision,
            tool,
            query
        ]))?)
    );
    let offset = offset(&scope, req.cursor.as_deref())?;
    Ok((scope, offset))
}
/// Scope hash and offset for a read whose identity is not a snapshot query, such as an
/// immutable render job: `identity` must hold every parameter the cursor binds.
pub fn scope_of(identity: &Value, cursor: Option<&str>) -> Result<(String, usize)> {
    let scope = format!("{:x}", Sha256::digest(serde_json::to_vec(identity)?));
    let offset = offset(&scope, cursor)?;
    Ok((scope, offset))
}
fn offset(scope: &str, cursor: Option<&str>) -> Result<usize> {
    let Some(raw) = cursor else { return Ok(0) };
    let decoded = URL_SAFE_NO_PAD.decode(raw).map_err(|_| invalid())?;
    let value: Value = serde_json::from_slice(&decoded).map_err(|_| invalid())?;
    if value["scope"] != scope {
        return Err(invalid());
    }
    value["offset"]
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(invalid)
}
fn invalid() -> ApiError {
    ApiError::new(
        "INVALID_CURSOR",
        "Cursor is malformed, stale or belongs to another read scope; restart the read",
    )
}
fn cursor(scope: &str, offset: usize) -> String {
    URL_SAFE_NO_PAD.encode(json!({"scope":scope,"offset":offset}).to_string())
}
/// Every entry is consumed once, including resources; a budget never silently drops content.
/// The response is serialized once; each step then adds the exact compact-JSON growth (the
/// entry, its comma, a newly created array key and the cursor length change), so a page costs
/// O(bytes) instead of re-serializing the whole response per entry. Page boundaries and bytes
/// are the same as measuring the full response after every entry (`pagination_tests.rs`).
pub fn page(
    mut response: Value,
    entries: Vec<(String, Value, usize)>,
    offset: usize,
    limit: usize,
    budget: usize,
    depth: usize,
    scope: &str,
) -> Result<Value> {
    if offset > entries.len() {
        return Err(invalid());
    }
    let mut next = offset;
    let base_depth = entries.get(offset).map_or(0, |e| e.2);
    response["truncated"] = json!(false);
    response["nextCursor"] = Value::Null;
    let base = json_len(&response)?;
    if base > budget {
        return Err(ApiError::new("CONTEXT_ITEM_TOO_LARGE", "Context metadata exceeds the response budget; request fewer viewports or a larger budget"));
    }
    // While a page is open it is measured as truncated:true with the cursor after the last
    // entry: `false`→`true` is −1 byte and `null`→`"<cursor>"` is +len−2, hence `len − 3`.
    let mut added = 0;
    while let Some((field, value, d)) = entries.get(next) {
        if next - offset >= limit || *d > base_depth + depth {
            break;
        }
        if field != "nodes"
            && response[field]
                .as_array()
                .is_some_and(|list| list.len() >= 100)
        {
            break;
        }
        if response.get(field).is_none() {
            response[field] = json!([]);
            added += json_len(field.as_str())? + 4; // `,"field":[]`
        }
        let list = response[field].as_array_mut().unwrap();
        let grow = json_len(value)? + usize::from(!list.is_empty());
        list.push(value.clone());
        if base + added + grow + cursor(scope, next + 1).len() - 3 > budget {
            list.pop();
            if next == offset {
                return Err(ApiError::new("CONTEXT_ITEM_TOO_LARGE", "One complete item exceeds the response budget; use a property filter or larger budget"));
            }
            break;
        }
        added += grow;
        next += 1;
    }
    let truncated = next < entries.len();
    response["truncated"] = json!(truncated);
    response["nextCursor"] = if truncated {
        json!(cursor(scope, next))
    } else {
        Value::Null
    };
    Ok(response)
}
/// Compact JSON length of `value`, counted without building the string.
fn json_len<T: Serialize + ?Sized>(value: &T) -> Result<usize> {
    struct Count(usize);
    impl std::io::Write for Count {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count(0);
    serde_json::to_writer(&mut count, value)?;
    Ok(count.0)
}
#[cfg(test)]
#[path = "pagination_tests.rs"]
mod tests;
