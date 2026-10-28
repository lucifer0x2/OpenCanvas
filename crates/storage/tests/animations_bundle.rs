mod bundle_support;
mod common;
use bundle_support::*;
use common::*;
use opencanvas_storage::*;
use serde_json::json;

#[test]
fn bundles_keep_animations_and_reject_invalid_ones() {
    let (temp, store, p) = setup();
    let mut ops = initial();
    ops.push(create(
        "card",
        NodeKind::Rectangle,
        Some("frame"),
        json!({"width":10,"height":10,"animations":[
            {"name":"enter","duration":400,"keyframes":[{"offset":0,"opacity":0}]},
            {"name":"lift","on":"hover","duration":200,"keyframes":[{"scale":1.02}]}]}),
    ));
    p.apply_batch(request(&p, 0, "initial", ops)).unwrap();
    let before = p.snapshot().unwrap();
    let bundle = p.export_bundle().unwrap();
    let imported = store.import_bundle("copy", &bundle.bytes).unwrap();
    assert_eq!(imported.snapshot().unwrap().nodes, before.nodes);

    // Import reports BUNDLE_INVALID without detail. The control edit changes the same JSON
    // path validly and imports, so the rejection below is the animation rule (R5).
    let edit = |keyframe: &str| {
        let mut files = unpack(&bundle.bytes);
        edit_database(
            &mut files,
            &format!(
                "UPDATE nodes SET props_json=json_set(props_json,\
                 '$.animations[1].keyframes[0]',json('{keyframe}')) WHERE id='card'"
            ),
        );
        pack(files)
    };
    store
        .import_bundle("control", &edit(r#"{"scale":1.5}"#))
        .unwrap();
    reject(&store, temp.path(), &edit(r#"{"opacity":0.5}"#));
    let snapshot = p.snapshot().unwrap();
    let mut invalid = snapshot.nodes.clone();
    let card = invalid.iter_mut().find(|n| n.id == "card").unwrap();
    card.props["animations"][1]["keyframes"][0] = json!({"opacity":0.5});
    let e = resolved_nodes(&Snapshot {
        nodes: invalid,
        ..snapshot
    })
    .unwrap_err();
    assert_eq!(
        e.field.as_deref(),
        Some("props.animations[1].keyframes[0].opacity")
    );
}
