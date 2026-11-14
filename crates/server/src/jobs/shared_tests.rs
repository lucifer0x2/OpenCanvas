use super::*;
use opencanvas_storage::{BatchRequest, SharedSnapshot};
use serde_json::json;
use std::sync::Arc;

#[allow(dead_code)]
#[path = "../reads/scoped_tree_fixture.rs"]
mod fixture;

fn config() -> Config {
    Config {
        workspace: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        base_url: "http://127.0.0.1:0".into(),
        node: "unused".into(),
        render_backend: crate::RenderBackend::Chromium,
        job_timeout: std::time::Duration::from_secs(1),
        pipeline_identity: "scoped-test-pipeline".into(),
        export_files: Default::default(),
    }
}

fn request(frame: &str, theme: &str) -> SceneRequest {
    SceneRequest {
        frame_id: frame.into(),
        revision: 1,
        width: 320.,
        height: 200.,
        theme: theme.into(),
        state: "hover".into(),
        scale: 2.,
    }
}

fn parity(shared: &SharedSnapshot, request: SceneRequest) {
    let config = config();
    let old = Prepared::new(&shared.snapshot, request.clone(), &config, None);
    let new = Prepared::shared(shared, request, &config, None);
    match (old, new) {
        (Ok(old), Ok(new)) => {
            assert_eq!(
                serde_json::to_vec(&new.input).unwrap(),
                serde_json::to_vec(&old.input).unwrap()
            );
            assert_eq!(new.key, old.key);
            assert_eq!(new.overflow, old.overflow);
            assert_eq!((new.project, new.revision), (old.project, old.revision));
        }
        (Err(old), Err(new)) => assert_eq!(new.value(), old.value()),
        _ => panic!("shared and full preparation disagree"),
    }
}

#[test]
fn scoped_prepared_input_keys_resources_and_errors_match_full_reference() {
    let (_root, store, id) = fixture::fixture();
    let shared = store.shared_snapshot(&id).unwrap();
    for frame in [
        "frame",
        "master",
        "placed",
        "placed/nested",
        "missing",
        "page",
        "placed/missing",
        "placed/nested/copy",
    ] {
        for theme in ["default", "night", "day", "missing"] {
            parity(&shared, request(frame, theme));
        }
    }
    for (revision, width, state) in [
        (0, f64::NAN, "wrong"),
        (1, f64::NAN, "wrong"),
        (1, 100., "wrong"),
    ] {
        let mut req = request("missing", "missing");
        req.revision = revision;
        req.width = width;
        req.state = state.into();
        parity(&shared, req);
    }
    let builds = opencanvas_storage::read_scope::resource_builds;
    let before = builds();
    let input = Prepared::shared(&shared, request("frame", "default"), &config(), None)
        .unwrap()
        .input;
    // The theme check and node resolution share one resource build.
    assert_eq!(builds() - before, 1);
    assert!(input["fonts"]
        .as_array()
        .unwrap()
        .iter()
        .all(|font| font["url"]
            .as_str()
            .unwrap()
            .contains(&format!("/projects/{id}/fonts/"))));
}

#[test]
fn indexed_overflow_intent_matches_full_for_ancestors_master_sources_and_unrelated_notes() {
    let (_root, store, id) = fixture::fixture();
    let project = store.open_project(&id).unwrap();
    for anchor in ["frame", "placed", "nested", "nested-master", "copy"] {
        let before = project.snapshot().unwrap();
        project.apply_batch(serde_json::from_value::<BatchRequest>(json!({"projectId":id,"expectedRevision":before.revision,"idempotencyKey":anchor,
            "operations":[{"op":"create_note","id":format!("overflow-{anchor}"),"nodeId":anchor,"text":"Intentional overflow: design","author":"tester"}]})).unwrap()).unwrap();
        let shared = store.shared_snapshot(&id).unwrap();
        for frame in ["frame", "placed", "placed/nested"] {
            let mut req = request(frame, "day");
            req.revision = shared.snapshot.revision;
            parity(&shared, req);
        }
    }
    let work = |shared: &SharedSnapshot| {
        let scoped = shared.trusted_scoped().unwrap().unwrap();
        let options = SceneOptions {
            frame_id: "frame".into(),
            width: 320.,
            height: 200.,
            theme: "default".into(),
            state: "default".into(),
        };
        let (_, nodes) = scoped.prepare_input_with_scale(&options, 1.).unwrap();
        (
            nodes.stats,
            scoped
                .notes_for_nodes(nodes.dependencies.iter().map(|n| n.id.as_str()))
                .len(),
        )
    };
    let before = store.shared_snapshot(&id).unwrap();
    let mut ops = Vec::new();
    for i in 0..20 {
        ops.push(fixture::node(
            &format!("extra-page-{i}"),
            "page",
            None,
            json!({}),
        ));
        ops.push(fixture::node(
            &format!("extra-frame-{i}"),
            "frame",
            Some(&format!("extra-page-{i}")),
            json!({"width":10,"height":10}),
        ));
        ops.push(json!({"op":"create_note","id":format!("unrelated-note-{i}"),"nodeId":format!("extra-frame-{i}"),"text":"Intentional overflow: unrelated","author":"tester"}));
    }
    project.apply_batch(serde_json::from_value::<BatchRequest>(json!({"projectId":id,"expectedRevision":before.snapshot.revision,"idempotencyKey":"unrelated","operations":ops})).unwrap()).unwrap();
    let after = store.shared_snapshot(&id).unwrap();
    let before_work = work(&before);
    let after_work = work(&after);
    assert_eq!(before_work, after_work);
    eprintln!("prepared closure/resolution/indexed notes: {before_work:?} => {after_work:?}");
    let mut req = request("frame", "default");
    req.revision = after.snapshot.revision;
    parity(&after, req);
}

#[test]
fn untrusted_or_replaced_shared_snapshot_keeps_global_validation() {
    let (_root, store, id) = fixture::fixture();
    let mut shared = store.shared_snapshot(&id).unwrap();
    let mut bad = shared.snapshot.as_ref().clone();
    bad.nodes
        .iter_mut()
        .find(|node| node.id == "button")
        .unwrap()
        .props["width"] = json!(-1);
    shared.snapshot = Arc::new(bad);
    let req = request("frame", "default");
    parity(&shared, req.clone());
    assert!(Prepared::shared(&shared, req.clone(), &config(), None).is_err());
    let untrusted = SharedSnapshot::untrusted(shared.snapshot.clone(), shared.indexes.clone());
    parity(&untrusted, req);
}

#[test]
fn trusted_preparation_over_corrupt_stored_rows_returns_errors() {
    for (id, props, field) in [
        (
            "placed",
            r#"{"componentId":"missing"}"#,
            "props.componentId",
        ),
        ("outgoing", "5", "props"),
    ] {
        let (_root, store, project) = fixture::corrupt(id, props);
        let shared = store.shared_snapshot(&project).unwrap();
        assert!(shared.trusted_scoped().unwrap().is_some());
        let error = Prepared::shared(&shared, request("frame", "default"), &config(), None)
            .err()
            .unwrap();
        assert_eq!(error.value()["error"]["field"], field);
    }
}
