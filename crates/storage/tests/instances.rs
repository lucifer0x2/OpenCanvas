mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::json;

fn fixture() -> Vec<Operation> {
    vec![
        create("page", NodeKind::Page, None, json!({})),
        create(
            "frame",
            NodeKind::Frame,
            Some("page"),
            json!({"width":800,"height":600,"layout":{"mode":"column"}}),
        ),
        create(
            "master",
            NodeKind::Frame,
            Some("page"),
            json!({"component":{"name":"Row"},"width":"fill","height":"hug","layout":{"mode":"row","padding":4}}),
        ),
        create(
            "label",
            NodeKind::Text,
            Some("master"),
            json!({"text":"Original","fontId":"font_bundled","fontSize":14,"lineHeight":20,"width":100,"height":20}),
        ),
        create(
            "one",
            NodeKind::Instance,
            Some("frame"),
            json!({"componentId":"master","width":"fill","overrides":{"label":{"text":"Override","fill":"#f00"}}}),
        ),
        create(
            "two",
            NodeKind::Instance,
            Some("frame"),
            json!({"componentId":"master","width":"fill"}),
        ),
    ]
}

#[test]
fn instances_expand_resolve_and_roundtrip_authored_only() {
    let (_temp, store, p) = setup();
    p.apply_batch(request(&p, 0, "create", fixture())).unwrap();
    let before = p.snapshot().unwrap();
    let expanded = resolved_expanded_nodes(&before, "default").unwrap();
    let child = expanded.iter().find(|n| n.id == "one/label").unwrap();
    assert_eq!(child.props["text"], "Override");
    assert_eq!(child.props["fill"], "#f00");
    assert_eq!(
        expanded.iter().find(|n| n.id == "two/label").unwrap().props["text"],
        "Original"
    );
    assert_eq!(
        expanded.iter().find(|n| n.id == "one").unwrap().kind,
        NodeKind::Frame
    );
    let bundle = p.export_bundle().unwrap();
    assert_eq!(bundle.manifest.database_schema_version, 4);
    let imported = store
        .import_bundle("copy", &bundle.bytes)
        .unwrap()
        .snapshot()
        .unwrap();
    assert_eq!(before.nodes, imported.nodes);
    assert!(!imported.nodes.iter().any(|n| n.id.contains('/')));
}

#[test]
fn instances_override_readonly_inuse_and_atomic_errors() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "create", fixture())).unwrap();
    let cases = [
        (
            Operation::UpdateNode {
                id: "one/label".into(),
                props: json!({"text":"bad"}),
            },
            "INSTANCE_CHILD_READONLY",
            None,
        ),
        (
            Operation::DeleteSubtree {
                id: "master".into(),
            },
            "COMPONENT_IN_USE",
            None,
        ),
        (
            Operation::UpdateNode {
                id: "one".into(),
                props: json!({"overrides":{"label":{"layout":{}}}}),
            },
            "VALIDATION_FAILED",
            Some("props.overrides.label.layout"),
        ),
        (
            Operation::UpdateNode {
                id: "one".into(),
                props: json!({"overrides":{"missing":{"text":"bad"}}}),
            },
            "VALIDATION_FAILED",
            Some("props.overrides.missing"),
        ),
        (
            create("child", NodeKind::Rectangle, Some("one"), json!({})),
            "VALIDATION_FAILED",
            Some("parentId"),
        ),
        (
            Operation::UpdateNode {
                id: "label".into(),
                props: json!({"links":[{"action":"back"}]}),
            },
            "VALIDATION_FAILED",
            Some("props.component"),
        ),
    ];
    for (i, (op, code, field)) in cases.into_iter().enumerate() {
        let error = p
            .apply_batch(request(&p, 1, &format!("bad{i}"), vec![op]))
            .unwrap_err();
        assert_eq!(error.code, code, "{error:?}");
        if let Some(field) = field {
            assert_eq!(error.field.as_deref(), Some(field), "{error:?}");
        }
        if code == "COMPONENT_IN_USE" {
            assert_eq!(error.node.unwrap().count, Some(2));
        }
        assert_eq!(p.snapshot().unwrap().revision, 1);
    }
    let error = p
        .apply_batch(request(
            &p,
            1,
            "whitelist",
            vec![Operation::UpdateNode {
                id: "one".into(),
                props: json!({"overrides":{"label":{"width":42}}}),
            }],
        ))
        .unwrap_err();
    assert!(error
        .node
        .unwrap()
        .allowed_keys
        .unwrap()
        .contains(&"text".into()));
}

#[test]
fn instances_cycles_depth_placements_and_dependency_deletion() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "create", fixture())).unwrap();
    let cycle = create(
        "cycle",
        NodeKind::Instance,
        Some("master"),
        json!({"componentId":"master"}),
    );
    assert_eq!(
        p.apply_batch(request(&p, 1, "cycle", vec![cycle]))
            .unwrap_err()
            .code,
        "INSTANCE_CYCLE"
    );
    let invalid = Operation::UpdateNode {
        id: "frame".into(),
        props: json!({"width":"hug"}),
    };
    assert!(p
        .apply_batch(request(&p, 1, "placement", vec![invalid]))
        .is_err());
    p.apply_batch(request(
        &p,
        1,
        "delete",
        vec![
            Operation::DeleteSubtree { id: "frame".into() },
            Operation::DeleteSubtree {
                id: "master".into(),
            },
        ],
    ))
    .unwrap();
    assert_eq!(p.snapshot().unwrap().nodes.len(), 1);
}

#[test]
fn instances_schema_two_and_three_migrate_without_authored_changes() {
    for version in [2, 3] {
        let (temp, store, p) = setup();
        p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
        let before = p.snapshot().unwrap();
        drop(p);
        drop(store);
        let conn = rusqlite::Connection::open(temp.path().join("fixture/db.sql")).unwrap();
        if version == 2 {
            conn.execute_batch("DROP TABLE notes").unwrap();
        }
        conn.execute_batch(&format!(
            "PRAGMA user_version={version}; UPDATE project_meta SET schema_version={version}"
        ))
        .unwrap();
        drop(conn);
        let store = Store::open(temp.path()).unwrap();
        let after = store.open_project("fixture").unwrap().snapshot().unwrap();
        assert_eq!(after.nodes, before.nodes);
        assert_eq!(after.revision, before.revision);
        assert_eq!(after.schema_version, 4);
    }
}
