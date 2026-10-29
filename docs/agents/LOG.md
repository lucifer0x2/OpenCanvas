# Nhật ký điều phối OpenCanvas

### Commit và push viewer/design memory — 2026-10-29

- Người dùng cho phép commit/push nhánh hiện tại và yêu cầu ghi nhận contributor Codex. Chốt phạm vi viewer actions/custom select/export, nghiên cứu design rules và English design-memory instructions đã kiểm; commit có trailer `Co-authored-by: OpenAI Codex <noreply@openai.com>`. Giữ các ghi chú LOG đã có từ phiên khác ngoài commit, không sửa author Git hoặc lịch sử cũ. Diff whitespace sạch; các kiểm chứng và release build đã đạt ở hai mục task dưới đây.

### Server instruction cho design memory — 2026-10-29

- Người dùng chốt thêm instruction bằng tiếng Anh: `AGENTS.md` riêng theo project là memory cho đọc/chỉnh sửa design, không dùng cho code/build/test hoặc công việc khác. Giao fresh worker sửa instruction và tài liệu MCP trong phạm vi nhỏ; giữ các thay đổi viewer đang có, không commit/push hoặc chạm dữ liệu/server thật.
- Nhận kết quả: CONNECT và hai skill MCP có policy English đọc/tạo/cập nhật memory từ quyết định design đã xác nhận, ưu tiên memory project trong phạm vi design. Không bịa công cụ memory MCP; khi không có authorized file access thì báo limitation. Worker kiểm MCP skill_layer/protocol_2026 4/4, rustfmt/diff sạch; ba file 86/168/157 dòng. Root đã đọc đối chiếu, đang kiểm độc lập và build release.
- Điểm dừng: root chạy độc lập skill_layer/protocol_2026 4/4 đạt; release build thành công. `AGENTS.md` 64 dòng/7.861 byte, mọi file sửa task này dưới250 dòng. Chưa commit/push hoặc restart server3847; cần restart server và reconnect MCP để client nhận instruction mới.

### Build toàn bộ app và launch scripts — 2026-10-29

- Người dùng yêu cầu root `package.json`: `build` toàn app debug, `build:release` toàn app release, `dev` chạy kết quả build debug và `start` chạy kết quả build release. Không dùng dev server FE cho hai lệnh chạy này.
- Giao fresh worker thêm scripts và runtime profile wiring cần thiết (layout worker/fingerprint đang mặc định debug), cập nhật README và kiểm debug/release trên server/data tạm. Giữ nhánh P24 hiện tại, không push hoặc chạm server/data thật.
- Nhận kết quả: 4 scripts build/run toàn app, launcher pin server/layout cùng profile và fingerprint băm layout được chọn. Worker chạy đủ hai build, hai live smoke, CLI/error/shutdown probes và Clippy/format đạt; chi tiết ở `docs/APP-BUILD-REPORT.md`.
- Root kiểm độc lập `dev` và `start` với target tạm chỉ có đúng profile: compiled viewer/assets HTTP200, MCP text → PNG320×200 đạt, data/process tạm đã dọn. Dọn symlink dependency QA, commit trên nhánh hiện tại, không push.

### P24 merge main — 2026-10-29

- Fetch `origin/main` @ `f901a31` (merge P27); rebase gặp conflicts. Theo yêu cầu mới của người dùng, đã abort rebase hoàn toàn, giữ commit P24 `236d116`, chuyển sang merge `origin/main` trên nhánh `codex/p24-components-instances`; không rebase tiếp.
- Merge có 5 content conflicts; giao fresh worker xử lý và kiểm integration P24/P27, giữ cả instance và interaction states. Root điều phối/kiểm chứng; không push hoặc chạm server/data thật.
- Nhận kết quả: giữ schema4/instance và P27 states; thêm regression và sửa `resolvedStates` trong component definitions, bảo toàn `visible:false` khi state đổi opacity. Root kiểm độc lập server44/44 và storage16/16; worker strict library Clippy/fmt đạt. Báo cáo: `docs/P24-MERGE-REPORT.md`.
- Chốt merge `f901a31` vào nhánh P24, giữ lịch sử commit `236d116`, dọn 4 symlink dependency tạm, tạo merge commit; không rebase hoặc push.

### P24 — 2026-10-29

- Người dùng yêu cầu triển khai P24 sau khi lấy main mới, trên nhánh riêng. Đã fetch `origin/main` @ `a0d7743`, tạo worktree riêng và nhánh `codex/p24-components-instances`; giữ nguyên checkout P27 có thay đổi đang làm.
- Giao fresh worker triển khai P24a+b theo `docs/recommend-features/tasks/P24-components-instances.md`, gồm storage/expand, MCP reads và kiểm AC. Root điều phối và kiểm chứng; không push, không chạm dữ liệu/server thật.
- Worker báo MCP/Chromium kiểm pixel, master invalidation, byte budgets và virtual reads đạt; root storage instances 4/4 pass, migration 2/4 còn assertion cũ schema3 cần cập nhật4 (base main đã dùng3 cho notes). Baseline MCP context6900 nodes đã đo trên main. Worker bị ngắt do model capacity, đã yêu cầu tiếp tục cùng task để hoàn tất kiểm và report.
- Root kiểm độc lập MCP/Chromium P24 3/3 pass. Worker tiếp tục bị capacity lần hai; giao fresh worker hoàn tất regression/migration/docs/lint/report từ diff hiện có, giữ nguyên nhánh/worktree.
- Nhận kết quả hoàn tất: `docs/P24-REPORT.md` và `artifacts/p24/` ghi đủ AC, schema4/legacy compatibility, pixel equality, byte budgets và hai workload latency. Root kiểm độc lập 22 storage tests (qua các lượt), MCP cuối 4/4, fmt/diff và benchmark mở rộng thực 6.900 descendants đạt; worker kiểm thêm server liên quan 19/19, unit/golden 35/35 và strict library Clippy.
- Chốt nghiệm thu P24 trên `codex/p24-components-instances`, lưu benchmark tái lập trong `spikes/p24/context-benchmark.mjs`; dọn các symlink dependency tạm, commit nhánh riêng, không push. Không đổi main, checkout P27 hoặc server/data thật.

Append-only. Cách ghi theo `AGENTS.md` mục "Nhật ký và memory": mỗi task một heading có ngày, mỗi lần ghi vài dòng (quyết định, kết quả cuối, blocker, link báo cáo).
Lịch sử đến hết Task01–08 và commit `215313c`: `docs/agents/history/2026-10-28.md`.

### Design rules và thao tác phần tử viewer — 2026-10-29

- Người dùng yêu cầu nghiên cứu design rules Figma dùng chung cho nhiều AI/phiên; bổ sung export phần tử dễ tìm, bỏ chọn khi bấm nền, custom select và context menu export/copy link, hỗ trợ Cmd/Ctrl+C.
- Giao ba fresh worker: nghiên cứu tài liệu design rules; custom select toàn viewer/preview; thao tác phần tử, deep link và export UI/API. Dùng checkout hiện tại `codex/p24-components-instances`, phân quyền file để tránh xung đột; không commit/push, không restart server hay chạm dữ liệu thật. Root kiểm chứng và cập nhật tiến độ.
- Nhận nghiên cứu: [đối chiếu Figma](../DESIGN-RULES-RESEARCH.md) và [protocol hai AI](../DESIGN-RULES.md). Có token/style/font/master và notes; chưa có policy theo project tự nạp/enforce. Protocol là draft cần manifest thực của project, không tự chọn palette/font. Root đọc đối chiếu tài liệu; worker kiểm links/Markdown sạch.
- Nhận custom selects: toàn bộ viewer/preview dùng combobox chung, portal/focus/keyboard/typeahead/empty/activity giữ hành vi; cập nhật E2E consumers. Root chạy độc lập `test:browser` đạt, gồm SSE/selection/zoom, các thao tác select và màn hình hẹp; kiểm screenshot không thấy clipping.
- Nhận node actions: Inspector/context menu có Copy link và Export PNG/SVG; deep links stable IDs gồm virtual children, Cmd/Ctrl+C giữ copy văn bản gốc; nền bỏ chọn/blur kể cả pending/failed và qua SSE. HTTP export dùng cùng schema/pipeline MCP, Vite proxy đã kiểm với backend thật. Worker chạy 9 nhóm browser đạt và dọn server/data tạm.
- Root kiểm độc lập web unit 23/23, typecheck, viewer browser; backend export_nodes/export_svg/viewer_exports/skill_layer 7/7 đạt. Đã xem screenshot context menu/Inspector; mọi file thay đổi ≤250 dòng. Build viewer và server release đạt (cảnh báo bundle >500KB); server3847 đang chạy release cũ, chưa restart. Đang chạy lượt node-actions browser độc lập cuối.
- Điểm dừng: lượt node-actions browser độc lập cuối đạt đủ 9 nhóm, gồm real Vite/PNG/SVG, pending/failed, virtual child và clipboard; server/cổng/data tạm đã dọn. Diff whitespace sạch; `AGENTS.md` 64 dòng/7.861 byte. Chưa commit/push, chưa restart server3847; cần restart bản release đang chạy để có endpoint export và MCP instructions mới.
- Trao đổi tiếp về memory chung tại `<data-root>/<project>/AGENTS.md`: source MCP hiện chỉ có resource tĩnh, chưa đọc file theo project. Hướng đề xuất là backend cấp nội dung/version qua MCP cho mọi client; phân biệt quy tắc agent với validator enforcement. Chưa triển khai hoặc tạo file trong dữ liệu thật.

### Sửa inputSchema render_frame cho Claude Code — 2026-10-28

- Người dùng báo phiên khác thêm MCP vào Claude Code nhận "Connected · tools fetch failed": `render_frame` công bố `inputSchema` chỉ `{"oneOf":[...]}` (`crates/server/src/schemas.rs:141`), thiếu `"type":"object"` ở cấp ngoài; 14 tool còn lại dùng helper `object(...)` nên đúng. Anthropic API cũng không nhận `oneOf/anyOf/allOf` ở cấp ngoài, nên chỉ thêm `type` là chưa đủ.
- Hướng chốt: schema công bố là object phẳng (hợp field hai mode, required `projectId`, additionalProperties false); validation runtime giữ schema hai-mode `oneOf` hiện có để hành vi và mã lỗi không đổi; regression test duyệt toàn catalog. Giao fresh worker một task, phạm vi `crates/server/src/schemas*`, test mới, `docs/SERVER.md`.
- Server thật của người dùng đang chạy từ checkout này (PID 8057, cổng 3847, `/Volumes/MySpace/Workspace/OpenCanvas`); worker không được kill/restart hay ghi vào nó, kiểm live bằng server tạm `--port 0` + data root tạm. Checkout chưa có `.runtime/browsers`; worker không tự tải Chromium.
- 21:15: người dùng cập nhật memory chung `$CLAUDE_CONFIG_DIR/CLAUDE.md` (Modular Architecture Rules, 21:14). Root đã đọc lại; đã gửi worker đang chạy yêu cầu đọc lại file đó và báo số dòng/trách nhiệm module/file vượt ngưỡng trong báo cáo cuối.
- Kết quả: `schemas/render.rs` (98 dòng) công bố object phẳng, chỉ bắt buộc `projectId`; validate vẫn dùng `oneOf` hai mode nên mã/message lỗi giữ nguyên. Test mới `tests/tool_schemas.rs` (103 dòng); `schemas.rs` 159 dòng; `docs/SERVER.md` ghi quy tắc schema. Root kiểm độc lập: lib 9, tool_schemas 2, transport 4, render_wait 5 pass; strict Clippy/fmt/diff-check sạch; `tools/list` live trên server tạm: 15 tool đều `type:"object"`, không combinator ở cấp ngoài cùng. Worker báo full suite 50 pass/10 fail, 10 test fail đều cần Chromium (checkout chưa cài browser), chưa chạy lại. Server 3847 vẫn chạy binary cũ, người dùng cần restart. Chưa commit.
- Người dùng yêu cầu cài Chromium: `npx playwright install chromium` với `PLAYWRIGHT_BROWSERS_PATH=$PWD/.runtime/browsers` (Chromium/headless shell 156.0.8078.4, `.runtime/` đã gitignore). Root chạy lại 10 test cần browser (render 2, wrap 1, inspect_paging 2, multi_resources 2, multi_viewport 2, prerender_targets 1): 10/10 pass.

### Tối ưu AGENTS.md — 2026-10-28

- Subagent rút `AGENTS.md` 227 dòng/48,9 KB → 65 dòng/7,7 KB, thêm quy tắc ghi nhật ký/memory và bảng tham chiếu; lịch sử chuyển nguyên văn sang `docs/agents/history/2026-10-28.md`, nhật ký mới ở file này. Root kiểm `grep -Fxv`: chỉ 2 dòng không còn nguyên văn (tiêu đề và dòng "Cập nhật nhật ký…" đã thay có chủ đích). Con trỏ đã sửa ở `docs/ACCEPTANCE.md:173` và `tasks/05`. Chưa commit.

### MCP 2026-07-28: ttlMs/cacheScope — 2026-10-28

- Phiên "Kết nối MCP OpenCanvas" dùng proxy ghi log, thấy Claude Code CLI 2.1.300 gọi `server/discover` với giao thức 2026-07-28. rmcp 3.4.0 mặc định quảng bá 2026-07-28, nhưng OpenCanvas để `ttl_ms`/`cache_scope` là `None` (`..Default::default()`) trong `tools/list`, `resources/list` và `resources/read`. Handler mặc định `resources/templates/list` cũng vậy. Bản 2026-07-28 bắt buộc hai trường này (SEP-2549), nên client từ chối toàn bộ danh sách. Phiên desktop dùng giao thức cũ vẫn nạp được 15 tool.
- Hướng chốt: điền `ttlMs` + `cacheScope:"private"` cho mọi kết quả list/read ở mọi phiên bản (Result của MCP cho phép thêm trường); override `list_resource_templates`; giữ 2026-07-28 trong danh sách hỗ trợ. Giao fresh worker; phiên kia sẽ kiểm lại bằng client thật sau khi người dùng restart 3847.
- Kết quả: `cache_hints.rs` (36 dòng) gắn `ttlMs:300000` + `cacheScope:"private"` vào `tools/list`, `resources/list`, `resources/templates/list` (override mới, trả rỗng) và `resources/read` ở mọi phiên bản; `mcp.rs` 163, `resources.rs` 47 dòng; test `tests/protocol_2026.rs` (158 dòng) phủ cả đường 2026 sessionless lẫn legacy `initialize`; `docs/SERVER.md` ghi request 2026 tối thiểu. Khác rmcp (chỉ gửi hint cho 2026) có chủ đích. Root kiểm độc lập: lib 9, protocol_2026 2, transport 4, skill_layer 2, tool_schemas 2 pass; strict Clippy/fmt/diff-check sạch; `curl` 2026-07-28 trên server tạm: discover/tools/resources/templates đều `resultType:"complete"` + hint, 15 tool đều `type:"object"`.
- Lúc nghiệm thu `node_modules` của checkout đã biến mất (lượt trước còn) và cổng 3847 không còn server; root và worker không xoá/dừng chúng. Worker chạy full suite: 52 pass/10 fail, 10 test render fail `WORKER_UNAVAILABLE` vì thiếu package `playwright` → cần `npm ci` rồi chạy lại. Chưa kiểm bằng Claude Code CLI thật; chưa commit.

### Bản chạy Docker — 2026-10-28

- Người dùng yêu cầu bản chạy trong Docker (Docker 29.8.2, linux/aarch64). Server bind cứng `127.0.0.1` và kiểm Host/Origin loopback (`main.rs`, `http.rs`). Hướng chốt: thêm cờ `--host` (mặc định giữ `127.0.0.1`); container bind `0.0.0.0` nhưng chỉ publish `127.0.0.1:3847:3847`, giữ nguyên kiểm Host/Origin. Image multi-stage, Chromium có trong image, data ở volume `/data`, không mount `~/.opencanvas`. Giao fresh worker; root sẽ review, tự build/chạy, sau đó chạy `/cp` (`reclaude cp start self`) khi không còn việc nền.
- Kết quả: `Dockerfile` (3 stage, Rust 1.99.1 + Node 24.21.0 trixie, image 1,29 GB, user `node`, `STOPSIGNAL SIGINT`), `compose.yaml` (`127.0.0.1:${OPENCANVAS_PORT:-3847}`, volume `opencanvas-data`, `init`, `shm_size` 512m, healthcheck), `.dockerignore`, `docker/smoke.mjs` (172 dòng), `main.rs` thêm `--host` (chỉ `127.0.0.1`|`0.0.0.0`, 81 dòng), mục Docker trong README, một đoạn trong `docs/SERVER.md`. `layout-step` để ở `target/debug` vì worker/fingerprint hardcode đường dẫn này. Root duyệt biến `OPENCANVAS_PORT` (worker thêm ngoài spec; mặc định không đổi).
- Root kiểm độc lập: container tạm `127.0.0.1:38471` + volume tạm, smoke pass toàn bộ (viewer, MCP legacy/2026, create/apply, render ready bằng Chromium 156.0.8078.4, PNG 320×200); `Host: evil.example` → 403; đã dọn. Sau đó `docker compose up -d` trên 3847 cho người dùng: healthy, viewer 200, projects rỗng (không tạo dữ liệu smoke trong volume thật). Worker báo strict Clippy/fmt/diff-check sạch, transport 4 + protocol_2026 2 pass. Chưa kiểm: full suite cargo, E2E viewer trong container, bind mount. Chưa commit.
- Context-watch: phiên con `local_c8346663-…` (haiku) chạy watcher nhắc /cp khi ≥40% context và nhận `RESUME-IN`; flow ghi ở memory `self-compact-resume`.

### P27 — 2026-10-29

- Người dùng yêu cầu triển khai P27 trên nhánh riêng. Đã tạo `codex/p27-interaction-states` từ `night/integration` @ `36b8b50`; working tree ban đầu sạch. Handoff nguồn: `origin/night/recommend-features:docs/recommend-features/tasks/P27-interaction-states.md`.
- Giao một fresh worker thực hiện P27a+b, gồm storage validation/resolution, context và render state; kiểm dữ liệu tạm, không chạm server/volume thật, không push. Root điều phối và nghiệm thu độc lập.
- Trước khi giao worker, người dùng yêu cầu pull main mới. Đã fetch `origin/main` và fast-forward nhánh P27 lên `a0d7743` (gồm hai PR integration + recommend-features); handoff hiện có trực tiếp trong `docs/recommend-features/tasks/P27-interaction-states.md`.
- Kết quả worker: P27a+b hoàn tất, state visual áp sau layout, context opt-in và validation theo theme; giữ schema stroke thực và animation keyframe hiện có. Hợp đồng/bằng chứng tại [P27 report](../P27-REPORT.md). Không thêm viewer picker (tùy chọn).
- Root nghiệm thu độc lập: storage states/refs/style aliases 8 pass, states/catalog/stroke 12 pass sau sửa Page gate; server integration 13, tools budget 1, library 35 pass (gồm golden bytes cũ). Clippy nghiêm ngặt, fmt và diff-check sạch. Tools/list tăng 195 B; mọi file sửa ≤350 dòng, mã Rust ≤216 dòng. Server guide 344 dòng được tách operations riêng.
- Điểm dừng: main local và origin/main cùng `a0d7743`, checkout vẫn `codex/p27-interaction-states`. Chưa commit/push; không restart server thật, không ghi dữ liệu thật. Không chạy full workspace suite/viewer E2E.
- Người dùng tiếp tục yêu cầu push nhánh P27. Root rà lại phạm vi thay đổi và diff-check sạch; chốt nội dung đã nghiệm thu để commit và push lên `origin/codex/p27-interaction-states`, không merge main.

### Điều tra animation QueryNest trên P24 — 2026-10-29

- Người dùng hỏi page có animation do Codex design sai hay preview render sai. Root đọc project `querynest-faithful-ui-motion` rev61 trên server 3847 (chỉ đọc). Kết luận: nhánh chỉ có P07a (dữ liệu); renderer/viewer/`render_frame` chưa đọc `animations` (P07c/P07d chưa làm), nên preview luôn vẽ trạng thái nghỉ, đúng thiết kế. Dữ liệu Codex hợp lệ và theo đúng hướng dẫn `author-frame.md`; ghi chú nhỏ: `tab-close` opacity 0 không có cách hiện ra, row-reveal ở page 03 không stagger. Không sửa mã.

### Motion preview trong viewer — 2026-10-29

- Người dùng chọn làm nhanh trang preview chính thức trong app (thay preview tạm Codex ở `/private/tmp/querynest-motion`, cổng 3848, hiểu sai `fillMode` mặc định và để style inline đè states). Worktree `../OpenCanvas-motion-preview`, nhánh `claude/motion-preview` từ `30cc7ff`, symlink node_modules/.runtime từ checkout chính. Giao fresh worker; không push, không chạm server 3847/`~/.opencanvas`.
- Nhận kết quả: `c7d532a` trên `claude/motion-preview` — route `/preview`, nút Preview, `web/src/preview/` 14 module (≤203 dòng), shared `sceneJob.ts`; báo cáo `docs/MOTION-PREVIEW-REPORT.md`. Root kiểm độc lập: `npm --prefix web test` 20/20, `test:preview` 8/8 (server tạm, data tạm). Kiểm tay bằng Vite dev worktree (cổng 5179, proxy GET tới 3847) trên project QueryNest: 7 load animation chạy, hover giữ translateY -1 + fill state, rời chuột trả về, link sang Table Data đúng. Chưa merge vào nhánh P24, không push.
- Người dùng yêu cầu merge vào P24: fast-forward `codex/p24-components-instances` 30cc7ff → `c7d532a`, không push. Root kiểm lại trên checkout chính: `npm --prefix web test` 20/20; server 3847 phục vụ `web/dist` mới không cần restart, `/preview` với QueryNest dựng 192 node, không lỗi console (SPA fallback trả HTTP 404, đã biết). Đã dừng Vite 5179, gỡ symlink và worktree `../OpenCanvas-motion-preview`; giữ nhánh `claude/motion-preview`.

### Bỏ theo dõi web/dist — 2026-10-29

- `web/dist` (bản build Vite) được track từ Initial commit, không tài liệu nào ghi lý do; build scripts, README và Dockerfile đều tự build nó. Người dùng yêu cầu bỏ theo dõi: thêm `web/dist/` vào `.gitignore`, `git rm --cached` (file trên đĩa giữ nguyên, 3847 vẫn phục vụ). Commit trên `codex/p24-components-instances`, không push. Clone mới cần `npm run build` trước `npm start` như README.

### Spike engine xuất ảnh thuần Rust — 2026-10-29

- Người dùng hỏi có hướng thuần Rust giữ chất lượng xuất ảnh không. Phiên Claude này bổ sung cho nghiên cứu rộng ở mục trên (không sửa `docs/EXPORT-ENGINE-RESEARCH.md`, `docs/export-engine-research/`). Giao một fresh subagent spike đo trong scratchpad: độ rộng text HarfRust so `measureText` Chromium 1248 (NotoSans bundled, có/không `--font-render-hinting=none`), wrap parity, pixel diff tiny-skia so Canvas2D trên scene cố định. Không sửa repo, không chạm 3847/`~/.opencanvas`.
- Kết quả spike (macOS, Chromium 156.0.8078.4, NotoSans): mô phỏng cắt advance 1/65536 px của Chrome cho HarfRust 0/1 232 width lệch sau round/ceil 1/64 và wrap 120/120 trùng; không mô phỏng thì 11 ca lệch 1/64. Pixel tiny-skia: hình/opacity mean Δ 0,16, text lệch mép glyph (CoreText). Root kiểm lại `widths_report.json`, `wrap_report.json`, ảnh ghép 2×. Phân tích + lộ trình: [PURE-RUST-EXPORT](../PURE-RUST-EXPORT.md); mã và bảng số: `spikes/rust-export/`. Gate còn mở: đo phía Chrome trong image Docker Linux. Chưa commit.
- Người dùng hỏi tham khảo cách render của `storytold/photocraft` (Rust thuần, alpha, MIT/Apache). Root đọc mã @ `5f9b439` (clone nông ở scratchpad, không chạy): Parley + skrifa không hinting, rasterizer glyph/path diện tích chính xác tự viết, compositor CPU làm oracle cho wgpu (dung sai 2/255), hằng số khớp theo output Photoshop. Ghi phần áp dụng/không áp dụng vào [PURE-RUST-EXPORT](../PURE-RUST-EXPORT.md); bước đo Linux nay gồm thử raster glyph diện tích chính xác. Chưa commit.
- Người dùng chốt phạm vi macOS (native), Linux/Docker để sau. Root ghi bảng phương án vào [PURE-RUST-EXPORT](../PURE-RUST-EXPORT.md): chọn C (Rust thuần HarfRust + skrifa + tiny-skia), giữ A (Chromium) làm đối chứng/rollback, loại B/D, C+ và E là thử nghiệm/dự phòng; lộ trình 3 task sau feature flag. Chờ người dùng duyệt ba quyết định đi kèm (tiêu chí pixel text, fallback glyph, parity `web-2` trước P10a) trước khi viết task handoff.
- Người dùng đề xuất một engine render Rust, build WASM cho viewer (viewer chỉ để hiển thị; agent tự code lại theo frontend dự án đích). Root đồng ý về hướng: viewer = export theo cấu tạo, bỏ rủi ro hai engine; vẫn giữ yêu cầu hình học khớp Chrome. Điểm cần thiết kế: crate render thuần (không I/O/thread) build cả native và `wasm32-unknown-unknown`, re-raster khi zoom dừng + tile, overlay tương tác giữ JS, preview motion DOM giữ riêng. Đề xuất spike WASM nhỏ (bundle, thời gian frame lớn, so pixel native/WASM) trước khi sửa lộ trình.
- Người dùng yêu cầu đóng gói thành task để làm sau: `tasks/09`–`13` (spike WASM, đo text native, crate render + PNG native, viewer WASM, đổi mặc định), lộ trình trong [PURE-RUST-EXPORT](../PURE-RUST-EXPORT.md), thêm dòng tham chiếu trong `AGENTS.md`. Người dùng cho commit + push nhánh `codex/p24-components-instances` khi xong; chỉ commit phần của phiên này (tài liệu nghiên cứu/đo của các phiên khác và mục nhật ký của họ để nguyên chưa commit).
