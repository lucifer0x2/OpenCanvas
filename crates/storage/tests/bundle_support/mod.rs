#![allow(dead_code)]
use opencanvas_storage::*;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read};

pub fn unpack(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    tar::Archive::new(bytes)
        .entries()
        .unwrap()
        .map(|entry| {
            let mut entry = entry.unwrap();
            let path = entry.path().unwrap().to_str().unwrap().to_string();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            (path, bytes)
        })
        .collect()
}

pub fn pack(mut files: BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut manifest: BundleManifest =
        serde_json::from_slice(&files.remove("manifest.json").unwrap()).unwrap();
    manifest.files = files
        .iter()
        .map(|(path, bytes)| {
            (
                path.clone(),
                BundleFile {
                    byte_size: bytes.len() as u64,
                    sha256: format!("{:x}", Sha256::digest(bytes)),
                },
            )
        })
        .collect();
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    let mut builder = tar::Builder::new(Vec::new());
    for (path, bytes) in files {
        append(&mut builder, &path, &bytes);
    }
    builder.into_inner().unwrap()
}

pub fn append(builder: &mut tar::Builder<Vec<u8>>, path: &str, bytes: &[u8]) {
    let mut header = tar::Header::new_ustar();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o600);
    header.set_entry_type(tar::EntryType::Regular);
    header.set_cksum();
    builder.append_data(&mut header, path, bytes).unwrap();
}

pub fn raw(path: &str, kind: tar::EntryType, size: u64) -> Vec<u8> {
    let mut header = tar::Header::new_ustar();
    header.as_mut_bytes()[..path.len()].copy_from_slice(path.as_bytes());
    header.set_size(size);
    header.set_mode(0o600);
    header.set_entry_type(kind);
    if kind.is_symlink() || kind.is_hard_link() {
        header.set_link_name("outside").unwrap();
    }
    header.set_cksum();
    let mut bytes = header.as_bytes().to_vec();
    bytes.extend_from_slice(&[0; 1024]);
    bytes
}

pub fn reject(store: &Store, root: &std::path::Path, bytes: &[u8]) {
    assert_eq!(
        store.import_bundle("rejected", bytes).err().unwrap().code,
        "BUNDLE_INVALID"
    );
    assert!(!root.join("rejected").exists());
    assert!(!std::fs::read_dir(root).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".bundle-")));
}

pub fn edit_database(files: &mut BTreeMap<String, Vec<u8>>, sql: &str) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("db.sql");
    std::fs::write(&path, &files["db.sql"]).unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch(sql).unwrap();
    drop(conn);
    files.insert("db.sql".into(), std::fs::read(path).unwrap());
}
