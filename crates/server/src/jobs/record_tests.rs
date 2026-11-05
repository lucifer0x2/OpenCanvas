use super::*;

fn ready(export: bool) -> Entry {
    let mut value = json!({"jobId":"job","status":"ready","imageUrl":"image",
        "image":{"width":1},"files":[{"url":"file"}],
        "scene":{"nodes":[{"id":"frame"}]},"provenance":{"engine":"native"}});
    if export {
        value["kind"] = json!("export");
    }
    Entry {
        key: "key".into(),
        value,
        requested: true,
        png: Some(vec![1]),
        files: vec![crate::jobs::export::StoredFile {
            meta: json!({}),
            bytes: vec![2],
        }],
    }
}

#[test]
fn expired_result_cannot_retain_any_download_but_keeps_computed_frame() {
    for export in [false, true] {
        let mut entry = ready(export);
        reject_expired(&mut entry, Instant::now() - Duration::from_secs(1));
        assert_eq!(entry.value["error"]["code"], "JOB_TIMEOUT");
        assert_eq!(
            entry.value["status"],
            if export { "failed" } else { "ready" }
        );
        assert!(entry.png.is_none() && entry.files.is_empty());
        for field in ["imageUrl", "image", "files"] {
            assert!(entry.value.get(field).is_none());
        }
        assert_eq!(entry.value["scene"]["nodes"][0]["id"], "frame");
        assert_eq!(entry.value["provenance"]["engine"], "native");
    }
}

#[test]
fn unexpired_result_is_unchanged() {
    let mut entry = ready(false);
    let before = entry.value.clone();
    reject_expired(&mut entry, Instant::now() + Duration::from_secs(10));
    assert_eq!(entry.value, before);
    assert!(entry.png.is_some() && !entry.files.is_empty());
}
