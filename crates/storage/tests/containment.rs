mod common;
use common::*;
use opencanvas_storage::*;

#[test]
fn slug_guards_and_exclusive_root_lock() {
    let (temp, store, p) = setup();
    assert_eq!(
        Store::open(temp.path()).err().unwrap().code,
        "DATA_ROOT_LOCKED"
    );
    for slug in [
        "../escape",
        "/absolute",
        "a/b",
        "a\\b",
        ".",
        "..",
        "A",
        "-bad",
        "bad-",
        "",
    ] {
        assert_eq!(
            store.create_project(slug, "bad").err().unwrap().code,
            "VALIDATION_FAILED"
        );
    }
    let id = p.id().to_string();
    assert_eq!(store.open_project(&id).unwrap().id(), p.id());
    drop(store);
    assert_eq!(
        Store::open(temp.path()).err().unwrap().code,
        "DATA_ROOT_LOCKED"
    );
    drop(p);
    assert!(Store::open(temp.path()).is_ok());
}

#[cfg(unix)]
#[test]
fn project_assets_database_and_asset_symlinks_are_rejected() {
    use std::os::unix::fs::symlink;
    let (temp, store, p) = setup();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), temp.path().join("escape")).unwrap();
    assert_eq!(
        store.open_project("escape").err().unwrap().code,
        "PATH_REJECTED"
    );
    let imported = p.import_asset(asset_request(&p, 0, "asset")).unwrap();
    let asset = temp
        .path()
        .join("fixture/assets")
        .join(imported.asset.relative_key);
    std::fs::remove_file(&asset).unwrap();
    std::fs::write(outside.path().join("secret"), b"never returned").unwrap();
    symlink(outside.path().join("secret"), &asset).unwrap();
    assert_eq!(
        p.read_asset(&imported.asset.id).unwrap_err().code,
        "ASSET_MISSING"
    );
    std::fs::remove_file(&asset).unwrap();
    std::fs::remove_dir(temp.path().join("fixture/assets")).unwrap();
    symlink(outside.path(), temp.path().join("fixture/assets")).unwrap();
    assert_eq!(p.snapshot().unwrap_err().code, "PATH_REJECTED");
    std::fs::remove_file(temp.path().join("fixture/assets")).unwrap();
    std::fs::create_dir(temp.path().join("fixture/assets")).unwrap();
    drop(p);
    let db = temp.path().join("fixture/db.sql");
    std::fs::remove_file(&db).unwrap();
    symlink(outside.path().join("secret"), db).unwrap();
    assert_eq!(
        store.open_project("fixture").err().unwrap().code,
        "PATH_REJECTED"
    );
}

#[test]
fn busy_errors_are_distinct_from_revision_conflicts() {
    let (temp, _store, p) = setup();
    let conn = rusqlite::Connection::open(temp.path().join("fixture/db.sql")).unwrap();
    conn.execute_batch("BEGIN IMMEDIATE").unwrap();
    let err = p
        .apply_batch(request(&p, 0, "busy", initial()))
        .unwrap_err();
    assert_eq!(err.code, "STORAGE_BUSY");
    assert!(err.retryable);
    conn.execute_batch("ROLLBACK").unwrap();
    assert_eq!(p.snapshot().unwrap().revision, 0);
}
