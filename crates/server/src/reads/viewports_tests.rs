//! The trusted viewport scope covers exactly the full expansion's frame subtree.
use super::scope_ids;
use crate::reads::scoped_tree_fixture::fixture;

#[test]
fn scoped_viewport_ids_match_full_expansion_for_authored_frames() {
    let (_dir, storage, id) = fixture();
    let shared = storage.shared_snapshot(&id).unwrap();
    let scoped = shared
        .trusted_scoped()
        .unwrap()
        .expect("builtin committed source");
    for frame in [
        "frame",
        "master",
        "definitions",
        "nested-master",
        "incoming",
    ] {
        let full = scope_ids(&shared.snapshot, None, frame).unwrap();
        let actual = scope_ids(&shared.snapshot, Some(&scoped), frame).unwrap();
        assert_eq!(actual, full, "{frame}");
        assert!(full.contains(frame));
    }
    let placed = scope_ids(&shared.snapshot, Some(&scoped), "frame").unwrap();
    assert!(placed.contains("placed/nested/copy"));
}
