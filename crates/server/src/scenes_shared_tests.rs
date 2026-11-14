//! Indexed page-frame selection matches the full source scan, errors included.
use super::ScenesQuery;
use opencanvas_storage::BatchRequest;
use serde_json::json;

#[allow(dead_code)]
#[path = "reads/scoped_tree_fixture.rs"]
mod fixture;

#[test]
fn shared_frames_match_full_frames_order_offsets_and_errors() {
    let (_root, storage, id) = fixture::fixture();
    let project = storage.open_project(&id).unwrap();
    let mut ops: Vec<_> = (0..3)
        .map(|i| {
            fixture::node(
                &format!("extra{i}"),
                "frame",
                Some("page"),
                json!({"width":10,"height":10}),
            )
        })
        .collect();
    ops.push(json!({"op":"reorder_node","id":"extra2","index":0}));
    ops.push(fixture::node(
        "nested-frame",
        "frame",
        Some("frame"),
        json!({"width":5,"height":5}),
    ));
    let request =
        json!({"projectId":id,"expectedRevision":1,"idempotencyKey":"frames","operations":ops});
    project
        .apply_batch(serde_json::from_value::<BatchRequest>(request).unwrap())
        .unwrap();
    let shared = storage.shared_snapshot(&id).unwrap();
    assert!(shared.trusted_scoped().unwrap().is_some());
    let mut compared = 0;
    for page in ["page", "library", "frame", "missing"] {
        for revision in [1, 2] {
            for offset in [0, 2, 4, 5] {
                let query: ScenesQuery = serde_json::from_value(
                    json!({"pageId":page,"revision":revision,"offset":offset}),
                )
                .unwrap();
                let ids = |frames: Vec<&opencanvas_storage::Node>| {
                    frames.into_iter().map(|n| n.id.clone()).collect::<Vec<_>>()
                };
                let full = query
                    .frames(&shared.snapshot)
                    .map(ids)
                    .map_err(|e| e.value());
                let indexed = query.shared_frames(&shared).map(ids).map_err(|e| e.value());
                assert_eq!(indexed, full, "{page} r{revision} +{offset}");
                compared += usize::from(full.as_ref().is_ok_and(|f| f.len() > 1));
            }
        }
    }
    assert!(compared >= 3);
}
