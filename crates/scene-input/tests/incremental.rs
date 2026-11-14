mod incremental_support;
use incremental_support::*;
use opencanvas_scene_input::{validate::Candidate, *};
use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn text_edit_work_stays_constant_across_unrelated_pages() {
    for copies in [1, 15, 30] {
        let before = fixture(copies, 100);
        let work = parity(&before, &[update("text0", json!({"text":"Changed"}))]).unwrap();
        assert!(!work.stats.full_fallback);
        assert_eq!(work.stats.affected_nodes, 1);
        assert_eq!(work.stats.affected_virtual_nodes, 0);
        assert!(work.stats.expanded_nodes > work.stats.authored_nodes);
        assert_eq!(work.affected_ids, BTreeSet::from(["text0".into()]));
    }
}

#[test]
fn changed_master_descendant_reaches_transitive_nested_instances() {
    let before = fixture(30, 20);
    let work = parity(&before, &[update("label", json!({"text":"Changed"}))]).unwrap();
    assert!(!work.stats.full_fallback);
    assert!(work.affected_ids.contains("nested"));
    assert_eq!(work.stats.affected_virtual_nodes, 61); // nested/label + 30*(nested,label)
    for i in 0..30 {
        assert!(work.affected_ids.contains(&format!("instance{i}")));
        assert!(!work.affected_ids.contains(&format!("text{i}")));
    }
    let error = parity(&before, &[update("master", json!({"width":"hug"}))])
        .err()
        .unwrap();
    assert_eq!(error.field.as_deref(), Some("width"));
}

#[test]
fn parent_layout_changes_validate_descendants_and_placed_overrides() {
    let before = fixture(3, 0);
    let error = parity(&before, &[update("group0", json!({"width":"hug"}))])
        .err()
        .unwrap();
    assert_eq!(error.field.as_deref(), Some("width"));
    let error = parity(
        &before,
        &[update(
            "instance0",
            json!({"overrides":{"nested":{"opacity":2}}}),
        )],
    )
    .err()
    .unwrap();
    assert_eq!(
        error.field.as_deref(),
        Some("props.overrides.nested.opacity")
    );
    parity(
        &before,
        &[update(
            "instance0",
            json!({"overrides":{"nested":{"opacity":0.5}}}),
        )],
    )
    .unwrap();
}

#[test]
fn delete_reparent_reorder_and_old_master_footprints_match_reference() {
    let before = fixture(3, 0);
    for op in [
        Operation::ReparentNode {
            id: "group0".into(),
            parent_id: "frame1".into(),
            index: None,
        },
        Operation::ReparentNode {
            id: "label".into(),
            parent_id: "frame1".into(),
            index: None,
        },
        Operation::ReorderNode {
            id: "text0".into(),
            index: 0,
        },
        Operation::DeleteSubtree { id: "label".into() },
        Operation::DeleteSubtree {
            id: "group0".into(),
        },
    ] {
        let _ = parity(&before, &[op]);
    }
    let work = parity(
        &before,
        &[Operation::ReparentNode {
            id: "group0".into(),
            parent_id: "frame1".into(),
            index: None,
        }],
    )
    .unwrap();
    assert!(work.affected_ids.contains("frame0"));
    assert!(work.affected_ids.contains("frame1"));
    assert!(work.affected_ids.contains("child0"));
}

#[test]
fn theme_resource_page_and_note_writes_explicitly_fall_back() {
    let before = fixture(2, 0);
    for op in [
        update("frame0", json!({"theme":"default"})),
        update("page0", json!({"name":"Renamed"})),
        Operation::UpsertToken {
            token: before.tokens[0].clone(),
        },
        Operation::UpsertStyle {
            style: before.styles[0].clone(),
        },
        Operation::CreateNote {
            id: "note".into(),
            node_id: "text0".into(),
            text: "Note".into(),
            author: None,
        },
    ] {
        assert!(parity(&before, &[op]).unwrap().stats.full_fallback);
    }
    let mut candidate = Candidate::from_snapshot(&before);
    candidate.fonts.get_mut("font").unwrap().family = "New".into();
    assert!(
        candidate
            .validate_after(&before, &[], &BTreeSet::new())
            .unwrap()
            .stats
            .full_fallback
    );
    let mut candidate = Candidate::from_snapshot(&before);
    candidate.assets.get_mut("image").unwrap().status = "missing".into();
    assert!(
        candidate
            .validate_after(&before, &[], &BTreeSet::new())
            .unwrap()
            .stats
            .full_fallback
    );
}

#[test]
fn refs_late_invalid_operations_and_failure_precedence_match() {
    let before = fixture(3, 20);
    for ops in [
        vec![update("text0", json!({"fontId":"missing"}))],
        vec![update("image0", json!({"assetId":"missing"}))],
        vec![update("text0", json!({"styleId":"missing"}))],
        vec![update("text0", json!({"fontSize":{"tokenId":"missing"}}))],
        vec![
            update("group0", json!({"width":"hug"})),
            update("group1", json!({"width":"hug"})),
        ],
        vec![
            update("text0", json!({"text":"Earlier valid"})),
            update("image0", json!({"assetId":"missing"})),
        ],
        vec![update(
            "text0",
            json!({"links":[{"on":"click","action":"navigate","to":"missing"}]}),
        )],
        vec![update(
            "master",
            json!({"links":[{"on":"click","action":"navigate","to":"frame0"}]}),
        )],
        vec![
            update("group0", json!({"width":"hug"})),
            update("nested", json!({"componentId":"outer"})),
        ],
    ] {
        assert!(parity(&before, &ops).is_err());
    }
}

#[test]
fn link_pruning_and_note_cascade_keep_valid_before_contract() {
    let mut before = fixture(2, 0);
    before
        .nodes
        .iter_mut()
        .find(|n| n.id == "text1")
        .unwrap()
        .props["links"] = json!([{"on":"click","action":"navigate","to":"frame0"}]);
    before.notes.push(Note {
        id: "note".into(),
        node_id: "child0".into(),
        text: "Note".into(),
        author: None,
        resolved: false,
        replies: vec![],
    });
    Candidate::from_snapshot(&before).validate().unwrap();
    let work = parity(
        &before,
        &[Operation::DeleteSubtree {
            id: "frame0".into(),
        }],
    )
    .unwrap();
    assert!(work.stats.full_fallback); // cascading note removal is a note write
    before.notes.clear();
    let work = parity(
        &before,
        &[Operation::DeleteSubtree {
            id: "frame0".into(),
        }],
    )
    .unwrap();
    assert!(work.affected_ids.contains("text1"));
}
