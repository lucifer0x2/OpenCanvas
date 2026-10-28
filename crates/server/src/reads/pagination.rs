use super::ReadRequest;
use crate::error::{ApiError, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use opencanvas_storage::Snapshot;
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
    let offset = if let Some(raw) = &req.cursor {
        let decoded = URL_SAFE_NO_PAD.decode(raw).map_err(|_| invalid())?;
        let value: Value = serde_json::from_slice(&decoded).map_err(|_| invalid())?;
        if value["scope"] != scope {
            return Err(invalid());
        }
        value["offset"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(invalid)?
    } else {
        0
    };
    Ok((scope, offset))
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
        }
        response[field].as_array_mut().unwrap().push(value.clone());
        response["nextCursor"] = json!(cursor(scope, next + 1));
        response["truncated"] = json!(true);
        if serde_json::to_vec(&response)?.len() > budget {
            response[field].as_array_mut().unwrap().pop();
            if next == offset {
                return Err(ApiError::new("CONTEXT_ITEM_TOO_LARGE", "One complete item exceeds the response budget; use a property filter or larger budget"));
            }
            break;
        }
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
