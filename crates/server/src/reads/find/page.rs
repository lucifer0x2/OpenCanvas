//! Retain only the requested response prefix while still counting every match.
use super::super::pagination;
use crate::error::{ApiError, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde_json::{json, Value};

pub(super) struct Matches {
    pub total: usize,
    offset: usize,
    limit: usize,
    budget: usize,
    bytes: usize,
    closed: bool,
    entries: Vec<Value>,
    pub materialized: usize,
}

impl Matches {
    pub fn new(offset: usize, limit: usize, budget: usize) -> Self {
        Self {
            total: 0,
            offset,
            limit,
            budget,
            bytes: 0,
            closed: false,
            entries: Vec::new(),
            materialized: 0,
        }
    }

    pub fn push(&mut self, make: impl FnOnce() -> Value) -> Result<()> {
        let position = self.total;
        self.total += 1;
        if position < self.offset || self.closed || self.entries.len() >= self.limit {
            return Ok(());
        }
        let value = make();
        self.materialized += 1;
        let bytes = pagination::json_len(&value)?;
        // An oversized first entry must reach finish(): the full reference reports its
        // item error only after scope/cursor/count validation and metadata sizing.
        if self.entries.is_empty() || self.bytes.saturating_add(bytes) <= self.budget {
            self.bytes = self.bytes.saturating_add(bytes);
            self.entries.push(value);
        } else {
            self.closed = true;
        }
        Ok(())
    }

    pub fn retained(&self) -> usize {
        self.entries.len()
    }

    pub fn finish(self, mut response: Value, scope: &str) -> Result<Value> {
        if self.offset > self.total {
            return Err(ApiError::new(
                "INVALID_CURSOR",
                "Cursor is malformed, stale or belongs to another read scope; restart the read",
            ));
        }
        response["truncated"] = json!(false);
        response["nextCursor"] = Value::Null;
        let base = pagination::json_len(&response)?;
        if base > self.budget {
            return Err(ApiError::new("CONTEXT_ITEM_TOO_LARGE", "Context metadata exceeds the response budget; request fewer viewports or a larger budget"));
        }
        let mut added = 0;
        let mut next = self.offset;
        for value in self.entries {
            let grow = pagination::json_len(&value)? + usize::from(next > self.offset);
            if base + added + grow + cursor(scope, next + 1).len() - 3 > self.budget {
                if next == self.offset {
                    return Err(ApiError::new("CONTEXT_ITEM_TOO_LARGE", "One complete item exceeds the response budget; use a property filter or larger budget"));
                }
                break;
            }
            response["nodes"].as_array_mut().unwrap().push(value);
            added += grow;
            next += 1;
        }
        let truncated = next < self.total;
        response["truncated"] = json!(truncated);
        response["nextCursor"] = if truncated {
            json!(cursor(scope, next))
        } else {
            Value::Null
        };
        Ok(response)
    }
}

fn cursor(scope: &str, offset: usize) -> String {
    URL_SAFE_NO_PAD.encode(json!({"scope":scope,"offset":offset}).to_string())
}

#[cfg(test)]
#[path = "page_tests.rs"]
mod tests;
