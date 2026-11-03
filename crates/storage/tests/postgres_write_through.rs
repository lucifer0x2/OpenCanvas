mod postgres_support;
use opencanvas_storage::*;
use postgres_support::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;

const FONT: &[u8] = include_bytes!("../../../fixtures/fonts/Lato-Regular.ttf");
// Complete 1x1 RGBA PNG.
const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 11, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1, 13, 10,
    45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

fn ops(value: Value) -> Vec<Operation> {
    serde_json::from_value(value).unwrap()
}

fn import(
    id: &str,
    revision: u64,
    key: &str,
    bytes: &[u8],
    font: Option<&str>,
) -> ImportAssetRequest {
    ImportAssetRequest {
        project_id: id.into(),
        expected_revision: revision,
        idempotency_key: key.into(),
        media_type: if font.is_some() {
            "font/ttf"
        } else {
            "image/png"
        }
        .into(),
        bytes: bytes.into(),
        font: font.map(|id| FontRegistration {
            id: id.into(),
            family: "Lato".into(),
            weight: 400,
            style: "normal".into(),
            license: Some("OFL-1.1".into()),
        }),
    }
}

/// The writer's cache serves the committed revision while the entity tables
/// are locked (no SQL entity read), identical to a fresh uncached read.
fn assert_written_through(cached: &Storage, fresh: &Storage, id: &str, revision: u64) -> Snapshot {
    let mut control = sql();
    let lock = lock_entity_tables(&mut control);
    let hot = cached.snapshot(id).unwrap();
    lock.rollback().unwrap();
    let cold = fresh.snapshot(id).unwrap();
    assert_eq!(hot.revision, revision);
    assert_eq!(hot, cold);
    // Domain equality treats -0.0 and 0.0 as equal; serialized bytes do not.
    assert_eq!(
        serde_json::to_string(&hot).unwrap(),
        serde_json::to_string(&cold).unwrap()
    );
    hot
}

fn zero(value: Value) -> Vec<Operation> {
    ops(json!([
        {"op":"upsert_token","token":{"id":"zero","name":"Zero","type":"number","value":value}},
        {"op":"update_node","id":"box","props":{"shadow":[{"x":value,"color":"#000000"}]}}
    ]))
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn every_commit_fills_the_cache_with_the_exact_uncached_snapshot() {
    let (backend, _) = cached();
    let storage = Storage::new(backend);
    let (_, fresh) = setup();
    let p = project(&storage, "write-through");
    let id = p.id();
    let mut seed = page("Seed");
    seed.extend(ops(json!([
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page","props":{"width":100,"height":100}},
        {"op":"create_node","id":"box","kind":"rectangle","parentId":"frame","props":{"width":10,"height":10}},
        {"op":"create_node","id":"gone","kind":"rectangle","parentId":"frame","props":{"width":5,"height":5}},
        {"op":"upsert_style","style":{"id":"fill","name":"Fill","kind":"fill","value":"#112233"}},
        {"op":"create_note","id":"note","nodeId":"box","text":"Note\0text","author":"a"}
    ])));
    seed.extend(zero(json!(-0.0)));
    let mut revision = p
        .apply_batch(request(id, 0, "seed", seed))
        .unwrap()
        .committed_revision;
    assert_written_through(&storage, &fresh, id, revision);

    let batches = [
        ops(json!([
            {"op":"update_node","id":"box","props":{"width":20}},
            {"op":"delete_subtree","id":"gone"},
            {"op":"create_node","id":"first","kind":"rectangle","parentId":"frame","index":0,"props":{"width":1,"height":1}}
        ])),
        ops(json!([
            {"op":"upsert_token","token":{"id":"accent","name":"Accent","type":"color","value":"#abcdef"}},
            {"op":"upsert_style","style":{"id":"fill","name":"Fill","kind":"fill","value":"#445566"}},
            {"op":"reply_note","id":"note","text":"Reply","author":"b"}
        ])),
        ops(json!([{"op":"delete_note","id":"note"}])),
        zero(json!(0.0)),
        zero(json!(-0.0)),
    ];
    for (index, operations) in batches.into_iter().enumerate() {
        let key = format!("batch-{index}");
        revision = p
            .apply_batch(request(id, revision, &key, operations))
            .unwrap()
            .committed_revision;
        assert_written_through(&storage, &fresh, id, revision);
    }
    let signed = assert_written_through(&storage, &fresh, id, revision);
    let token = signed
        .tokens
        .iter()
        .find(|token| token.id == "zero")
        .unwrap();
    assert_eq!(serde_json::to_string(&token.value).unwrap(), "-0.0");

    let imports = [
        ("png-new", PNG, None),
        ("png-present", PNG, None),
        ("font-new", FONT, Some("lato")),
        ("font-present", FONT, Some("lato-copy")),
    ];
    for (key, bytes, font) in imports {
        let imported = p
            .import_asset(import(id, revision, key, bytes, font))
            .unwrap();
        revision = imported.result.committed_revision;
        let hot = assert_written_through(&storage, &fresh, id, revision);
        assert!(hot.assets.iter().all(|asset| asset.status == "ready"));
    }

    // A binary removed outside the domain is reported missing by the next
    // commit's snapshot exactly as by an uncached read.
    let png_hash = format!("{:x}", Sha256::digest(PNG));
    sql()
        .execute(
            "DELETE FROM opencanvas_runtime_v1.binaries WHERE project_id=$1 AND content_hash=$2",
            &[&id, &png_hash],
        )
        .unwrap();
    let operations = ops(json!([{"op":"update_node","id":"page","props":{"name":"Renamed"}}]));
    revision = p
        .apply_batch(request(id, revision, "after-missing", operations))
        .unwrap()
        .committed_revision;
    let hot = assert_written_through(&storage, &fresh, id, revision);
    let png = hot
        .assets
        .iter()
        .find(|a| a.content_hash == png_hash)
        .unwrap();
    assert_eq!(png.status, "missing");
    let fonts: Vec<_> = hot
        .fonts
        .iter()
        .map(|f| (f.id.as_str(), f.status.as_str()))
        .collect();
    assert!(fonts.contains(&("lato", "ready")));
    assert!(fonts.contains(&("lato-copy", "ready")));
}

/// Shared resident snapshot handed out by the cache, or a busy error when the
/// read needed the locked entity tables.
fn resident(storage: &Storage, id: &str) -> std::result::Result<Arc<Snapshot>, String> {
    let mut control = sql();
    let lock = lock_entity_tables(&mut control);
    let read = storage.snapshot_with_cache_identity(id);
    lock.rollback().unwrap();
    read.map(|(snapshot, _)| snapshot)
        .map_err(|error| error.code)
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn replays_dry_runs_failures_and_conflicts_leave_the_cache_unchanged() {
    let (writer, _) = cached();
    let writer = Storage::new(writer);
    let p = project(&writer, "no-fill");
    let id = p.id();
    let seed = request(id, 0, "seed", page("Seed"));
    p.apply_batch(seed.clone()).unwrap();
    let asset = import(id, 1, "png", PNG, None);
    p.import_asset(asset.clone()).unwrap();
    let warm = resident(&writer, id).unwrap();
    assert_eq!(warm.revision, 2);

    // A second backend with an empty cache proves no outcome below fills it.
    let (empty, _) = cached();
    let empty = Storage::new(empty);
    let handle = empty.open_project(id).unwrap();
    assert!(handle.apply_batch(seed.clone()).unwrap().replayed);
    assert!(handle.import_asset(asset.clone()).unwrap().result.replayed);
    let rename = request(id, 2, "dry", rename("Dry"));
    assert!(handle.dry_run_batch(rename.clone()).unwrap().dry_run);
    let invalid = ops(json!([{"op":"update_node","id":"absent","props":{"name":"x"}}]));
    assert!(handle
        .apply_batch(request(id, 2, "invalid", invalid))
        .is_err());
    let conflict = handle.apply_batch(request(id, 1, "stale", page("Stale")));
    assert_eq!(conflict.unwrap_err().code, "REVISION_CONFLICT");
    assert_eq!(resident(&empty, id).unwrap_err(), "STORAGE_BUSY");

    // The same outcomes through the warm writer keep its resident entry.
    assert!(p.apply_batch(seed).unwrap().replayed);
    assert!(p.import_asset(asset).unwrap().result.replayed);
    p.dry_run_batch(rename).unwrap();
    assert!(Arc::ptr_eq(&resident(&writer, id).unwrap(), &warm));
}
