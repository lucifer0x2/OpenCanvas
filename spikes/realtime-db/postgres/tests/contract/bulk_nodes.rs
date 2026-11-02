use super::support::*;
use crate::{batch, db};
use oc_spike_core::apply;
use serde_json::{json, Value};

fn child(id: &str, parent: &str) -> Value {
    json!({"op":"create_node","id":id,"parentId":parent,"kind":"group","props":{}})
}

#[tokio::test]
async fn bulk_mixed_create_update_delete_and_reparent_matches_complete_candidate() {
    let (url, id) = project("bulk_mixed").await;
    let initial = vec![
        page("root"),
        child("old", "root"),
        child("child", "old"),
        child("leaf", "child"),
        child("doomed", "root"),
        child("trash", "doomed"),
        child("kept", "root"),
    ];
    batch::execute(&url, &request(&id, "seed", 0, initial))
        .await
        .unwrap();
    let before = db::snapshot(&url, &id).await.unwrap();
    let command = request(
        &id,
        "mixed",
        1,
        vec![
            child("new_child", "new_parent"), // Parent is created later in the batch.
            child("new_parent", "root"),
            update("kept", 42),
            json!({"op":"reparent_node","id":"child","parentId":"new_parent","sortKey":i64::MAX}),
            json!({"op":"delete_subtree","id":"old"}),
            json!({"op":"delete_subtree","id":"doomed"}),
            json!({"op":"reparent_node","id":"new_parent","parentId":"kept","sortKey":i64::MIN}),
            json!({"op":"update_node","id":"new_child","props":{"name":"Việt \"quoted\" 🦀"}}),
        ],
    );
    let (expected, changed) = apply(&before, &command).unwrap();
    let mut dry = command.clone();
    dry.dry_run = true;
    let preview = batch::execute(&url, &dry).await.unwrap();
    assert_eq!(preview["changedIds"], json!(changed));
    assert_eq!(db::snapshot(&url, &id).await.unwrap(), before);
    assert_eq!(counts(&url, &id).await, [1, 7, 1, 1]);
    let result = batch::execute(&url, &command).await.unwrap();
    assert_eq!(result["changedIds"], json!(changed));
    assert_eq!(db::snapshot(&url, &id).await.unwrap(), expected);
    assert_eq!(counts(&url, &id).await, [2, 6, 2, 2]);
    clean(&url, &id).await;
}

#[tokio::test]
async fn same_id_recreated_or_created_then_deleted_uses_only_final_candidate_rows() {
    let (url, id) = project("bulk_final").await;
    seed(&url, &id).await;
    let command = request(
        &id,
        "replace",
        1,
        vec![
            json!({"op":"delete_subtree","id":"root"}),
            page("root"),
            update("root", 7),
            child("ephemeral", "root"),
            json!({"op":"delete_subtree","id":"ephemeral"}),
        ],
    );
    let before = db::snapshot(&url, &id).await.unwrap();
    let (expected, changed) = apply(&before, &command).unwrap();
    let result = batch::execute(&url, &command).await.unwrap();
    assert_eq!(changed, ["ephemeral", "root"]);
    assert_eq!(result["changedIds"], json!(changed));
    assert_eq!(db::snapshot(&url, &id).await.unwrap(), expected);
    let later = request(&id, "later", 2, vec![update("paired", 8)]);
    batch::execute(&url, &later).await.unwrap();
    let mut replay = batch::execute(&url, &command).await.unwrap();
    assert_eq!(replay["replayed"], true);
    replay["replayed"] = json!(false);
    assert_eq!(replay, result);
    let mut collision = command;
    collision.operations = vec![update("root", 9)];
    assert_eq!(
        batch::execute(&url, &collision).await.unwrap_err().code,
        "IDEMPOTENCY_KEY_REUSED"
    );
    assert_eq!(counts(&url, &id).await, [3, 2, 3, 3]);
    clean(&url, &id).await;
}

#[tokio::test]
async fn five_hundred_operation_create_update_delete_batches_are_atomic() {
    let (url, id) = project("bulk_500").await;
    for revision in 0..3 {
        let operations = (0..500)
            .map(|n| {
                let node = format!("page_{n:03}");
                match revision {
                    0 => page(&node),
                    1 => update(&node, n),
                    _ => json!({"op":"delete_subtree","id":node}),
                }
            })
            .collect();
        let before = db::snapshot(&url, &id).await.unwrap();
        let command = request(&id, &format!("bulk_{revision}"), revision, operations);
        let (expected, changed) = apply(&before, &command).unwrap();
        let result = batch::execute(&url, &command).await.unwrap();
        assert_eq!(changed.len(), 500);
        assert_eq!(result["changedIds"], json!(changed));
        assert_eq!(db::snapshot(&url, &id).await.unwrap(), expected);
    }
    assert_eq!(counts(&url, &id).await, [3, 0, 3, 3]);
    clean(&url, &id).await;
}

#[tokio::test]
async fn one_subtree_delete_can_touch_more_than_the_operation_limit() {
    let (url, id) = project("bulk_subtree").await;
    let mut initial = vec![page("root")];
    initial.extend((0..499).map(|n| child(&format!("child_{n}"), "root")));
    batch::execute(&url, &request(&id, "seed", 0, initial))
        .await
        .unwrap();
    let more = (499..999)
        .map(|n| child(&format!("child_{n}"), "root"))
        .collect();
    batch::execute(&url, &request(&id, "more", 1, more))
        .await
        .unwrap();
    let deleted = batch::execute(
        &url,
        &request(
            &id,
            "subtree",
            2,
            vec![json!({"op":"delete_subtree","id":"root"})],
        ),
    )
    .await
    .unwrap();
    assert_eq!(deleted["changedIds"].as_array().unwrap().len(), 1000);
    assert_eq!(counts(&url, &id).await, [3, 0, 3, 3]);
    clean(&url, &id).await;
}
