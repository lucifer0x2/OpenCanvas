mod bundle_support;
mod common;
use bundle_support::*;
use common::*;
use opencanvas_storage::*;

#[test]
fn rejects_malformed_duplicate_trailing_paths_links_and_extensions() {
    let (temp, store, p) = setup();
    let bundle = p.export_bundle().unwrap();
    for bytes in [
        vec![],
        b"not a tar archive".to_vec(),
        bundle.bytes[..512].to_vec(),
    ] {
        reject(&store, temp.path(), &bytes);
    }
    let mut trailing = bundle.bytes.clone();
    trailing.extend_from_slice(&raw("db.sql", tar::EntryType::Regular, 0));
    reject(&store, temp.path(), &trailing);
    let mut duplicate = tar::Builder::new(Vec::new());
    append(&mut duplicate, "db.sql", b"one");
    append(&mut duplicate, "db.sql", b"two");
    reject(&store, temp.path(), &duplicate.into_inner().unwrap());
    for path in [
        "../escape",
        "/absolute",
        "assets/../db.sql",
        "assets\\escape",
        "assets//escape",
        "./db.sql",
        "licenses/../../escape",
    ] {
        reject(&store, temp.path(), &raw(path, tar::EntryType::Regular, 0));
    }
    for kind in [
        tar::EntryType::Symlink,
        tar::EntryType::Link,
        tar::EntryType::Directory,
        tar::EntryType::XHeader,
        tar::EntryType::GNULongName,
    ] {
        reject(&store, temp.path(), &raw("db.sql", kind, 0));
    }
}

#[test]
fn rejects_hash_file_set_entry_count_and_size_limits() {
    let (temp, store, p) = setup();
    let asset = p.import_asset(asset_request(&p, 0, "image")).unwrap();
    let original = unpack(&p.export_bundle().unwrap().bytes);
    let asset_path = format!("assets/{}", asset.asset.relative_key);
    let mut files = original.clone();
    files.get_mut(&asset_path).unwrap()[0] ^= 1;
    // Preserve original manifest to exercise its digest check.
    let mut builder = tar::Builder::new(Vec::new());
    for (path, bytes) in &files {
        append(&mut builder, path, bytes);
    }
    reject(&store, temp.path(), &builder.into_inner().unwrap());
    for removed in ["db.sql", "licenses/OFL.txt", &asset_path] {
        let mut files = original.clone();
        files.remove(removed);
        reject(&store, temp.path(), &pack(files));
    }
    let mut files = original.clone();
    files.insert(format!("assets/{}", "f".repeat(64)), vec![0]);
    reject(&store, temp.path(), &pack(files));
    for (path, bytes) in [
        ("db.sql", 64 * 1024 * 1024 + 1),
        ("manifest.json", 2 * 1024 * 1024 + 1),
        (
            "assets/0000000000000000000000000000000000000000000000000000000000000000",
            MAX_ASSET_BYTES as u64 + 1,
        ),
    ] {
        reject(
            &store,
            temp.path(),
            &raw(path, tar::EntryType::Regular, bytes),
        );
    }
    let mut builder = tar::Builder::new(Vec::new());
    for i in 0..4098 {
        append(&mut builder, &format!("assets/{i:064x}"), &[]);
    }
    reject(&store, temp.path(), &builder.into_inner().unwrap());
    reject(&store, temp.path(), &vec![0; MAX_BUNDLE_BYTES + 1]);
}

#[cfg(unix)]
#[test]
fn rejects_symlink_destinations_and_live_asset_escape_without_touching_target() {
    use std::os::unix::fs::symlink;
    let (temp, store, p) = setup();
    let asset = p.import_asset(asset_request(&p, 0, "asset")).unwrap();
    let bytes = p.export_bundle().unwrap().bytes;
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("sentinel"), b"untouched").unwrap();
    symlink(outside.path(), temp.path().join("escape")).unwrap();
    assert_eq!(
        store.import_bundle("escape", &bytes).err().unwrap().code,
        "PATH_REJECTED"
    );
    let path = temp
        .path()
        .join("fixture/assets")
        .join(asset.asset.relative_key);
    std::fs::remove_file(&path).unwrap();
    symlink(outside.path().join("sentinel"), path).unwrap();
    assert_eq!(p.export_bundle().err().unwrap().code, "PATH_REJECTED");
    assert_eq!(
        std::fs::read(outside.path().join("sentinel")).unwrap(),
        b"untouched"
    );
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn replaced_data_root_symlink_is_rejected_before_staging_or_create() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("data");
    let store = Store::open(&root).unwrap();
    let p = store.create_project("source", "Source").unwrap();
    let bytes = p.export_bundle().unwrap().bytes;
    let held = temp.path().join("held");
    let outside = tempfile::tempdir().unwrap();
    std::fs::rename(&root, &held).unwrap();
    symlink(outside.path(), &root).unwrap();
    assert_eq!(
        store.import_bundle("copy", &bytes).err().unwrap().code,
        "PATH_REJECTED"
    );
    assert_eq!(
        store.create_project("copy", "Copy").err().unwrap().code,
        "PATH_REJECTED"
    );
    assert_eq!(store.list_projects().unwrap_err().code, "PATH_REJECTED");
    assert_eq!(outside.path().read_dir().unwrap().count(), 0);
    std::fs::remove_file(&root).unwrap();
    std::fs::rename(held, root).unwrap();
}
