-- Additive schema v2. Existing v1 gateway explicitly selects known columns.
ALTER TABLE projects ADD COLUMN IF NOT EXISTS description TEXT NOT NULL DEFAULT '';
CREATE TABLE IF NOT EXISTS spike_schema_versions (
  version INTEGER PRIMARY KEY,
  installed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO spike_schema_versions(version) VALUES(2) ON CONFLICT DO NOTHING;
