-- Additive spike indexes; primary keys already cover project/node/revision/key reads.
-- Referencing side of the deferred composite node-parent FK (delete/reparent checks).
CREATE INDEX IF NOT EXISTS nodes_parent ON nodes(project_id, parent_id);
-- Actual artifact authorization lookup; pending/unrendered jobs need no entries.
CREATE INDEX IF NOT EXISTS render_jobs_artifact ON render_jobs(artifact_id)
    WHERE artifact_id IS NOT NULL;
