CREATE TABLE IF NOT EXISTS projects (
    project_id TEXT PRIMARY KEY,
    revision BIGINT NOT NULL DEFAULT 0 CHECK (revision >= 0),
    generation TEXT NOT NULL DEFAULT gen_random_uuid()::text,
    validated_revision BIGINT,
    validator_version TEXT
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
