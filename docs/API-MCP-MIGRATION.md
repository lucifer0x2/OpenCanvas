# API → MCP project transfer — 2026-11-03

Transferred the current authored state of both projects from
`http://127.0.0.1:3847` to `http://127.0.0.1:3857/mcp`.
Source data and binaries were read exclusively through public HTTP GET APIs.
All destination project mutations used MCP `create_project`, `import_asset`
and `apply_batch` directed explicitly to port 3857. No database access or
database import was used. The source received no mutations.

## Projects

| Project | Source ID / revision | Destination ID / revision |
| --- | --- | --- |
| AXIS Courses | `p_74bc09524dc44cd99cc1c33fa6bad25c` / 206 | `p_493b127cceb1474d9c0ce8a0dae32227` / 60 |
| AXIS Workspace — Login | `p_40ef8c7b982a445e9640928a27a128fb` / 21 | `p_96bb45df27b649abb2bc04cfba803864` / 18 |

| Project | Nodes | Tokens | Styles | Notes | Fonts | Assets |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| AXIS Courses | 7,034 | 0 | 2 | 16 | 45 | 45 |
| AXIS Workspace — Login | 33 | 11 | 0 | 3 | 12 | 17 |

Every destination count equals its source count. Entity IDs and complete
properties, component references and overrides, states, sibling sort keys,
notes and resource metadata are identical.

## Verification

- Destination MCP `list_projects` confirmed an empty destination before transfer.
- Source revisions were checked before capture, after binary capture and after
  verification; they remained 206 and 21. Fresh source snapshots also equaled
  the captured snapshots.
- Full source and destination API snapshots compare equal after excluding only
  `projectId` and `revision`, sorting the six top-level entity collections by
  ID, and sorting JSON object keys. No authored fields were removed or normalized.
- All 62 source and destination asset binaries match their snapshot byte sizes
  and SHA-256 hashes, including resources not referenced by current nodes.
  The bundled font also matches its recorded SHA-256.
- Destination MCP `list_assets` and `list_fonts` returned complete matching counts;
  `get_outline` and `get_design_context` succeeded for both projects.
- Exact destination revisions rendered through MCP at scale 0.25:
  `u02-a-tile` produced a ready 310 × 440 PNG; `login_desktop` produced a ready
  360 × 225 PNG. All referenced resources reported ready. PNG downloads,
  signatures and dimensions were verified. Courses reported four text-ink
  overflow advisories; Login reported no diagnostics. No authored changes were
  made in response to renderer advisories.

Canonical normalized snapshot SHA-256:

```text
axis-courses
aa7c9e670c906085fc70ae8bf20b4ef0d1ee2c7f0ffcf885abebedf1039af9bc
axis-workspace-login
07c2dcf22898949bf1813b1c55b61550587551cbd3da8b8590f66dfc194fd8a3
```

## Scope and limits

This is a verified one-time transfer of current project state. Creating projects
through MCP generates new project IDs and new revision/commit history; historical
commits and the source idempotency history are not migrated. Current authored
data had no representational differences. This does not establish ongoing sync.

Native MCP tool handles in the active session still addressed the old server,
so destination calls used MCP JSON-RPC initialization and `tools/call` over the
explicit port-3857 endpoint. No service, connection configuration or Git state
was changed. Private API captures and binary transfer material stayed in a
restricted temporary directory and are not included in this repository.

## Follow-up: U07 renderer diagnostics

User reported ten `TEXT_CONTENT_OUTSIDE_BOX` warnings in `u07-a3-tile`.
Independent MCP renders at viewport 1240 × 4567, scale 0.25 reproduced zero
warnings on source3847 and ten on destination3857; both jobs completed ready.
The ten authored nodes match exactly, and all five referenced font hashes match.
Layout bounds, line widths and baselines match in the inspected examples, while
measured ink bounds differ between runtimes:

| Node / metric | Source3847 | Destination3857 |
| --- | ---: | ---: |
| `u07-a3-title` box/advance width | 241.09375 | 241.09375 |
| Title ink left relative to its box | -0.1280000061 | -1 |
| `u07-a3-sc-primary-l5-x` box/advance width | 49.421875 | 49.421875 |
| Same label ink width | 48.8879356384 | 50.1203002930 |

The differing ink measurement explains the destination-only diagnostics;
the underlying platform/font-rasterization cause is not yet isolated. Both
runtimes report Chromium156.0.8078.4, Node24.21.0, scene-3 and web-4.
Snapshot/binary equality therefore establishes data fidelity, not visual or
diagnostic parity across runtimes. The initial two-frame render smoke did not
cover this U07 frame. No authored design changes were made to suppress warnings.

Task21 subsequently corrected the own-box advisory policy and deployed the app
update to3857. The same frame now returns no overflow diagnostics, with PNG and
both target snapshots byte-identical to their pre-fix baselines. See
[text ink diagnostic fix](TEXT-INK-DIAGNOSTICS-FIX.md).
