mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};

fn fill(id: &str, value: Value) -> Style {
    Style {
        id: id.into(),
        name: id.into(),
        kind: "fill".into(),
        value,
    }
}

#[test]
fn three_ordered_upserts_cannot_create_mutual_style_alias_cycle() {
    let (_temp, _store, p) = setup();
    let before = p.snapshot().unwrap();
    let operations = vec![
        Operation::UpsertStyle {
            style: fill("a", json!("#fff")),
        },
        Operation::UpsertStyle {
            style: fill("b", json!({"styleId":"a"})),
        },
        Operation::UpsertStyle {
            style: fill("a", json!({"styleId":"b"})),
        },
    ];
    let error = p
        .apply_batch(request(&p, 0, "mutual-aliases", operations))
        .unwrap_err();
    assert_eq!(error.code, "VALIDATION_FAILED");
    assert_eq!(error.op_index, Some(1));
    assert_eq!(error.field.as_deref(), Some("style.value"));
    assert_eq!(p.snapshot().unwrap(), before);
    assert!(p.commits_after(0, 100).unwrap().is_empty());
}

#[test]
fn resolver_rejects_legacy_mutual_aliases_without_recursion() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let mut s = p.snapshot().unwrap();
    s.styles = vec![
        fill("a", json!({"styleId":"b"})),
        fill("b", json!({"styleId":"a"})),
    ];
    s.nodes.iter_mut().find(|n| n.id == "rect").unwrap().props["fill"] = json!({"styleId":"a"});
    assert_eq!(resolved_nodes(&s).unwrap_err().code, "VALIDATION_FAILED");
}
