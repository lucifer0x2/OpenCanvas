//! Which frames a commit's prerender should warm. The viewer shows top-level frames, so a changed
//! node maps to the outermost frame that contains it (itself included), found through the index.
use super::index::TreeIndex;
use opencanvas_storage::{Node, NodeKind, Snapshot};
use std::collections::HashSet;

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
}
