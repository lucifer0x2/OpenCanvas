# U07 text ink diagnostics — Task21

## Diagnosis

The immutable `u07-a3-tile` scene on Kubernetes3857 at Courses revision60,
viewport1240×4567 and scale0.25 reports ten own-box ink warnings. Every affected
line's advance matches both horizontal box edges exactly; no actual clip is
crossed and no advance exceeds its box. Source3847 revision206 has matching
authored nodes/resources and layout but different Canvas ink extents. Identical
font bytes and Chromium/Node versions do not imply identical cross-OS ink metrics.

The prior diagnostic treated a snug advance box like a clipping boundary, using
the same0.5px allowance for both. Canvas glyph ink can extend beyond advance
width. Platform raster metrics/glyph bearings are consistent with these small
differences; a specific font hinting mechanism has not been established.

Measured destination geometry (all dimensions in scene pixels; IDs have prefix
`u07-a3-`; ink x is relative to the advance origin):

| Node | Box = advance width | Ink x | Ink width | Excess edge |
| --- | ---: | ---: | ---: | ---: |
| title | 241.09375 | -1 | 241.088394165 | left1 |
| sc-primary-l5-x | 49.421875 | 0 | 50.120300293 | right0.698425293 |
| sc-secondary-l4-x | 48.890625 | 0 | 49.592300415 | right0.701675415 |
| sc-accent-l0-x | 48.0625 | 0 | 48.752319336 | right0.689819336 |
| sc-accent-l6-x | 47.265625 | 0 | 47.960296631 | right0.694671631 |
| sc-accent-l9-x | 47.265625 | 0 | 47.960296631 | right0.694671631 |
| sc-neutral-l0-x | 46.609375 | 0 | 47.312316895 | right0.702941895 |
| sem-danger-l2-x | 51.09375 | 0 | 51.620330811 | right0.526580811 |
| tok0-x | 46.609375 | 0 | 47.312316895 | right0.702941895 |
| bn-success-ig | 9.40625 | 0 | 10 | right0.59375 |

## Correction

`text_diagnostics/overflow.rs` permits at most1 horizontal scene pixel of ink
overhang only when both advance edges match the own box within the existing
measurement grid1/64px. This is a bounded advisory policy for snug measured
advance boxes, not a Canvas precision guarantee or an assertion that no pixel
exists outside the box. It applies independently of export scale. A fixed box
with the same snug measured geometry receives the same policy; no authored
sizing-mode inference is needed.

Advance checks, vertical ink checks and actual own/ancestor clipping retain
their0.5px threshold. Thus a0.6px ink overhang in a clipping box still reports
the clip even when the own-box advisory is clear. Greater-than1px ink overhangs
still warn. Measurement, rendering and authored data are unchanged.

## Source verification

- New regression failed on the original code for U07 and own-clip policy cases.
- `cargo fmt --all --check` passed.
- `cargo clippy --offline -p opencanvas-server --lib -- -D warnings` passed.
- `cargo test --offline -p opencanvas-server text_diagnostics --lib -- --test-threads=2`
  passed14 tests. Exact ten numerical cases, substantial left/right ink,
  advance overflow, non-snug/shifted edges, vertical overflow, subpixel own and
  ancestor clips, absent ink and hidden nodes are covered. Existing tests retain
  intentional severity, budgets, omission summaries and provenance coverage.
- Source modules remain focused: overflow132 lines, regression tests93,
  module registration13. No renderer source changed.

## Deployment verification — 2026-11-03

Coordinator reviewed the source and independently reran the14 diagnostics tests
before deployment. `bash deploy/kubernetes/local.sh build` passed with cached
renderer/viewer layers; only the Rust server needed compilation. The reviewed
image digest is `7bb75a6cb02d9e91e6b3f8f3deda619b6d3251a9c10c2fd85104c1a8fef781e8`.
`bash deploy/kubernetes/local.sh restart-app` completed; no manifest apply was
performed. The existing public overlay/origin and Service ports were retained.
The PostgreSQL Pod UID, StatefulSet spec, PVC UID and Secret UID are unchanged.
The Deployment spec is identical except its restart annotation. Both loopback
forwards3857/3858 were restored with `public-viewer.sh access` after replacement.

Fresh explicit3857 MCP `render_frame` at the original revision/viewport/scale:

- Ready job `job_ffcd4c7de6d8479497550f83f0df46e8`, diagnostics `[]`; all five
  referenced fonts ready. The ten warnings and overflow summary are absent.
- Scene nodes, ink measurements, fonts and assets exactly match the prior
  immutable measured scene. No geometry or design content was changed.
- PNG146169 bytes, bit-identical before/after, SHA256
  `b27894125f5a28a3435372fd56677e0f9aa3d7330d03e969423a2d48d4aee5df`.
- Source Courses/Login raw snapshots unchanged at revisions206/21; destination
  Courses/Login raw snapshots unchanged at revisions60/18. All62 destination
  asset downloads plus each project's bundled font match their predeploy hash.
- Public3858 still reports `readOnly:true`, rejects an untrusted Host with403,
  and serves the identical U07 PNG with the configured public Host/Origin.

No native service restart, authored mutation, DB manipulation, commit or push
was performed. Private captures and generated PNG stayed outside the repository.
This verifies the reported U07 frame and regression cases; it does not establish
pixel parity between operating systems or identify the exact rasterizer cause.
