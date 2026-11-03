use super::support::*;
use crate::{batch, db};
use serde_json::json;
use std::time::Duration;
use tokio_postgres::{AsyncMessage, NoTls};

#[tokio::test]
async fn deferred_foreign_key_failure_rolls_back_every_row_and_queued_notification() {
    let (url, id) = project("bulk_commit_fail").await;
    seed(&url, &id).await;
    let before = db::snapshot(&url, &id).await.unwrap();
    let (listener, mut connection) = tokio_postgres::connect(&url, NoTls).await.unwrap();
    let (notifications, mut received) = tokio::sync::mpsc::unbounded_channel();
    let observed_id = id.clone();
    let driver = tokio::spawn(async move {
        while let Some(message) = std::future::poll_fn(|cx| connection.poll_message(cx)).await {
            if let Ok(AsyncMessage::Notification(value)) = message {
                if value.payload() == observed_id {
                    let _ = notifications.send(());
                }
            }
        }
    });
    listener
        .batch_execute("LISTEN oc_spike_changes")
        .await
        .unwrap();
    let client = db::connect(&url).await.unwrap();
    let trigger = format!("corrupt_{id}");
    // The candidate passed domain validation. Corrupt only the SQL row, so the
    // DEFERRABLE constraint fails at COMMIT, after all CTEs and NOTIFY ran.
    client.batch_execute(&format!("CREATE FUNCTION pg_temp.{trigger}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.parent_id='missing'; RETURN NEW; END $$; CREATE TRIGGER {trigger} BEFORE INSERT OR UPDATE ON nodes FOR EACH ROW WHEN (NEW.project_id='{id}' AND NEW.node_id='child') EXECUTE FUNCTION pg_temp.{trigger}();")).await.unwrap();
    let command = request(
        &id,
        "retryable",
        1,
        vec![
            update("root", 55),
            json!({"op":"delete_subtree","id":"paired"}),
            json!({"op":"create_node","id":"child","parentId":"root","kind":"group","props":{}}),
        ],
    );
    assert_eq!(
        batch::execute(&url, &command).await.unwrap_err().code,
        "INTERNAL_ERROR"
    );
    assert_eq!(db::snapshot(&url, &id).await.unwrap(), before);
    assert_eq!(counts(&url, &id).await, [1, 2, 1, 1]);
    assert!(
        tokio::time::timeout(Duration::from_millis(250), received.recv())
            .await
            .is_err(),
        "failed COMMIT must discard its queued PostgreSQL notification"
    );
    client
        .batch_execute(&format!(
            "DROP TRIGGER {trigger} ON nodes; DROP FUNCTION pg_temp.{trigger}();"
        ))
        .await
        .unwrap();
    assert_eq!(
        batch::execute(&url, &command).await.unwrap()["committedRevision"],
        2
    );
    tokio::time::timeout(Duration::from_secs(2), received.recv())
        .await
        .expect("successful durable retry must publish")
        .unwrap();
    assert_eq!(counts(&url, &id).await, [2, 2, 2, 2]);
    driver.abort();
    let _ = driver.await;
    clean(&url, &id).await;
}
