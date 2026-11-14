//! Which frames a commit's prerender should warm. The viewer shows top-level frames, so a changed
//! node maps to the outermost frame that contains it (itself included), found through the index.
use super::index::TreeIndex;
use opencanvas_storage::{read_scope::ScopedSnapshot, Node, NodeKind, SharedSnapshot, Snapshot};
use std::collections::HashSet;

pub(crate) fn prerender_shared<'a>(
    shared: &'a SharedSnapshot,
    changed_ids: &[&str],
    max: usize,
) -> crate::error::Result<Vec<&'a Node>> {
    let Some(scoped) = shared.trusted_scoped()? else {
        return Ok(prerender_frames(&shared.snapshot, changed_ids, max));
    };
    let touched: HashSet<&str> = changed_ids
        .iter()
        .filter_map(|id| outermost_frame(&scoped, id))
        .collect();
    Ok(scoped
        .frames()
        .filter(|frame| touched.is_empty() || touched.contains(frame.id.as_str()))
        .take(max)
        .collect())
}

/// The outermost frame on `id`'s parent chain, itself included. Storage rejects cycles, but rows
/// written behind the validator may hold one: the walk stops after one step per snapshot node,
/// like `TreeIndex::ancestors`, keeping the frames seen so far (best-effort warm-up selection).
fn outermost_frame<'a>(scoped: &ScopedSnapshot<'a>, id: &str) -> Option<&'a str> {
    let mut node = scoped.node(id);
    let mut frame = None;
    for _ in 0..=scoped.snapshot().nodes.len() {
        let Some(current) = node else { break };
        if current.kind == NodeKind::Frame {
            frame = Some(current.id.as_str());
        }
        node = current.parent_id.as_deref().and_then(|id| scoped.node(id));
    }
    frame
}

/// Frames to prerender after a commit, in storage order, at most `max`. Frames containing a
/// changed node are chosen; when no changed ID resolves to a frame (token/style-only
/// changes, deletions, page edits) it is the first `max` frames, as before.
pub(crate) fn prerender_frames<'a>(
    snapshot: &'a Snapshot,
    changed_ids: &[&str],
    max: usize,
) -> Vec<&'a Node> {
    let index = TreeIndex::new(&snapshot.nodes);
    let touched: HashSet<&str> = changed_ids
        .iter()
        .filter_map(|id| index.get(id))
        .filter_map(|node| {
            index
                .ancestors(&node.id, None)
                .into_iter()
                .chain([node])
                .find(|n| n.kind == NodeKind::Frame)
        })
        .map(|frame| frame.id.as_str())
        .collect();
    snapshot
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Frame)
        .filter(|n| touched.is_empty() || touched.contains(n.id.as_str()))
        .take(max)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencanvas_storage::Retention;

    fn node(id: &str, kind: NodeKind, parent: Option<&str>, sort_key: i64) -> Node {
        let (id, parent_id, props) = (id.into(), parent.map(Into::into), serde_json::json!({}));
        Node {
            id,
            parent_id,
            sort_key,
            kind,
            props,
        }
    }

    /// One page with six frames (`f1`…`f6`), each holding a group with a text; `f6` also holds a
    /// nested frame `inner`.
    fn six_frames() -> Snapshot {
        let mut nodes = vec![node("page", NodeKind::Page, None, 0)];
        for i in 1..=6 {
            let f = format!("f{i}");
            nodes.push(node(&f, NodeKind::Frame, Some("page"), i));
            nodes.push(node(&format!("{f}-g"), NodeKind::Group, Some(&f), 0));
            nodes.push(node(
                &format!("{f}-t"),
                NodeKind::Text,
                Some(&format!("{f}-g")),
                0,
            ));
        }
        nodes.push(node("inner", NodeKind::Frame, Some("f6"), 1));
        nodes.push(node("inner-t", NodeKind::Text, Some("inner"), 0));
        let retention = Retention {
            oldest_commit_revision: 1,
            idempotency_policy: String::new(),
        };
        let (project_id, slug, name) = ("p".into(), "p".into(), "p".into());
        let (tokens, styles, assets, fonts) = (vec![], vec![], vec![], vec![]);
        Snapshot {
            schema_version: 1,
            project_id,
            slug,
            name,
            revision: 2,
            nodes,
            tokens,
            styles,
            assets,
            fonts,
            notes: vec![],
            retention,
        }
    }

    fn ids(frames: Vec<&Node>) -> Vec<&str> {
        frames.into_iter().map(|n| n.id.as_str()).collect()
    }

    #[test]
    fn targets_the_outermost_frame_of_changed_nodes() {
        let s = six_frames();
        assert_eq!(ids(prerender_frames(&s, &["f6-t"], 4)), ["f6"]);
        assert_eq!(ids(prerender_frames(&s, &["inner-t"], 4)), ["f6"]);
        assert_eq!(ids(prerender_frames(&s, &["f6", "f2-g"], 4)), ["f2", "f6"]);
        let all = ["f6-t", "f5-t", "f4-t", "f3-t", "f2-t", "f1-t"];
        assert_eq!(ids(prerender_frames(&s, &all, 4)), ["f1", "f2", "f3", "f4"]);
        // Token IDs mixed with a node change: the node decides.
        assert_eq!(ids(prerender_frames(&s, &["tok", "f5-t"], 4)), ["f5"]);
    }

    #[test]
    fn unresolvable_changes_keep_the_first_frames() {
        let s = six_frames();
        let first = ["f1", "f2", "f3", "f4"];
        assert_eq!(ids(prerender_frames(&s, &["token-1", "style-1"], 4)), first);
        assert_eq!(ids(prerender_frames(&s, &["deleted-node"], 4)), first);
        assert_eq!(ids(prerender_frames(&s, &["page"], 4)), first);
        assert_eq!(ids(prerender_frames(&s, &[], 4)), first);
    }

    #[test]
    fn indexed_prerender_selection_retains_source_order_and_legacy_fallback() {
        let root = tempfile::tempdir().unwrap();
        let store: opencanvas_storage::Storage =
            opencanvas_storage::Store::open(root.path()).unwrap().into();
        let project = store.create_project("targets", "Targets").unwrap();
        let ops: Vec<_> = six_frames().nodes.into_iter().map(|mut node| {
            if node.kind == NodeKind::Frame { node.props = serde_json::json!({"width":10,"height":10}); }
            if node.kind == NodeKind::Text { node.props = serde_json::json!({"text":"copy","fontId":"font_bundled","fontSize":10,"lineHeight":12}); }
            serde_json::json!({"op":"create_node","id":node.id,"parentId":node.parent_id,"kind":node.kind,"props":node.props})
        }).collect();
        project.apply_batch(serde_json::from_value(serde_json::json!({"projectId":project.id(),"expectedRevision":0,"idempotencyKey":"seed","operations":ops})).unwrap()).unwrap();
        let shared = store.shared_snapshot(project.id()).unwrap();
        for changed in [
            vec!["inner-t"],
            vec!["f6", "f2-g"],
            vec!["token"],
            vec!["deleted"],
            vec!["page"],
            vec![],
        ] {
            for max in [0, 1, 4, 10] {
                assert_eq!(
                    ids(prerender_shared(&shared, &changed, max).unwrap()),
                    ids(prerender_frames(&shared.snapshot, &changed, max))
                );
            }
        }
    }
}
