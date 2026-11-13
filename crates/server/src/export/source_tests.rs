use super::*;
use serde_json::json;

#[allow(dead_code)]
#[path = "../reads/scoped_tree_fixture.rs"]
mod fixture;

#[test]
fn scoped_export_targets_match_full_root_frames_settings_names_and_errors() {
    let (_root, store, id) = fixture::fixture();
    let shared = store.shared_snapshot(&id).unwrap();
    for theme in ["default", "night", "day", "missing"] {
        let old = opencanvas_storage::resolved_expanded_nodes(&shared.snapshot, theme);
        let new = Source::new(&shared, theme);
        let (resolved, source) = match (old, new) {
            (Ok(resolved), Ok(source)) => (resolved, source),
            (Err(old), Err(new)) => {
                assert_eq!(serde_json::to_value(old).unwrap(), new.value()["error"]);
                continue;
            }
            _ => panic!("global theme check differs"),
        };
        let expanded = opencanvas_storage::instances::expand(&shared.snapshot.nodes).unwrap();
        let nodes = Nodes::new(&expanded, &resolved);
        for id in [
            "frame",
            "placed",
            "placed/box",
            "placed/nested",
            "placed/nested/copy",
            "master",
            "copy",
            "missing",
            "placed/missing",
            "page",
        ] {
            for settings in [
                None,
                Some(
                    json!([{"format":"svg","suffix":"-vector","svgIdAttribute":true},{"format":"png","scale":2,"suffix":"@2x"}]),
                ),
            ] {
                let old = target::resolve(&nodes, &shared.snapshot, id, theme, settings.as_ref());
                let new = source.resolve(id, theme, settings.as_ref());
                let value = |result: Result<(SceneRequest, ExportSpec)>| {
                    result
                        .map(|(request, spec)| json!([request, spec]))
                        .map_err(|e| e.value())
                };
                assert_eq!(value(new), value(old), "{id}/{theme}");
            }
        }
    }
}

#[test]
fn export_untrusted_snapshot_retains_unrelated_global_validation_failure() {
    let (_root, store, id) = fixture::fixture();
    let mut shared = store.shared_snapshot(&id).unwrap();
    let mut snapshot = shared.snapshot.as_ref().clone();
    snapshot
        .nodes
        .iter_mut()
        .find(|node| node.id == "button")
        .unwrap()
        .props["width"] = json!(-1);
    shared.snapshot = std::sync::Arc::new(snapshot);
    assert!(shared.trusted_scoped().unwrap().is_none());
    let old = opencanvas_storage::resolved_expanded_nodes(&shared.snapshot, "default")
        .err()
        .unwrap();
    let new = Source::new(&shared, "default").err().unwrap();
    assert_eq!(serde_json::to_value(old).unwrap(), new.value()["error"]);
}

#[test]
fn one_scoped_source_checks_the_theme_and_builds_resources_once() {
    let (_root, store, id) = fixture::fixture();
    let shared = store.shared_snapshot(&id).unwrap();
    let builds = opencanvas_storage::read_scope::resource_builds;
    let before = builds();
    let source = Source::new(&shared, "night").unwrap();
    for id in ["frame", "placed", "placed/nested/copy", "master", "page"] {
        let _ = source.resolve(id, "night", None);
    }
    // Previously: token values once, then one build per resolved target.
    assert_eq!(builds() - before, 1);
    let error = source.resolve("page", "night", None).err().unwrap();
    assert_eq!(error.value()["error"]["code"], "EXPORT_UNSUPPORTED");
}
