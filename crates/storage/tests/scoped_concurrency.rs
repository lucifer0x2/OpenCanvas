mod postgres_support;
mod scoped_support;
use scoped_support::*;

#[test]
fn sqlite_independent_pages_replay_preview_aba_and_ordered_history() {
    let (_temp, _storage, p) = sqlite("concurrency");
    semantics::concurrency(p);
}
#[test]
fn sqlite_transitive_dependencies_and_expanded_font_override() {
    let (_temp, _storage, p) = sqlite("dependencies");
    dependencies::dependencies(p);
}
#[test]
fn sqlite_restrictions_coverage_and_rollback() {
    let (_temp, _storage, p) = sqlite("restrictions");
    semantics::restrictions(p);
}
#[test]
#[ignore = "fresh disposable PostgreSQL only"]
fn postgres_scoped_contract() {
    let (_, storage) = postgres_support::setup();
    semantics::concurrency(postgres_support::project(&storage, "scoped-concurrency"));
    dependencies::dependencies(postgres_support::project(&storage, "scoped-dependencies"));
    semantics::restrictions(postgres_support::project(&storage, "scoped-restrictions"));
    let p = postgres_support::project(&storage, "scoped-reconnect");
    seed(&p);
    strict(
        &p,
        "other",
        vec![update("r2", serde_json::json!({"name":"other"}))],
    );
    let request = scoped(
        &p,
        1,
        "durable",
        &["p1"],
        vec![update("r1", serde_json::json!({"name":"durable"}))],
    );
    assert_eq!(
        p.apply_scoped_batch(request.clone())
            .unwrap()
            .committed_revision,
        3
    );
    let id = p.id().to_owned();
    drop(p);
    drop(storage);
    let (_, reopened) = postgres_support::setup();
    let p = reopened.open_project(&id).unwrap();
    let replay = p.apply_scoped_batch(request).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.committed_revision, 3);
    assert_eq!(p.commits_after(0, 10).unwrap()[2].base_revision, 2);
}
