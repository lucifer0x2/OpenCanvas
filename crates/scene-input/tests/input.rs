use opencanvas_scene_input::{
    make_input, make_input_with_scale, Node, NodeKind, SceneOptions, Snapshot,
};
use serde_json::{json, Value};

fn fixture() -> Snapshot {
    serde_json::from_value(json!({
        "schemaVersion":4,"projectId":"project","slug":"fixture","name":"Fixture","revision":7,
        "retention":{"oldestCommitRevision":0,"idempotencyPolicy":"lifetime"},"notes":[],
        "tokens":[{"id":"ink","name":"Ink","type":"color",
            "value":{"default":"#111111","themes":{"night":"#eeeeee"}}}],
        "styles":[{"id":"copy","name":"Copy","kind":"text","value":{
            "fontId":"font_bundled","fontSize":16,"lineHeight":24,"fill":{"tokenId":"ink"}}}],
        "fonts":[{"id":"unused_font","family":"Unused","weight":400,"style":"normal","status":"ready"},
            {"id":"font_bundled","family":"Noto","weight":400,"style":"normal",
                "contentHash":"abc","status":"ready"}],
        "assets":[{"id":"used","contentHash":"def","mediaType":"image/png","byteSize":1,
                "relativeKey":"opaque","status":"ready"},
            {"id":"unused","contentHash":"ghi","mediaType":"image/png","byteSize":1,
                "relativeKey":"opaque2","status":"ready"}],
        "nodes":[
            {"id":"page","kind":"page","parentId":null,"sortKey":0,"props":{}},
            {"id":"master","kind":"frame","parentId":"page","sortKey":0,
                "props":{"width":100,"height":100,"component":{"name":"Tile"}}},
            {"id":"label","kind":"text","parentId":"master","sortKey":0,
                "props":{"text":"Master","styleId":"copy"}},
            {"id":"frame","kind":"frame","parentId":"page","sortKey":1,
                "props":{"width":320,"height":200,"theme":"night"}},
            {"id":"image","kind":"image","parentId":"frame","sortKey":2,
                "props":{"assetId":"used","width":20,"height":20}},
            {"id":"placed","kind":"instance","parentId":"frame","sortKey":1,
                "props":{"componentId":"master","overrides":{"label":{"text":"Placed","visible":false}}}}
        ]
    })).unwrap()
}

fn options() -> SceneOptions {
    SceneOptions {
        frame_id: "frame".into(),
        width: 320.,
        height: 200.,
        theme: "default".into(),
        state: "default".into(),
    }
}

#[test]
fn pure_frame_input_keeps_placed_scope_order_resources_and_authored_revision() {
    let snapshot = fixture();
    let before = snapshot.clone();
    let input = make_input(&snapshot, &options()).unwrap();
    assert_eq!(snapshot, before);
    assert_eq!(input["schemaVersion"], 1);
    assert_eq!(input["revision"], 7);
    let nodes = input["nodes"].as_array().unwrap();
    let ids: Vec<_> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["frame", "placed", "placed/label", "image"]);
    assert_eq!(nodes[0]["parentId"], Value::Null);
    assert_eq!(nodes[1]["kind"], "frame");
    assert_eq!(nodes[2]["props"]["text"], "Placed");
    assert_eq!(nodes[2]["props"]["fill"], "#eeeeee");
    assert_eq!(nodes[2]["props"]["effectiveTheme"], "night");
    assert_eq!(nodes[2]["props"]["opacity"], 0);
    assert_eq!(
        input["fonts"],
        json!([{"id":"font_bundled","family":"Noto","hash":"abc",
        "url":"","weight":400,"style":"normal"}])
    );
    assert_eq!(
        input["assets"],
        json!([{"id":"used","hash":"def","url":"","mediaType":"image/png"}])
    );
    let typed: opencanvas_layout::LayoutInput = serde_json::from_value(input).unwrap();
    assert_eq!(typed.nodes.len(), 4);
}

#[test]
fn pure_frame_boundary_preserves_frame_theme_and_validation_errors() {
    let snapshot = fixture();
    let mut request = options();
    request.frame_id = "page".into();
    let error = make_input(&snapshot, &request).unwrap_err();
    assert_eq!(error.code, "INVALID_REQUEST");
    assert_eq!(error.message, "frameId must identify a frame");
    request.frame_id = "frame".into();
    request.theme = "absent".into();
    assert_eq!(
        make_input(&snapshot, &request).unwrap_err().code,
        "UNSUPPORTED_THEME"
    );
    let mut invalid = snapshot;
    invalid.nodes[5].props["overrides"]["label"]["opacity"] = json!(2);
    let error = make_input(&invalid, &options()).unwrap_err();
    assert_eq!(error.code, "VALIDATION_FAILED");
    assert_eq!(
        error.field.as_deref(),
        Some("props.overrides.label.opacity")
    );
}

#[test]
fn render_node_budget_applies_after_subtree_filter_and_instance_expansion() {
    let mut snapshot = fixture();
    for i in 0..2044 {
        snapshot.nodes.push(Node {
            id: format!("node{i}"),
            parent_id: Some("frame".into()),
            sort_key: 3 + i,
            kind: NodeKind::Rectangle,
            props: json!({"width":1,"height":1}),
        });
    }
    assert_eq!(
        make_input(&snapshot, &options()).unwrap()["nodes"]
            .as_array()
            .unwrap()
            .len(),
        2048
    );
    snapshot.nodes.push(Node {
        id: "overflow".into(),
        parent_id: Some("frame".into()),
        sort_key: 3000,
        kind: NodeKind::Rectangle,
        props: json!({"width":1,"height":1}),
    });
    let error = make_input(&snapshot, &options()).unwrap_err();
    assert_eq!(error.code, "INVALID_REQUEST");
    assert_eq!(error.message, "Frame exceeds 2048 render nodes");
}

#[test]
fn viewport_guard_precedes_resolution_and_scene_only_defaults_to_scale_one() {
    let snapshot = fixture();
    let mut request = options();
    request.width = 8000.0;
    request.height = 8000.0;
    assert!(make_input_with_scale(&snapshot, &request, 0.5).is_ok());
    request.theme = "absent".into();
    let error = make_input(&snapshot, &request).unwrap_err();
    assert_eq!(error.code, "INVALID_REQUEST");
    assert_eq!(error.message, "Viewport or scale exceeds the render budget");
}
