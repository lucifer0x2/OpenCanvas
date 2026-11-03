use super::support::*;
use crate::{batch, db};
use serde_json::json;
use std::time::Duration;

async fn waiter(observer: &db::Lease, blocker: i32) -> i32 {
    tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            let rows = observer
                .query(
                    "SELECT pid FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid))",
                    &[&blocker],
                )
                .await
                .unwrap();
            if let Some(row) = rows.first() {
                break row.get(0);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("request must demonstrably wait on the other transaction's lock")
}

#[tokio::test]
async fn concurrent_same_key_replays_after_waiting_for_the_committing_writer() {
    let (url, id) = project("bulk_wait_replay").await;
    seed(&url, &id).await;
    let mut blocker = db::connect(&url).await.unwrap();
    let blocker_pid: i32 = blocker
        .query_one("SELECT pg_backend_pid()", &[])
        .await
        .unwrap()
        .get(0);
    // Hold the first real batch inside its replay INSERT, after it owns the
    // project lock. This creates a deterministic READ COMMITTED snapshot trap.
    let trigger = format!("pause_{id}");
    blocker.batch_execute(&format!("CREATE FUNCTION pg_temp.{trigger}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(hashtextextended(NEW.project_id,0)); RETURN NEW; END $$; CREATE TRIGGER {trigger} BEFORE INSERT ON idempotency FOR EACH ROW WHEN (NEW.project_id='{id}') EXECUTE FUNCTION pg_temp.{trigger}();")).await.unwrap();
    let held = blocker.transaction().await.unwrap();
    held.query_one(
        "SELECT pg_advisory_xact_lock(hashtextextended($1,0))",
        &[&id],
    )
    .await
    .unwrap();
    let command = request(&id, "same_key", 1, vec![update("root", 22)]);
    let first_url = url.clone();
    let first_command = command.clone();
    let first = tokio::spawn(async move { batch::execute(&first_url, &first_command).await });
    let observer = db::connect(&url).await.unwrap();
    let writer_pid = waiter(&observer, blocker_pid).await;
    let second_url = url.clone();
    let second_command = command.clone();
    let second = tokio::spawn(async move { batch::execute(&second_url, &second_command).await });
    let replay_pid = waiter(&observer, writer_pid).await;
    assert_ne!(replay_pid, writer_pid);
    assert!(!first.is_finished() && !second.is_finished());
    held.commit().await.unwrap();
    let original = first.await.unwrap().unwrap();
    let mut replay = second.await.unwrap().unwrap();
    assert_eq!(replay["replayed"], true);
    replay["replayed"] = json!(false);
    assert_eq!(replay, original);
    blocker
        .batch_execute(&format!(
            "DROP TRIGGER {trigger} ON idempotency; DROP FUNCTION pg_temp.{trigger}();"
        ))
        .await
        .unwrap();
    assert_eq!(counts(&url, &id).await, [2, 2, 2, 2]);
    batch::execute(&url, &request(&id, "later", 2, vec![update("paired", 23)]))
        .await
        .unwrap();
    assert_eq!(
        batch::execute(&url, &command).await.unwrap()["committedRevision"],
        2
    );
    let mut collision = command;
    collision.operations = vec![update("root", 24)];
    assert_eq!(
        batch::execute(&url, &collision).await.unwrap_err().code,
        "IDEMPOTENCY_KEY_REUSED"
    );
    clean(&url, &id).await;
}
