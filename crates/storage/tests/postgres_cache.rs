mod postgres_support;
use opencanvas_storage::*;
use postgres_support::*;
use std::sync::Barrier;

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn hot_read_uses_canonical_identity_and_skips_locked_entity_tables() {
    let (backend, _) = cached();
    let storage = Storage::new(backend);
    let p = project(&storage, "hot");
    let mut seed = page("Cold\0then hot");
    seed.extend(serde_json::from_value::<Vec<Operation>>(serde_json::json!([
        {"op":"upsert_token","token":{"id":"zero","name":"Number\0name","type":"number","value":-0.0}}
    ])).unwrap());
    p.apply_batch(request(p.id(), 0, "seed", seed)).unwrap();
    let slug = storage.project_meta(p.id()).unwrap().slug;
    let cold = storage.snapshot(&slug).unwrap();
    let mut control = sql();
    let lock = lock_entity_tables(&mut control);
    let hot = storage.snapshot(p.id()).unwrap();
    assert_eq!(hot, cold);
    assert_eq!(hot.nodes[0].props["name"], "Cold\0then hot");
    assert_eq!(
        hot.tokens[0].value.as_f64().unwrap().to_bits(),
        (-0.0_f64).to_bits()
    );
    let (empty, _) = cached();
    let error = empty.snapshot(p.id()).unwrap_err();
    assert_eq!(error.code, "STORAGE_BUSY");
    lock.rollback().unwrap();
    assert_eq!(empty.snapshot(p.id()).unwrap(), cold);
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn independent_and_shared_caches_follow_another_backends_commit() {
    let (backend, cache) = cached();
    let storage = Storage::new(backend);
    let p = project(&storage, "cache-race");
    p.apply_batch(request(p.id(), 0, "seed", page("Old")))
        .unwrap();
    let old = p.snapshot().unwrap();

    let (other, _) = cached();
    assert_eq!(other.snapshot(p.id()).unwrap(), old);
    let write = other
        .apply_batch(p.id(), request(p.id(), 1, "new", rename("New")))
        .unwrap();
    assert_eq!(write.committed_revision, 2);

    let mut control = sql();
    let lock = lock_entity_tables(&mut control);
    assert_eq!(p.snapshot().unwrap_err().code, "STORAGE_BUSY");
    // Only the writer's own cache was filled by its commit.
    let written = other.snapshot(p.id()).unwrap();
    assert_eq!(written.revision, 2);
    assert_eq!(written.nodes[0].props["name"], "New");
    lock.rollback().unwrap();

    let shared = PostgresBackend::with_config(config())
        .unwrap()
        .with_cache(cache)
        .unwrap();
    let barrier = Barrier::new(4);
    let id = p.id();
    let reads = std::thread::scope(|scope| {
        let independent = scope.spawn(|| {
            barrier.wait();
            other.snapshot(id).unwrap()
        });
        let shared = scope.spawn(|| {
            barrier.wait();
            shared.snapshot(id).unwrap()
        });
        let original = scope.spawn(|| {
            barrier.wait();
            p.snapshot().unwrap()
        });
        barrier.wait();
        [
            independent.join().unwrap(),
            shared.join().unwrap(),
            original.join().unwrap(),
        ]
    });
    let newest = p.snapshot().unwrap();
    assert_eq!(newest.revision, 2);
    assert_eq!(newest.nodes[0].props["name"], "New");
    assert!(reads.iter().all(|snapshot| *snapshot == newest));
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn project_deletion_and_recreation_block_a_warm_previous_generation() {
    let (backend, _) = cached();
    let storage = Storage::new(backend);
    let p = project(&storage, "generation");
    let old = p.snapshot().unwrap();
    let mut control = sql();
    control
        .execute(
            "DELETE FROM opencanvas_runtime_v1.projects WHERE project_id=$1",
            &[&p.id()],
        )
        .unwrap();
    assert_eq!(p.snapshot().unwrap_err().code, "PROJECT_NOT_FOUND");
    control.execute("INSERT INTO opencanvas_runtime_v1.projects(project_id,slug,name,schema_version,revision) VALUES($1,$2,$3,$4,0)", &[&p.id(), &old.slug, &b"Replacement".to_vec(), &(SCHEMA_VERSION as i32)]).unwrap();
    let new = p.snapshot().unwrap();
    assert_eq!(new.project_id, old.project_id);
    assert_eq!(new.revision, old.revision);
    assert_eq!(new.name, "Replacement");
    let lock = lock_entity_tables(&mut control);
    assert_eq!(p.snapshot().unwrap(), new);
    lock.rollback().unwrap();
}
