-- Additive, idempotent upgrade for existing spike fixtures. Existing projects
-- start uncertified and must pass full candidate validation before delta use.
ALTER TABLE projects ADD COLUMN IF NOT EXISTS generation TEXT NOT NULL DEFAULT gen_random_uuid()::text;
ALTER TABLE projects ADD COLUMN IF NOT EXISTS validated_revision BIGINT;
ALTER TABLE projects ADD COLUMN IF NOT EXISTS validator_version TEXT;
