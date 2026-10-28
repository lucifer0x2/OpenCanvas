mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::json;

#[test]
fn durable_import_idempotency_manifest_and_readback() {
    let (temp, store, p) = setup();
    let req = asset_request(&p, 0, "asset");
    let imported = p.import_asset(req.clone()).unwrap();
    assert_eq!(imported.result.committed_revision, 1);
    assert_eq!(imported.asset.width, Some(1));
    assert!(p.import_asset(req.clone()).unwrap().result.replayed);
    assert_eq!(p.read_asset(&imported.asset.id).unwrap().1, png());
    let mut ops = initial();
    ops.push(create("image",NodeKind::Image,Some("frame"),json!({"assetId":imported.asset.id,"width":100,"height":100,"fit":"cover","crop":{"x":0.1,"y":0.1,"width":0.8,"height":0.8}})));
    p.apply_batch(request(&p, 1, "image", ops)).unwrap();
    let id = p.id().to_string();
    let key = imported.asset.relative_key.clone();
    drop(p);
    drop(store);
    let store = Store::open(temp.path()).unwrap();
    let p = store.open_project(&id).unwrap();
    assert_eq!(p.snapshot().unwrap().revision, 2);
    assert_eq!(p.read_asset(&imported.asset.id).unwrap().1, png());
    assert!(p.orphan_asset_keys().unwrap().is_empty());
    std::fs::write(temp.path().join("fixture/assets").join(&key), b"broken").unwrap();
    assert_eq!(
        p.read_asset(&imported.asset.id).unwrap_err().code,
        "ASSET_INVALID"
    );
    std::fs::remove_file(temp.path().join("fixture/assets").join(key)).unwrap();
    assert_eq!(p.snapshot().unwrap().assets[0].status, "missing");
    assert_eq!(
        p.read_asset(&imported.asset.id).unwrap_err().code,
        "ASSET_MISSING"
    );
}

#[test]
fn bounded_asset_input_and_font_metadata() {
    let (_temp, _store, p) = setup();
    let mut req = asset_request(&p, 0, "invalid");
    req.media_type = "image/svg+xml".into();
    assert_eq!(p.import_asset(req).unwrap_err().code, "VALIDATION_FAILED");
    let mut req = asset_request(&p, 0, "large");
    req.bytes = vec![0; MAX_ASSET_BYTES + 1];
    assert_eq!(p.import_asset(req).unwrap_err().code, "VALIDATION_FAILED");
    let req = ImportAssetRequest {
        project_id: p.id().into(),
        expected_revision: 0,
        idempotency_key: "font".into(),
        media_type: "font/ttf".into(),
        bytes: vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        font: Some(FontRegistration {
            id: "font_custom".into(),
            family: "Custom".into(),
            weight: 400,
            style: "normal".into(),
            license: Some("test fixture header, not a usable font".into()),
        }),
    };
    p.import_asset(req).unwrap();
    let s = p.snapshot().unwrap();
    assert_eq!(s.fonts.len(), 2);
    assert_eq!(
        s.fonts
            .iter()
            .find(|f| f.id == "font_custom")
            .unwrap()
            .status,
        "ready"
    );
    // Binary signatures are checked at import; full decode/font readiness belongs
    // to the renderer. A malformed but signed font cannot yield strict render success.
}
