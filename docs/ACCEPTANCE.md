# OpenCanvas — bản đồ nghiệm thu

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
| M1 E2E | MCP → DB → viewer → PNG → restart/reload giữ IDs/refs/revision; invalid batch không commit | Đạt: root E2E 12 nhóm kiểm, exact RGBA 0 sai khác desktop/mobile/restart; snapshot/assets giữ nguyên; xem artifacts/e2e/evidence.json |
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

- Test phải đi qua seam thật; mock không thay bằng chứng browser worker hoặc SQLite durable.
- Không lấy ảnh đẹp làm bằng chứng persistence/events. Scene line lists và provenance phải kiểm riêng.
- Root data tests tạm, không dùng project thật trong `~/.opencanvas`.
- Phân biệt chưa triển khai, chưa kiểm, thất bại và đã đạt. Không ghi toàn bộ milestone đạt chỉ vì một happy path xanh.
- Ghi command, runtime, fixture, kết quả, giới hạn vào báo cáo của task; giữ nhật ký tóm tắt trong `AGENTS.md`.
