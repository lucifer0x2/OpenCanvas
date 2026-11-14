//! Asset files in a project's filesystem inbox, read through the engine-neutral handle.
//! SQLite uses its data root; PostgreSQL uses the root given to `with_inbox_root`.
mod common;
mod postgres_support;
use common::png;
use opencanvas_storage::*;
use std::{fs, path::Path};

fn import(p: &ProjectHandle, revision: u64, key: &str, bytes: Vec<u8>) -> ImportAssetRequest {
    ImportAssetRequest {
        project_id: p.id().into(),
        expected_revision: revision,
        idempotency_key: key.into(),
        media_type: "image/png".into(),
        bytes,
        font: None,
    }
}

/// Same contract on both engines: read, import, replay by bytes, conflict on changed bytes.
fn check_inbox_import(storage: &Storage, p: &ProjectHandle, inbox: &Path) {
    fs::write(inbox.join("logo.png"), png()).unwrap();
    let bytes = p.read_inbox_asset("logo.png").unwrap();
    assert_eq!(bytes, png());
    let imported = p.import_asset(import(p, 0, "logo", bytes)).unwrap();
    assert_eq!(imported.result.committed_revision, 1);
    assert!(
        p.import_asset(import(p, 0, "logo", png()))
            .unwrap()
            .result
            .replayed
    );
    let mut changed = png();
    changed.push(0);
    let error = p.import_asset(import(p, 0, "logo", changed)).unwrap_err();
    assert_eq!(error.code, "IDEMPOTENCY_KEY_REUSED");
    for (path, code) in [
        ("../logo.png", "PATH_REJECTED"),
        ("missing.png", "FILE_NOT_FOUND"),
    ] {
        assert_eq!(p.read_inbox_asset(path).unwrap_err().code, code, "{path}");
    }
    // Batches keep their own wording, so the shared walk did not change their errors.
    let error = p.read_inbox_asset("").unwrap_err();
    assert!(error.message.starts_with("Asset path"), "{}", error.message);
    assert_eq!(storage.snapshot(p.id()).unwrap().revision, 1);
}

#[test]
fn sqlite_reads_assets_from_its_data_root_inbox() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::from(Store::open(temp.path()).unwrap());
    let p = storage.create_project("fixture", "Fixture").unwrap();
    check_inbox_import(&storage, &p, &temp.path().join("fixture/inbox"));
    let error = p
        .read_batch_file_operations(&BatchFileRequest {
            project_id: p.id().into(),
            expected_revision: 1,
            idempotency_key: "batch".into(),
            path: "".into(),
        })
        .unwrap_err();
    assert!(error.message.starts_with("Batch path"), "{}", error.message);
}

#[test]
fn a_backend_without_an_inbox_root_rejects_path_imports() {
    let temp = tempfile::tempdir().unwrap();
    let backend = std::sync::Arc::new(SqliteBackend::new(Store::open(temp.path()).unwrap()));
    let p = Storage::from_shared(backend)
        .create_project("fixture", "Fixture")
        .unwrap();
    let error = p.read_inbox_asset("logo.png").unwrap_err();
    assert_eq!(error.code, "STAGING_UNSUPPORTED");
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn postgres_reads_assets_from_the_configured_inbox_root() {
    let temp = tempfile::tempdir().unwrap();
    let (_backend, storage) = postgres_support::setup();
    let storage = storage.with_inbox_root(temp.path());
    let p = postgres_support::project(&storage, "inbox");
    let slug = storage.project_meta(p.id()).unwrap().slug;
    // Nothing creates the inbox for PostgreSQL; a missing one reads as a missing file.
    assert_eq!(
        p.read_inbox_asset("logo.png").unwrap_err().code,
        "FILE_NOT_FOUND"
    );
    let inbox = temp.path().join(slug).join("inbox");
    fs::create_dir_all(&inbox).unwrap();
    check_inbox_import(&storage, &p, &inbox);
    let error = p
        .read_batch_file_operations(&BatchFileRequest {
            project_id: p.id().into(),
            expected_revision: 1,
            idempotency_key: "batch".into(),
            path: "logo.png".into(),
        })
        .unwrap_err();
    assert_eq!(error.code, "STAGING_UNSUPPORTED");
}
