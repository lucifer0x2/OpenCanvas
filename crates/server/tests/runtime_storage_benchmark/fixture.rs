use opencanvas_storage::{
    BatchRequest, NodeKind, Operation, PostgresBackend, ProjectHandle, Snapshot, SnapshotCache,
    Storage, Store,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub struct Project {
    pub storage: Storage,
    pub handle: Arc<ProjectHandle>,
    pub moka: bool,
    pub count: usize,
    pub seed_revision: u64,
    pub _root: Option<tempfile::TempDir>,
}

pub fn prepare(engine: &str, count: usize) -> Project {
    let mut root = None;
    let moka = engine == "pgmoka";
    let storage = if engine == "sqlite" {
        let temp = tempfile::tempdir().unwrap();
        let storage = Storage::from(Store::open(temp.path()).unwrap());
        root = Some(temp);
        storage
    } else {
        let backend =
            PostgresBackend::connect(&std::env::var("TASK19_POSTGRES_URL").unwrap()).unwrap();
        if moka {
            let cache = SnapshotCache::new(backend.cache_namespace());
            Storage::new(backend.with_cache(cache).unwrap())
        } else {
            Storage::new(backend)
        }
    };
    // Engine tags are all six bytes; project UUIDs are all the same length.
    let slug = format!("bench-{count:04}-{engine}");
    let handle = storage.create_project(&slug, "Runtime benchmark").unwrap();
    let operations = nodes(count);
    for (batch, operations) in operations.chunks(500).enumerate() {
        let result = handle
            .apply_batch(request(
                handle.id(),
                batch as u64,
                &format!("seed-{batch}"),
                operations.to_vec(),
            ))
            .unwrap();
        assert_eq!(result.committed_revision, batch as u64 + 1);
    }
    let seed_revision = count.div_ceil(500) as u64;
    let snapshot = handle.snapshot().unwrap();
    assert_eq!(snapshot.nodes.len(), count);
    assert_eq!(snapshot.revision, seed_revision);
    Project {
        storage,
        handle,
        moka,
        count,
        seed_revision,
        _root: root,
    }
}

fn nodes(count: usize) -> Vec<Operation> {
    let mut nodes = vec![
        create(
            "page",
            NodeKind::Page,
            None,
            json!({"name":"Benchmark page"}),
        ),
        create(
            "frame",
            NodeKind::Frame,
            Some("page"),
            json!({
                "name":"Dashboard", "width":1280,"height":10000,
                "layout":{"mode":"free"}, "fill":"#f4f5f7"
            }),
        ),
    ];
    for index in 2..count {
        let text = index % 10 == 0;
        let mut props = json!({
            "name":format!("Element {index:05}"), "x":(index % 8)*150,
            "y":(index / 8)*20, "width":140,"height":18,
            "fill":if text {"#172b4d"} else {"#dfe3eb"}
        });
        if text {
            props["text"] = json!(format!("Metric {index:05}: 128 requests"));
            props["fontId"] = json!("font_bundled");
            props["fontSize"] = json!(12);
            props["lineHeight"] = json!(18);
        }
        nodes.push(create(
            &format!("node-{index:05}"),
            if text {
                NodeKind::Text
            } else {
                NodeKind::Rectangle
            },
            Some("frame"),
            props,
        ));
    }
    nodes
}

fn create(id: &str, kind: NodeKind, parent: Option<&str>, props: Value) -> Operation {
    Operation::CreateNode {
        id: id.into(),
        kind,
        parent_id: parent.map(Into::into),
        props,
        index: None,
    }
}

pub fn request(id: &str, revision: u64, key: &str, operations: Vec<Operation>) -> BatchRequest {
    BatchRequest {
        project_id: id.into(),
        expected_revision: revision,
        idempotency_key: key.into(),
        operations,
    }
}

pub fn normalized(snapshot: &Snapshot) -> Value {
    let mut value = serde_json::to_value(snapshot).unwrap();
    value["projectId"] = json!("<generated-project-id>");
    value["slug"] = json!("<engine-slug>");
    value
}
