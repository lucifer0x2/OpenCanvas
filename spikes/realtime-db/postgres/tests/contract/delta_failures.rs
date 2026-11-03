use super::{delta_contract::certificate, support::*};
use crate::{batch, db};
use oc_spike_core::{apply, VALIDATOR_VERSION};
use serde_json::json;

#[tokio::test]
async fn delta_validation_sql_and_deferred_commit_failures_preserve_certificate_and_state() {
    let (url, id) = project("delta_failures").await;
    seed(&url, &id).await;
    let before = db::snapshot(&url, &id).await.unwrap();
    let certificate_before = certificate(&url, &id).await;
    for operations in [
        vec![
            update("root", 7),
            json!({"op":"update_node","id":"paired","props":{"opacity":2}}),
        ],
        vec![update("root", 7), update("missing", 8)],
    ] {
        assert_eq!(
            batch::execute(&url, &request(&id, "failed", 1, operations))
                .await
                .unwrap_err()
                .code,
            "VALIDATION_FAILED"
        );
        assert_eq!(certificate(&url, &id).await, certificate_before);
        assert_eq!(db::snapshot(&url, &id).await.unwrap(), before);
    }
    let client = db::connect(&url).await.unwrap();
    let sql_trigger = format!("fail_{id}");
    client.batch_execute(&format!("CREATE FUNCTION pg_temp.{sql_trigger}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'delta SQL rollback'; END $$; CREATE TRIGGER {sql_trigger} BEFORE INSERT ON idempotency FOR EACH ROW WHEN (NEW.project_id='{id}') EXECUTE FUNCTION pg_temp.{sql_trigger}();")).await.unwrap();
    let command = request(&id, "retry", 1, vec![update("root", 99)]);
    assert_eq!(
        batch::execute(&url, &command).await.unwrap_err().code,
        "INTERNAL_ERROR"
    );
    assert_eq!(certificate(&url, &id).await, certificate_before);
    assert_eq!(db::snapshot(&url, &id).await.unwrap(), before);
    client
        .batch_execute(&format!(
            "DROP TRIGGER {sql_trigger} ON idempotency; DROP FUNCTION pg_temp.{sql_trigger}()"
        ))
        .await
        .unwrap();
    let commit_trigger = format!("corrupt_{id}");
    client.batch_execute(&format!("CREATE FUNCTION pg_temp.{commit_trigger}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.parent_id='missing'; RETURN NEW; END $$; CREATE TRIGGER {commit_trigger} BEFORE INSERT OR UPDATE ON nodes FOR EACH ROW WHEN (NEW.project_id='{id}' AND NEW.node_id='root') EXECUTE FUNCTION pg_temp.{commit_trigger}();")).await.unwrap();
    assert_eq!(
        batch::execute(&url, &command).await.unwrap_err().code,
        "INTERNAL_ERROR"
    );
    assert_eq!(certificate(&url, &id).await, certificate_before);
    assert_eq!(db::snapshot(&url, &id).await.unwrap(), before);
    assert_eq!(counts(&url, &id).await, [1, 2, 1, 1]);
    client
        .batch_execute(&format!(
            "DROP TRIGGER {commit_trigger} ON nodes; DROP FUNCTION pg_temp.{commit_trigger}()"
        ))
        .await
        .unwrap();
    assert_eq!(
        batch::execute(&url, &command).await.unwrap()["committedRevision"],
        2
    );
    assert_eq!(
        certificate(&url, &id).await,
        (2, Some(2), Some(VALIDATOR_VERSION.into()))
    );
    assert_eq!(counts(&url, &id).await, [2, 2, 2, 2]);
    clean(&url, &id).await;
}

#[tokio::test]
async fn postgres_unsafe_target_ids_keep_full_path_error_precedence() {
    let (url, id) = project("delta_target_ids").await;
    seed(&url, &id).await;
    let before = db::snapshot(&url, &id).await.unwrap();
    let certificate_before = certificate(&url, &id).await;
    for operations in [
        vec![update("missing\0", 1)],
        vec![update("root", 7), update("missing\0", 1)],
        vec![
            json!({"op":"update_node","id":"root","props":null}),
            update("missing\0", 1),
        ],
    ] {
        let command = request(&id, "unsafe_target", 1, operations);
        let expected = apply(&before, &command).unwrap_err();
        let mut profile = crate::profile::BatchProfile::new(command.operations.len());
        let actual = batch::execute_profiled(&url, &command, &mut profile)
            .await
            .unwrap_err();
        assert_eq!(actual.code, expected.code);
        assert_eq!(actual.message, expected.message);
        assert_eq!(profile.evidence.path, "full");
        assert_eq!(
            profile.evidence.fallback_reason,
            Some("structuralOrMalformed")
        );
        assert_eq!(certificate(&url, &id).await, certificate_before);
        assert_eq!(db::snapshot(&url, &id).await.unwrap(), before);
        assert_eq!(counts(&url, &id).await, [1, 2, 1, 1]);
    }
    clean(&url, &id).await;
}
