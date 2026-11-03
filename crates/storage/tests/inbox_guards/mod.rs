use super::*;

#[test]
fn traversal_symlinks_size_json_counts_and_special_files() {
    let (temp, _store, p) = setup();
    let inbox = temp.path().join("fixture/inbox");
    let path = inbox.join("ops.json");
    write(&path, &initial());
    assert!(p
        .dry_run_batch_file(file(
            &p,
            0,
            "absolute",
            path.canonicalize().unwrap().to_str().unwrap()
        ))
        .is_ok());
    fs::create_dir(inbox.join("nested")).unwrap();
    write(&inbox.join("nested/ops.json"), &initial());
    assert!(p
        .dry_run_batch_file(file(&p, 0, "nested", "nested/ops.json"))
        .is_ok());
    for bad in [
        "../inbox/ops.json",
        "sub/../../ops.json",
        "/etc/passwd",
        "",
        "\0",
        "nested",
    ] {
        let error = p.apply_batch_file(file(&p, 0, "bad", bad)).unwrap_err();
        assert_eq!(error.code, "PATH_REJECTED", "{bad:?}");
        assert!(!serde_json::to_string(&error)
            .unwrap()
            .contains(temp.path().to_str().unwrap()));
    }
    for body in [
        "{}",
        "[]",
        "null",
        "export default []",
        "[1]",
        "[{\"op\":\"delete_subtree\",\"id\":\"x\",\"extra\":1}]",
    ] {
        fs::write(&path, body).unwrap();
        assert!(
            p.apply_batch_file(file(&p, 0, "bad", "ops.json")).is_err(),
            "{body}"
        );
    }
    let too_many = vec![json!({"op":"delete_subtree","id":"x"}); 501];
    fs::write(&path, serde_json::to_vec(&too_many).unwrap()).unwrap();
    assert!(p.apply_batch_file(file(&p, 0, "bad", "ops.json")).is_err());
    fs::File::create(&path)
        .unwrap()
        .set_len(MAX_BATCH_BYTES as u64 + 1)
        .unwrap();
    assert_eq!(
        p.apply_batch_file(file(&p, 0, "bad", "ops.json"))
            .unwrap_err()
            .field
            .as_deref(),
        Some("path")
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(temp.path(), inbox.join("escape")).unwrap();
        symlink(&path, inbox.join("link.json")).unwrap();
        for bad in ["escape/fixture/db.sql", "link.json", "."] {
            assert_eq!(
                p.apply_batch_file(file(&p, 0, "bad", bad))
                    .unwrap_err()
                    .code,
                "PATH_REJECTED"
            );
        }
        let fifo = inbox.join("fifo");
        let name = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert_eq!(
            p.apply_batch_file(file(&p, 0, "bad", "fifo"))
                .unwrap_err()
                .code,
            "PATH_REJECTED"
        );
        fs::rename(&inbox, temp.path().join("hidden-inbox")).unwrap();
        symlink(temp.path().join("hidden-inbox"), &inbox).unwrap();
        assert_eq!(
            p.apply_batch_file(file(&p, 0, "bad", "ops.json"))
                .unwrap_err()
                .code,
            "PATH_REJECTED"
        );
    }
    assert_eq!(p.snapshot().unwrap().revision, 0);
}
