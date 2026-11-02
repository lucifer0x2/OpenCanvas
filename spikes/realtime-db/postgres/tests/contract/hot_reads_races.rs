#[cfg(feature = "compat-v2")]
use super::hot_reads_contract::redis;
use super::{
    hot_reads_contract::{cache, read},
    support::*,
};
use crate::{batch, db, hot_reads, hot_snapshot};
use serde_json::Value;

#[tokio::test]
async fn commit_between_probe_and_fill_caches_the_actual_snapshot_revision() {
    let (url, id) = project("cache_probe_race").await;
    seed(&url, &id).await;
    let cache = cache(&url);
    let old_key = hot_snapshot::probe(&url, &id)
        .await
        .unwrap()
        .key(cache.namespace(), &id);
    let bytes = hot_reads::snapshot_interleaved(&url, &id, Some(&cache), || async {
        batch::execute(
            &url,
            &request(
                &id,
                "between_probe",
                1,
                vec![update("root", 2), update("paired", 2)],
            ),
        )
        .await
        .unwrap();
    })
    .await
    .unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["revision"], 2);
    assert!(value["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|n| n["props"]["x"] == 2));
    assert!(
        cache.get(&old_key).await.is_none(),
        "never label new rows with the old revision"
    );
    let actual = hot_snapshot::probe(&url, &id)
        .await
        .unwrap()
        .key(cache.namespace(), &id);
    assert_eq!(cache.get(&actual).await.unwrap(), bytes);
    clean(&url, &id).await;
}

#[tokio::test]
async fn cold_snapshot_keeps_metadata_and_nodes_consistent_during_commit() {
    let (url, id) = project("cache_rr").await;
    seed(&url, &id).await;
    let cache = cache(&url);
    let (identity, snapshot) = hot_snapshot::load_interleaved(&url, &id, || async {
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
        #[cfg(feature = "compat-v2")]
        db::connect(&url)
            .await
            .unwrap()
            .execute(
                "UPDATE projects SET description='after metadata read' WHERE project_id=$1",
                &[&id],
            )
            .await
            .unwrap();
    })
    .await
    .unwrap();
    assert_eq!(snapshot["revision"], 1);
    assert!(snapshot["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|n| n["props"].get("x").is_none()));
    #[cfg(feature = "compat-v2")]
    assert_eq!(snapshot["description"], "");
    let actual = hot_snapshot::probe(&url, &id)
        .await
        .unwrap()
        .key(cache.namespace(), &id);
    assert_ne!(identity.key(cache.namespace(), &id), actual);
    assert_eq!(read(&url, &id, &cache).await["revision"], 2);
    clean(&url, &id).await;
}

#[tokio::test]
async fn second_gateway_and_recreated_project_ignore_old_revision_entries() {
    let (url, id) = project("cache_gateway").await;
    seed(&url, &id).await;
    let first = cache(&url);
    let second = cache(&url);
    assert_eq!(read(&url, &id, &first).await["revision"], 1);
    let old_key = hot_snapshot::probe(&url, &id)
        .await
        .unwrap()
        .key(first.namespace(), &id);
    batch::execute(
        &url,
        &request(&id, "other_gateway", 1, vec![update("root", 2)]),
    )
    .await
    .unwrap();
    assert_eq!(read(&url, &id, &second).await["revision"], 2);
    assert_eq!(read(&url, &id, &first).await["revision"], 2);
    assert!(
        first.get(&old_key).await.is_some(),
        "correctness must not rely on invalidation"
    );
    clean(&url, &id).await;
    let client = db::connect(&url).await.unwrap();
    client
        .execute("INSERT INTO projects(project_id) VALUES($1)", &[&id])
        .await
        .unwrap();
    batch::execute(&url, &request(&id, "recreated", 0, vec![page("fresh")]))
        .await
        .unwrap();
    let recreated = read(&url, &id, &first).await;
    assert_eq!(recreated["revision"], 1);
    assert_eq!(recreated["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(recreated["nodes"][0]["id"], "fresh");
    let recreated_key = hot_snapshot::probe(&url, &id)
        .await
        .unwrap()
        .key(first.namespace(), &id);
    assert_ne!(old_key, recreated_key);
    assert_ne!(
        first.namespace(),
        hot_snapshot::namespace("another_database")
    );
    clean(&url, &id).await;
}

#[tokio::test]
async fn failed_pg_commit_does_not_publish_or_modify_a_cached_candidate() {
    let (url, id) = project("cache_commit_failure").await;
    seed(&url, &id).await;
    let cache = cache(&url);
    let before = read(&url, &id, &cache).await;
    let key = hot_snapshot::probe(&url, &id)
        .await
        .unwrap()
        .key(cache.namespace(), &id);
    let entry = cache.get(&key).await.unwrap();
    let client = db::connect(&url).await.unwrap();
    let trigger = format!("cache_fail_{id}");
    client.batch_execute(&format!("CREATE FUNCTION pg_temp.{trigger}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.parent_id='missing'; RETURN NEW; END $$; CREATE TRIGGER {trigger} BEFORE UPDATE ON nodes FOR EACH ROW WHEN (NEW.project_id='{id}' AND NEW.node_id='root') EXECUTE FUNCTION pg_temp.{trigger}();")).await.unwrap();
    let command = request(&id, "retry_failure", 1, vec![update("root", 9)]);
    assert_eq!(
        batch::execute(&url, &command).await.unwrap_err().code,
        "INTERNAL_ERROR"
    );
    assert_eq!(cache.get(&key).await.unwrap(), entry);
    assert_eq!(read(&url, &id, &cache).await, before);
    assert_eq!(counts(&url, &id).await, [1, 2, 1, 1]);
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
    assert_eq!(read(&url, &id, &cache).await["revision"], 2);
    clean(&url, &id).await;
}

#[cfg(feature = "compat-v2")]
#[tokio::test]
async fn description_change_without_revision_uses_a_distinct_consistent_key() {
    let (url, id) = project("cache_description").await;
    let cache = cache(&url);
    assert_eq!(read(&url, &id, &cache).await["description"], "");
    let old_key = hot_snapshot::probe(&url, &id)
        .await
        .unwrap()
        .key(cache.namespace(), &id);
    let client = db::connect(&url).await.unwrap();
    client
        .execute(
            "UPDATE projects SET description='new metadata' WHERE project_id=$1",
            &[&id],
        )
        .await
        .unwrap();
    assert_eq!(read(&url, &id, &cache).await["description"], "new metadata");
    let new_key = hot_snapshot::probe(&url, &id)
        .await
        .unwrap()
        .key(cache.namespace(), &id);
    assert_ne!(old_key, new_key);
    assert!(cache.get(&old_key).await.is_some());
    let _: i64 = redis::cmd("DEL")
        .arg(old_key)
        .arg(new_key)
        .query_async(&mut redis().await)
        .await
        .unwrap();
    clean(&url, &id).await;
}
