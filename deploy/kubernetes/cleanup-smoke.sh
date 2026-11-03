#!/usr/bin/env bash
# Removes ONLY a fixture produced by docker/kubernetes-smoke.mjs in this local
# namespace. The product has no delete-project API. Never use for real projects.
set -euo pipefail
[ "$#" -eq 1 ] || { echo 'Usage: bash deploy/kubernetes/cleanup-smoke.sh STATE.json' >&2; exit 2; }
read -r project_id slug < <(node --input-type=module - "$1" <<'JS'
import { readFile } from 'node:fs/promises';
const fixture = JSON.parse(await readFile(process.argv[2], 'utf8'));
if (!/^p_[a-f0-9]{32}$/.test(fixture.projectId) || !/^kubernetes-smoke-\d+$/.test(fixture.slug))
  throw new Error('Refusing a state file without a generated smoke identity');
console.log(fixture.projectId, fixture.slug);
JS
)
# psql variables are SQL-quoted with :'name'; no shell interpolation in SQL.
# Both exact ID AND exact generated slug must match. FK cascades remove only its
# entities, binaries, commits and idempotency records; schema/PVC are preserved.
deleted=$(kubectl --context orbstack -n opencanvas-local exec -i postgres-0 -- \
  psql -X -At -v ON_ERROR_STOP=1 -U opencanvas -d opencanvas \
  --set=project_id="$project_id" --set=slug="$slug" <<'SQL'
WITH removed AS (
  DELETE FROM opencanvas_runtime_v1.projects
  WHERE project_id = :'project_id' AND slug = :'slug'
  RETURNING project_id
) SELECT count(*) FROM removed;
SQL
)
[ "$deleted" = 1 ] || { echo "Expected exactly one smoke project; removed $deleted." >&2; exit 1; }
echo "Removed disposable fixture $project_id ($slug)."
