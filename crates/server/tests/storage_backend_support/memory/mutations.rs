use super::*;

pub(super) fn apply(document: &mut Document, request: BatchRequest) -> Result<BatchResult> {
    if request.project_id != document.snapshot.project_id {
        return Err(not_found());
    }
    let fingerprint = canonical_json(&serde_json::to_value(&request)?);
    if let Some((_, original, result)) = document
        .replays
        .iter()
        .find(|(key, _, _)| key == &request.idempotency_key)
    {
        if *original != fingerprint {
            return Err(StorageError::new(
                "IDEMPOTENCY_KEY_REUSED",
                "Fixture key collision",
            ));
        }
        let mut result = result.clone();
        result.replayed = true;
        return Ok(result);
    }
    if request.expected_revision != document.snapshot.revision {
        return Err(StorageError::conflict(document.snapshot.revision));
    }
    // Fixture supports just one page creation, sufficient to prove mutation routing.
    let [Operation::CreateNode {
        id,
        kind: NodeKind::Page,
        parent_id: None,
        props,
        ..
    }] = request.operations.as_slice()
    else {
        return Err(unsupported());
    };
    if document.snapshot.nodes.iter().any(|n| n.id == *id) {
        return Err(unsupported());
    }
    document.snapshot.nodes.push(Node {
        id: id.clone(),
        parent_id: None,
        sort_key: 0,
        kind: NodeKind::Page,
        props: props.clone(),
    });
    document.snapshot.revision += 1;
    let result = BatchResult {
        project_id: request.project_id,
        committed_revision: document.snapshot.revision,
        changed_ids: vec![id.clone()],
        render_status: "pending".into(),
        replayed: false,
    };
    document.commits.push(Commit {
        revision: result.committed_revision,
        base_revision: request.expected_revision,
        batch_id: request.idempotency_key.clone(),
        operations: serde_json::to_value(request.operations)?,
        changed_ids: result.changed_ids.clone(),
    });
    document
        .replays
        .push((request.idempotency_key, fingerprint, result.clone()));
    Ok(result)
}
