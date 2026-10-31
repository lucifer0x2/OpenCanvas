mod common;
use common::*;
use opencanvas_storage::*;

const NOTO: &[u8] = include_bytes!("../../../renderer/fonts/NotoSans.ttf");
const LATO: &[u8] = include_bytes!("../../../fixtures/fonts/Lato-Regular.ttf");

fn font(p: &Project, revision: u64, id: &str, bytes: &[u8]) -> ImportAssetRequest {
    ImportAssetRequest {
        project_id: p.id().into(),
        expected_revision: revision,
        idempotency_key: id.into(),
        media_type: "font/ttf".into(),
        bytes: bytes.into(),
        font: Some(FontRegistration {
            id: id.into(),
            family: "Shared Family".into(),
            weight: 400,
            style: "normal".into(),
            license: Some("OFL-1.1".into()),
        }),
    }
}

#[test]
fn original_font_bytes_survive_reopen_bundle_and_independent_project_import() {
    let (temp, store, p) = setup();
    let first = font(&p, 0, "noto", NOTO);
    p.import_asset(first.clone()).unwrap();
    assert!(p.import_asset(first).unwrap().result.replayed);
    p.import_asset(font(&p, 1, "lato", LATO)).unwrap();
    assert_ne!(
        p.read_font("noto").unwrap().0.content_hash,
        p.read_font("lato").unwrap().0.content_hash
    );
    let id = p.id().to_owned();
    drop(p);
    drop(store);
    let store = Store::open(temp.path()).unwrap();
    let p = store.open_project(&id).unwrap();
    assert_eq!(p.snapshot().unwrap().revision, 2);
    assert_eq!(p.read_font("noto").unwrap().2, NOTO);
    assert_eq!(p.read_font("lato").unwrap().2, LATO);
    assert_eq!(p.read_font("font_bundled").unwrap().2, NOTO);
    let other_root = tempfile::tempdir().unwrap();
    let other = Store::open(other_root.path()).unwrap();
    let imported = other
        .import_bundle("bundle-copy", &p.export_bundle().unwrap().bytes)
        .unwrap();
    assert_eq!(imported.read_font("lato").unwrap().2, LATO);
    assert_eq!(imported.read_font("font_bundled").unwrap().2, NOTO);
    let second = other.create_project("font-copy", "Font copy").unwrap();
    second
        .import_asset(font(&second, 0, "copied", &p.read_font("lato").unwrap().2))
        .unwrap();
    assert_eq!(second.read_font("copied").unwrap().2, LATO);
}

#[test]
fn invalid_and_stale_imports_are_atomic_and_corruption_is_reported() {
    let (temp, _store, p) = setup();
    let mut bad = font(&p, 0, "bad", b"not a font");
    assert_eq!(
        p.import_asset(bad.clone()).unwrap_err().code,
        "VALIDATION_FAILED"
    );
    bad.bytes = LATO.into();
    bad.expected_revision = 1;
    assert_eq!(p.import_asset(bad).unwrap_err().code, "REVISION_CONFLICT");
    assert_eq!(p.snapshot().unwrap().revision, 0);
    assert_eq!(p.snapshot().unwrap().fonts.len(), 1);
    assert!(p.orphan_asset_keys().unwrap().is_empty());
    let result = p.import_asset(font(&p, 0, "lato", LATO)).unwrap();
    std::fs::write(
        temp.path()
            .join("fixture/assets")
            .join(result.asset.relative_key),
        b"corrupt",
    )
    .unwrap();
    assert_eq!(p.read_font("lato").unwrap_err().code, "ASSET_INVALID");
    assert_eq!(p.read_font("unknown").unwrap_err().code, "FONT_NOT_FOUND");
}
