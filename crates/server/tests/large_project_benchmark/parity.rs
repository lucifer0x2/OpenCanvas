//! Untimed full-reference checks against the exact Task24 scaled source fixture.
//! Runs before every cell's measurements; timed samples keep their existing path.
use opencanvas_storage::{validate::Candidate, NodeKind, Operation, Result, Snapshot};
use serde_json::json;
use std::collections::BTreeSet;

pub fn validate_fixture(snapshot: &Snapshot) {
    Candidate::from_snapshot(snapshot).validate().unwrap();
    let text = snapshot
        .nodes
        .iter()
        .find(|n| n.kind == NodeKind::Text)
        .unwrap();
    let root = snapshot
        .nodes
        .iter()
        .find(|n| {
            n.kind == NodeKind::Frame
                && n.parent_id.as_ref().is_some_and(|parent| {
                    snapshot
                        .nodes
                        .iter()
                        .any(|p| p.id == *parent && p.kind == NodeKind::Page)
                })
        })
        .unwrap();
    let update = |id: &str, props| Operation::UpdateNode {
        merge: false,
        id: id.into(),
        props,
    };
    let mut cases = vec![
        vec![update(&text.id, json!({"text":"Task25 differential"}))],
        vec![update(
            &text.id,
            json!({"fontId":"task25_missing_resource"}),
        )],
        vec![
            update(&text.id, json!({"text":"Earlier valid"})),
            update(&text.id, json!({"fontSize":-1})),
        ],
        vec![Operation::DeleteSubtree {
            id: text.id.clone(),
        }],
        vec![Operation::ReparentNode {
            id: text.id.clone(),
            parent_id: root.id.clone(),
            index: None,
        }],
        vec![update(&root.id, json!({"theme":"default"}))],
        vec![update(&root.id, json!({"theme":"task25_missing_theme"}))],
    ];
    if let Some(token) = snapshot.tokens.first() {
        cases.push(vec![Operation::UpsertToken {
            token: token.clone(),
        }]);
    }
    if let Some(style) = snapshot.styles.first() {
        cases.push(vec![Operation::UpsertStyle {
            style: style.clone(),
        }]);
    }
    if let Some(instance) = snapshot.nodes.iter().find(|n| n.kind == NodeKind::Instance) {
        cases.push(vec![update(
            &instance.id,
            json!({"overrides":{"task25_missing_target":{"opacity":2}}}),
        )]);
    }
    for ops in &cases {
        let reference = apply(snapshot, ops).and_then(|(c, _)| c.validate());
        let incremental = apply(snapshot, ops)
            .and_then(|(c, changed)| c.validate_after(snapshot, ops, &changed).map(|_| ()));
        assert_eq!(
            reference.map_err(|e| serde_json::to_value(e).unwrap()),
            incremental.map_err(|e| serde_json::to_value(e).unwrap()),
            "Task24 scaled fixture incremental/full parity"
        );
    }
    eprintln!(
        "Task25 differential: {} authored nodes, {} operation cases passed",
        snapshot.nodes.len(),
        cases.len()
    );
}

fn apply(snapshot: &Snapshot, ops: &[Operation]) -> Result<(Candidate, BTreeSet<String>)> {
    let mut candidate = Candidate::from_snapshot(snapshot);
    let mut changed = BTreeSet::new();
    for (index, op) in ops.iter().enumerate() {
        candidate.apply(op, &mut changed).map_err(|e| e.at(index))?;
    }
    candidate.prune_links(&mut changed);
    Ok((candidate, changed))
}
