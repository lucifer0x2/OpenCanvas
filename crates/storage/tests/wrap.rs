mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};

fn update(id: &str, props: Value) -> Operation {
    Operation::UpdateNode {
        id: id.into(),
        props,
    }
}
fn fixture() -> Vec<Operation> {
    vec![
        create("page", NodeKind::Page, None, json!({})),
        create(
            "frame",
            NodeKind::Frame,
            Some("page"),
            json!({"width":1040,"height":844,"layout":{"mode":"column"}}),
        ),
        create(
            "row",
            NodeKind::Group,
            Some("frame"),
            json!({"width":"fill","height":"hug","layout":{"mode":"row","wrap":true}}),
        ),
        create(
            "card",
            NodeKind::Group,
            Some("row"),
            json!({"width":"fill","minWidth":280,"height":80}),
        ),
        create(
            "hug",
            NodeKind::Group,
            Some("frame"),
            json!({"width":"hug","height":80}),
        ),
        create(
            "plain",
            NodeKind::Group,
            Some("frame"),
            json!({"width":"fill","height":80}),
        ),
    ]
}
fn reject(p: &Project, key: &str, operations: Vec<Operation>) {
    let before = p.snapshot().unwrap();
    let commits = p.commits_after(0, 100).unwrap();
    let mut ops = vec![update("frame", json!({"name":"must roll back"}))];
    ops.extend(operations);
    let error = p
        .apply_batch(request(p, before.revision, key, ops))
        .unwrap_err();
    assert_eq!(error.code, "VALIDATION_FAILED");
    assert!(error.field.is_some());
    assert_eq!(p.snapshot().unwrap(), before);
    assert_eq!(
        serde_json::to_value(p.commits_after(0, 100).unwrap()).unwrap(),
        serde_json::to_value(commits).unwrap()
    );
    // Failed request did not reserve the idempotency key.
    let ok = p
        .apply_batch(request(
            p,
            before.revision,
            key,
            vec![update("frame", json!({"name":"valid"}))],
        ))
        .unwrap();
    assert!(!ok.replayed);
}
#[test]
fn schema_guards_and_late_failure_are_atomic() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", fixture())).unwrap();
    let mut cases = vec![
        update("row", json!({"width":"hug"})),
        update("row", json!({"width":0})),
        update("row", json!({"layout":{"mode":"column","wrap":true}})),
        update("row", json!({"layout":{"mode":"free","wrap":true}})),
        update("card", json!({"minWidth":0})),
        update("card", json!({"minWidth":300,"maxWidth":200})),
        update("card", json!({"minHeight":300,"maxHeight":200})),
        update("row", json!({"layout":{"mode":"row","wrap":true,"typo":1}})),
        create(
            "leaf",
            NodeKind::Rectangle,
            Some("frame"),
            json!({"width":20,"layout":{"mode":"row","wrap":true}}),
        ),
        create(
            "missing-min",
            NodeKind::Group,
            Some("row"),
            json!({"width":"fill","height":40}),
        ),
    ];
    for bad in [
        Value::Null,
        json!(1),
        json!("true"),
        json!({"tokenId":"wrap"}),
    ] {
        cases.push(update("row", json!({"layout":{"mode":"row","wrap":bad}})));
    }
    for (i, op) in cases.into_iter().enumerate() {
        reject(&p, &format!("bad-{i}"), vec![op]);
    }
}
#[test]
fn final_candidate_reparent_ancestor_and_theme_changes_are_checked() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", fixture())).unwrap();
    reject(
        &p,
        "reparent",
        vec![Operation::ReparentNode {
            id: "plain".into(),
            parent_id: "row".into(),
            index: None,
        }],
    );
    // This update is individually valid; only the descendant wrap rule rejects it.
    reject(
        &p,
        "zero-ancestor",
        vec![update("frame", json!({"width":0}))],
    );
    reject(
        &p,
        "ancestor-hug",
        vec![Operation::ReparentNode {
            id: "row".into(),
            parent_id: "hug".into(),
            index: None,
        }],
    );
    // Enabling wrap on an existing no-min child must fail at final candidate validation.
    let revision = p.snapshot().unwrap().revision;
    p.apply_batch(request(
        &p,
        revision,
        "off",
        vec![
            update("row", json!({"layout":{"mode":"row","wrap":false}})),
            update("card", json!({"minWidth":0})),
        ],
    ))
    .unwrap();
    reject(
        &p,
        "enable",
        vec![update("row", json!({"layout":{"mode":"row","wrap":true}}))],
    );
    let revision = p.snapshot().unwrap().revision;
    let token = |value| Operation::UpsertToken {
        token: Token {
            id: "minimum".into(),
            name: "Minimum".into(),
            token_type: "dimension".into(),
            value,
        },
    };
    p.apply_batch(request(
        &p,
        revision,
        "token",
        vec![
            token(json!({"default":280,"themes":{"dark":300}})),
            update("card", json!({"minWidth":{"tokenId":"minimum"}})),
            update("row", json!({"layout":{"mode":"row","wrap":true}})),
        ],
    ))
    .unwrap();
    reject(
        &p,
        "theme-zero",
        vec![token(json!({"default":280,"themes":{"dark":0}}))],
    );
    reject(&p, "literal-zero", vec![token(json!(0))]);
    reject(
        &p,
        "theme-max",
        vec![update("card", json!({"maxWidth":290}))],
    );
}
#[test]
fn valid_fixed_hug_nested_equal_bounds_and_oversized_minimum_persist() {
    let (temp, store, p) = setup();
    let mut ops = fixture();
    ops.extend([
        create("fixed",NodeKind::Group,Some("row"),json!({"width":60,"height":20,"minWidth":80,"maxWidth":80,"minHeight":20,"maxHeight":20})),
        create("intrinsic",NodeKind::Group,Some("row"),json!({"width":"hug","height":20})),
        create("nested",NodeKind::Group,Some("card"),json!({"width":"fill","height":20,"layout":{"mode":"row","wrap":true}})),
        update("card",json!({"minWidth":2000})),
    ]);
    p.apply_batch(request(&p, 0, "valid", ops)).unwrap();
    let before = p.snapshot().unwrap();
    let id = p.id().to_owned();
    drop(p);
    drop(store);
    let store = Store::open(temp.path()).unwrap();
    assert_eq!(store.open_project(&id).unwrap().snapshot().unwrap(), before);
}
