use crate::{batch, db};
use oc_spike_core::BatchRequest;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(1);

pub async fn project(label: &str) -> (String, String) {
    let url = std::env::var("PG_URL").expect("PG_URL required: use isolated spike database");
    let id = format!(
        "contract_{}_{}_{}",
        std::process::id(),
        label,
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let client = db::connect(&url).await.unwrap();
    db::initialize(&*client).await.unwrap();
    client
        .execute("INSERT INTO projects(project_id) VALUES($1)", &[&id])
        .await
        .unwrap();
    (url, id)
}

pub fn request(id: &str, key: &str, revision: u64, operations: Vec<Value>) -> BatchRequest {
    BatchRequest {
        project_id: id.into(),
        idempotency_key: key.into(),
        expected_revision: revision,
        operations,
        dry_run: false,
    }
}

pub fn page(id: &str) -> Value {
    json!({"op":"create_node","id":id,"parentId":null,"kind":"page","props":{}})
}

pub fn update(id: &str, value: u64) -> Value {
    json!({"op":"update_node","id":id,"props":{"x":value,"y":value}})
}

pub async fn seed(url: &str, id: &str) {
    batch::execute(
        url,
        &request(id, "seed", 0, vec![page("root"), page("paired")]),
    )
    .await
    .unwrap();
}

pub async fn counts(url: &str, id: &str) -> Vec<i64> {
    let client = db::connect(url).await.unwrap();
    let row = client.query_one("SELECT (SELECT revision FROM projects WHERE project_id=$1), (SELECT count(*) FROM nodes WHERE project_id=$1), (SELECT count(*) FROM commits WHERE project_id=$1), (SELECT count(*) FROM idempotency WHERE project_id=$1)", &[&id]).await.unwrap();
    (0..4).map(|i| row.get(i)).collect()
}

pub async fn clean(url: &str, id: &str) {
    let mut client = db::connect(url).await.unwrap();
    let tx = client.transaction().await.unwrap();
    for table in ["idempotency", "commits", "nodes", "projects"] {
        tx.execute(&format!("DELETE FROM {table} WHERE project_id=$1"), &[&id])
            .await
            .unwrap();
    }
    tx.commit().await.unwrap();
}
