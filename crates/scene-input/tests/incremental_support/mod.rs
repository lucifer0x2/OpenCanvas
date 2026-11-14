use opencanvas_scene_input::{
    validate::{Candidate, ValidationWork},
    *,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub fn node(id: &str, kind: NodeKind, parent: Option<&str>, props: Value) -> Node {
    Node {
        id: id.into(),
        kind,
        parent_id: parent.map(str::to_owned),
        sort_key: 0,
        props,
    }
}

pub fn fixture(copies: usize, extra: usize) -> Snapshot {
    let mut snapshot: Snapshot = serde_json::from_value(json!({
        "schemaVersion":4,"projectId":"project","slug":"fixture","name":"Fixture","revision":1,
        "retention":{"oldestCommitRevision":0,"idempotencyPolicy":"lifetime"},"notes":[],
        "tokens":[{"id":"size","name":"Size","type":"dimension",
            "value":{"default":20,"themes":{"night":30}}}],
        "styles":[{"id":"copy","name":"Copy","kind":"text","value":{
            "fontId":"font","fontSize":{"tokenId":"size"},"lineHeight":40}}],
        "fonts":[{"id":"font","family":"Fixture","weight":400,"style":"normal","status":"ready"}],
        "assets":[{"id":"image","contentHash":"abc","mediaType":"image/png","byteSize":1,
            "relativeKey":"opaque","status":"ready"}],"nodes":[]
    }))
    .unwrap();
    snapshot.nodes.extend([
        node("library", NodeKind::Page, None, json!({})),
        node(
            "master",
            NodeKind::Frame,
            Some("library"),
            json!({"component":{"name":"Inner"},"width":100,"height":100}),
        ),
        node(
            "label",
            NodeKind::Text,
            Some("master"),
            json!({"text":"Master","styleId":"copy","width":"fill"}),
        ),
        node(
            "outer",
            NodeKind::Frame,
            Some("library"),
            json!({"component":{"name":"Outer"},"width":200,"height":200}),
        ),
        node(
            "nested",
            NodeKind::Instance,
            Some("outer"),
            json!({"componentId":"master","width":"fill","overrides":{"label":{"text":"Nested"}}}),
        ),
    ]);
    for i in 0..copies {
        let page = format!("page{i}");
        let frame = format!("frame{i}");
        snapshot.nodes.extend([
            node(&page, NodeKind::Page, None, json!({})),
            node(
                &frame,
                NodeKind::Frame,
                Some(&page),
                json!({"width":800,"height":600,"theme":"night"}),
            ),
            node(
                &format!("instance{i}"),
                NodeKind::Instance,
                Some(&frame),
                json!({"componentId":"outer","width":"fill"}),
            ),
            node(
                &format!("text{i}"),
                NodeKind::Text,
                Some(&frame),
                json!({"text":"Direct","styleId":"copy"}),
            ),
            node(
                &format!("group{i}"),
                NodeKind::Group,
                Some(&frame),
                json!({"width":100,"height":100}),
            ),
            node(
                &format!("child{i}"),
                NodeKind::Rectangle,
                Some(&format!("group{i}")),
                json!({"width":"fill"}),
            ),
            node(
                &format!("image{i}"),
                NodeKind::Image,
                Some(&frame),
                json!({"assetId":"image"}),
            ),
        ]);
        for j in 0..extra {
            snapshot.nodes.push(node(
                &format!("extra{i}_{j}"),
                NodeKind::Rectangle,
                Some(&frame),
                json!({"width":20,"height":20}),
            ));
        }
    }
    Candidate::from_snapshot(&snapshot).validate().unwrap();
    snapshot
}

pub fn update(id: &str, props: Value) -> Operation {
    Operation::UpdateNode {
        merge: false,
        id: id.into(),
        props,
    }
}

/// Both modes start from an explicitly full-valid before and apply independently.
/// Compare the entire serialized error, including opIndex and node details.
pub fn parity(before: &Snapshot, ops: &[Operation]) -> Result<ValidationWork> {
    fn apply(before: &Snapshot, ops: &[Operation]) -> Result<(Candidate, BTreeSet<String>)> {
        let mut candidate = Candidate::from_snapshot(before);
        let mut changed = BTreeSet::new();
        for (index, op) in ops.iter().enumerate() {
            candidate.apply(op, &mut changed).map_err(|e| e.at(index))?;
        }
        candidate.prune_links(&mut changed);
        Ok((candidate, changed))
    }
    let reference = apply(before, ops).and_then(|(c, _)| c.validate());
    let incremental =
        apply(before, ops).and_then(|(c, changed)| c.validate_after(before, ops, &changed));
    assert_eq!(
        incremental
            .as_ref()
            .map(|_| ())
            .map_err(|e| serde_json::to_value(e).unwrap()),
        reference.map_err(|e| serde_json::to_value(e).unwrap())
    );
    incremental
}
