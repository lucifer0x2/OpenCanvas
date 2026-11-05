mod postgres_support;

use opencanvas_storage::{
    BatchRequest, PostgresBackend, PrivacyStatus, ProjectHandle, ProjectMeta, Storage, Store,
};
use postgres_support::{page, request};

fn seed(handle: &ProjectHandle) -> BatchRequest {
    let batch = request(handle.id(), 0, "initial", page("Privacy test"));
    handle.apply_batch(batch.clone()).unwrap();
    batch
}

fn publish(storage: &Storage, batch: BatchRequest) -> ProjectMeta {
    let before = storage.project_meta(&batch.project_id).unwrap();
    assert_eq!(before.privacy_status, PrivacyStatus::Private);
    let handle = storage.open_project(&batch.project_id).unwrap();
    let snapshot = handle.snapshot().unwrap();
    let commits = handle.commits_after(0, 10).unwrap();
    assert_eq!(snapshot.revision, 1);
    assert_eq!(commits.len(), 1);

    let (public, changed) = storage
        .update_project_privacy(&before.slug, PrivacyStatus::Public)
        .unwrap();
    assert!(changed);
    assert_eq!(public.privacy_status, PrivacyStatus::Public);
    assert_eq!(public.revision, snapshot.revision);
    assert_eq!(public.schema_version, snapshot.schema_version);
    assert_eq!(
        storage
            .update_project_privacy(handle.id(), PrivacyStatus::Public)
            .unwrap(),
        (public.clone(), false)
    );
    assert_eq!(storage.project_meta(handle.id()).unwrap(), public);
    assert!(storage.list_projects().unwrap().contains(&public));
    assert_eq!(handle.snapshot().unwrap(), snapshot);
    assert_eq!(
        serde_json::to_value(handle.commits_after(0, 10).unwrap()).unwrap(),
        serde_json::to_value(commits).unwrap()
    );
    assert!(handle.apply_batch(batch).unwrap().replayed);

    let meta_json = serde_json::to_value(&public).unwrap();
    assert_eq!(meta_json["privacy_status"], "public");
    assert!(meta_json.get("privacyStatus").is_none());
    assert!(serde_json::to_value(snapshot)
        .unwrap()
        .get("privacy_status")
        .is_none());
    public
}

fn unpublish(storage: &Storage, mut expected: ProjectMeta) -> ProjectMeta {
    assert_eq!(
        storage.project_meta(&expected.project_id).unwrap(),
        expected
    );
    expected.privacy_status = PrivacyStatus::Private;
    assert_eq!(
        storage
            .update_project_privacy(&expected.project_id, PrivacyStatus::Private)
            .unwrap(),
        (expected.clone(), true)
    );
    assert!(storage.list_projects().unwrap().contains(&expected));
    expected
}

#[test]
fn sqlite_privacy_survives_reopen_and_bundle_activation_starts_private() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path()).unwrap();
    let storage = Storage::from(store.clone());
    let project = storage.create_project("shared", "Shared").unwrap();
    let id = project.id().to_owned();
    let batch = seed(&project);
    drop(project);
    let public = publish(&storage, batch);
    let bundle = store.open_project(&id).unwrap().export_bundle().unwrap();
    assert_eq!(bundle.manifest.database_schema_version, 5);

    let imported = store.import_bundle("copy", &bundle.bytes).unwrap();
    let copy = storage.project_meta(imported.id()).unwrap();
    assert_ne!(copy.project_id, id);
    assert_eq!(copy.revision, 0);
    assert_eq!(copy.privacy_status, PrivacyStatus::Private);

    let restored_root = tempfile::tempdir().unwrap();
    let restored_store = Store::open(restored_root.path()).unwrap();
    let restored = restored_store
        .restore_bundle("restored", &bundle.bytes)
        .unwrap();
    assert_eq!(restored.id(), id);
    let restored_storage = Storage::from(restored_store);
    let recovered = restored_storage.project_meta(restored.id()).unwrap();
    assert_eq!(recovered.revision, public.revision);
    assert_eq!(recovered.privacy_status, PrivacyStatus::Private);

    drop(imported);
    drop(storage);
    drop(store);
    let reopened = Storage::from(Store::open(root.path()).unwrap());
    let private = unpublish(&reopened, public);
    drop(reopened);
    let reopened = Storage::from(Store::open(root.path()).unwrap());
    assert_eq!(reopened.project_meta(&id).unwrap(), private);
}

#[test]
#[ignore = "requires a fresh isolated PostgreSQL database via OPENCANVAS_TEST_POSTGRES_URL"]
fn postgres_privacy_is_added_to_legacy_rows_and_survives_reconnect() {
    let dsn = std::env::var("OPENCANVAS_TEST_POSTGRES_URL").unwrap();
    let connect = || Storage::new(PostgresBackend::connect(&dsn).unwrap());
    let storage = connect();
    let legacy = storage.create_project("legacy", "Legacy").unwrap();
    let id = legacy.id().to_owned();
    let batch = seed(&legacy);
    let before = legacy.snapshot().unwrap();
    drop(legacy);
    drop(storage);

    let mut sql = postgres::Client::connect(&dsn, postgres::NoTls).unwrap();
    sql.batch_execute("ALTER TABLE opencanvas_runtime_v1.projects DROP COLUMN privacy_status")
        .unwrap();
    let storage = connect();
    assert_eq!(storage.snapshot(&id).unwrap(), before);
    assert!(storage
        .list_projects()
        .unwrap()
        .iter()
        .any(|meta| { meta.project_id == id && meta.privacy_status == PrivacyStatus::Private }));
    let fresh = storage.create_project("fresh", "Fresh").unwrap();
    assert_eq!(
        storage.project_meta(fresh.id()).unwrap().privacy_status,
        PrivacyStatus::Private
    );
    let err = sql
        .execute("UPDATE opencanvas_runtime_v1.projects SET privacy_status='unknown' WHERE project_id=$1", &[&id])
        .unwrap_err();
    assert_eq!(
        err.code(),
        Some(&postgres::error::SqlState::CHECK_VIOLATION)
    );
    let public = publish(&storage, batch);
    drop(fresh);
    drop(storage);

    let storage = connect();
    let private = unpublish(&storage, public);
    drop(storage);
    assert_eq!(connect().project_meta(&id).unwrap(), private);
}
