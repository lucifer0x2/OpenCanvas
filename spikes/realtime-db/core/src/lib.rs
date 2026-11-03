//! Deliberately small portable command core; not production validator parity.
mod command;
mod delta;
#[cfg(test)]
mod delta_tests;
mod profile;
#[cfg(test)]
mod tests;
mod validation;

pub use delta::{apply_delta, scalar_update_ids, VALIDATOR_VERSION};
pub use profile::ApplyProfile;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct State {
    pub project_id: String,
    pub revision: u64,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub parent_id: Option<String>,
    pub kind: String,
    pub sort_key: i64,
    pub props: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchRequest {
    pub project_id: String,
    pub expected_revision: u64,
    pub idempotency_key: String,
    pub operations: Vec<Value>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchResult {
    pub project_id: String,
    pub committed_revision: u64,
    pub changed_ids: Vec<String>,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_revision: Option<u64>,
}

impl DomainError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            current_revision: None,
        }
    }
}

pub fn canonical_hash(request: &BatchRequest) -> Result<String, DomainError> {
    validation::request(request)?;
    let mut value = serde_json::to_value(request).map_err(validation::json_error)?;
    value.as_object_mut().unwrap().remove("dryRun");
    let bytes = serde_json::to_vec(&canonical(value)).map_err(validation::json_error)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

// Explicit sorting survives feature unification enabling serde_json/preserve_order.
fn canonical(value: Value) -> Value {
    match value {
        Value::Object(object) => {
            let sorted: std::collections::BTreeMap<_, _> = object.into_iter().collect();
            Value::Object(sorted.into_iter().map(|(k, v)| (k, canonical(v))).collect())
        }
        Value::Array(values) => Value::Array(values.into_iter().map(canonical).collect()),
        other => other,
    }
}

pub fn apply(state: &State, request: &BatchRequest) -> Result<(State, Vec<String>), DomainError> {
    apply_inner(state, request, None)
}

pub fn apply_profiled(
    state: &State,
    request: &BatchRequest,
    profile: &mut ApplyProfile,
) -> Result<(State, Vec<String>), DomainError> {
    apply_inner(state, request, Some(profile))
}

fn apply_inner(
    state: &State,
    request: &BatchRequest,
    mut profile: Option<&mut ApplyProfile>,
) -> Result<(State, Vec<String>), DomainError> {
    validation::request(request)?;
    if state.project_id != request.project_id {
        return Err(DomainError::new(
            "PROJECT_NOT_FOUND",
            "Project does not match state",
        ));
    }
    if state.revision != request.expected_revision {
        return Err(DomainError {
            code: "REVISION_CONFLICT".into(),
            message: "Expected revision does not match".into(),
            current_revision: Some(state.revision),
        });
    }
    let mut next = profile::measured(profile.as_mut().map(|p| &mut p.clone_us), || state.clone());
    let changed = profile::measured(profile.as_mut().map(|p| &mut p.execute_us), || {
        command::execute(&mut next.nodes, &request.operations)
    })?;
    profile::measured(profile.as_mut().map(|p| &mut p.validate_us), || {
        validation::candidate(&next.nodes)
    })?;
    next.revision = state
        .revision
        .checked_add(1)
        .ok_or_else(|| DomainError::new("VALIDATION_FAILED", "Revision exhausted"))?;
    profile::measured(profile.as_mut().map(|p| &mut p.sort_us), || {
        next.nodes.sort_by(|a, b| a.id.cmp(&b.id));
    });
    Ok((next, changed))
}
