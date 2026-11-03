//! Selected-row updates over an already certified project revision.
use crate::{command, profile, validation, ApplyProfile, BatchRequest, DomainError, State};
use std::collections::BTreeSet;

// Bump whenever any candidate rule, operation behavior, or supported kind changes.
// Certificates are private metadata, never proof of production validator parity.
pub const VALIDATOR_VERSION: &str = "spike-scalar-v1";

/// Ineligible or malformed operations use the full path, preserving its ordered
/// errors. Classification does not validate props or reject transient bad values.
pub fn scalar_update_ids(request: &BatchRequest) -> Option<Vec<String>> {
    let mut ids = BTreeSet::new();
    for value in &request.operations {
        match serde_json::from_value::<command::Operation>(value.clone()).ok()? {
            command::Operation::Update { id, .. } => {
                // PostgreSQL TEXT cannot bind NUL. Let the full executor retain
                // ordered missing-node/props errors instead of a premature SQL error.
                if id.contains('\0') {
                    return None;
                }
                ids.insert(id);
            }
            _ => return None,
        }
    }
    Some(ids.into_iter().collect())
}

/// `selected` must contain exactly the existing rows named by an update-only
/// request, from the locked revision certified with VALIDATOR_VERSION. These
/// operations cannot change IDs, kinds, parent topology or untouched predicates.
pub fn apply_delta(
    selected: &State,
    request: &BatchRequest,
    mut metrics: Option<&mut ApplyProfile>,
) -> Result<(State, Vec<String>), DomainError> {
    validation::request(request)?;
    if scalar_update_ids(request).is_none() {
        return Err(DomainError::new(
            "VALIDATION_FAILED",
            "Delta requires updates only",
        ));
    }
    if selected.project_id != request.project_id {
        return Err(DomainError::new(
            "PROJECT_NOT_FOUND",
            "Project does not match state",
        ));
    }
    if selected.revision != request.expected_revision {
        return Err(DomainError {
            code: "REVISION_CONFLICT".into(),
            message: "Expected revision does not match".into(),
            current_revision: Some(selected.revision),
        });
    }
    let mut next = profile::measured(metrics.as_mut().map(|p| &mut p.clone_us), || {
        selected.clone()
    });
    let changed = profile::measured(metrics.as_mut().map(|p| &mut p.execute_us), || {
        command::execute(&mut next.nodes, &request.operations)
    })?;
    // Full validation checks properties only after all operations. In particular,
    // a bad scalar temporarily overwritten by a valid one must remain accepted.
    profile::measured(metrics.as_mut().map(|p| &mut p.validate_us), || {
        next.nodes.iter().try_for_each(validation::properties)
    })?;
    next.revision = selected
        .revision
        .checked_add(1)
        .ok_or_else(|| DomainError::new("VALIDATION_FAILED", "Revision exhausted"))?;
    // Selected SQL rows already arrive in ID order; updates preserve this order.
    Ok((next, changed))
}
