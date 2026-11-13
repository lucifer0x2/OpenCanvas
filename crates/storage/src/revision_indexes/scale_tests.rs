use super::*;

#[test]
#[ignore = "metadata-only 200k admission probe; no validation or runtime writes"]
fn two_hundred_thousand_nodes_remain_cacheable_above_payload_limit() {
    let mut snapshot: Snapshot = serde_json::from_value(json!({
        "schemaVersion":SCHEMA_VERSION,"projectId":"scale","slug":"scale","name":"Scale","revision":1,
        "nodes":[],"tokens":[],"styles":[],"fonts":[],"assets":[],"notes":[],
        "retention":{"oldestCommitRevision":1,"idempotencyPolicy":"kept"}
    })).unwrap();
    for page in 0..20 {
        snapshot.nodes.push(Node {
            id: format!("page_{page}"),
            kind: NodeKind::Page,
            parent_id: None,
            sort_key: page,
            props: json!({}),
        });
    }
    for i in 0..200_000 {
        snapshot.nodes.push(Node {
            id: format!("node_{i}"),
            kind: NodeKind::Rectangle,
            parent_id: Some(format!("page_{}", i % 20)),
            sort_key: i,
            props: json!({"width":10,"height":10,"name":"x".repeat(100)}),
        });
    }
    // The names alone exceed payload admission; never encode/hash the snapshot.
    assert!(200_000 * 100 > 16 * 1024 * 1024);
    let identity = SnapshotCacheIdentity {
        project_id: snapshot.project_id.clone(),
        schema_version: snapshot.schema_version,
        revision: 1,
        incarnation: "scale:1".into(),
        asset_readiness: vec![],
    };
    let cache = IndexCache::default();
    let time = Instant::now();
    let first = cache.for_snapshot(&snapshot, Some(identity.clone()));
    let cold = time.elapsed();
    let time = Instant::now();
    let second = cache.for_snapshot(&snapshot, Some(identity));
    let warm = time.elapsed();
    assert!(Arc::ptr_eq(&first, &second));
    assert!(first.weight() < CACHE_BUDGET as u32);
    assert!(first.dependencies.lock().unwrap().is_none());
    eprintln!("metadata scale authored={} weight_bytes={} reserved_dependency_bytes={} cold_ms={:.3} warm_us={:.3}", snapshot.nodes.len(), first.weight(), DEPENDENCY_BUDGET, cold.as_secs_f64()*1000.0, warm.as_secs_f64()*1_000_000.0);
}
