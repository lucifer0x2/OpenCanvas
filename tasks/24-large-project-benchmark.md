# Task24 — Large-project baseline benchmark (Phase 0)

**Status:** assigned 2026-11-12. Requested by the user. Measurement only; no
product behaviour change. Context: [LARGE-PROJECT-PROPOSAL](../docs/LARGE-PROJECT-PROPOSAL.md).

## Why

Projects must grow to hundreds of thousands of nodes. Today everything is
whole-project (load, validate, expand, snapshot GET, viewer fetch) and capped at
20,000 nodes. Before redesigning storage per page, measure where time and memory
go as a real project grows, so Phase 1/2 target the right costs.

## Source data (read-only)

- Use a real design from the native runtime, not synthetic content. Primary
  source: project slug `axis-training` (≈6.7k authored nodes, 19 pages, ≈1.7k
  instances, ≈120 tokens, ≈90 styles, themes, fonts, assets). Optional second
  source: `axis-courses` (≈7k nodes, few instances, many assets).
- Read it only through the native loopback HTTP API on `127.0.0.1:3857`
  (`GET /api/v1/projects/{id}/snapshot`, font/asset GET routes if needed).
  Never write to the native server, PostgreSQL 2703, `~/.opencanvas` or the
  preview on 3877. Never call MCP mutation tools against native.
- The fetched snapshot is private user data: store it only in an OS temp dir or a
  gitignored path, never commit it, never paste its content into reports.

## Scaled fixtures

Build projects in **temporary storage only** (temporary SQLite data root and a
disposable PostgreSQL, e.g. `scripts/task19-check.sh` or a temp native cluster
on a random loopback port, as earlier tasks did).

- Clone the source project's pages N times with deterministic ID rewriting (all
  node IDs, parent IDs, instance `componentId`/override keys, links/flowStart,
  note targets) so references stay valid. Tokens/styles/fonts/assets stay
  shared (one copy), as in a real large project.
- Scales: 1× (source), ≈30k, ≈100k, ≈200k authored nodes. Report authored and
  expanded counts for each.
- Variant for the instance question: report expanded count per scale; if
  feasible, one variant where cloned pages reuse the original masters (instances
  only) to separate authored vs expanded growth.
- The 20,000 caps must be bypassed only for the benchmark: a cargo feature or
  test-only override that the default build does not enable. Default behaviour
  and tests must be unchanged.

## What to measure (per scale, SQLite and PostgreSQL)

1. Snapshot cold load from storage; PostgreSQL Moka warm hit.
2. HTTP `GET /snapshot`: response bytes (object and `?nodeFormat=rows`, plus
   gzip size), serialization time.
3. Commit, strict mode: one small `update_node` text change, end to end, with a
   breakdown: `Candidate::from_snapshot`, apply, `validate()` (per theme and
   `validate_instances`/`instances::expand`), persist, cache fill.
4. Commit, scoped mode (`pageIds` of one page): same change and breakdown.
5. Reads: `instances::expand` whole project; `find_nodes` (name/text contains,
   project-wide); `get_outline` one page; `get_design_context` one frame.
6. Viewer cost: `JSON.parse` of the snapshot + `nodeIndex`/`snapshotNodeIndex`
   build in Node (and Chromium heap if cheap), to show the browser cost of
   fetching the whole project.
7. Peak RSS of the server/test process per scale.

Warm-up then ≥5 samples per cell; report median and p95. Release build; record
machine, toolchain, HEAD and limits as `docs/RUNTIME-STORAGE-BENCHMARK.md` does.

## Scope and outputs

- Workspace: `/Users/luis/Workspace/OpenCanvas`, branch `feat/project-node-limit`.
  Do not commit or push.
- Code: a small benchmark harness (e.g. `spikes/large-project/` and/or a release
  integration test plus a `scripts/` wrapper). Modular files, ideally ≤200 lines.
- Generated samples/logs/snapshots are ignored (see
  `spikes/realtime-db/evidence/README.md`); version only source, the rerun command
  and the report.
- Report: `docs/TASK24-REPORT.md` — method, scales, tables, which costs grow
  fastest, and concrete implications for Phase 1 (page-scoped reads) and Phase 2
  (page-partitioned storage/validation). Note anything that failed or was skipped.

## Acceptance

- Native runtime, PostgreSQL 2703, `~/.opencanvas` and 3877 untouched (only GETs
  to 3857).
- Default build still enforces 20,000; `cargo test` of touched crates and
  clippy pass; `git status` shows no private data or generated evidence.
- Report tables cover every scale × backend × item above, or state why not.
