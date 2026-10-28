mod bundle_support;
mod common;
use bundle_support::*;
use common::*;

#[test]
fn rejects_unexpected_schema_and_invalid_identity_retention_replay_or_nodes() {
    let (temp, store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let original = unpack(&p.export_bundle().unwrap().bytes);
    for sql in [
        "CREATE TABLE unexpected(secret TEXT)",
        "CREATE VIEW unexpected AS SELECT * FROM nodes",
        "CREATE TRIGGER unexpected AFTER INSERT ON nodes BEGIN DELETE FROM nodes; END",
        "PRAGMA user_version=99",
        "UPDATE project_meta SET project_id='another'",
        "UPDATE project_meta SET current_revision=-1",
        "UPDATE project_meta SET schema_version=1",
        "DELETE FROM retention_metadata",
        "UPDATE retention_metadata SET oldest_commit_revision=2",
        "UPDATE retention_metadata SET idempotency_policy='expires'",
        "DELETE FROM idempotency",
        "DELETE FROM commits",
        "UPDATE idempotency SET response_json=json_set(response_json,'$.projectId','different')",
        "UPDATE idempotency SET committed_revision=9",
        "UPDATE commits SET base_revision=9",
        "UPDATE commits SET summary_json='[\"wrong\"]'",
        "UPDATE idempotency SET request_hash='ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff'",
        "UPDATE commits SET ops_json='[{\"op\":\"unknown\"}]'",
        "UPDATE nodes SET sort_key=-1 WHERE id='text'",
        "UPDATE nodes SET props_json='{}' WHERE id='text'",
    ] {
        let mut files = original.clone();
        edit_database(&mut files, sql);
        reject(&store, temp.path(), &pack(files));
    }
    let mut files = original.clone();
    files.insert("db.sql".into(), b"not SQLite".to_vec());
    reject(&store, temp.path(), &pack(files));
}

#[test]
fn rejects_metadata_that_disagrees_with_resource_columns_or_binary() {
    let (temp, store, p) = setup();
    p.import_asset(asset_request(&p, 0, "image")).unwrap();
    let original = unpack(&p.export_bundle().unwrap().bytes);
    for sql in [
        "UPDATE assets SET data_json=json_set(data_json,'$.id','different')",
        "UPDATE assets SET content_hash='wrong'",
        "UPDATE assets SET data_json=json_set(data_json,'$.contentHash','wrong')",
        "UPDATE assets SET data_json=json_set(data_json,'$.byteSize',1)",
        "UPDATE assets SET data_json=json_set(data_json,'$.width',100)",
        "UPDATE assets SET data_json=json_set(data_json,'$.mediaType','image/svg+xml')",
        "UPDATE assets SET data_json=json_set(data_json,'$.status','missing')",
        "UPDATE fonts SET data_json=json_set(data_json,'$.contentHash','wrong')",
    ] {
        let mut files = original.clone();
        edit_database(&mut files, sql);
        reject(&store, temp.path(), &pack(files));
    }
}
