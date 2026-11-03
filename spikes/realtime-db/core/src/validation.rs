use crate::{BatchRequest, DomainError, Node};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub fn json_error(error: serde_json::Error) -> DomainError {
    DomainError::new("VALIDATION_FAILED", error.to_string())
}

fn fail(message: &str) -> DomainError {
    DomainError::new("VALIDATION_FAILED", message)
}

pub fn request(request: &BatchRequest) -> Result<(), DomainError> {
    for value in [&request.project_id, &request.idempotency_key] {
        if value.is_empty()
            || value.len() > 128
            || !value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
        {
            return Err(fail(
                "Project and idempotency IDs require 1–128 ASCII letters/digits/_/-",
            ));
        }
    }
    if request.operations.is_empty() || request.operations.len() > 500 {
        return Err(fail("Batch requires 1–500 operations"));
    }
    if serde_json::to_vec(request).map_err(json_error)?.len() > 2 * 1024 * 1024 {
        return Err(fail("Batch exceeds 2 MiB"));
    }
    Ok(())
}

pub fn candidate(nodes: &[Node]) -> Result<(), DomainError> {
    let by_id: BTreeMap<_, _> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    if by_id.len() != nodes.len() {
        return Err(fail("Duplicate node ID"));
    }
    for node in nodes {
        if node.id.is_empty() || node.id.len() > 128 {
            return Err(fail("Node ID requires 1–128 bytes"));
        }
        if !["page", "frame", "group", "text"].contains(&node.kind.as_str()) {
            return Err(fail("Unsupported node kind"));
        }
        match (&node.parent_id, node.kind.as_str()) {
            (None, "page") => {}
            (Some(parent), kind) if kind != "page" => {
                let parent = by_id
                    .get(parent.as_str())
                    .ok_or_else(|| fail("Parent missing"))?;
                if parent.kind == "text" {
                    return Err(fail("Text cannot parent nodes"));
                }
            }
            _ => return Err(fail("Pages are roots; all other nodes require a parent")),
        }
        let mut seen = BTreeSet::new();
        let mut cursor = Some(node.id.as_str());
        while let Some(id) = cursor {
            if !seen.insert(id) {
                return Err(fail("Parent cycle"));
            }
            cursor = by_id.get(id).and_then(|n| n.parent_id.as_deref());
        }
        properties(node)?;
    }
    Ok(())
}

// The scalar domain has no references or cross-node property predicates. Every
// property rule is shared by full and delta validation, at final-candidate time.
pub(crate) fn properties(node: &Node) -> Result<(), DomainError> {
    let props = node
        .props
        .as_object()
        .ok_or_else(|| fail("Props must be an object"))?;
    for (key, value) in props {
        if ![
            "name", "text", "x", "y", "width", "height", "fontSize", "opacity", "fill",
        ]
        .contains(&key.as_str())
        {
            return Err(fail("Unknown property in spike subset"));
        }
        match key.as_str() {
            "name" | "text" | "fill" => {
                let text = value
                    .as_str()
                    .ok_or_else(|| fail("Text properties require strings"))?;
                if text.len() > 100_000 {
                    return Err(fail("Text exceeds 100000 bytes"));
                }
                if key == "text" && node.kind != "text" {
                    return Err(fail("Text property requires text node"));
                }
            }
            _ => numeric(key, value)?,
        }
    }
    Ok(())
}

fn numeric(key: &str, value: &Value) -> Result<(), DomainError> {
    let n = value
        .as_f64()
        .ok_or_else(|| fail("Numeric property requires a number"))?;
    let range = match key {
        "x" | "y" => -100_000.0..=100_000.0,
        "opacity" => 0.0..=1.0,
        _ => 0.0..=100_000.0,
    };
    if !n.is_finite() || !range.contains(&n) {
        return Err(fail("Numeric property out of bounds"));
    }
    Ok(())
}
