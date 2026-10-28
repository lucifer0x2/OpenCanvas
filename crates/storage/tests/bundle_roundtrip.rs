mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::json;

#[test]
fn import_and_restore_preserve_authored_resources_and_define_replay_identity() {
    let (_temp, store, p) = setup();
    let image = p.import_asset(asset_request(&p, 0, "image")).unwrap();
    let font_bytes = include_bytes!("../../../renderer/fonts/NotoSans.ttf").to_vec();
    let font = p
        .import_asset(ImportAssetRequest {
            project_id: p.id().into(),
            expected_revision: 1,
            idempotency_key: "font".into(),
            media_type: "font/ttf".into(),
            bytes: font_bytes.clone(),
            font: Some(FontRegistration {
                id: "font_uploaded".into(),
                family: "Uploaded Noto".into(),
                weight: 400,
                style: "normal".into(),
                license: Some("OFL-1.1".into()),
            }),
        })
        .unwrap();
    let mut ops = initial();
    ops.push(create(
        "image",
        NodeKind::Image,
        Some("frame"),
        json!({"assetId":image.asset.id,"width":24,"height":24}),
    ));
    ops.push(Operation::UpsertToken {
        token: Token {
            id: "accent".into(),
            name: "Accent".into(),
            token_type: "color".into(),
            value: json!("#112233"),
        },
    });
    ops.push(Operation::UpsertStyle {
        style: Style {
            id: "box".into(),
            name: "Box".into(),
            kind: "fill".into(),
            value: json!({"tokenId":"accent"}),
        },
    });
    let req = request(&p, 2, "authored", ops);
    p.apply_batch(req.clone()).unwrap();
    let before = p.snapshot().unwrap();
    let bundle = p.export_bundle().unwrap();
    assert_eq!(bundle.manifest.source_revision, 3);
    let imported = store.import_bundle("copy", &bundle.bytes).unwrap();
    let after = imported.snapshot().unwrap();
    assert_ne!(after.project_id, before.project_id);
    assert_eq!(after.revision, 0);
    assert_eq!(after.nodes, before.nodes);
    assert_eq!(after.tokens, before.tokens);
    assert_eq!(after.styles, before.styles);
    assert_eq!(after.assets, before.assets);
    assert_eq!(after.fonts, before.fonts);
    assert!(imported.commits_after(0, 10).unwrap().is_empty());
    assert_eq!(imported.read_asset(&image.asset.id).unwrap().1, png());
    assert_eq!(imported.read_asset(&font.asset.id).unwrap().1, font_bytes);
    let fresh = request(
        &imported,
        0,
        "authored",
        vec![Operation::UpdateNode {
            id: "text".into(),
            props: json!({"text":"copy"}),
        }],
    );
    assert!(!imported.apply_batch(fresh).unwrap().replayed);
    assert_eq!(
        store
            .restore_bundle("collision", &bundle.bytes)
            .err()
            .unwrap()
            .code,
        "PROJECT_EXISTS"
    );
    assert!(!store
        .list_projects()
        .unwrap()
        .iter()
        .any(|m| m.slug == "collision"));
    let destination = tempfile::tempdir().unwrap();
    let restored_store = Store::open(destination.path()).unwrap();
    let restored = restored_store
        .restore_bundle("recovered", &bundle.bytes)
        .unwrap();
    let recovered = restored.snapshot().unwrap();
    assert_eq!(recovered.project_id, before.project_id);
    assert_eq!(recovered.revision, before.revision);
    assert_eq!(recovered.nodes, before.nodes);
    assert_eq!(restored.commits_after(0, 10).unwrap().len(), 3);
    assert!(restored.apply_batch(req).unwrap().replayed);
    drop(restored);
    drop(restored_store);
    let reopened = Store::open(destination.path())
        .unwrap()
        .open_project("recovered")
        .unwrap();
    assert_eq!(reopened.snapshot().unwrap().revision, 3);
    assert_eq!(reopened.read_asset(&image.asset.id).unwrap().1, png());
}

#[test]
fn writer_exports_and_owned_render_snapshots_keep_their_resource_revision() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let owned = p.snapshot().unwrap();
    let writer = p.clone();
    let thread = std::thread::spawn(move || {
        for i in 0..16 {
            let revision = writer.snapshot().unwrap().revision;
            let mut req = asset_request(&writer, revision, &format!("asset_{i}"));
            // Distinct valid PNG headers/resources, retaining all previous bytes.
            req.bytes.extend_from_slice(&[i]);
            writer.import_asset(req).unwrap();
            writer
                .apply_batch(request(
                    &writer,
                    revision + 1,
                    &format!("text_{i}"),
                    vec![Operation::UpdateNode {
                        id: "text".into(),
                        props: json!({"text":format!("version {i}")}),
                    }],
                ))
                .unwrap();
        }
    });
    for _ in 0..8 {
        let bundle = p.export_bundle().unwrap();
        let destination = tempfile::tempdir().unwrap();
        let store = Store::open(destination.path()).unwrap();
        let restored = store.restore_bundle("snapshot", &bundle.bytes).unwrap();
        let snapshot = restored.snapshot().unwrap();
        assert_eq!(snapshot.revision, bundle.manifest.source_revision);
        assert_eq!(
            restored.commits_after(0, 1000).unwrap().len() as u64,
            snapshot.revision
        );
        for asset in &snapshot.assets {
            assert_eq!(
                restored.read_asset(&asset.id).unwrap().0.content_hash,
                asset.content_hash
            );
        }
        assert_eq!(resolved_nodes(&owned).unwrap().len(), owned.nodes.len());
    }
    thread.join().unwrap();
    assert_eq!(owned.revision, 1);
    assert!(owned.assets.is_empty());
    assert_eq!(p.snapshot().unwrap().revision, 33);
}

#[test]
fn deletion_reparenting_preserve_order_gaps_and_bundled_font_is_not_orphaned() {
    let (_temp, store, p) = setup();
    let mut ops = initial();
    ops.push(create(
        "group",
        NodeKind::Group,
        Some("frame"),
        json!({"width":100,"height":100}),
    ));
    p.apply_batch(request(&p, 0, "initial", ops)).unwrap();
    p.apply_batch(request(
        &p,
        1,
        "remove",
        vec![
            Operation::ReparentNode {
                id: "rect".into(),
                parent_id: "group".into(),
                index: None,
            },
            Operation::DeleteSubtree { id: "text".into() },
        ],
    ))
    .unwrap();
    let before = p.snapshot().unwrap();
    assert_eq!(
        before
            .nodes
            .iter()
            .find(|n| n.id == "group")
            .unwrap()
            .sort_key,
        2
    );
    let bundle = p.export_bundle().unwrap();
    let imported = store.import_bundle("gaps", &bundle.bytes).unwrap();
    assert_eq!(imported.snapshot().unwrap().nodes, before.nodes);
    assert!(imported.orphan_asset_keys().unwrap().is_empty());
    let destination = tempfile::tempdir().unwrap();
    let restored = Store::open(destination.path())
        .unwrap()
        .restore_bundle("restore", &bundle.bytes)
        .unwrap();
    assert_eq!(restored.snapshot().unwrap().nodes, before.nodes);
    assert!(restored.orphan_asset_keys().unwrap().is_empty());
}
