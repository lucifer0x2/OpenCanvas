use super::*;
use opencanvas_storage::{FontRegistration, ImportAssetRequest, Store};
use serde_json::json;

#[tokio::test]
async fn stale_scene_font_identity_cannot_export_replacement_bytes() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path()).unwrap();
    let project = store.create_project("fixture", "Fixture").unwrap();
    let register = |revision, bytes: &[u8], weight, style: &str| {
        project
            .import_asset(ImportAssetRequest {
                project_id: project.id().into(),
                expected_revision: revision,
                idempotency_key: format!("face-{revision}"),
                media_type: "font/ttf".into(),
                bytes: bytes.into(),
                font: Some(FontRegistration {
                    id: "face".into(),
                    family: "Shared".into(),
                    weight,
                    style: style.into(),
                    license: None,
                }),
            })
            .unwrap()
    };
    let first = register(
        0,
        include_bytes!("../../../../renderer/fonts/NotoSans.ttf"),
        400,
        "normal",
    );
    let mut scene = json!({"nodes":[{"kind":"text","fontId":"face"}],"fonts":[{"id":"face","hash":first.asset.content_hash,"weight":400,"style":"normal"}]});
    let source = AssetSource {
        store,
        project_id: project.id().into(),
    };
    let (_, css) = embed(&source, &scene).await.unwrap();
    assert!(css.contains("data:font/ttf;base64,"));
    let next = register(
        1,
        include_bytes!("../../../../fixtures/fonts/Lato-Regular.ttf"),
        400,
        "normal",
    );
    assert_eq!(
        embed(&source, &scene).await.unwrap_err().0.code,
        "RESOURCE_NOT_READY"
    );
    scene["fonts"][0]["hash"] = json!(next.asset.content_hash);
    register(
        2,
        include_bytes!("../../../../fixtures/fonts/Lato-Regular.ttf"),
        700,
        "italic",
    );
    assert_eq!(
        embed(&source, &scene).await.unwrap_err().0.code,
        "RESOURCE_NOT_READY"
    );
}
