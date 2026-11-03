//! Authoritative identity probes and consistent cold snapshot reads.
use crate::{cache_metrics, db};
use oc_spike_core::DomainError;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio_postgres::{types::Type, GenericClient, IsolationLevel};

pub struct Identity {
    generation: String,
    revision: u64,
    #[cfg(feature = "compat-v2")]
    description: String,
}

impl Identity {
    pub fn key(&self, namespace: &str, id: &str) -> String {
        // Description may change without advancing revision in the migration fixture.
        // Include its exact content hash rather than placing it under a revision-only key.
        #[cfg(feature = "compat-v2")]
        let metadata = format!("{:x}", Sha256::digest(self.description.as_bytes()));
        #[cfg(not(feature = "compat-v2"))]
        let metadata = "none";
        format!(
            "oc:snapshot:v{}:{namespace}:{}:{}:{}:{metadata}",
            if cfg!(feature = "compat-v2") { 2 } else { 1 },
            hex(id.as_bytes()),
            hex(self.generation.as_bytes()),
            self.revision,
        )
    }
}

// Hex encodes untrusted identity components so separators cannot alias another key.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn namespace(url: &str) -> String {
    format!("{:x}", Sha256::digest(url.as_bytes()))
}

pub async fn probe(url: &str, id: &str) -> Result<Identity, DomainError> {
    cache_metrics::increment(&cache_metrics::PROBES);
    let client = db::connect(url).await?;
    identity(&*client, id).await
}

async fn identity<C: GenericClient>(client: &C, id: &str) -> Result<Identity, DomainError> {
    #[cfg(feature = "compat-v2")]
    let query = "SELECT generation,revision,description FROM projects WHERE project_id=$1";
    #[cfg(not(feature = "compat-v2"))]
    let query = "SELECT generation,revision FROM projects WHERE project_id=$1";
    let row = client
        .query_typed_opt(query, &[(&id, Type::TEXT)])
        .await
        .map_err(db::internal)?
        .ok_or_else(|| DomainError::new("PROJECT_NOT_FOUND", "Project does not exist"))?;
    Ok(Identity {
        generation: row.get(0),
        revision: row.get::<_, i64>(1) as u64,
        #[cfg(feature = "compat-v2")]
        description: row.get(2),
    })
}

pub async fn load(url: &str, id: &str) -> Result<(Identity, Value), DomainError> {
    load_interleaved(url, id, || async {}).await
}

// The test seam commits after metadata, before nodes. REPEATABLE READ must retain
// metadata, description and node rows from the same transaction snapshot.
pub(crate) async fn load_interleaved<F, Fut>(
    url: &str,
    id: &str,
    between: F,
) -> Result<(Identity, Value), DomainError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let mut client = db::connect(url).await?;
    let tx = client
        .build_transaction()
        .isolation_level(IsolationLevel::RepeatableRead)
        .read_only(true)
        .start()
        .await
        .map_err(db::internal)?;
    let identity = identity(&tx, id).await?;
    between().await;
    cache_metrics::increment(&cache_metrics::NODE_QUERIES);
    let state = db::state(&tx, id, identity.revision).await?;
    let value = serde_json::to_value(state).map_err(db::internal)?;
    #[cfg(feature = "compat-v2")]
    let value = {
        let mut value = value;
        value["description"] = Value::String(identity.description.clone());
        value
    };
    tx.commit().await.map_err(db::internal)?;
    Ok((identity, value))
}
