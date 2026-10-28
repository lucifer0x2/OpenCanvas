mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};
#[path = "resolution/limits.rs"]
mod limits;
#[path = "resolution/safety.rs"]
mod safety;
#[path = "resolution/styles.rs"]
mod styles;

fn token(id: &str, kind: &str, value: Value) -> Operation {
    Operation::UpsertToken {
        token: Token {
            id: id.into(),
            name: id.into(),
            token_type: kind.into(),
            value,
        },
    }
}
fn fill(nodes: &[Node]) -> &Value {
    &nodes.iter().find(|node| node.id == "rect").unwrap().props["fill"]
}

#[test]
fn multihop_themes_preserve_authored_readback_and_revision_replay() {
    let (_temp, _store, p) = setup();
    let mut operations = vec![
        token(
            "brand",
            "color",
            json!({"default":"#fff","themes":{"dark":"#123456"}}),
        ),
        token("alias", "color", json!({"aliasId":"brand"})),
        token(
            "semantic",
            "color",
            json!({"default":{"aliasId":"alias"},"themes":{"dark":{"aliasId":"brand"}}}),
        ),
        Operation::UpsertStyle {
            style: Style {
                id: "surface".into(),
                name: "Surface".into(),
                kind: "fill".into(),
                value: json!({"tokenId":"semantic"}),
            },
        },
    ];
    operations.extend(initial());
    operations.push(Operation::UpdateNode {
        id: "rect".into(),
        props: json!({"fill":{"styleId":"surface"}}),
    });
    let req = request(&p, 0, "themed", operations);
    p.apply_batch(req.clone()).unwrap();
    let snapshot = p.snapshot().unwrap();
    assert_eq!(fill(&resolved_nodes(&snapshot).unwrap()), "#fff");
    assert_eq!(
        fill(&resolved_nodes_for_theme(&snapshot, "dark").unwrap()),
        "#123456"
    );
    assert_eq!(
        resolved_tokens_for_theme(&snapshot, "dark").unwrap()["alias"],
        "#123456"
    );
    assert_eq!(
        snapshot
            .nodes
            .iter()
            .find(|n| n.id == "rect")
            .unwrap()
            .props["fill"],
        json!({"styleId":"surface"})
    );
    assert_eq!(
        snapshot
            .tokens
            .iter()
            .find(|t| t.id == "alias")
            .unwrap()
            .value,
        json!({"aliasId":"brand"})
    );
    let result = p.apply_batch(req).unwrap();
    assert!(result.replayed);
    assert_eq!(result.committed_revision, 1);
    assert_eq!(p.snapshot().unwrap(), snapshot);

    let update = token(
        "brand",
        "color",
        json!({"default":"#eee","themes":{"dark":"#abcdef"}}),
    );
    let result = p
        .apply_batch(request(&p, 1, "theme-change", vec![update]))
        .unwrap();
    assert_eq!(result.committed_revision, 2);
    assert!(result.changed_ids.contains(&"brand".into()));
    assert_eq!(
        fill(&resolved_nodes_for_theme(&p.snapshot().unwrap(), "dark").unwrap()),
        "#abcdef"
    );
    assert_eq!(
        fill(&resolved_nodes_for_theme(&snapshot, "dark").unwrap()),
        "#123456"
    );
    assert_eq!(p.commits_after(1, 10).unwrap()[0].revision, 2);
}

#[test]
fn aliases_reject_cycles_missing_targets_wrong_types_and_forward_refs_atomically() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(
        &p,
        0,
        "seed",
        vec![
            token("a", "color", json!("#fff")),
            token("b", "color", json!({"aliasId":"a"})),
            token("size", "dimension", json!(12)),
        ],
    ))
    .unwrap();
    let before = p.snapshot().unwrap();
    for (index, operations) in [
        vec![token("a", "color", json!({"aliasId":"b"}))],
        vec![token(
            "unused",
            "color",
            json!({"default":"#000","themes":{"dark":{"aliasId":"unused"}}}),
        )],
        vec![token("a", "color", json!({"aliasId":"absent"}))],
        vec![token("a", "color", json!({"aliasId":"size"}))],
        vec![
            token("a", "color", json!({"aliasId":"late"})),
            token("late", "color", json!("#fff")),
        ],
        vec![token("a", "number", json!(3))],
        vec![token("a", "color", json!({"tokenId":"b"}))],
        vec![token("a", "color", json!({"aliasId":"b","extra":true}))],
    ]
    .into_iter()
    .enumerate()
    {
        let error = p
            .apply_batch(request(&p, 1, &format!("bad-{index}"), operations))
            .unwrap_err();
        assert_eq!(error.code, "VALIDATION_FAILED");
        assert_eq!(p.snapshot().unwrap(), before);
        assert_eq!(p.commits_after(1, 100).unwrap().len(), 0);
    }
    // Cycle spans different branches, even though no single selected theme loops.
    let operations = vec![
        token(
            "a",
            "color",
            json!({"default":"#fff","themes":{"dark":{"aliasId":"b"}}}),
        ),
        token(
            "b",
            "color",
            json!({"default":{"aliasId":"a"},"themes":{"dark":"#000"}}),
        ),
    ];
    assert!(p
        .apply_batch(request(&p, 1, "cross-theme", operations))
        .is_err());
    assert_eq!(p.snapshot().unwrap(), before);
}

#[test]
fn theme_fallback_and_node_specific_ranges_are_validated_before_commit() {
    let (_temp, _store, p) = setup();
    let mut operations = vec![
        token(
            "size",
            "dimension",
            json!({"default":20,"themes":{"dark":24}}),
        ),
        token("height", "dimension", json!(30)),
        token(
            "opacity",
            "number",
            json!({"default":1,"themes":{"dark":0.5}}),
        ),
    ];
    operations.extend(initial());
    operations.push(Operation::UpdateNode {id:"text".into(),props:json!({"fontSize":{"tokenId":"size"},"lineHeight":{"tokenId":"height"},"opacity":{"tokenId":"opacity"}})});
    p.apply_batch(request(&p, 0, "dimensions", operations))
        .unwrap();
    let before = p.snapshot().unwrap();
    let dark = resolved_nodes_for_theme(&before, "dark").unwrap();
    let text = dark.iter().find(|n| n.id == "text").unwrap();
    assert_eq!(text.props["fontSize"], 24);
    assert_eq!(text.props["lineHeight"], 30);
    assert_eq!(text.props["opacity"], 0.5);
    for (index, op) in [
        token(
            "size",
            "dimension",
            json!({"default":20,"themes":{"dark":0}}),
        ),
        token(
            "opacity",
            "number",
            json!({"default":1,"themes":{"dark":2}}),
        ),
        token(
            "size",
            "dimension",
            json!({"default":20,"themes":{"bad theme":24}}),
        ),
        token(
            "size",
            "dimension",
            json!({"default":20,"themes":{"default":24}}),
        ),
        token(
            "size",
            "dimension",
            json!({"default":20,"themes":{"dark":{"default":10,"themes":{}}}}),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(p
            .apply_batch(request(&p, 1, &format!("range-{index}"), vec![op]))
            .is_err());
        assert_eq!(p.snapshot().unwrap(), before);
    }
}
