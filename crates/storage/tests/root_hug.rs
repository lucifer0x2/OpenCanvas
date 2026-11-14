//! Root frames keep a numeric width; their height is numeric or `hug` over a stack layout.
mod common;
use common::{create, request, setup};
use opencanvas_storage::*;
use serde_json::{json, Value};

fn update(id: &str, props: Value) -> Operation {
    Operation::UpdateNode {
        id: id.into(),
        props,
        merge: false,
    }
}

fn fixture() -> Vec<Operation> {
    vec![
        create("page", NodeKind::Page, None, json!({"name":"Home"})),
        create(
            "root",
            NodeKind::Frame,
            Some("page"),
            json!({"width":390,"height":"hug","layout":{"mode":"column","padding":16,"gap":8,
                "breakpoints":[{"maxWidth":600,"layout":{"mode":"row"}}]}}),
        ),
        create(
            "card",
            NodeKind::Rectangle,
            Some("root"),
            json!({"width":"fill","height":40}),
        ),
    ]
}

fn field(p: &Project, ops: Vec<Operation>) -> String {
    let revision = p.snapshot().unwrap().revision;
    let error = p
        .apply_batch(request(p, revision, "rejected", ops))
        .unwrap_err();
    assert_eq!(error.code, "VALIDATION_FAILED", "{error:?}");
    error.field.unwrap()
}

#[test]
fn hug_height_is_accepted_and_scene_input_ignores_the_viewport_height() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", fixture())).unwrap();
    let snapshot = p.snapshot().unwrap();
    let input = |height| {
        let options = SceneOptions {
            frame_id: "root".into(),
            width: 390.,
            height,
            theme: "default".into(),
            state: "default".into(),
        };
        make_input(&snapshot, &options).unwrap()
    };
    assert_eq!(input(844.), input(600.));
    assert_eq!(input(844.)["viewport"], json!({"width":390.,"height":1.}));
    // A free component master with a hug height keeps the requested viewport, as before.
    let master = create(
        "master",
        NodeKind::Frame,
        Some("page"),
        json!({"component":{"name":"Tile"},"width":200,"height":"hug"}),
    );
    p.apply_batch(request(&p, 1, "master", vec![master]))
        .unwrap();
    let snapshot = p.snapshot().unwrap();
    let options = SceneOptions {
        frame_id: "master".into(),
        width: 200.,
        height: 600.,
        theme: "default".into(),
        state: "default".into(),
    };
    let input = make_input(&snapshot, &options).unwrap();
    assert_eq!(input["viewport"], json!({"width":200.,"height":600.}));
}

#[test]
fn width_stays_numeric_and_height_hug_needs_stacks() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", fixture())).unwrap();
    for width in [json!("hug"), json!("fill"), json!("50%")] {
        assert_eq!(
            field(&p, vec![update("root", json!({"width":width}))]),
            "width"
        );
    }
    for height in [json!("fill"), json!("50%")] {
        assert_eq!(
            field(&p, vec![update("root", json!({"height":height}))]),
            "height"
        );
    }
    let free = json!({"mode":"column","breakpoints":[{"maxWidth":600,"layout":{"mode":"free"}}]});
    for layout in [json!({"mode":"free"}), free] {
        assert_eq!(
            field(&p, vec![update("root", json!({"layout":layout}))]),
            "height"
        );
    }
    // A component master directly under the page keeps its existing freedom.
    let master = create(
        "master",
        NodeKind::Frame,
        Some("page"),
        json!({"component":{"name":"Tile"},"width":"hug","height":"hug"}),
    );
    let revision = p.snapshot().unwrap().revision;
    p.apply_batch(request(&p, revision, "master", vec![master]))
        .unwrap();
}

#[test]
fn hug_root_height_is_not_definite_for_children() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", fixture())).unwrap();
    for height in [json!("fill"), json!("50%")] {
        assert_eq!(
            field(&p, vec![update("card", json!({"height":height}))]),
            "height"
        );
    }
    // A numeric root accepts them; turning it back to hug re-checks the children.
    let revision = p.snapshot().unwrap().revision;
    p.apply_batch(request(
        &p,
        revision,
        "fixed",
        vec![
            update("root", json!({"height":800})),
            update("card", json!({"height":"50%"})),
        ],
    ))
    .unwrap();
    assert_eq!(
        field(&p, vec![update("root", json!({"height":"hug"}))]),
        "height"
    );
}
