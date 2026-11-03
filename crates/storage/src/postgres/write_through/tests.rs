use super::*;
use serde_json::json;

fn before() -> Snapshot {
    serde_json::from_value(json!({
        "schemaVersion": crate::SCHEMA_VERSION, "projectId": "p", "slug": "p", "name": "P",
        "revision": 4,
        "nodes": [
            {"id":"a","parentId":null,"sortKey":1,"kind":"page","props":{"name":"A"}},
            {"id":"b","parentId":null,"sortKey":2,"kind":"page","props":{"name":"B"}}
        ],
        "tokens": [{"id":"t","name":"T","type":"number","value":0.0}],
        "styles": [], "notes": [],
        "assets": [{"id":"asset_1","contentHash":"h1","mediaType":"image/png","byteSize":1,
            "relativeKey":"h1","width":1,"height":1,"status":"ready"}],
        "fonts": [{"id":"f","family":"F","weight":400,"style":"normal","assetId":"asset_2",
            "bundledId":null,"contentHash":"h2","license":null,"status":"ready"}],
        "retention": {"oldestCommitRevision": 1, "idempotencyPolicy": "kept"}
    }))
    .unwrap()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn rebuilds_rows_from_stored_text_with_shared_order_and_readiness() {
    let asset = r#"{"id":"asset_2","contentHash":"h2","mediaType":"font/ttf","byteSize":1,"relativeKey":"h2","width":null,"height":null,"status":"ready"}"#;
    let written = transaction::Written {
        kinds: strings(&["node", "token", "asset"]),
        ids: strings(&["c", "t", "asset_2"]),
        payloads: vec![
            r#"{"id":"c","parentId":null,"sortKey":0,"kind":"page","props":{"name":"C"}}"#.into(),
            r#"{"id":"t","name":"T","type":"number","value":-0.0}"#.into(),
            asset.into(),
        ],
        removed_kinds: strings(&["node"]),
        removed_ids: strings(&["b"]),
    };
    let binaries = Binaries::from(["h2".to_string()]);
    let committed = Committed::new(before(), binaries, written, 5, (7, "db".into()));
    let snapshot = committed.snapshot().unwrap();
    assert_eq!(snapshot.revision, 5);
    let nodes: Vec<_> = snapshot.nodes.iter().map(|node| node.id.as_str()).collect();
    assert_eq!(nodes, ["c", "a"]);
    let zero = snapshot.tokens[0].value.as_f64().unwrap();
    assert_eq!(zero.to_bits(), (-0.0_f64).to_bits());
    let statuses: Vec<_> = snapshot.assets.iter().map(|a| a.status.as_str()).collect();
    assert_eq!(statuses, ["missing", "ready"]);
    assert_eq!(snapshot.fonts[0].status, "ready");
}

#[test]
fn fills_only_a_cache_bound_to_the_observed_namespace() {
    let empty = || transaction::Written {
        kinds: vec![],
        ids: vec![],
        payloads: vec![],
        removed_kinds: vec![],
        removed_ids: vec![],
    };
    let cache = SnapshotCache::new("db");
    Committed::new(before(), Binaries::new(), empty(), 5, (7, "other".into())).fill(Some(&cache));
    assert!(cache.get("p", 7, 5).is_none());
    Committed::new(before(), Binaries::new(), empty(), 5, (7, "db".into())).fill(Some(&cache));
    let cached = cache.get("p", 7, 5).unwrap();
    assert_eq!(cached.assets[0].status, "missing");
    assert_eq!(cached.fonts[0].status, "missing");
}
