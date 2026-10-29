# Motion preview — report (2026-10-29)

Branch `claude/motion-preview` (from `30cc7ff`), worktree `OpenCanvas-motion-preview`.
Behavior and semantics: `docs/VIEWER.md`, section "Motion preview".

## Files

Created (`wc -l`):

| File | Lines | Responsibility |
| --- | ---: | --- |
| `web/src/preview/keyframes.ts` | 203 | Pure spec: animation definition + resting values → WAAPI keyframes/timing |
| `web/src/preview/tokens.ts` | 87 | Pure token/fill-style resolution at the default theme |
| `web/src/preview/types.ts` | 60 | Authored animation/link/token types, state names |
| `web/src/preview/model.ts` | 90 | Pure snapshot reads: root frames, frame size, declared states (incl. instance masters), animations/links per node |
| `web/src/preview/paintStyle.ts` | 61 | Pure CSS declarations for fill, radius, opacity, stroke overlay |
| `web/src/preview/stateStyles.ts` | 95 | Pure diff of state scenes vs default → `!important` state rules |
| `web/src/preview/sceneDom.ts` | 141 | Scene → absolutely positioned DOM (SVG text lines, images, stroke overlay) |
| `web/src/preview/motion.ts` | 172 | Event wiring: load/Replay, hover forward/reverse, click restart, links, reduced motion |
| `web/src/preview/usePreviewData.ts` | 126 | Snapshot fetch; default + declared-state scenes and resources |
| `web/src/preview/usePreviewUi.ts` | 38 | Reduced-motion preference hook, toast hook |
| `web/src/preview/route.ts` | 45 | `/preview` URL parse/build, history depth |
| `web/src/preview/PreviewStage.tsx` | 111 | Mounts DOM + stylesheet + motion, fit scale, disabled toggle |
| `web/src/preview/PreviewApp.tsx` | 163 | Shell: frame picker, Replay, reduced motion, link navigation, warnings |
| `web/src/preview/preview.css` | 135 | Preview shell and scene DOM styles |
| `web/src/sceneJob.ts` | 66 | Shared scene request + job polling (500 ms, 45 s) for any state |
| `web/src/components/PreviewButton.tsx` | 17 | Viewer header button opening the preview |
| `web/tests/preview-keyframes.test.ts` | 202 | Unit tests of the keyframe spec |
| `web/tests/preview-model.test.ts` | 95 | Unit tests of declared states and state rules |
| `web/tests/preview-fixture.mjs` | 143 | MCP fixture project |
| `web/tests/preview-parity.mjs` | 80 | DOM vs worker PNG pixel comparison |
| `web/tests/preview-smoke.mjs` | 187 | Playwright smoke against a temporary server |

Modified: `web/src/main.tsx` (12, route switch), `web/src/App.tsx` (236, +5: Preview button),
`web/src/hooks/useScene.ts` (102, now uses `fetchSceneJob`, same messages), `web/src/state.ts`
(197, `acceptsJob` takes an optional `state`, default `"default"`), `web/package.json` (test
files, `test:preview`), `docs/VIEWER.md` (200), rebuilt `web/dist` (tracked).

No file is over 250 lines.

## Commands run

| Command | Result |
| --- | --- |
| `cargo build --offline -p opencanvas-layout -p opencanvas-server` | pass (worktree `target/debug`) |
| `npm run build:renderer` | pass |
| `npm run check:renderer` | pass (renderer untouched) |
| `npm --prefix web run build` | pass (tsc + vite; existing >500 kB chunk warning, 601.8 kB) |
| `npm --prefix web test` | pass, 20/20 (10 keyframes, 2 model/state-style, 8 existing) |
| `npm --prefix web run test:preview` | pass, 8 checks: load (fill both, offset-0 during delay, rest at end, token fill on text); resting DOM vs worker PNG 0.901% pixels differ >48/255; hover holds scale 1.1 + state fill `rgb(255,0,0)`, reverses (rate −1, running) and returns to rest; pressed `:active` opacity 0.6; click restarts; navigate updates `?frame=second`, back returns; reduced motion; GET-only, no page errors; viewer Preview button opens `/preview?project=…&frame=home` |
| `npm --prefix web run test:browser` (existing viewer smoke) | pass (exit 0, after the `useScene` refactor) |

The smoke starts `target/debug/opencanvas-server --port 0 --data-root <mkdtemp> --workspace <worktree>`
through `spikes/e2e/server.mjs`, stops the process group and deletes the data root in
`finally`. Ports 3847/3848 and `~/.opencanvas` were not used.

## Deviations from the design brief

- **One CSS property per channel instead of one `transform` string.** The validator gives
  `translate`, `scale` and `rotate` separate owners, so two animations on one node may animate
  different channels; a shared `transform` would let the later one clobber the earlier. The
  unit test asserts the three channels land in one keyframe as `translate`/`scale`/`rotate`.
- **Images** use an `<img>` with the scene asset URL positioned by the shared `placeImage`
  (crop included) rather than CSS `background-image`; fonts register through the renderer's
  `loadResources` (FontFace) instead of a generated `@font-face`, matching the canvas viewer.
- **Text** is SVG `<text>` at the scene baselines (exact canvas `fillText` positions).
- Hover animations are forced to 1 iteration, `normal`, `both` (the validator already forbids
  those keys on hover).
- A **Disabled state** toolbar toggle exposes the disabled variant (no pointer trigger exists).

## Opacity/fill vs states

State rules are `!important`, so they beat resting inline styles and animation values of the
same property while the state is active; animations keep owning translate/scale/rotate, and an
animated opacity/fill shows again when the state ends. Animated `opacity` replaces the resting
opacity on the same element (spec: "replaces the authored opacity"), no multiplication.

## Known limitations

Default theme only; `overlay`/`close` links show a toast, hover links a warning; states are CSS
pseudo-classes (child hover also hovers ancestors; focus needs keyboard focus); no live SSE
follow (Reload button); URL `width`/`height` persist across navigation. P24: state visuals of
virtual instance children come from the state scenes by scene ID; component subtrees and
instances cannot author animations or links, so no further mapping is needed. The SPA fallback
serves `/preview` through `ServeDir::not_found_service`, so the HTML response carries a 404
status (checked with curl on a temporary server); browsers render it normally (the smoke loads it).
