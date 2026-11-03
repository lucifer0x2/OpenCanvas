use serde_json::{json, Value};
use std::process::Command;

#[derive(Debug)]
pub struct Counters {
    probes: u64,
    entity_reads: u64,
}

pub fn counters(postgres: bool) -> Option<Counters> {
    if !postgres {
        return None;
    }
    let container = std::env::var("TASK19_POSTGRES_CONTAINER").unwrap();
    assert!(container.starts_with("opencanvas-task19.") && container.ends_with("-pg"));
    let query = "
        SELECT
            COALESCE(SUM(calls) FILTER (
                WHERE query LIKE 'SELECT projects.*,%WHERE project_id=%ORDER BY (project_id=%'
                    AND query NOT LIKE '%FOR UPDATE'
            ), 0),
            COALESCE(SUM(calls) FILTER (
                WHERE query LIKE 'SELECT kind,id,data FROM entities WHERE project_id=%'
            ), 0)
        FROM pg_stat_statements
        WHERE dbid = (SELECT oid FROM pg_database WHERE datname = current_database())
    ";
    let result = Command::new("docker")
        .args(["exec", &container, "psql", "-XAt", "-v", "ON_ERROR_STOP=1"])
        .args(["-U", "task19", "-d", "task19", "-c", query])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "isolated PostgreSQL counters query failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let output = String::from_utf8(result.stdout).unwrap();
    let (probes, entity_reads) = output.trim().split_once('|').unwrap();
    Some(Counters {
        probes: probes.parse().unwrap(),
        entity_reads: entity_reads.parse().unwrap(),
    })
}

pub fn verify(before: Option<Counters>, reads: usize) -> Value {
    let Some(before) = before else {
        return Value::Null;
    };
    let after = counters(true).unwrap();
    let probes = after.probes - before.probes;
    let entity_reads = after.entity_reads - before.entity_reads;
    assert_eq!(probes, reads as u64, "each hot read probes PostgreSQL");
    assert_eq!(entity_reads, 0, "no hot read materializes SQL entities");
    json!({
        "readsIncludingWarmup": reads,
        "postgresIdentityProbes": probes,
        "postgresEntityReads": entity_reads,
    })
}
