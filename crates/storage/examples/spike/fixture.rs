use opencanvas_storage::*;
use serde_json::json;

pub fn req(p: &Project, rev: u64, key: String, operations: Vec<Operation>) -> BatchRequest {
    BatchRequest {
        project_id: p.id().into(),
        expected_revision: rev,
        idempotency_key: key,
        operations,
    }
}
pub fn node(
    id: String,
    kind: NodeKind,
    parent: Option<&str>,
    props: serde_json::Value,
) -> Operation {
    Operation::CreateNode {
        id,
        kind,
        parent_id: parent.map(str::to_string),
        props,
        index: None,
    }
}
pub fn seed(p: &Project, size: usize) {
    let mut ops = vec![
        Operation::UpsertToken {
            token: Token {
                id: "brand".into(),
                name: "Brand".into(),
                token_type: "color".into(),
                value: json!("#1a73e8"),
            },
        },
        Operation::UpsertStyle {
            style: Style {
                id: "surface".into(),
                name: "Surface".into(),
                kind: "fill".into(),
                value: json!({"tokenId":"brand"}),
            },
        },
        node(
            "page".into(),
            NodeKind::Page,
            None,
            json!({"name":"Storage fixture"}),
        ),
        node(
            "frame-a".into(),
            NodeKind::Frame,
            Some("page"),
            json!({"width":1440,"height":900,"layout":{"mode":"column","gap":8,"padding":24}}),
        ),
        node(
            "frame-b".into(),
            NodeKind::Frame,
            Some("page"),
            json!({"width":1440,"height":900,"layout":{"mode":"row","gap":8,"padding":24}}),
        ),
    ];
    let mut rev = 0;
    let mut batch = 0;
    for i in 0..size - 3 {
        let text = i >= 10 && i % 3 == 0;
        let props = if text {
            json!({"name":"base","text":"Thiết kế tiếng Việt: durable revision","width":240,"height":"hug","fontId":"font_bundled","fontSize":16,"lineHeight":24,"fill":{"tokenId":"brand"}})
        } else {
            json!({"name":"base","width":80,"height":40,"fill":{"styleId":"surface"}})
        };
        ops.push(node(
            format!("leaf-{i}"),
            if text {
                NodeKind::Text
            } else {
                NodeKind::Rectangle
            },
            Some(if i % 2 == 0 { "frame-a" } else { "frame-b" }),
            props,
        ));
        if ops.len() == 500 {
            rev = p
                .apply_batch(req(
                    p,
                    rev,
                    format!("seed-{batch}"),
                    std::mem::take(&mut ops),
                ))
                .unwrap()
                .committed_revision;
            batch += 1;
        }
    }
    if !ops.is_empty() {
        p.apply_batch(req(p, rev, format!("seed-{batch}"), ops))
            .unwrap();
    }
    assert_eq!(p.snapshot().unwrap().nodes.len(), size);
}

pub fn updates(marker: &str) -> Vec<Operation> {
    (0..10)
        .map(|i| Operation::UpdateNode {
            id: format!("leaf-{i}"),
            props: json!({"name":marker,"width":80+marker.len()}),
        })
        .collect()
}
pub fn assert_coherent(s: &Snapshot) {
    let marker = &s.nodes.iter().find(|n| n.id == "leaf-0").unwrap().props["name"];
    for i in 1..10 {
        assert_eq!(
            &s.nodes
                .iter()
                .find(|n| n.id == format!("leaf-{i}"))
                .unwrap()
                .props["name"],
            marker
        );
    }
}
