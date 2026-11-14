//! `import_asset` with `path` reads the project's inbox under the same guards as
//! `apply_batch_file`, and shares replay and hashing with `base64` through the decoded bytes.
#[allow(dead_code)]
mod support;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use support::Server;

const PNG: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII=";

fn png() -> Vec<u8> {
    STANDARD.decode(PNG).unwrap()
}
/// Only the signature and minimum length are checked for WOFF2; no font is registered.
fn woff2() -> Vec<u8> {
    let mut bytes = b"wOF2".to_vec();
    bytes.resize(64, 0);
    bytes
}
fn inbox(s: &Server) -> PathBuf {
    s.root.path().join("test-project/inbox")
}
fn args(project: &str, revision: u64, key: &str, media: &str, source: Value) -> Value {
    let mut args = json!({"projectId":project,"expectedRevision":revision,"idempotencyKey":key,"mediaType":media});
    args.as_object_mut()
        .unwrap()
        .extend(source.as_object().unwrap().clone());
    args
}
async fn code(s: &Server, args: Value) -> Value {
    let result = s.call("import_asset", args).await;
    assert!(result["error"].is_object(), "{result}");
    assert!(!result.to_string().contains(s.root.path().to_str().unwrap()));
    result["error"].clone()
}

#[tokio::test]
async fn path_and_base64_share_bytes_replay_and_conflict() {
    let s = Server::start().await;
    let p = s.project().await;
    fs::write(inbox(&s).join("logo.png"), png()).unwrap();
    let inline = json!({"base64":PNG});
    let first = s
        .call("import_asset", args(&p, 0, "logo", "image/png", inline))
        .await;
    assert_eq!(first["committedRevision"], 1, "{first}");
    // base64 first, then the same bytes from the inbox under the same key: replay.
    let replay = s
        .call(
            "import_asset",
            args(&p, 0, "logo", "image/png", json!({"path":"logo.png"})),
        )
        .await;
    assert_eq!(replay["replayed"], true, "{replay}");
    let mut expected = first.clone();
    expected["replayed"] = json!(true);
    expected["result"]["replayed"] = json!(true);
    assert_eq!(replay, expected);
    // path first, then the same bytes inline, using an absolute path inside the inbox.
    fs::create_dir(inbox(&s).join("fonts")).unwrap();
    fs::write(inbox(&s).join("fonts/brand.woff2"), woff2()).unwrap();
    let absolute = fs::canonicalize(inbox(&s).join("fonts/brand.woff2")).unwrap();
    let font = s
        .call(
            "import_asset",
            args(&p, 1, "font", "font/woff2", json!({"path":absolute})),
        )
        .await;
    assert_eq!(font["committedRevision"], 2, "{font}");
    assert_eq!(font["asset"]["byteSize"], 64);
    let inline = json!({"base64":STANDARD.encode(woff2())});
    let replay = s
        .call("import_asset", args(&p, 1, "font", "font/woff2", inline))
        .await;
    assert_eq!(replay["replayed"], true, "{replay}");
    assert_eq!(replay["asset"], font["asset"]);
    // Changed bytes under a used key conflict, whatever the source.
    let mut changed = png();
    changed.push(0);
    fs::write(inbox(&s).join("logo.png"), &changed).unwrap();
    let reused = code(
        &s,
        args(&p, 0, "logo", "image/png", json!({"path":"logo.png"})),
    )
    .await;
    assert_eq!(reused["code"], "IDEMPOTENCY_KEY_REUSED", "{reused}");
    let both = json!({"path":"logo.png","base64":PNG});
    let invalid = code(&s, args(&p, 2, "both", "image/png", both)).await;
    assert_eq!(invalid["code"], "INVALID_REQUEST");
    let neither = code(&s, args(&p, 2, "neither", "image/png", json!({}))).await;
    assert_eq!(neither["code"], "INVALID_REQUEST");
    let snapshot = s.get(&format!("/api/v1/projects/{p}/snapshot")).await;
    assert_eq!(snapshot["revision"], 2);
}

#[tokio::test]
async fn path_rejects_escapes_links_special_missing_and_oversize_files() {
    let s = Server::start().await;
    let p = s.project().await;
    let inbox = inbox(&s);
    let outside = s.root.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("logo.png"), png()).unwrap();
    fs::write(inbox.join("logo.png"), png()).unwrap();
    fs::create_dir(inbox.join("nested")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(outside.join("logo.png"), inbox.join("link.png")).unwrap();
        symlink(&outside, inbox.join("linked")).unwrap();
    }
    let outside_abs = fs::canonicalize(outside.join("logo.png")).unwrap();
    let mut rejected = vec![
        json!("../inbox/logo.png"),
        json!("nested/../logo.png"),
        json!("./logo.png"),
        json!("../../outside/logo.png"),
        json!(outside_abs),
        json!("/etc/hosts"),
        json!("nested"),
    ];
    if cfg!(unix) {
        rejected.extend([json!("link.png"), json!("linked/logo.png")]);
    }
    for path in rejected {
        let error = code(&s, args(&p, 0, "bad", "image/png", json!({"path":path}))).await;
        assert_eq!(error["code"], "PATH_REJECTED", "{path}: {error}");
    }
    let missing = code(
        &s,
        args(&p, 0, "bad", "image/png", json!({"path":"missing.png"})),
    )
    .await;
    assert_eq!(missing["code"], "FILE_NOT_FOUND", "{missing}");
    let large = fs::File::create(inbox.join("large.png")).unwrap();
    large
        .set_len(opencanvas_storage::MAX_ASSET_BYTES as u64 + 1)
        .unwrap();
    let error = code(
        &s,
        args(&p, 0, "bad", "image/png", json!({"path":"large.png"})),
    )
    .await;
    assert_eq!(error["code"], "VALIDATION_FAILED", "{error}");
    assert_eq!(error["field"], "path");
    let snapshot = s.get(&format!("/api/v1/projects/{p}/snapshot")).await;
    assert_eq!(snapshot["revision"], 0);
}

#[tokio::test]
async fn bytes_must_match_the_declared_media_type() {
    let s = Server::start().await;
    let p = s.project().await;
    fs::write(inbox(&s).join("logo.png"), png()).unwrap();
    fs::write(inbox(&s).join("brand.woff2"), woff2()).unwrap();
    let jpeg = [0xff, 0xd8, 0xff, 0xe0, 0, 16, b'J', b'F', b'I', b'F', 0, 1];
    for (media, source) in [
        ("font/woff2", json!({"path":"logo.png"})),
        ("image/jpeg", json!({"path":"logo.png"})),
        ("image/png", json!({"path":"brand.woff2"})),
        ("font/ttf", json!({"path":"brand.woff2"})),
        ("image/png", json!({"base64":STANDARD.encode(woff2())})),
        ("image/webp", json!({"base64":STANDARD.encode(jpeg)})),
    ] {
        let error = code(&s, args(&p, 0, "mismatch", media, source.clone())).await;
        assert_eq!(error["code"], "VALIDATION_FAILED", "{media} {source}");
        assert_eq!(error["field"], "mediaType");
    }
    // Matching signatures pass, so the rejections above are about the mismatch alone.
    let png = s
        .call(
            "import_asset",
            args(&p, 0, "png", "image/png", json!({"path":"logo.png"})),
        )
        .await;
    assert_eq!(png["committedRevision"], 1, "{png}");
    let jpeg = json!({"base64":STANDARD.encode(jpeg)});
    let jpeg = s
        .call("import_asset", args(&p, 1, "jpeg", "image/jpeg", jpeg))
        .await;
    assert_eq!(jpeg["committedRevision"], 2, "{jpeg}");
}
