# Browser WASM scene acceptance

See [the checked parity and timing report](../../docs/VIEWER-WASM-PARITY.md)
for the complete 20-case matrix, exact gate/exclusions, runtime, original font
pins and reproduction commands. Native scenes come from real MCP jobs backed
by a temporary SQLite store; browser scenes run in a pinned Chromium Worker.

- `run.mjs`: loopback artifact server, fresh browser Workers and result ledger.
- `worker.mjs`: release WASM font preparation, first compute and 20 warm calls.
- `compare.mjs`: exact whole Scene comparison and explicit provenance exclusions.
- `compare.test.mjs`: zero-tolerance, order, diagnostics and contract gate probes.

Generated data belongs only in ignored `artifacts/viewer-wasm-scene/`.
Do not commit captures, JSON/logs, downloaded fonts or generated WASM/glue.
