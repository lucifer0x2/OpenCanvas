use super::*;
use ::postgres::Client;

pub(super) fn initialize(client: &mut Client) -> Result<String> {
    let mut tx = client.transaction().map_err(sql)?;
    // Serialize only schema initialization, independently from project writes.
    tx.batch_execute(
        "SELECT pg_advisory_xact_lock(6801942319);
        CREATE SCHEMA IF NOT EXISTS opencanvas_runtime_v1;
        CREATE TABLE IF NOT EXISTS schema_registry (
            singleton BOOLEAN PRIMARY KEY CHECK(singleton), version INTEGER NOT NULL,
            namespace TEXT NOT NULL);",
    )
    .map_err(sql)?;
    tx.execute("INSERT INTO schema_registry(singleton,version,namespace) VALUES(true,1,$1) ON CONFLICT DO NOTHING",
        &[&uuid::Uuid::new_v4().to_string()]).map_err(sql)?;
    let row = tx
        .query_one(
            "SELECT version,namespace FROM schema_registry WHERE singleton",
            &[],
        )
        .map_err(sql)?;
    if row.get::<_, i32>(0) != 1 {
        return Err(crate::StorageError::new(
            "SCHEMA_UNSUPPORTED",
            "Database schema requires a supported migration",
        ));
    }
    let namespace = row.get(1);
    tx.batch_execute("CREATE TABLE IF NOT EXISTS projects (
            project_id TEXT PRIMARY KEY, slug TEXT NOT NULL UNIQUE, name BYTEA NOT NULL,
            schema_version INTEGER NOT NULL, revision BIGINT NOT NULL CHECK(revision>=0),
            generation BIGINT GENERATED ALWAYS AS IDENTITY, oldest_revision BIGINT NOT NULL DEFAULT 1);
        CREATE TABLE IF NOT EXISTS entities (
            project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
            kind TEXT NOT NULL CHECK(kind IN ('node','token','style','asset','font','note')),
            id TEXT NOT NULL, data TEXT NOT NULL, PRIMARY KEY(project_id,kind,id));
        CREATE TABLE IF NOT EXISTS binaries (
            project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
            content_hash TEXT NOT NULL, bytes BYTEA NOT NULL,
            PRIMARY KEY(project_id,content_hash));
        CREATE TABLE IF NOT EXISTS commits (
            project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
            revision BIGINT NOT NULL, base_revision BIGINT NOT NULL, batch_id TEXT NOT NULL,
            operations TEXT NOT NULL, changed_ids TEXT NOT NULL, PRIMARY KEY(project_id,revision));
        CREATE TABLE IF NOT EXISTS idempotency (
            project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
            key TEXT NOT NULL, request_hash TEXT NOT NULL, response TEXT NOT NULL,
            import_asset TEXT, PRIMARY KEY(project_id,key));
        ALTER TABLE projects ADD COLUMN IF NOT EXISTS privacy_status TEXT NOT NULL DEFAULT 'private'
            CHECK(privacy_status IN ('private','public'));")
        .map_err(sql)?;
    tx.batch_execute(viewer_auth::SCHEMA).map_err(sql)?;
    tx.commit().map_err(sql)?;
    Ok(namespace)
}
