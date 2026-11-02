use super::support::*;
use crate::{batch, db, events};

#[tokio::test]
async fn snapshot_keeps_revision_and_paired_entities_from_one_database_snapshot() {
    let (url, id) = project("snapshot").await;
    seed(&url, &id).await;
    let snapshot = db::snapshot_interleaved(&url, &id, || async {
        batch::execute(
            &url,
            &request(
                &id,
                "between_selects",
                1,
                vec![update("root", 2), update("paired", 2)],
            ),
        )
        .await
        .unwrap();
    })
    .await
    .unwrap();
    assert_eq!(snapshot.revision, 1);
    assert!(snapshot
        .nodes
        .iter()
        .all(|node| node.props.get("x").is_none()));
    let after = db::snapshot(&url, &id).await.unwrap();
    assert_eq!(after.revision, 2);
    assert_eq!(after.nodes.len(), 2);
    assert!(after
        .nodes
        .iter()
        .all(|node| node.props["x"] == 2 && node.props["y"] == 2));
    clean(&url, &id).await;
}

#[tokio::test]
async fn concurrent_snapshots_match_gapless_revision_and_both_entity_values() {
    let (url, id) = project("paired").await;
    seed(&url, &id).await;
    let writer_url = url.clone();
    let writer_id = id.clone();
    let writer = tokio::spawn(async move {
        for revision in 2..=31 {
            batch::execute(
                &writer_url,
                &request(
                    &writer_id,
                    &format!("pair_{revision}"),
                    revision - 1,
                    vec![update("root", revision), update("paired", revision)],
                ),
            )
            .await
            .unwrap();
            tokio::task::yield_now().await;
        }
    });
    for _ in 0..45 {
        let snapshot = db::snapshot(&url, &id).await.unwrap();
        assert_eq!(snapshot.nodes.len(), 2);
        for node in &snapshot.nodes {
            if snapshot.revision == 1 {
                assert!(node.props.get("x").is_none());
            } else {
                assert_eq!(node.props["x"], snapshot.revision);
                assert_eq!(node.props["y"], snapshot.revision);
            }
        }
        let changes = events::changes(&url, &id, 0).await.unwrap();
        let current = changes["currentRevision"].as_u64().unwrap();
        let revisions: Vec<_> = changes["commits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["revision"].as_u64().unwrap())
            .collect();
        assert_eq!(revisions, (1..=current).collect::<Vec<_>>());
    }
    writer.await.unwrap();
    assert_eq!(counts(&url, &id).await, [31, 2, 31, 31]);
    clean(&url, &id).await;
}

#[tokio::test]
async fn dry_run_and_validation_failure_reserve_no_key_or_durable_rows() {
    let (url, id) = project("dryrun").await;
    seed(&url, &id).await;
    let before = db::snapshot(&url, &id).await.unwrap();
    let mut dry = request(&id, "same_key", 1, vec![update("root", 99)]);
    dry.dry_run = true;
    let preview = batch::execute(&url, &dry).await.unwrap();
    assert_eq!(preview["dryRun"], true);
    assert_eq!(preview["baseRevision"], 1);
    assert_eq!(db::snapshot(&url, &id).await.unwrap(), before);
    assert_eq!(counts(&url, &id).await, [1, 2, 1, 1]);
    assert_eq!(
        events::changes(&url, &id, 1).await.unwrap()["commits"],
        serde_json::json!([])
    );
    // A different body with the preview key is valid: preview did not reserve it.
    dry.dry_run = false;
    dry.operations = vec![update("paired", 2)];
    assert_eq!(
        batch::execute(&url, &dry).await.unwrap()["committedRevision"],
        2
    );
    clean(&url, &id).await;
}

#[tokio::test]
async fn future_cursor_and_missing_commit_are_explicit_errors() {
    let (url, id) = project("cursor").await;
    seed(&url, &id).await;
    let invalid = events::changes(&url, &id, u64::MAX).await.unwrap_err();
    assert_eq!(invalid.code, "INVALID_CURSOR");
    assert_eq!(invalid.current_revision, Some(1));
    let client = db::connect(&url).await.unwrap();
    client
        .execute("DELETE FROM commits WHERE project_id=$1", &[&id])
        .await
        .unwrap();
    let gap = events::changes(&url, &id, 0).await.unwrap_err();
    assert_eq!(gap.code, "REPLAY_GAP");
    assert_eq!(gap.current_revision, Some(1));
    clean(&url, &id).await;
}
