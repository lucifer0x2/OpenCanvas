mod postgres_support;
use opencanvas_storage::*;
use postgres_support::*;
use std::time::{Duration, Instant};

const RUNTIME: &str = "SELECT pid FROM pg_stat_activity \
    WHERE application_name='opencanvas-runtime' AND pid<>pg_backend_pid()";

fn runtime_pids(control: &mut postgres::Client) -> Vec<i32> {
    let rows = control.query(RUNTIME, &[]).unwrap();
    rows.iter().map(|row| row.get(0)).collect()
}

fn terminate_runtime(control: &mut postgres::Client) -> Vec<i32> {
    let before = runtime_pids(control);
    assert!(!before.is_empty(), "runtime connection expected");
    control
        .execute(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
             WHERE application_name='opencanvas-runtime' AND pid<>pg_backend_pid()",
            &[],
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while runtime_pids(control).iter().any(|pid| before.contains(pid)) {
        assert!(Instant::now() < deadline, "terminated backend still listed");
        std::thread::sleep(Duration::from_millis(20));
    }
    before
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn backend_reconnects_after_its_session_is_terminated() {
    let mut pg_config = config();
    pg_config.pool_size = 1;
    let storage = Storage::new(PostgresBackend::with_config(pg_config).unwrap());
    let p = project(&storage, "reconnect");
    p.apply_batch(request(p.id(), 0, "seed", page("Before")))
        .unwrap();
    let mut control = sql();
    let terminated = terminate_runtime(&mut control);

    let renamed = request(p.id(), 1, "rename", rename("After"));
    if let Err(error) = p.apply_batch(renamed.clone()) {
        assert_eq!(error.code, "STORAGE_UNAVAILABLE", "{error}");
        assert!(error.retryable);
        assert!(!error.message.contains("task19"));
        p.apply_batch(renamed).unwrap();
    }
    let snapshot = serde_json::to_value(p.snapshot().unwrap()).unwrap();
    assert!(snapshot.to_string().contains("After"));
    let current = runtime_pids(&mut control);
    assert_eq!(current.len(), 1);
    assert!(!terminated.contains(&current[0]));
}
