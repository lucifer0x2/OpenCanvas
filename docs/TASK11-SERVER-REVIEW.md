# Task10/11 server — independent R1 correctness review

Reviewed 2026-11-04 on `feat/task11-native-png`, HEAD
`9e19d38b6a448a69e0c8f0e2255a730a5ff55e9d` plus the existing dirty integration.

**Outcome: three required corrections (P2). No optional findings.** This is not
Task11 pixel acceptance or a review of the actively changing text/paint engines.
No product source, tests, logs, other documentation, commits, or deployments were
changed by this reviewer. Temporary reproduction programs/data stayed outside
the repository; this report is the only authored repository file.

## Required findings

### R1 — SVG-only exports inherit native raster capability restrictions

- **Location:** `crates/server/src/jobs/export.rs:99–105`, specifically the new
  `painter.inspect_resources(&scene).await?` at line 101;
  `crates/server/src/jobs/painter.rs:81–101` propagates its raster capability error.
- **Confidence:** high; reproduced against both backends with a real, valid
  public COLRv1 font, not the synthetic table-renaming test fixture.
- **Contract:** Task11 preserves SVG; `docs/TASK11-NATIVE-PLAN.md:21–22,30–33`
  explicitly retains SVG and its font/image embedding path. SVG stores the
  original font bytes in `@font-face` and live `<text>`; it does not ask the Rust
  monochrome glyph painter to rasterize that font. Experimental raster limits
  and a new test expecting failure do not change this existing export contract.

**Controlled reproduction:**

1. Download the public fixture
   `https://raw.githubusercontent.com/googlefonts/color-fonts/main/fonts/twemoji-glyf_colr_1.ttf`
   into a temporary directory. The tested SHA256 was
   `50eabcbdf1edae2de36a211cea0e257d9b7195abe1d4ee8a260d6b39d82e35ba`.
2. For each backend, use a fresh temporary store; import the unmodified font as
   `color`, family `Color Probe`, weight 400, style `normal`. Create a 100×100
   frame with a 64×64 text node containing `😀`, font size 32, line height 40.
3. Start a loopback server using `Config::for_backend(..., backend)` and call
   `export_nodes` with `nodeIds:["text"]`, `settings:[{"format":"svg"}]`.
4. Download the successful file and compare its embedded base64 font with the
   imported bytes.

**Observed:** Chromium returned `ready`, a 1,735,291-byte SVG, exact original font
bytes, live emoji text, and ready font readiness. Rust returned `failed`,
`UNSUPPORTED_FEATURE`, no file, and failed font readiness. Native measurement
had succeeded; the added raster inspection rejected the color tables before
the unchanged SVG embedder ran.

**Required correction:** preserve SVG-only validity/embedding independently of
whether the native rasterizer can paint color glyphs. Keep strict missing,
corrupt, changed-resource checks and immutable bytes. This does **not** require
implementing color-font rasterization. Add the valid-font cross-backend SVG
regression case; the synthetic `native_unsupported` expectation is not an AC.
ICC-only SVG restrictions share this gate, but were not independently reproduced
and are not a separate finding here.

### R2 — Post-paint PNG work can publish a successful image after the deadline

- **Location:** `crates/server/src/jobs/runner.rs:151–163` accepts the result
  after a synchronous PNG decode outside `native::blocking`; there is no final
  deadline check. `crates/server/src/jobs/record.rs:38–42` decodes it again and
  publishes the image URL without a deadline guard. The analogous final PNG
  export conversion is at `crates/server/src/jobs/export.rs:159–162`.
- **Confidence:** high; successful late frame publication reproduced. The export
  path has the same unchecked tail, but the timed reproduction exercised frames.
- **Contract:** `docs/TASK11-NATIVE-PLAN.md:38,60–61`: drain owned CPU work and
  prevent late publication. Cooperative draining may finish after the deadline;
  returning a successful deliverable after it is a different behavior.

**Controlled reproduction:** an external Rust program used the public
`Store`, `Config`, `AppState`, and `jobs.enqueue`/`jobs.wait` APIs on a
current-thread Tokio runtime. It imported an ordinary generated 800×800 RGBA PNG
and rendered an identically sized frame/image at scale 1. RGB channels used a
deterministic xorshift32 stream (seed 17; shifts 13,17,5), alpha 255; the PNG was
encoded using `png::Encoder` defaults. Every sample had a new job table and an
idle lane. A long-deadline sample measured normal completion; the next two used
that measured duration minus 10 ms as `config.job_timeout`, repeating five
groups. Start timing immediately before enqueue, then wait for completion.

**Observed:** six shortened-deadline samples returned a ready PNG URL with no
error after the deadline. Examples:

| Job budget | Enqueue overhead | Completed/published by wait | Outcome |
| --- | --- | --- | --- |
| 1770.44 ms | 0.24 ms | 1783.24 ms | PNG, no error |
| 1770.23 ms | 0.34 ms | 1784.14 ms | PNG, no error |
| 1771.71 ms | 0.26 ms | 1782.88 ms | PNG, no error |

Enqueue overhead was below 0.4 ms, not an explanation for the 11–14 ms overrun;
the runner starts after enqueue. An earlier 1200×1200/21-sample run produced only
`JOB_TIMEOUT` for shortened budgets, correctly draining work; the gap needs a
budget close to successful completion, rather than simply an extremely short
timeout. No hard wall-clock cancellation guarantee is requested.

**Required correction:** check the native deadline after post-paint conversion
and before accepting/publishing PNG/files, retaining the existing drain policy.
Ensure the final storage/conversion tail cannot turn an expired result into a
successful download. Cover this at the runner/public-job level, not only the
existing `native::blocking` timeout unit test.

### R3 — Native frame PNG provenance drops the requested scale

- **Location:** `crates/server/src/jobs/painter.rs:57–60` copies scene provenance
  into the PNG result but never adds `scale`;
  `crates/server/src/jobs/record.rs:42` publishes that incomplete object as
  `image.provenance`.
- **Confidence:** high; reproduced at fractional and integer scales.
- **Contract:** Task11 retains result shape/provenance apart from truthful engine
  identity (`tasks/11-render-crate-native-png.md:56–58`);
  `docs/CONTRACT.md:63–66` explicitly includes scale in successful PNG metadata.
  The existing Chromium path adds `scale` in `worker/pipeline.mjs`'s render
  result. This finding concerns **`image.provenance.scale`**, not an invented
  requirement that the layout scene's provenance contain scale.

**Controlled reproduction:** create a 10×10 filled frame in a fresh temporary
store, select Rust, enqueue/render it at scales 0.3 and 2, wait and retrieve PNG
bytes. Both renders succeeded and the PNGs were downloadable, with dimensions
3×3 and 20×20 respectively. In both records,
`image.provenance` had no `scale` key. Its engine/runtime/source scene identity
survived correctly. Thus callers lose the exact render scale even though pixels
and the scale-dependent job keys differ.

**Required correction:** include the actual `f64` render scale in native PNG
provenance without disturbing source scene identity or opaque fields. Assert it
for both fractional/integer frame scales. Node export file metadata already
has scale and is not the demonstrated failure.

## Scope, checks and limits

- Read `AGENTS.md`, `PLAN.md`, Task10/11, the native plan, and relevant current
  `SERVER`/`CONTRACT` sections. Reviewed backend/config/startup/fingerprint,
  job dispatch/native resource loading/lane ownership, paint adapter, export
  resource/scope/font/SVG integration, record transitions and relevant new tests.
- Native selection precedes fingerprinting; its branch does not start/probe the
  worker. Resource reads use storage and exact manifest/bytes, not supplied URLs
  or a data-root bypass. Original bytes stay job-owned across PNG scales/SVG.
  States, source scene keys and opaque diagnostics survive the integration.
  The lane permit lives in the blocking closure; timeout drains that work.
  No additional concrete cache, crash or abandoned-lane defect was found.
- Subtree resource pruning is native-only and intentional; no separate AC
  violation is asserted merely because it differs from Chromium's resource scope.
- Ran three temporary reproduction programs linked to the existing debug server
  library `libopencanvas_server-c0ae5c737c459f22.rlib` and its matching dependencies:
  deadline probe twice (21 + 15 samples), scale-provenance probe once (2 cases),
  and real COLRv1 SVG probe once (both backends, real MCP/HTTP download).
  Compilation used `rustc --edition 2021`, `-L dependency=target/debug/deps`,
  explicit `--extern` paths, and temporary output paths. Initial standalone
  linking attempts selected mismatched serde artifacts and failed to compile;
  selecting the server fingerprint's matching artifacts resolved that harness
  issue without changing dependencies or product source.
- `git diff --check -- crates/server` passed. Inspected server module line counts:
  largest touched/new production modules are `jobs.rs` 246, `export.rs` 216,
  `startup.rs` 174, `runner.rs` 171 and `native.rs` 166; none exceeds 250.
  `wc -l -c AGENTS.md`: 69 lines / 9476 bytes; no modification.
- No Cargo suite, lint, global pixel/parity gate, proprietary font test, live
  deployment or user-data access was run by this reviewer. The root's reported
  focused checks and worker's layout results are not relabeled as my checks.
  Active text H2 / paint P2 implementations are not reviewed as final. Existing
  pixel-gate failures and Task09 speed no-go are not used as server findings.
