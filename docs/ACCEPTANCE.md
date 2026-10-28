# OpenCanvas — bản đồ nghiệm thu

Task04 đã được root review và nghiệm thu độc lập: xem
`docs/TASK04-REPORT.md` và `artifacts/e2e/wrap.json`. Row wrap opt-in đã qua
pipeline16 tests (giữ11 cũ), storage41 tests/14 suites, server19 tests (giữ18 cũ),
production E2E16 nhóm (giữ15 cũ), strict library Clippy layout/storage/server.
Root tự chạy layout7/7, pipeline16/16, storage41/41, server19/19 và Node E2E16
nhóm trên final builds; E2E hoàn tất2026-10-28T09:11:22.774Z, cleanup=true.
Fmt, JS format/syntax, diff và size checks pass. Các kết quả cũ bên dưới là lịch sử.

Fixture mới min280/gap16/padding24:1040→3 card/1 hàng,640→2+1,390→1+1+1;
row hug hình học80/156/212, text thật128/224/320. Bounds toàn bộ descendants
nằm trong frame/card, line widths fit, không overlap; không miễn node vì clip.
Restart giữ authored wrap/revision/bounds và PNG0 RGBA differences ở cả3 viewport;
viewer GET-only0 browser errors, scene/PNG0 differences ở1040×844 và390×844.
Ảnh tại `artifacts/e2e/wrap-{1040,640,390}.png` và `wrap-viewer-{1040,390}.png`.
Root và worker đã xem PNG desktop/mobile thật. Giới hạn cũ của welcome dashboards vẫn giữ:
không author lại sidebar/body minima, fixed chart/progress hoặc fixed text heights.
Không %/breakpoint/column wrap (align/justify thêm sau ở P08a); backend/layout cần nâng đồng bộ. Real3847
không bị chạm; temp process/data được cleanup. Báo cáo chi tiết có lệnh và lint caveat.

Nguồn yêu cầu: `PLAN.md`. Tài liệu này do phiên điều phối quản lý. Nghiệm thu
2026-10-28: **MVP M0–M2 đạt trên runtime local đã ghi nhận**; M3 và M4 đạt các
phần trong phạm vi công bố dưới đây. Không suy rộng fixture thành bảo đảm mọi
browser/hệ điều hành, power-loss phần cứng hoặc tính năng ngoài scope.

| Mốc | Bằng chứng bắt buộc | Trạng thái |
| --- | --- | --- |
| M0 pipeline | Taffy + batch measurement/cache có giới hạn; hai width, Vietnamese wrap/hug; shared scene/renderer/font manifests; missing font/delayed image strict readiness | Đạt phạm vi fixture: root lượt cuối11/11 Chromium pass, font hash thật khớp; xem spikes/pipeline/RESULTS.md và docs/RESOLUTION.md |
| M0 storage | SQLite patched runtime, WAL/FULL/FK; budget trước benchmark; small/large create/update/reparent; 1/4/8 clients + reader; queue/conflict/WAL/checkpoint; correctness/restart | Đạt workload công bố; patched SQLite 3.54.1, latency/queue dưới budget; không coi số đo successful-batch là end-to-end retry latency hoặc physical writes; xem spikes/storage/README.md |
| M1 domain/storage | Create/open project guards; typed atomic ops, revision/idempotency; resource refs; durable assets; tokens/styles resolver | Root chạy full storage sau M3/M4: 38 tests / 13 suites pass, 12.66s, có fault-injection; MCP/browser xem dòng E2E |
| M1 API/MCP | Streamable HTTP MCP schemas; list/create, outline/subtree/context/resources, apply/import/render/inspect; HTTP versioned snapshot/SSE/assets/jobs; caps và client-safe errors | Đạt: root server suite 9/9 pass; actual official rmcp transport + Axum + worker Chromium. Xem docs/SERVER.md |
| M1 viewer | Light editor shell, collapsible layers/inspector, project/page/frame/viewport, read-only selection/highlight/pan/zoom/fit; scene revision rõ | Đạt: fixture frontend4 state tests/9 nhóm smoke trước đó và production E2E thật ở lượt cuối; root đã xem ảnh desktop/mobile; xem docs/VIEWER.md |
| M1 E2E | MCP → DB → viewer → PNG → restart/reload giữ IDs/refs/revision; invalid batch không commit | Đạt: Task05 mở rộng15 nhóm, root kiểm độc lập pass. RGBA0 sai khác chứng minh renderer tất định; khả năng fit/reflow báo riêng bên dưới. Snapshot/assets giữ nguyên; xem artifacts/e2e/evidence.json |
| M2 transaction/recovery | Concurrent base revision; lost-response retry/key reuse; crash trước/sau commit; asset publish/commit durable/orphan | Đạt: root full storage gồm abrupt-exit/asset recovery; server concurrent conflict + valid changed-key reuse; E2E discarded-response replay và invalid batch rollback |
| M2 events/jobs | Snapshot/replay không rơi commit; cursor reset; reconnect; old job không thay scene mới; worker failure/missing resource statuses | Đạt seam công bố: root server 9/9, pipeline11/11, live SSE/restart E2E; frontend stale/reconnect fixture trước đó. Crash/timeout orchestration dùng protocol fixture; actual Chromium kiểm render/resource. Retention lifetime, không pruning |
| M3 context/resolution | Cursor pagination, consistent revision, typed alias/theme cycle checks/invalidation; layout fixtures and diagnostics | Đạt phạm vi v1: resolver9/9, pipeline11/11, server cursor/theme suite và live token invalidation; authored/resolved tách rõ. Chưa có deep rule explanation/diff hoặc style aliases |
| M4 hardening | Versioned migration giữ DB khi lỗi; consistent backup DB + resource manifest; round-trip new project/revision reset; hash/archive/symlink guards | Đạt storage API trong phạm vi công bố: 12 M4 tests trong full storage độc lập, gồm writer/export/owned snapshots; không có GC hoặc backup UI/CLI/MCP. Xem docs/BACKUP.md |

## Các kiểm ban đầu của phiên tiếp tục — 2026-10-28

- Đã đọc working tree/checkpoint, giữ các thay đổi có sẵn. Ba worker mới tiếp tục resolver/layout fixtures, hardening và server; root chỉ điều phối/kiểm chứng.
- `resume_resolution` chạy `cargo test -p opencanvas-storage --offline --test resolution -- --test-threads=2`: **5/5 pass**. Đây là kết quả worker, chưa thay cho kiểm độc lập hoặc toàn bộ M3.
- `resume_server` ban đầu phát hiện lỗi partially moved `ReadRequest.project_id`; đã sửa, kiểm cuối bên dưới thay trạng thái ban đầu này.

## Kiểm độc lập bởi root trong phiên tiếp tục

| Command | Kết quả |
| --- | --- |
| `npm run test:pipeline` | 11 passed, 0 failed/skipped/cancelled; 2.37s; loopback escalation được duyệt |
| `cargo test -p opencanvas-storage --offline --features fault-injection -- --test-threads=2` | 38 passed / 13 suites, exit 0, 12.66s |
| `rtk bypass cargo test -p opencanvas-server --offline -- --test-threads=1` | 9 integration tests passed trong một lượt; context1 + failures3 + render2 + transport3; exit0; actual Chromium và loopback escalation được duyệt |
| `rtk bypass cargo clippy --offline -p opencanvas-storage -p opencanvas-server --lib --bin opencanvas-server -- -D warnings` | Pass, exit0; cả production libraries và server binary |
| `cargo fmt --all --check` | Pass, exit0 |
| `cargo build --offline -p opencanvas-server && node spikes/e2e/run.mjs` | Pass, exit0 trên binary cuối; 12 nhóm kiểm, completed 2026-10-28 12:16:19 Asia/Ho_Chi_Minh; cleanup project/process tạm thành công |

Worker E2E cũng đã chạy `npm run test:e2e`: build renderer/layout/server và web
production (có TypeScript check) rồi E2E pass. Root dùng các build này và rebuild
server cuối cho lượt kiểm độc lập. Không ghi dữ liệu demo vào `~/.opencanvas`.

RGBA: 640×480 revisions2/3 và sau restart so 1.228.800 kênh/lượt, 390×844 so
1.316.640 kênh, tất cả 0 sai khác. Scene/line list/provenance cũng so nguyên vẹn.
Ảnh thật đã xem: `artifacts/e2e/desktop.png`, `mobile.png`; PNG backend tại
`frame.png`. Script và giới hạn tại `spikes/e2e/README.md`.

**Giới hạn thước đo pixel:** 0 sai khác RGBA ở viewport hẹp chỉ chứng minh
renderer cho cùng kết quả trên runtime đã ghi. Nó không chứng minh mọi nội dung
nằm trong khung, thiết kế reflow, hoặc dùng được trên mobile. `clip:true` có thể
che toàn bộ phần tràn trong cả hai ảnh mà pixel vẫn bằng nhau. Kiểm scrollWidth
của viewer chỉ nói về shell trình xem, không thay cho kiểm bounds thiết kế.

Soát kích thước toàn bộ Rust/TS/TSX/JS/CSS viết tay trong crates/renderer/worker/
web/spikes: không file nào vượt250 dòng; source/tests lớn nhất242 dòng.
Generated lockfiles, `web/dist` và renderer bundle không áp giới hạn mã viết tay.
`git diff --check` pass. Server test helper dùng chung có cảnh báo dead_code ở
các test binary chỉ dùng một phần helper; production strict lint không warning.
Build web còn warning chunk ~574,59kB, không chặn build.

Giới hạn công bố: một fixture/runtime Chromium; history authored bất kỳ, state
ngoài default, hot runtime replacement, log expiration/GC và backup UI/CLI/MCP
chưa hỗ trợ. Backup/restore là Rust storage API. Token4096 được kiểm validator/
readback, chưa load-test mutation4096 tokens. Benchmark không đo physical writes
hoặc end-to-end retry latency. Không commit/push; working tree giữ thay đổi cũ/mới.

Review đã sửa và có regression: failed render cache cho phép thử lại; frame root
không mang parent page vào layout input; bundle giữ sort-key gaps hợp lệ sau
delete/reparent và nhận bundled-font copy là resource đang được tham chiếu.
Retention metadata nay được đọc trong cùng snapshot transaction.

## Quy tắc báo cáo

### Bổ sung Task 01–02 — 2026-10-28

- Task01: root server12/12 integration tests pass, gồm MCP resource list/read,
  capability, URI lỗi, tool compatibility và diagnostics toàn-frame trước
  filter/pagination. Strict production Clippy, fmt và diff-check pass.
  Xem `tasks/01-mcp-skill-layer.md` và `docs/SERVER.md`.
- Task02: root chạy `node samples/welcome-stacks/verify.mjs` pass trên data root
  tạm/cổng ngẫu nhiên, cleanup=true. Root xem PNG1040/800/390. Desktop sai khác
  **0 pixel**, MAE0; body co864→624→480 và cards248→168→120, sidebar giữ176.
  Có2 free overlays (logo/chart plot), phần còn lại dùng stacks. Viewport390
  bị clip do tổng min656; chart chưa rescale, text có thể overlap ở min.
  Báo cáo đầy đủ tại `samples/welcome-stacks/README.md`, evidence tại
  `artifacts/sample-stacks/verification.json`. Không coi đây là wrap/breakpoints
  hoặc layout mobile hoàn chỉnh. Sau QA và đồng ý của người dùng, project thật
  `opencanvas-welcome-stacks` đã tạo revision1. Live desktop PNG khớp QA0 pixel;
  viewer so PNG0/2995200 channels sai khác, API chỉ GET, không page error.
  Root đọc lại qua connected MCP và xem screenshot thật; mẫu cũ giữ revision1.
  Bằng chứng live lưu riêng tại `artifacts/sample-stacks/live/`.

### Bổ sung Task05 — nghiệm thu đa viewport, 2026-10-28

Worker chạy thật `npm run test:e2e` thành công (15 nhóm kiểm; giữ đủ12 nhóm cũ),
Chromium156.0.8078.4 / Node24.21.0 / MCP2025-11-25. Lượt sandbox đầu build
thành công nhưng bị EPERM khi bind loopback; cùng lệnh chạy lại qua approval
được duyệt và exit0. Root kiểm độc lập bounds tests3/3 và
`node spikes/e2e/run.mjs`15 nhóm đều pass; lượt root hoàn tất
2026-10-28T08:33:57.842Z, temporaryDataRemoved=true.
Kết quả máy tại `artifacts/e2e/responsive.json`; báo cáo tại
`docs/TASK05-REPORT.md`.

Mỗi mẫu được tạo qua MCP trong data root tạm/cổng ngẫu nhiên. Render và
`inspect_layout` kiểm đủ70 hậu duệ free /180 hậu duệ stacks, không tính frame
root; so cả x/y/width/height, epsilon1e-6 logical px. Không cắt bounds theo clip.
Render lại sau restart tạo job mới, giữ scene key/bounds, so decoded RGBA với
PNG trước restart tại cả ba viewport, tolerance0.

| Viewport | Free: pixel / số node tràn frame / reflow từ1040 | Stacks: pixel / số node tràn frame / reflow từ1040 |
| --- | --- | --- |
| 1040×720 | Pass0 / 0 / mốc so | Pass0 / 0 / mốc so |
| 800×720 | Pass0 / 21 / không (0/70 node đổi) | Pass0 / 0 / có (106/180 node đổi) |
| 390×844 | Pass0 / 52 / không (0/70 node đổi) | Pass0 / 104 / có (116/180 node đổi) |

Free giữ sidebar `{x:0,y:0,width:176,height:720}` và focus card
`{x:744,y:148,width:248,height:112}` tại mọi viewport. Ở390, date tràn phải238px,
greeting418px, new-task-button602px và focus602px; clip chỉ che chúng khỏi PNG.
Đây là thiết kế cố định, không phải lỗi renderer.

Stacks body co864→624→480px, stat card248→168→120px. Ở800 không node nào vượt
**frame**, nhưng10 node vượt **card weekly-chart** (chart-grid-0 vượt212px)
do plot giữ tọa độ cứng; không được gọi đây là frame overflow. Ở390, tổng
sidebar176 + body min480 =656px làm body tràn phải266px. Có104 node tràn frame;
wrap text còn có thể chồng theo chiều dọc, ngoài kiểm overflow ngang này.
Hai mẫu đều **chưa đạt thiết kế mobile**; có reflow không tự chứng minh fit.

Report lưu mọi node tràn kèm `nodeId/leftPx/rightPx/totalPx`, toàn bộ bounds,
`changedNodeIds` và các dimension thay đổi, scene/PNG provenance, dimensions,
count và pixelPass riêng. `spikes/e2e/known-overflow.json` khóa đúng từng ID và
số px từ evidence Task02 đã review; E2E không tự tạo lại allowlist. Bất kỳ node
tràn mới, mất node hoặc px khác đều làm kiểm fail để review thay đổi thiết kế.
Không bỏ qua overflow vì parent clip hay loại node.

`node --test --test-concurrency=1 spikes/e2e/bounds.test.mjs`:3/3 pass, kiểm
tràn trái/phải, root-only change không reflow và scope không được rỗng/thiếu.
Browser/MCP/backend/worker tạm đã đóng, data root tạm đã xóa; server người dùng
3847/PID12919 được giữ nguyên. Không sửa engine, sample design hay Task03.

### Bổ sung Task03 — context nhiều viewport, 2026-10-28

Root chạy độc lập full server **18/18 integration tests pass**, gồm6 tests mới
actual MCP/temp roots cho hai mẫu, full-scope bounds/pagination/cursor/compat,
cold text worker, timeout, queue và resource recovery. Strict production Clippy,
fmt và diff-check pass; code server lớn nhất233 dòng. API `viewports`1–4,
tổng16Mpx, cold read trả pending/reflow:null; đọc lại khi ready nhận bounds
theo viewport và reflow, không tính frame root vào so sánh. Legacy request giữ
tương thích. Chi phí và contract tại `docs/TASK03-REPORT.md`, `docs/SERVER.md`,
`docs/CONTRACT.md`. Server thật3847 không restart nên chưa tải delta03.

- Test phải đi qua seam thật; mock không thay bằng chứng browser worker hoặc SQLite durable.
- Không lấy ảnh đẹp làm bằng chứng persistence/events. Scene line lists và provenance phải kiểm riêng.
- Root data tests tạm, không dùng project thật trong `~/.opencanvas`.
- Phân biệt chưa triển khai, chưa kiểm, thất bại và đã đạt. Không ghi toàn bộ milestone đạt chỉ vì một happy path xanh.
- Ghi command, runtime, fixture, kết quả, giới hạn vào báo cáo của task; giữ nhật ký tóm tắt trong `docs/agents/LOG.md`.

### Bổ sung P08a — stack align/justify, padding từng cạnh (đợt đêm 2026-10-28)

AC trong `.night/proposals/P08a-stack-align-padding.md`. Kiểm: `cargo test -p opencanvas-layout`
(`stack_tests`, `stretch_tests`), `-p opencanvas-storage --test stack_layout`, `-p opencanvas-server
--test stack_context`, `npm run test:pipeline` (`stack-align.test.mjs`). Không `alignSelf`,
`space-around/evenly`, baseline, margin, %/breakpoint/column wrap (P08b và sau).
