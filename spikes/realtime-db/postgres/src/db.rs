use oc_spike_core::{DomainError, Node, State};
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, OnceLock},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_postgres::{types::Type, Client, GenericClient, NoTls};

struct Pool {
    idle: Mutex<Vec<Client>>,
    permits: Arc<Semaphore>,
}
static POOL: OnceLock<Pool> = OnceLock::new();

pub struct Lease {
    client: Option<Client>,
    _permit: OwnedSemaphorePermit,
}
impl Deref for Lease {
    type Target = Client;
    fn deref(&self) -> &Client {
        self.client.as_ref().unwrap()
    }
}
impl DerefMut for Lease {
    fn deref_mut(&mut self) -> &mut Client {
        self.client.as_mut().unwrap()
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            if !client.is_closed() {
                POOL.get().unwrap().idle.lock().unwrap().push(client);
            }
        }
    }
}

pub fn internal(error: impl std::fmt::Display) -> DomainError {
    eprintln!("postgres: {error}");
    DomainError::new("INTERNAL_ERROR", "Database operation failed")
}

// Called once at gateway startup, before cache initialization or accepting reads.
// connect() stays a pool lease and never runs DDL on the hot path.
pub async fn initialize<C: GenericClient>(client: &C) -> Result<(), DomainError> {
    for sql in [
        include_str!("../schema.sql"),
        include_str!("../delta-migration.sql"),
        include_str!("../durable-schema.sql"),
        include_str!("../indexes.sql"),
    ] {
        client.batch_execute(sql).await.map_err(internal)?;
    }
    Ok(())
}

pub async fn connect(url: &str) -> Result<Lease, DomainError> {
    let pool = POOL.get_or_init(|| Pool {
        idle: Mutex::new(Vec::new()),
        permits: Arc::new(Semaphore::new(16)),
    });
    let permit = pool
        .permits
        .clone()
        .acquire_owned()
        .await
        .map_err(internal)?;
    loop {
        let idle = pool.idle.lock().unwrap().pop();
        match idle {
            Some(client) if !client.is_closed() => {
                return Ok(Lease {
                    client: Some(client),
                    _permit: permit,
                })
            }
            Some(_) => continue,
            None => break,
        }
    }
    let (client, connection) = tokio_postgres::connect(url, NoTls)
        .await
        .map_err(internal)?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            eprintln!("postgres connection: {error}");
        }
    });
    Ok(Lease {
        client: Some(client),
        _permit: permit,
    })
}

pub async fn state<C: GenericClient>(
    client: &C,
    id: &str,
    revision: u64,
) -> Result<State, DomainError> {
    state_inner(client, id, revision, None).await
}

pub async fn state_profiled<C: GenericClient>(
    client: &C,
    id: &str,
    revision: u64,
    profile: &mut crate::profile::BatchProfile,
) -> Result<State, DomainError> {
    state_inner(client, id, revision, Some(profile)).await
}

async fn state_inner<C: GenericClient>(
    client: &C,
    id: &str,
    revision: u64,
    mut profile: Option<&mut crate::profile::BatchProfile>,
) -> Result<State, DomainError> {
    let timer = profile.as_ref().and_then(|p| p.timer());
    let rows = client.query_typed("SELECT node_id,parent_id,kind,sort_key,props FROM nodes WHERE project_id=$1 ORDER BY node_id", &[(&id, Type::TEXT)]).await.map_err(internal)?;
    if let Some(profile) = profile.as_mut() {
        profile.record("nodeQuery", timer);
    }
    let timer = profile.as_ref().and_then(|p| p.timer());
    let nodes = rows
        .into_iter()
        .map(|row| Node {
            id: row.get(0),
            parent_id: row.get(1),
            kind: row.get(2),
            sort_key: row.get(3),
            props: row.get(4),
        })
        .collect();
    if let Some(profile) = profile {
        profile.record("nodeDecode", timer);
    }
    Ok(State {
        project_id: id.into(),
        revision,
        nodes,
    })
}

pub async fn snapshot(url: &str, id: &str) -> Result<State, DomainError> {
    snapshot_interleaved(url, id, || async {}).await
}

// A single indexed project-row read, with no cached revision or node materialization.
pub async fn current_revision(url: &str, id: &str) -> Result<u64, DomainError> {
    let client = connect(url).await?;
    revision(&*client, id).await
}

// The callback lets integration tests commit between the two SELECTs. Keeping the
// transaction here makes the snapshot guarantee independent of the HTTP adapter.
pub(crate) async fn snapshot_interleaved<F, Fut>(
    url: &str,
    id: &str,
    between: F,
) -> Result<State, DomainError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let mut client = connect(url).await?;
    let transaction = client
        .build_transaction()
        .isolation_level(tokio_postgres::IsolationLevel::RepeatableRead)
        .read_only(true)
        .start()
        .await
        .map_err(internal)?;
    let revision = revision(&transaction, id).await?;
    between().await;
    let result = state(&transaction, id, revision).await?;
    transaction.commit().await.map_err(internal)?;
    Ok(result)
}

pub async fn revision<C: GenericClient>(client: &C, id: &str) -> Result<u64, DomainError> {
    let row = client
        .query_typed_opt(
            "SELECT revision FROM projects WHERE project_id=$1",
            &[(&id, Type::TEXT)],
        )
        .await
        .map_err(internal)?;
    row.map(|r| r.get::<_, i64>(0) as u64)
        .ok_or_else(|| DomainError::new("PROJECT_NOT_FOUND", "Project does not exist"))
}
