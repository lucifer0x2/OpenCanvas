//! A full export retains binaries even if their backing files disappear after compute.
use super::*;
use crate::jobs::{
    export::{self, ExportSetting, ExportSpec, Rendered},
    export_svg::AssetSource,
    painter::Painter,
};
use opencanvas_storage::{FontRegistration, ImportAssetRequest, Store};
use std::time::Duration;

#[tokio::test]
async fn native_png_scales_and_svg_share_job_bytes_without_reading_store_again() {
    let root = tempfile::tempdir().unwrap();
    let store: Storage = Store::open(root.path()).unwrap().into();
    let project = store.create_project("cache", "Cache").unwrap();
    let font = project
        .import_asset(ImportAssetRequest {
            project_id: project.id().into(),
            expected_revision: 0,
            idempotency_key: "font".into(),
            media_type: "font/ttf".into(),
            bytes: include_bytes!("../../../../../renderer/fonts/NotoSans.ttf").to_vec(),
            font: Some(FontRegistration {
                id: "face".into(),
                family: "Exact".into(),
                weight: 400,
                style: "normal".into(),
                license: None,
            }),
        })
        .unwrap();
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[23, 200, 41, 255])
            .unwrap();
    }
    let image = project
        .import_asset(ImportAssetRequest {
            project_id: project.id().into(),
            expected_revision: 1,
            idempotency_key: "image".into(),
            media_type: "image/png".into(),
            bytes: png,
            font: None,
        })
        .unwrap();
    let input = json!({"schemaVersion":1,"projectId":project.id(),"revision":2,
        "frameId":"frame","viewport":{"width":64,"height":64},
        "nodes":[{"id":"frame","kind":"frame","parentId":null,"props":{"width":64,"height":64}},
            {"id":"text","kind":"text","parentId":"frame","props":{"text":"Cached text","width":64,"height":24,"fontId":"face","fontSize":12}},
            {"id":"image","kind":"image","parentId":"frame","props":{"assetId":image.asset.id,"width":16,"height":16}}],
        "fonts":[{"id":"face","family":"Exact","hash":font.asset.content_hash,"weight":400,"style":"normal","url":"never-fetch-font"}],
        "assets":[{"id":image.asset.id,"hash":image.asset.content_hash,"mediaType":"image/png","url":"never-fetch-image"}]});
    let deadline = Instant::now() + Duration::from_secs(10);
    let native = compute_sync(&store, input.clone(), Control::new(deadline)).unwrap();
    let mut computed = native.value;
    computed["scene"]["provenance"]["sourceSceneKey"] = json!("original-root-key");
    computed["scene"]["diagnostics"]
        .as_array_mut()
        .unwrap()
        .push(json!({"code":"OPAQUE_ADVISORY"}));
    for file in std::fs::read_dir(root.path().join("cache/assets")).unwrap() {
        std::fs::remove_file(file.unwrap().path()).unwrap();
    }
    let source = AssetSource {
        store: store.clone(),
        project_id: project.id().into(),
        cached: Some(native.bytes.clone()),
        deadline: Some(deadline),
    };
    let mut painter = Painter::Native {
        bytes: native.bytes,
        deadline,
        readiness: computed["readiness"].as_array().unwrap().clone(),
    };
    let mut settings: Vec<_> = [1.0, 2.0, 2.4]
        .into_iter()
        .map(|scale| ExportSetting {
            format: "png".into(),
            scale: Some(scale),
            width: None,
            height: None,
            file_name: format!("cached-{scale}.png"),
            svg_id_attribute: false,
        })
        .collect();
    settings.push(ExportSetting {
        format: "svg".into(),
        scale: None,
        width: None,
        height: None,
        file_name: "cached.svg".into(),
        svg_id_attribute: true,
    });
    let rendered = export::render(
        &mut painter,
        &source,
        &computed,
        &ExportSpec {
            node_id: "frame".into(),
            settings,
        },
    )
    .await
    .unwrap();
    let Rendered::Export(files) = rendered else {
        panic!("expected export files");
    };
    assert_eq!(files.len(), 4);
    assert_eq!(files[2].meta["width"], 154);
    assert!(std::str::from_utf8(&files[3].bytes)
        .unwrap()
        .contains("data:image/png;base64,"));
    assert!(std::str::from_utf8(&files[3].bytes)
        .unwrap()
        .contains("data:font/ttf;base64,"));
    assert_eq!(
        computed["scene"]["provenance"]["sourceSceneKey"],
        "original-root-key"
    );
    assert_eq!(
        compute_sync(&store, input, Control::new(deadline))
            .unwrap_err()
            .0
            .code,
        "FONT_MISSING"
    );
}
