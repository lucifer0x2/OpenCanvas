//! Certificate gating and selected-row materialization for scalar spike updates.
use crate::{db, profile::BatchProfile};
use oc_spike_core::{
    apply, apply_delta, apply_profiled, scalar_update_ids, BatchRequest, DomainError, Node, State,
    VALIDATOR_VERSION,
};
use tokio_postgres::{types::Type, Transaction};

pub async fn candidate(
    tx: &Transaction<'_>,
    request: &BatchRequest,
    revision: u64,
    validated_revision: Option<i64>,
    validator_version: Option<&str>,
    profile: &mut BatchProfile,
) -> Result<(State, Vec<String>), DomainError> {
    let ids = scalar_update_ids(request);
    let fallback = if ids.is_none() {
        Some("structuralOrMalformed")
    } else if validated_revision.is_none() || validator_version.is_none() {
        Some("missingCertificate")
    } else if validator_version != Some(VALIDATOR_VERSION) {
        Some("validatorVersion")
    } else if validated_revision != i64::try_from(revision).ok() {
        Some("certificateRevision")
    } else {
        None
    };
    profile.evidence.path = if fallback.is_some() { "full" } else { "delta" };
    profile.evidence.fallback_reason = fallback;
    let before = if fallback.is_none() {
        selected(tx, request, revision, &ids.unwrap(), profile).await?
    } else {
        let state = db::state_profiled(tx, &request.project_id, revision, profile).await?;
        profile.evidence.queried_node_count = state.nodes.len();
        state
    };
    profile.evidence.validated_node_count = before.nodes.len();
    let timer = profile.timer();
    let result = if fallback.is_none() {
        let metrics = profile.enabled().then_some(&mut profile.core);
        apply_delta(&before, request, metrics)
    } else if profile.enabled() {
        apply_profiled(&before, request, &mut profile.core)
    } else {
        apply(&before, request)
    };
    profile.record("apply", timer);
    result
}

async fn selected(
    tx: &Transaction<'_>,
    request: &BatchRequest,
    revision: u64,
    ids: &[String],
    profile: &mut BatchProfile,
) -> Result<State, DomainError> {
    let timer = profile.timer();
    let rows = tx.query_typed(
        "SELECT node_id,parent_id,kind,sort_key,props FROM nodes WHERE project_id=$1 AND node_id=ANY($2) ORDER BY node_id",
        &[(&request.project_id, Type::TEXT), (&ids, Type::TEXT_ARRAY)],
    ).await.map_err(db::internal)?;
    profile.record("nodeQuery", timer);
    profile.evidence.queried_node_count = rows.len();
    let timer = profile.timer();
    let nodes = rows
        .into_iter()
        .map(|row| Node {
            id: row.get(0),
            parent_id: row.get(1),
            kind: row.get(2),
            sort_key: row.get(3),
            props: row.get(4),
        })
        .collect();
    profile.record("nodeDecode", timer);
    Ok(State {
        project_id: request.project_id.clone(),
        revision,
        nodes,
    })
}
