mod backend_contract_support;
use backend_contract_support::*;
use opencanvas_storage::*;
use std::fs;

const FONT: &[u8] = include_bytes!("../../../fixtures/fonts/Lato-Regular.ttf");

#[test]
fn asset_font_bytes_and_import_replay_survive_facade_and_reopen() {
    let (root, backend, storage) = setup();
    let p = storage.create_project("resources", "Resources").unwrap();
    let req = ImportAssetRequest {
        project_id: p.id().into(),
        expected_revision: 0,
        idempotency_key: "font-import".into(),
        media_type: "font/ttf".into(),
        bytes: FONT.into(),
        font: Some(FontRegistration {
            id: "lato".into(),
            family: "Lato".into(),
            weight: 400,
            style: "normal".into(),
            license: Some("OFL-1.1".into()),
        }),
    };
    let imported = p.import_asset(req.clone()).unwrap();
    assert!(
        backend
            .import_asset("resources", req)
            .unwrap()
            .result
            .replayed
    );
    let (asset, bytes) = backend.read_asset(p.id(), &imported.asset.id).unwrap();
    assert_eq!(asset, imported.asset);
    assert_eq!(bytes, FONT);
    let (font, mime, bytes) = p.read_font("lato").unwrap();
    assert_eq!(font.asset_id.as_deref(), Some(asset.id.as_str()));
    assert_eq!(mime, "font/ttf");
    assert_eq!(bytes, FONT);
    let other = storage.create_project("unrelated", "Unrelated").unwrap();
    assert_eq!(
        other.read_asset(&asset.id).unwrap_err().code,
        "ASSET_NOT_FOUND"
    );
    assert_eq!(other.read_font("lato").unwrap_err().code, "FONT_NOT_FOUND");
    let id = p.id().to_owned();
    drop((p, other, storage, backend));
    let reopened: Storage = Store::open(root.path()).unwrap().into();
    assert_eq!(
        reopened
            .open_project(&id)
            .unwrap()
            .read_font("lato")
            .unwrap()
            .2,
        FONT
    );
}

#[test]
fn staged_operations_use_the_guarded_project_inbox_and_share_inline_replay() {
    let (root, backend, storage) = setup();
    let p = storage.create_project("staging", "Staging").unwrap();
    let operations = page("From inbox");
    let inbox = root.path().join("staging/inbox");
    fs::write(
        inbox.join("ops.json"),
        serde_json::to_vec(&operations).unwrap(),
    )
    .unwrap();
    let req = file(p.id(), 0, "file-key", "ops.json");
    assert_eq!(
        backend.read_batch_file_operations("staging", &req).unwrap(),
        serde_json::to_value(&operations).unwrap()
    );
    assert!(p.dry_run_batch_file(req.clone()).unwrap().dry_run);
    assert_eq!(p.snapshot().unwrap().revision, 0);
    assert_eq!(p.apply_batch_file(req).unwrap().committed_revision, 1);
    assert!(
        p.apply_batch(request(p.id(), 0, "file-key", operations))
            .unwrap()
            .replayed
    );
    let before = p.snapshot().unwrap();
    fs::write(root.path().join("outside.json"), b"[]").unwrap();
    for path in ["../outside.json", "/tmp/outside.json", "missing.json"] {
        assert!(
            p.apply_batch_file(file(p.id(), 1, "guard", path)).is_err(),
            "{path}"
        );
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.path().join("outside.json"), inbox.join("escape.json"))
            .unwrap();
        assert!(p
            .dry_run_batch_file(file(p.id(), 1, "escape", "escape.json"))
            .is_err());
    }
    assert_eq!(p.snapshot().unwrap(), before);
}

#[test]
fn pinned_slug_replacement_never_returns_the_previous_identity() {
    let (root, _backend, storage) = setup();
    let old = storage.create_project("replace", "Old").unwrap();
    old.apply_batch(request(old.id(), 0, "seed", page("Old data")))
        .unwrap();
    for _ in 0..3 {
        assert_eq!(storage.snapshot("replace").unwrap().revision, 1);
    }
    fs::remove_dir_all(root.path().join("replace")).unwrap();
    let new = storage.create_project("replace", "New").unwrap();
    assert_ne!(old.id(), new.id());
    assert_eq!(
        storage.open_project(old.id()).err().unwrap().code,
        "PROJECT_NOT_FOUND"
    );
    assert!(old.snapshot().is_err());
    assert_eq!(storage.snapshot("replace").unwrap().project_id, new.id());
    assert_eq!(storage.snapshot(new.id()).unwrap().revision, 0);
}

#[test]
fn dropping_adapter_and_last_project_handle_releases_the_root_lock() {
    let root = tempfile::tempdir().unwrap();
    let storage: Storage = Store::open(root.path()).unwrap().into();
    let p = storage.create_project("pinned", "Pinned").unwrap();
    let id = p.id().to_owned();
    assert_eq!(storage.snapshot("pinned").unwrap().project_id, id);
    drop(storage);
    // A live facade still owns its backend and its root lock.
    assert_eq!(
        Store::open(root.path()).err().unwrap().code,
        "DATA_ROOT_LOCKED"
    );
    drop(p);
    // Strong project pins must not form an ownership cycle with the root registry.
    let reopened = Store::open(root.path()).unwrap();
    assert_eq!(reopened.open_project(&id).unwrap().id(), id);
}
