//! Bounded warm work and computed-metadata parity, independent of runtime jobs.
use super::{
    pagination, scoped_tree_fixture::fixture, tree, tree_legacy, viewports::Computed, ReadRequest,
};
use opencanvas_storage::{
    read_scope::{ReadScope, ScopedSnapshot},
    Node, NodeKind, Note, Snapshot,
};
use serde_json::json;

fn counts(s: &Snapshot) -> (usize, usize, usize, usize) {
    let scoped = ScopedSnapshot::validate(s).unwrap();
    let mut nodes = scoped.expand(ReadScope::Target("frame"), true).unwrap();
    let notes = scoped.notes_for_nodes(nodes.dependencies.iter().map(|n| n.id.as_str()));
    scoped.resolve(&mut nodes, "default").unwrap();
    (
        nodes.stats.closure_nodes,
        nodes.stats.expanded_nodes,
        nodes.stats.resolved_nodes,
        notes.len(),
    )
}

#[test]
fn unrelated_pages_and_notes_do_not_grow_tree_expansion_resolution_or_note_work() {
    let (_dir, storage, id) = fixture();
    let mut s = storage.snapshot(&id).unwrap();
    let before = counts(&s);
    for page in 0..8 {
        let parent = format!("extra-page-{page}");
        s.nodes.push(Node {
            id: parent.clone(),
            kind: NodeKind::Page,
            parent_id: None,
            sort_key: page,
            props: json!({}),
        });
        let frame = format!("extra-frame-{page}");
        s.nodes.push(Node {
            id: frame.clone(),
            kind: NodeKind::Frame,
            parent_id: Some(parent),
            sort_key: 0,
            props: json!({"width":300,"height":200}),
        });
        for child in 0..20 {
            let id = format!("extra-{page}-{child}");
            s.nodes.push(Node {
                id: id.clone(),
                kind: NodeKind::Rectangle,
                parent_id: Some(frame.clone()),
                sort_key: child,
                props: json!({"width":10,"height":10}),
            });
            s.notes.push(Note {
                id: format!("note-{id}"),
                node_id: id,
                text: "Unrelated intent".into(),
                author: None,
                resolved: false,
                replies: Vec::new(),
            });
        }
    }
    s.notes.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(counts(&s), before);
    assert_eq!(before.3, 4);
    assert!(before.0 < s.nodes.len() / 4);
    eprintln!(
        "scoped work before/after unrelated growth: {before:?}; total nodes={}, notes={}",
        s.nodes.len(),
        s.notes.len()
    );
}

fn computed(reflow: bool) -> Computed {
    Computed {
        metadata: json!({"status":"ready","viewports":[]}),
        bounds: [(
            "frame".into(),
            vec![json!({"x":1,"y":2,"width":3,"height":4})],
        )]
        .into(),
        lines: Default::default(),
        diagnostics: vec![json!({"code":"MEASURED"})],
        reflow: Some(reflow),
    }
}

#[test]
fn scoped_rows_and_reference_instances_preserve_computed_bounds_null_and_diagnostics() {
    let (_dir, storage, id) = fixture();
    let s = storage.snapshot(&id).unwrap();
    let scoped = ScopedSnapshot::validate(&s).unwrap();
    for reflow in [false, true] {
        for rows in [false, true] {
            let mut args = json!({"projectId":id,"frameId":"frame","limit":2});
            if rows {
                args["nodeFormat"] = json!("rows");
            }
            loop {
                let req: ReadRequest = serde_json::from_value(args.clone()).unwrap();
                let (scope, offset) = pagination::scope(&s, "get_design_context", &req).unwrap();
                let expected = tree_legacy::read(
                    &s,
                    "get_design_context",
                    serde_json::from_value(args.clone()).unwrap(),
                    scope.clone(),
                    offset,
                    Some(computed(reflow)),
                )
                .unwrap();
                let actual = tree::read_scoped(
                    &scoped,
                    "get_design_context",
                    req,
                    scope,
                    offset,
                    Some(computed(reflow)),
                )
                .unwrap();
                assert_eq!(
                    serde_json::to_vec(&actual).unwrap(),
                    serde_json::to_vec(&expected).unwrap()
                );
                if actual["truncated"] != true {
                    break;
                }
                args["cursor"] = actual["nextCursor"].clone();
            }
        }
    }
}

#[test]
fn scoped_fragmentation_diagnostics_and_indexed_ancestor_intent_match_full_reference() {
    let (_dir, storage, id) = fixture();
    let mut s = storage.snapshot(&id).unwrap();
    let marker = s
        .notes
        .iter()
        .find(|n| n.node_id == "frame")
        .unwrap()
        .clone();
    s.notes
        .retain(|n| !matches!(n.node_id.as_str(), "page" | "frame"));
    let base = s.nodes.iter().find(|n| n.id == "copy").unwrap().clone();
    for (i, letter) in "abcdefgh".chars().enumerate() {
        let mut node = base.clone();
        node.id = format!("glyph-{i}");
        node.parent_id = Some("frame".into());
        node.props["text"] = json!(letter.to_string());
        node.props["x"] = json!(i * 8);
        s.nodes.push(node);
    }
    for intentional in [false, true] {
        if intentional {
            s.notes.push(marker.clone());
        }
        let req = || {
            serde_json::from_value::<ReadRequest>(json!({"projectId":id,"frameId":"frame"}))
                .unwrap()
        };
        let (scope, offset) = pagination::scope(&s, "get_design_context", &req()).unwrap();
        let scoped = ScopedSnapshot::validate(&s).unwrap();
        let expected =
            tree_legacy::read(&s, "get_design_context", req(), scope.clone(), offset, None)
                .unwrap();
        let actual =
            tree::read_scoped(&scoped, "get_design_context", req(), scope, offset, None).unwrap();
        assert_eq!(
            serde_json::to_vec(&actual).unwrap(),
            serde_json::to_vec(&expected).unwrap()
        );
        assert_eq!(
            actual["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d["code"] == "TEXT_FRAGMENTATION_SUSPECTED"),
            !intentional
        );
    }
}
