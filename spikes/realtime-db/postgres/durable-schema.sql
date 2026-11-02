-- Additive spike-only schema: tiny fixture bytes live in PostgreSQL for shared backup.
CREATE TABLE IF NOT EXISTS shared_blobs (
    blob_id TEXT PRIMARY KEY,
    bytes BYTEA NOT NULL CHECK (octet_length(bytes) <= 2097152)
);
CREATE TABLE IF NOT EXISTS resources (
    resource_id TEXT NOT NULL REFERENCES shared_blobs(blob_id),
    kind TEXT NOT NULL CHECK (kind IN ('asset', 'font')),
    PRIMARY KEY(resource_id, kind)
);
CREATE TABLE IF NOT EXISTS staged_batches (
    upload_id TEXT PRIMARY KEY REFERENCES shared_blobs(blob_id),
    project_id TEXT NOT NULL REFERENCES projects(project_id)
);
CREATE TABLE IF NOT EXISTS render_jobs (
    job_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    revision BIGINT NOT NULL CHECK (revision >= 0),
    pipeline_fingerprint TEXT NOT NULL,
    input JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','running','completed')),
    attempt BIGINT NOT NULL DEFAULT 0 CHECK (attempt >= 0),
    worker_id TEXT,
    lease_expires_at TIMESTAMPTZ,
    artifact_id TEXT REFERENCES shared_blobs(blob_id)
);
CREATE INDEX IF NOT EXISTS render_jobs_claim ON render_jobs(status, lease_expires_at);
