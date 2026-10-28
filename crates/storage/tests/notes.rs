//! P02a notes: write operations, replay/conflict and validation.
mod common;
mod notes_support;
use common::*;
use notes_support::*;
use opencanvas_storage::*;
use serde_json::json;

#[test]
fn notes_are_created_replied_resolved_reopened_and_deleted() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let r = p
        .apply_batch(request(
            &p,
            1,
            "n1",
            vec![note("note-cta", "rect", "Sticky CTA")],
        ))
        .unwrap();
    assert_eq!(
        (r.committed_revision, r.changed_ids.clone()),
        (2, vec!["note-cta".into()])
    );
    assert_eq!(
        notes_json(&p),
        json!([{"id":"note-cta","nodeId":"rect","text":"Sticky CTA","author":"designer"}])
    );
    let ops = vec![reply("note-cta", "Tablet too?"), reply("note-cta", "Yes")];
    p.apply_batch(request(&p, 2, "n2", ops)).unwrap();
    p.apply_batch(request(&p, 3, "n3", vec![update("note-cta", Some(true))]))
        .unwrap();
    assert_eq!(
        notes_json(&p),
        json!([{"id":"note-cta","nodeId":"rect","text":"Sticky CTA","author":"designer","resolved":true,
            "replies":[{"text":"Tablet too?","author":"coder"},{"text":"Yes","author":"coder"}]}])
    );
    p.apply_batch(request(&p, 4, "n4", vec![update("note-cta", Some(false))]))
        .unwrap();
    assert!(notes_json(&p)[0].get("resolved").is_none());
    let re_anchor = Operation::UpdateNote {
        id: "note-cta".into(),
        text: Some("Edited".into()),
        resolved: None,
        node_id: Some("text".into()),
    };
    p.apply_batch(request(&p, 5, "n5", vec![re_anchor]))
        .unwrap();
    assert_eq!(notes_json(&p)[0]["nodeId"], "text");
    assert_eq!(notes_json(&p)[0]["text"], "Edited");
    let r = p
        .apply_batch(request(
            &p,
            6,
            "n6",
            vec![Operation::DeleteNote {
                id: "note-cta".into(),
            }],
        ))
        .unwrap();
    assert_eq!(r.changed_ids, ["note-cta"]);
    assert!(p.snapshot().unwrap().notes.is_empty());
}

#[test]
fn note_batches_replay_and_conflict_like_any_batch() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let req = request(&p, 1, "note", vec![note("n", "frame", "Intent")]);
    assert!(!p.apply_batch(req.clone()).unwrap().replayed);
    let replay = p.apply_batch(req).unwrap();
    assert!(replay.replayed);
    assert_eq!(p.snapshot().unwrap().revision, 2);
    let other = request(&p, 1, "note", vec![note("n", "frame", "Other")]);
    assert_eq!(
        p.apply_batch(other).unwrap_err().code,
        "IDEMPOTENCY_KEY_REUSED"
    );
    let stale = request(&p, 1, "stale", vec![reply("n", "late")]);
    assert_eq!(p.apply_batch(stale).unwrap_err().code, "REVISION_CONFLICT");
    assert_eq!(p.commits_after(1, 10).unwrap()[0].changed_ids, ["n"]);
}

#[test]
fn invalid_note_operations_name_field_and_index_and_write_nothing() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let mut full = vec![note("full", "frame", "Thread")];
    full.extend((0..100).map(|i| reply("full", &format!("r{i}"))));
    full.push(note("taken", "frame", "Taken"));
    p.apply_batch(request(&p, 1, "seed", full)).unwrap();
    let empty = Operation::UpdateNote {
        id: "full".into(),
        text: None,
        resolved: None,
        node_id: None,
    };
    let cases = [
        (note("x", "missing", "t"), "nodeId"),
        (note("taken", "frame", "t"), "id"),
        (note("rect", "frame", "t"), "id"),
        (
            create("taken", NodeKind::Group, Some("frame"), json!({})),
            "id",
        ),
        (note("x", "frame", ""), "text"),
        (note("x", "frame", &"a".repeat(2001)), "text"),
        (reply("full", "one more"), "replies"),
        (empty, "update_note"),
        (reply("missing", "t"), "id"),
        (
            Operation::DeleteNote {
                id: "missing".into(),
            },
            "id",
        ),
    ];
    for (i, (op, field)) in cases.into_iter().enumerate() {
        let ops = vec![note(&format!("ok{i}"), "frame", "valid first"), op];
        let e = p
            .apply_batch(request(&p, 2, &format!("bad{i}"), ops))
            .unwrap_err();
        assert_eq!(
            (e.code.as_str(), e.field.as_deref(), e.op_index),
            ("VALIDATION_FAILED", Some(field), Some(1)),
            "case {i}"
        );
    }
    let s = p.snapshot().unwrap();
    assert_eq!((s.revision, s.notes.len()), (2, 2));
    assert_eq!(s.notes[0].replies.len(), 100);
    let ok = request(&p, 2, "max", vec![note("long", "frame", &"á".repeat(2000))]);
    assert_eq!(p.apply_batch(ok).unwrap().committed_revision, 3);
}
