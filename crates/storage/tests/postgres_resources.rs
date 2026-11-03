mod postgres_support;
use opencanvas_storage::*;
use postgres_support::*;
use sha2::{Digest, Sha256};

const FONT: &[u8] = include_bytes!("../../../fixtures/fonts/Lato-Regular.ttf");

fn import(id: &str, revision: u64, key: &str) -> ImportAssetRequest {
    ImportAssetRequest {
        project_id: id.into(),
        expected_revision: revision,
        idempotency_key: key.into(),
        media_type: "font/ttf".into(),
        bytes: FONT.into(),
        font: Some(FontRegistration {
            id: "lato".into(),
            family: "Lato".into(),
            weight: 400,
            style: "normal".into(),
            license: Some("OFL-1.1".into()),
        }),
    }
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn exact_font_asset_bytes_bundled_scope_reconnect_and_original_import_replay() {
    let (backend, storage) = setup();
    let p = project(&storage, "resources");
    let other = project(&storage, "other-resources");
    let bundled = p.read_font("font_bundled").unwrap();
    assert_eq!(bundled.1, "font/ttf");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bundled.2)),
        bundled.0.content_hash.unwrap()
    );
    let req = import(p.id(), 0, "font-import");
    let first = p.import_asset(req.clone()).unwrap();
    p.apply_batch(request(p.id(), 1, "later", page("Later")))
        .unwrap();
    let (asset, bytes) = backend.read_asset(p.id(), &first.asset.id).unwrap();
    assert_eq!(asset, first.asset);
    assert_eq!(bytes, FONT);
    assert_eq!(asset.content_hash, format!("{:x}", Sha256::digest(FONT)));
    assert_eq!(p.read_font("lato").unwrap().2, FONT);
    assert_eq!(
        other.read_asset(&asset.id).unwrap_err().code,
        "ASSET_NOT_FOUND"
    );
    assert_eq!(other.read_font("lato").unwrap_err().code, "FONT_NOT_FOUND");
    let (_, reconnected) = setup();
    let reconnected = reconnected.open_project(p.id()).unwrap();
    // A replay must contain the original response, independent of current metadata.
    let mut changed_manifest = first.asset.clone();
    changed_manifest.media_type = "font/otf".into();
    sql().execute("UPDATE opencanvas_runtime_v1.entities SET data=$1 WHERE project_id=$2 AND kind='asset' AND id=$3", &[&serde_json::to_string(&changed_manifest).unwrap(), &p.id(), &first.asset.id]).unwrap();
    let replay = reconnected.import_asset(req.clone()).unwrap();
    let mut expected = first;
    expected.result.replayed = true;
    assert_eq!(
        serde_json::to_value(replay).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    assert_eq!(reconnected.snapshot().unwrap().revision, 2);
    assert_eq!(reconnected.read_font("lato").unwrap().2, FONT);
    let mut collision = req;
    collision.font.as_mut().unwrap().family = "Different".into();
    assert_eq!(
        reconnected.import_asset(collision).unwrap_err().code,
        "IDEMPOTENCY_KEY_REUSED"
    );
    assert_eq!(reconnected.commits_after(0, 10).unwrap().len(), 2);
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn invalid_resource_and_registration_are_atomic_corruption_is_detected() {
    let (_, storage) = setup();
    let p = project(&storage, "resource-validation");
    let before = p.snapshot().unwrap();
    let cases = [
        ("empty", Vec::new(), "font/ttf"),
        ("signature", b"bad font".to_vec(), "font/ttf"),
        ("mime", FONT.to_vec(), "image/png"),
        ("oversize", vec![0; MAX_ASSET_BYTES + 1], "font/ttf"),
    ];
    for (key, bytes, mime) in cases {
        let mut req = import(p.id(), 0, key);
        req.bytes = bytes;
        req.media_type = mime.into();
        assert!(p.import_asset(req).is_err());
        assert_eq!(p.snapshot().unwrap(), before);
        assert!(p.commits_after(0, 10).unwrap().is_empty());
    }
    let mut bad = import(p.id(), 0, "reusable");
    bad.font.as_mut().unwrap().weight = 0;
    assert!(p.import_asset(bad).is_err());
    assert_eq!(p.snapshot().unwrap(), before);
    let first = p.import_asset(import(p.id(), 0, "reusable")).unwrap();
    let mut sql = sql();
    sql.execute("UPDATE opencanvas_runtime_v1.binaries SET bytes=$1 WHERE project_id=$2 AND content_hash=$3", &[&b"corrupt".to_vec(), &p.id(), &first.asset.content_hash]).unwrap();
    assert_eq!(
        p.read_asset(&first.asset.id).unwrap_err().code,
        "ASSET_INVALID"
    );
    assert_eq!(p.read_font("lato").unwrap_err().code, "ASSET_INVALID");
    let error = p.read_font("lato").unwrap_err().to_string();
    assert!(!error.contains(&url()));
    assert!(!error.contains("opencanvas_runtime_v1"));
    let file = BatchFileRequest {
        project_id: p.id().into(),
        expected_revision: 1,
        idempotency_key: "staged".into(),
        path: "inbox/operations.json".into(),
    };
    let unsupported = p.apply_batch_file(file).unwrap_err();
    assert!(unsupported.code.contains("UNSUPPORTED"), "{unsupported:?}");
    assert_eq!(p.snapshot().unwrap().revision, 1);
}
