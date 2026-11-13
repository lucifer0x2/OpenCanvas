#[allow(dead_code)]
mod incremental_support;
use incremental_support::*;
use opencanvas_scene_input::{read_scope::*, *};
use serde_json::json;

fn options(id: &str) -> SceneOptions {
    SceneOptions {
        frame_id: id.into(),
        width: 800.,
        height: 600.,
        theme: "default".into(),
        state: "default".into(),
    }
}

#[test]
fn scoped_scene_bytes_match_full_for_authored_and_nested_virtual_frames() {
    let snapshot = fixture(5, 2);
    let scoped = ScopedSnapshot::validate(&snapshot).unwrap();
    for id in ["frame0", "instance0", "instance0/nested", "master", "outer"] {
        for theme in ["default", "night"] {
            let mut request = options(id);
            request.theme = theme.into();
            assert_eq!(
                serde_json::to_vec(&make_input(&snapshot, &request).unwrap()).unwrap(),
                serde_json::to_vec(&scoped.make_input_with_scale(&request, 1.).unwrap()).unwrap()
            );
            let (prepared, nodes) = scoped.prepare_input_with_scale(&request, 1.).unwrap();
            assert_eq!(
                prepared,
                scoped.make_input_with_scale(&request, 1.).unwrap()
            );
            assert_eq!(nodes.stats.resolved_nodes, nodes.stats.expanded_nodes);
        }
    }
}

#[test]
fn scoped_scene_keeps_full_request_error_precedence() {
    let snapshot = fixture(1, 0);
    let scoped = ScopedSnapshot::validate(&snapshot).unwrap();
    for id in ["page0", "missing", "instance0/missing", "text0", "frame0"] {
        for theme in ["default", "absent"] {
            for width in [800., 80000.] {
                let mut request = options(id);
                request.theme = theme.into();
                request.width = width;
                let reference =
                    make_input(&snapshot, &request).map_err(|e| serde_json::to_value(e).unwrap());
                let actual = scoped
                    .make_input_with_scale(&request, 1.)
                    .map_err(|e| serde_json::to_value(e).unwrap());
                assert_eq!(actual, reference);
            }
        }
    }
}

#[test]
fn scoped_render_node_budget_remains_exact_at_2048_and_2049() {
    let mut snapshot = fixture(1, 0);
    // Existing frame contains 9 expanded nodes.
    let count = make_input(&snapshot, &options("frame0")).unwrap()["nodes"]
        .as_array()
        .unwrap()
        .len();
    for i in 0..2048 - count {
        snapshot.nodes.push(node(
            &format!("bulk{i}"),
            NodeKind::Rectangle,
            Some("frame0"),
            json!({"width":1,"height":1}),
        ));
    }
    let scoped = ScopedSnapshot::validate(&snapshot).unwrap();
    assert_eq!(
        scoped
            .make_input_with_scale(&options("frame0"), 1.)
            .unwrap()["nodes"]
            .as_array()
            .unwrap()
            .len(),
        2048
    );
    snapshot.nodes.push(node(
        "overflow",
        NodeKind::Rectangle,
        Some("frame0"),
        json!({"width":1,"height":1}),
    ));
    let scoped = ScopedSnapshot::validate(&snapshot).unwrap();
    assert_eq!(
        serde_json::to_value(
            scoped
                .make_input_with_scale(&options("frame0"), 1.)
                .unwrap_err()
        )
        .unwrap(),
        serde_json::to_value(make_input(&snapshot, &options("frame0")).unwrap_err()).unwrap()
    );
}
