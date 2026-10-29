mod bundle_support;
mod common;
use bundle_support::*;
use common::*;
use opencanvas_storage::*;

#[test]
fn instances_legacy_bundles_migrate_in_private_staging_and_keep_data_and_history() {
    let (_temp, store, project) = setup();
    project
        .apply_batch(request(&project, 0, "original", initial()))
        .unwrap();
    let before = project.snapshot().unwrap();
    let original = unpack(&project.export_bundle().unwrap().bytes);
    for version in [1, 2, 3] {
        let mut files = original.clone();
        let mut sql = format!(
            "PRAGMA user_version={version}; UPDATE project_meta SET schema_version={version};"
        );
        if version < 3 {
            sql.push_str("DROP TABLE notes;");
        }
        if version < 2 {
            sql.push_str("DROP TABLE retention_metadata;");
        }
        edit_database(&mut files, &sql);
        let mut manifest: BundleManifest = serde_json::from_slice(&files["manifest.json"]).unwrap();
        manifest.database_schema_version = version;
        files.insert(
            "manifest.json".into(),
            serde_json::to_vec(&manifest).unwrap(),
        );
        let bytes = pack(files);
        let imported = store
            .import_bundle(&format!("legacy-{version}"), &bytes)
            .unwrap();
        assert_eq!(imported.snapshot().unwrap().nodes, before.nodes);
        assert_eq!(imported.snapshot().unwrap().schema_version, SCHEMA_VERSION);
        let restored_root = tempfile::tempdir().unwrap();
        let restored_store = Store::open(restored_root.path()).unwrap();
        let restored = restored_store.restore_bundle("restored", &bytes).unwrap();
        assert_eq!(restored.snapshot().unwrap().nodes, before.nodes);
        assert_eq!(restored.snapshot().unwrap().revision, 1);
        assert!(
            restored
                .apply_batch(request(&restored, 0, "original", initial()))
                .unwrap()
                .replayed
        );
        assert_eq!(project.snapshot().unwrap().schema_version, SCHEMA_VERSION);
    }
}

#[test]
fn instances_legacy_bundle_rejects_schema_objects_and_mismatched_versions_before_upgrade() {
    let (temp, store, project) = setup();
    project
        .apply_batch(request(&project, 0, "original", initial()))
        .unwrap();
    let original = unpack(&project.export_bundle().unwrap().bytes);
    for sql in [
        "CREATE TABLE unexpected(secret TEXT);",
        "CREATE VIEW unexpected AS SELECT * FROM nodes;",
        "CREATE TRIGGER unexpected AFTER INSERT ON nodes BEGIN DELETE FROM nodes; END;",
        "UPDATE project_meta SET schema_version=2;",
    ] {
        let mut files = original.clone();
        edit_database(
            &mut files,
            &format!("PRAGMA user_version=3; UPDATE project_meta SET schema_version=3; {sql}"),
        );
        let mut manifest: BundleManifest = serde_json::from_slice(&files["manifest.json"]).unwrap();
        manifest.database_schema_version = 3;
        files.insert(
            "manifest.json".into(),
            serde_json::to_vec(&manifest).unwrap(),
        );
        reject(&store, temp.path(), &pack(files));
        assert_eq!(project.snapshot().unwrap().schema_version, SCHEMA_VERSION);
    }
}
