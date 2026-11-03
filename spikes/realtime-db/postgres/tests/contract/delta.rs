use super::support::*;
use crate::{
    batch, db,
    profile::{BatchEvidence, BatchProfile},
};
use oc_spike_core::{apply, BatchRequest, DomainError, VALIDATOR_VERSION};
use serde_json::{json, Value};

async fn observed(
    url: &str,
    request: &BatchRequest,
) -> Result<(Value, BatchEvidence), DomainError> {
    let mut profile = BatchProfile::new(request.operations.len());
    let result = batch::execute_profiled(url, request, &mut profile).await?;
    Ok((result, profile.evidence.clone()))
}

pub async fn certificate(url: &str, id: &str) -> (i64, Option<i64>, Option<String>) {
    let client = db::connect(url).await.unwrap();
    let row = client.query_one(
        "SELECT revision,validated_revision,validator_version FROM projects WHERE project_id=$1", &[&id],
    ).await.unwrap();
    (row.get(0), row.get(1), row.get(2))
}

#[tokio::test]
async fn certified_scalar_updates_select_and_validate_only_unique_touched_rows() {
    let (url, id) = project("delta_rows").await;
    let mut operations = vec![page("root")];
    operations.extend((0..40).map(|n| json!({"op":"create_node","id":format!("text_{n:02}"),"parentId":"root","kind":"text","props":{"text":"before","fontSize":12}})));
    let (_, evidence) = observed(&url, &request(&id, "seed", 0, operations))
        .await
        .unwrap();
    assert_eq!(evidence.path, "full");
    assert_eq!(
        certificate(&url, &id).await,
        (1, Some(1), Some(VALIDATOR_VERSION.into()))
    );
    let before = db::snapshot(&url, &id).await.unwrap();
    let command = request(
        &id,
        "scalar",
        1,
        vec![
            json!({"op":"update_node","id":"text_03","props":{"opacity":2,"text":123}}),
            json!({"op":"update_node","id":"text_01","props":{"name":"new","fill":"#abc","x":-4,"width":50}}),
            json!({"op":"update_node","id":"text_03","props":{"opacity":0.5,"text":"fixed","fontSize":30,"height":40,"y":2}}),
        ],
    );
    let (expected, changed) = apply(&before, &command).unwrap();
    let (response, evidence) = observed(&url, &command).await.unwrap();
    assert_eq!(response["changedIds"], json!(changed));
    assert_eq!(evidence.path, "delta");
    assert_eq!(evidence.fallback_reason, None);
    assert_eq!(evidence.queried_node_count, 2);
    assert_eq!(evidence.validated_node_count, 2);
    assert_eq!(db::snapshot(&url, &id).await.unwrap(), expected);
    assert_eq!(
        certificate(&url, &id).await,
        (2, Some(2), Some(VALIDATOR_VERSION.into()))
    );
    // Replay is before revision and node reads, and cannot restamp a later head.
    batch::execute(&url, &request(&id, "later", 2, vec![update("root", 3)]))
        .await
        .unwrap();
    let (replay, evidence) = observed(&url, &command).await.unwrap();
    assert_eq!(replay["committedRevision"], 2);
    assert_eq!(replay["replayed"], true);
    assert_eq!(evidence.queried_node_count, 0);
    assert_eq!(certificate(&url, &id).await.1, Some(3));
    clean(&url, &id).await;
}

#[tokio::test]
async fn missing_wrong_version_and_wrong_revision_certificates_force_full_validation() {
    let (url, id) = project("delta_fallback").await;
    assert_eq!(certificate(&url, &id).await, (0, None, None));
    seed(&url, &id).await;
    let client = db::connect(&url).await.unwrap();
    for (revision, sql, reason) in [
        (
            1,
            "validated_revision=NULL,validator_version=NULL",
            "missingCertificate",
        ),
        (
            2,
            "validator_version='previous-validator'",
            "validatorVersion",
        ),
        (3, "validated_revision=0", "certificateRevision"),
    ] {
        client
            .execute(
                &format!("UPDATE projects SET {sql} WHERE project_id=$1"),
                &[&id],
            )
            .await
            .unwrap();
        let (_, evidence) = observed(
            &url,
            &request(
                &id,
                &format!("fix_{revision}"),
                revision,
                vec![update("root", revision)],
            ),
        )
        .await
        .unwrap();
        assert_eq!(evidence.path, "full");
        assert_eq!(evidence.fallback_reason, Some(reason));
        assert_eq!(evidence.queried_node_count, 2);
        assert_eq!(evidence.validated_node_count, 2);
        assert_eq!(certificate(&url, &id).await.1, Some(revision as i64 + 1));
    }
    let before = db::snapshot(&url, &id).await.unwrap();
    let mixed = request(
        &id,
        "mixed",
        4,
        vec![
            json!({"op":"create_node","id":"child","parentId":"parent","kind":"group","props":{"width":-1}}),
            json!({"op":"update_node","id":"child","props":{"width":5}}),
            json!({"op":"create_node","id":"parent","parentId":"root","kind":"frame","props":{}}),
        ],
    );
    let (expected, _) = apply(&before, &mixed).unwrap();
    let (_, evidence) = observed(&url, &mixed).await.unwrap();
    assert_eq!(evidence.fallback_reason, Some("structuralOrMalformed"));
    assert_eq!(db::snapshot(&url, &id).await.unwrap(), expected);
    clean(&url, &id).await;
}

#[tokio::test]
async fn dry_run_does_not_create_or_advance_a_certificate() {
    let (url, id) = project("delta_dry").await;
    let mut dry = request(&id, "seed", 0, vec![page("root")]);
    dry.dry_run = true;
    batch::execute(&url, &dry).await.unwrap();
    assert_eq!(certificate(&url, &id).await, (0, None, None));
    assert_eq!(counts(&url, &id).await, [0, 0, 0, 0]);
    seed(&url, &id).await;
    let before = db::snapshot(&url, &id).await.unwrap();
    let mut dry = request(&id, "dry", 1, vec![update("root", 77)]);
    dry.dry_run = true;
    let (_, evidence) = observed(&url, &dry).await.unwrap();
    assert_eq!(evidence.path, "delta");
    assert_eq!(
        certificate(&url, &id).await,
        (1, Some(1), Some(VALIDATOR_VERSION.into()))
    );
    assert_eq!(db::snapshot(&url, &id).await.unwrap(), before);
    assert_eq!(counts(&url, &id).await, [1, 2, 1, 1]);
    clean(&url, &id).await;
}

#[tokio::test]
async fn uncertified_baseline_checks_untouched_rows_and_does_not_stamp_invalid_candidates() {
    let (url, id) = project("delta_uncertified").await;
    seed(&url, &id).await;
    let client = db::connect(&url).await.unwrap();
    client.execute("UPDATE projects SET validated_revision=NULL,validator_version=NULL WHERE project_id=$1", &[&id]).await.unwrap();
    client
        .execute(
            "UPDATE nodes SET props='{\"opacity\":2}' WHERE project_id=$1 AND node_id='paired'",
            &[&id],
        )
        .await
        .unwrap();
    assert_eq!(
        batch::execute(&url, &request(&id, "invalid", 1, vec![update("root", 3)]))
            .await
            .unwrap_err()
            .message,
        "Numeric property out of bounds"
    );
    assert_eq!(certificate(&url, &id).await, (1, None, None));
    assert_eq!(counts(&url, &id).await, [1, 2, 1, 1]);
    let corrected = request(
        &id,
        "corrected",
        1,
        vec![json!({"op":"update_node","id":"paired","props":{"opacity":0.5}})],
    );
    let (_, evidence) = observed(&url, &corrected).await.unwrap();
    assert_eq!(evidence.path, "full");
    assert_eq!(evidence.validated_node_count, 2);
    assert_eq!(
        certificate(&url, &id).await,
        (2, Some(2), Some(VALIDATOR_VERSION.into()))
    );
    clean(&url, &id).await;
}

#[tokio::test]
async fn additive_migration_is_idempotent_and_old_rows_start_uncertified() {
    let url = std::env::var("PG_URL").expect("PG_URL required");
    let mut client = db::connect(&url).await.unwrap();
    let tx = client.transaction().await.unwrap();
    // The temporary table shadows public.projects; no fixture/user table changes.
    tx.batch_execute("CREATE TEMP TABLE projects(project_id TEXT PRIMARY KEY,revision BIGINT NOT NULL DEFAULT 0) ON COMMIT DROP; INSERT INTO projects(project_id) VALUES('old');").await.unwrap();
    for _ in 0..2 {
        tx.batch_execute(include_str!("../../delta-migration.sql"))
            .await
            .unwrap();
    }
    let row = tx.query_one("SELECT generation,validated_revision,validator_version FROM projects WHERE project_id='old'", &[]).await.unwrap();
    let old: String = row.get(0);
    assert!(!old.is_empty());
    assert_eq!(row.get::<_, Option<i64>>(1), None);
    assert_eq!(row.get::<_, Option<String>>(2), None);
    tx.batch_execute("DELETE FROM projects; INSERT INTO projects(project_id) VALUES('old')")
        .await
        .unwrap();
    let new: String = tx
        .query_one("SELECT generation FROM projects", &[])
        .await
        .unwrap()
        .get(0);
    assert_ne!(old, new);
    tx.rollback().await.unwrap();
}
