CREATE TABLE IF NOT EXISTS projects (
    project_id TEXT PRIMARY KEY,
    revision BIGINT NOT NULL DEFAULT 0 CHECK (revision >= 0)
);
CREATE TABLE IF NOT EXISTS nodes (
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    node_id TEXT NOT NULL,
    parent_id TEXT,
    kind TEXT NOT NULL,
    sort_key BIGINT NOT NULL,
    props JSONB NOT NULL,
    PRIMARY KEY (project_id, node_id),
    FOREIGN KEY (project_id, parent_id) REFERENCES nodes(project_id, node_id)
        DEFERRABLE INITIALLY DEFERRED
);
CREATE TABLE IF NOT EXISTS commits (
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    revision BIGINT NOT NULL CHECK (revision > 0),
    changed_ids JSONB NOT NULL,
    PRIMARY KEY (project_id, revision)
);
CREATE TABLE IF NOT EXISTS idempotency (
    project_id TEXT NOT NULL REFERENCES projects(project_id),
    request_key TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    response JSONB NOT NULL,
    PRIMARY KEY (project_id, request_key)
);

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


INSERT INTO projects SELECT 'p'||p, 1000 FROM generate_series(1,10000) p;
INSERT INTO nodes(project_id,node_id,parent_id,kind,sort_key,props)
SELECT 'p'||p, 'n'||n, CASE WHEN n=0 THEN NULL WHEN n<50 THEN 'n0'
 ELSE 'n'||(1+(n-50)%49) END, CASE WHEN n=0 THEN 'page' ELSE 'group' END,
 n, jsonb_build_object('name',repeat('x',64))
FROM generate_series(1,20) p CROSS JOIN generate_series(0,4999) n;
INSERT INTO commits SELECT 'p'||p,r,'[]'::jsonb
FROM generate_series(1,20) p CROSS JOIN generate_series(1,1000) r;
INSERT INTO idempotency SELECT 'p'||p,'key'||r,'fixture-hash','{}'::jsonb
FROM generate_series(1,20) p CROSS JOIN generate_series(1,1000) r;
INSERT INTO shared_blobs VALUES ('artifact-fixture',decode('00','hex'));
INSERT INTO render_jobs(job_id,project_id,revision,pipeline_fingerprint,input,status,artifact_id)
SELECT 'job'||j,'p1',1000,'fixture','{}'::jsonb,
 CASE WHEN j=10000 THEN 'completed' ELSE 'pending' END,
 CASE WHEN j=10000 THEN 'artifact-fixture' ELSE NULL END
FROM generate_series(1,10000) j;
ANALYZE projects; ANALYZE nodes; ANALYZE commits; ANALYZE idempotency; ANALYZE render_jobs;