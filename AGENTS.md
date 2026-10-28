# OpenCanvas — quy tắc và nhật ký bàn giao

## Quy tắc làm việc

- Đọc `PLAN.md` trước khi triển khai. Đây là nguồn yêu cầu sản phẩm và AC.
- Phiên chính chỉ điều phối, kiểm chứng và cập nhật tài liệu tiến độ; mọi triển khai giao cho subagent mới, một task mỗi agent. Không tái sử dụng agent đã hoàn thành, worker không tạo agent tiếp.
- Handoff phải có mục tiêu, phạm vi file, workspace/branch, AC, cách kiểm và đầu ra. Dùng `fork_turns="none"`, giữ model/effort mặc định.
- Dùng `rg`/`rg --files`, đọc bằng shell; dùng `apply_patch` sửa mã. Không dùng Python cho thao tác text thông thường. Loại node_modules, .git, .venv, dist, build khỏi tìm kiếm.
- Ưu tiên RTK tường minh khi hỗ trợ (`rtk ls`, `rtk git status`); dùng output gốc cho diff, điều tra lỗi và bằng chứng. Không tự bật hook/proxy/experimental. Xem `/Users/luis/.codex/RTK.md`.
- Chỉ chạy kiểm tra nhắm đúng vùng thay đổi, hạn chế song song; không ghi pass khi chưa chạy. Không tự tạo chat độc lập.
- Không ghi dữ liệu demo vào `~/.opencanvas`; tests dùng data root tạm. Chỉ API create_project thực mới khởi tạo project người dùng.
- Cập nhật nhật ký này tại mỗi lần giao việc, nhận kết quả, phát hiện blocker và trước khi dừng. Không ghi secret hoặc đường dẫn dữ liệu riêng của người dùng vào API.

## Modular Architecture Rules — bổ sung của người dùng 2026-10-28

- Luôn ưu tiên module nhỏ, mỗi file một trách nhiệm; theo kiến trúc hiện có, public API nhỏ và tường minh.
- Trước feature: khảo sát cấu trúc, xác định ranh giới trách nhiệm và các file tạo/sửa, rồi triển khai tách module.
- Mục tiêu 100–200 dòng/file; quá 250 dòng phải đánh giá tách; tránh quá 350 dòng mã viết tay. Ngoại lệ: generated/config hoặc tách làm giảm khả năng đọc, phải nêu lý do.
- Tách business logic thành services, hàm dùng lại thành utilities, types/interfaces khi hữu ích, phần UI thành components và stateful UI logic thành hooks phù hợp.
- Refactor file lớn khi thay đổi làm chúng phình thêm; giữ hành vi/public interface, tránh circular dependencies và abstraction không cần thiết; cập nhật imports/tests sau tách.
- Nghiệm thu phải kiểm số dòng, một trách nhiệm/module, chạy test/lint/type check liên quan và báo mọi file còn quá ngưỡng khuyến nghị.
- Quy tắc này đã được gửi ngay cho hai worker M0 đang chạy; áp dụng cho mọi handoff tiếp theo.
- Đã lưu nguyên bộ quy tắc vào `/Users/luis/.codex/AGENTS.md` (hướng dẫn dùng chung), đọc lại xác nhận thành công. Không sửa cấu hình model, hook hoặc runtime memory database.

## Trạng thái cuối phiên tiếp tục — 2026-10-28, 12:18 Asia/Ho_Chi_Minh

- **MVP M0–M2 đã nghiệm thu local thật.** M3 context/token/theme/layout và M4 migration/backup/bundle đã hoàn thiện trong phạm vi công bố. Xem `docs/ACCEPTANCE.md` cho AC, lệnh và giới hạn; `README.md` cho setup/launch/MCP. Không còn implementation task đang chạy hoặc blocker trong scope PLAN hiện tại.
- Phiên chính chỉ điều phối, review, chạy kiểm và cập nhật tài liệu; mọi code được giao worker mới một lần. `resume_resolution`, `resume_hardening`, `resume_server`, `live_e2e`, `storage_lint` đã hoàn tất, không tái sử dụng.
- Root độc lập: pipeline **11/11**, storage **38 tests / 13 suites**, server **9/9 integration tests** pass; production storage+server Clippy `-D warnings`, `cargo fmt --all --check`, `git diff --check` pass. Web production build/TypeScript và full E2E script pass theo worker; root rebuild binary cuối và chạy Node E2E độc lập pass.
- E2E actual MCP→SQLite→HTTP/SSE→production viewer→PNG→process restart/browser reload; 12 nhóm kiểm. Scene/line list/provenance khớp; RGBA0 sai khác ở640×480 revisions2/3/restart và390×844. Root đã xem ảnh desktop/mobile thật. Artifacts `artifacts/e2e/` được tái tạo bởi `node spikes/e2e/run.mjs`; completed2026-10-28T05:16:19.745Z. Cleanup data root/process tạm thành công, không tạo project ở `~/.opencanvas`.
- Không source/test Rust/TS/TSX/JS/CSS viết tay nào vượt250 dòng trong các thư mục code đã soát. Generated lockfiles/dist không áp ngưỡng. Warning còn lại: web chunk~574,59kB và shared test helper dead_code ở một số test binary; production strict lint sạch.
- Giữ các giới hạn đã công bố: default interaction state, không arbitrary authored history, deep layout explanation/diff, log pruning/GC, runtime hot replacement hoặc backup UI/CLI/MCP. Backup/import/restore là public Rust storage API. Xem `docs/SERVER.md`, `docs/RESOLUTION.md`, `docs/BACKUP.md` và `spikes/e2e/README.md`.
- Workspace vẫn branch `main`, có nhiều thay đổi/untracked từ cả phiên trước và phiên này; chưa commit/push. Không reset hoặc bỏ các thay đổi đó. Không có backend/fixture/browser test process để lại theo cleanup. Không tự khởi chạy server dùng data root người dùng; chỉ tạo project khi API thực được gọi.
- Khi có yêu cầu mới: đọc trạng thái này + README/ACCEPTANCE, kiểm working tree; giao task triển khai cho subagent mới với ownership/AC/checks rõ. Các checkpoint cũ dưới đây là lịch sử, không phải việc còn dang dở.

## Trạng thái checkpoint cũ trước phiên tiếp tục — 2026-10-28

- Yêu cầu: triển khai PLAN, phiên chính chỉ quản lý/kiểm chứng; lưu đủ tiến độ để phiên khác tiếp tục nếu limit.
- Điểm gốc: workspace ban đầu chỉ có `PLAN.md`, không có Git repository. Lúc checkpoint đã thấy Git với commit `9f1a3c1 Initial commit`; phiên chính không tạo commit này. Có nhiều thay đổi/untracked đang triển khai, chưa commit hoặc push chúng.
- Runtime đã kiểm: cargo 1.99.1, rustc 1.99.1, Node 24.21.0, npm 11.19.0; pnpm và RTK có sẵn. Chưa cài dependency tại thời điểm bắt đầu.
- M0 đã đạt trong phạm vi fixtures và benchmark công bố. Viewer hoàn tất theo fixture; **M1/M2 chưa đạt E2E thật**, backend đang viết. M3/M4 đã giao triển khai song song, chưa nghiệm thu.
- **Chỉ đạo mới nhất (phiên tiếp tục 2026-10-28):** người dùng đã yêu cầu tiếp tục công việc phiên khác. Đã đọc AGENTS, PLAN, ba checkpoint và working tree; chỉ có root đang sống. Lệnh dừng giao việc cũ được thay bằng yêu cầu tiếp tục này. Giữ mọi thay đổi có sẵn, không commit/push khi chưa có nhu cầu rõ.

## Nhật ký giao việc và nhận kết quả

Yêu cầu mới sau nghiệm thu: thêm `http://127.0.0.1:3847/mcp` vào Codex và tạo design mẫu nhỏ để xem. Root kiểm server ngoài sandbox: đang chạy, projects rỗng; không khởi chạy/restart server người dùng. Active CODEX_HOME là profile account-2 trong CodexPortable, chưa có MCP tên opencanvas. Giao fresh `connect_mcp` cấu hình đúng profile bằng CLI và `sample_design` tạo project người dùng thực qua MCP, render/QA và mở viewer. Đây là dữ liệu mẫu do người dùng yêu cầu rõ, khác fixture test; không ghi trực tiếp DB/filesystem data root. Root chỉ điều phối/kiểm chứng. Giữ code không đổi.

Nhận `connect_mcp`: `/opt/homebrew/bin/codex mcp add opencanvas --url http://127.0.0.1:3847/mcp` thành công ngoài sandbox, entry profile account-2 config.toml, enabled=true, transport streamable_http; không sửa model/hooks hay source. Tool catalog chat hiện tại chưa có OpenCanvas, không khẳng định hot-load/restart khi chưa có bằng chứng. Sample worker dùng actual MCP protocol client cho lần tạo này; đang tạo và kiểm design.

Hoàn tất yêu cầu mẫu: `sample_design` tạo qua MCP project `OpenCanvas · Mẫu đầu tiên` / slug `opencanvas-welcome`, ID `p_dfafcf1451e543a5b1fcedb6ad441a81`, revision1, frame `welcome-dashboard` 1040×720, 72 nodes/5 tokens/2 styles. Root independent GET projects và `codex mcp get opencanvas --json` xác nhận đúng trạng thái; root đã xem dashboard.png + viewer.png thật, visual đạt. Worker actual viewer/strict PNG so 2.995.200 RGBA channels, 0 sai khác; no page errors, GET-only, context đầy đủ. Evidence tại `artifacts/sample/`, scripts tái tạo tại `samples/welcome/` (max116 dòng); không source app changes. `git diff --check` pass. Server người dùng tại3847 được giữ nguyên, không stop/restart; browser test đã đóng. Đã yêu cầu mở viewer trong Codex panel (`open_in_codex` trả queued); URL `http://127.0.0.1:3847`. Đây là project thật được người dùng yêu cầu giữ để xem, không xóa như fixture.

Phiên tiếp tục giao ba task mới (workspace chung `/Users/luis/Workspace/OpenCanvas`, branch `main`; không reset/check out):

- `resume_resolution`: hoàn thiện và kiểm resolver M3 + fixture layout nâng cao; sở hữu validate modules, resolution tests, spikes/pipeline fixture/tests và docs/RESOLUTION.md. Không sửa DB/store/server.
- `resume_hardening`: kiểm/sửa migration và bundle M4, thêm failure/round-trip/concurrency tests, docs/BACKUP.md; sở hữu storage backup/bundle/migration/db/store/model/lib, tests M4. Không sửa validate/server.
- `resume_server`: hoàn thiện backend M1/M2/M3 theo checkpoint, contract HTTP/MCP/theme/cursor/jobs, integration tests và docs/SERVER.md. Sở hữu crates/server và Cargo.lock khi cần; không sửa storage/viewer/pipeline. Đợi thông tin storage khi gặp phụ thuộc.

Mọi worker mới phải đọc PLAN/AGENTS/checkpoint liên quan, dùng temp roots, không tạo subagent, báo lệnh thật và số dòng. Root chỉ điều phối, kiểm chứng và ghi tiến độ.

Nhận tiến độ đầu phiên: M3 chạy `cargo test -p opencanvas-storage --offline --test resolution -- --test-threads=2` **5/5 pass** (worker). Server offline check tới lỗi partially moved ReadRequest.project_id, worker đang sửa. M4 đang bổ sung runtime suites; đã mở thêm scope visibility helper trong assets.rs và đọc retention trong snapshot transaction. Chưa nghiệm thu M1/M2/M3/M4 từ các kết quả đầu này.

Nhận hoàn tất `resume_resolution`: resolution **9/9**, references/style_aliases **5/5**, Chromium constraints **2/2** (worker; sandbox EPERM rồi escalation được duyệt). Không đổi public API/engine; tests split, file lớn nhất233 dòng. Root sẽ chạy kiểm độc lập sau khi storage ổn định. Server compile offline đã pass theo worker; HTTP/MCP/SSE/jobs tests đang viết.

Giao task mới `live_e2e`: sở hữu `spikes/e2e/`, `artifacts/e2e/`, root README và tối đa npm script `test:e2e`; xác minh backend production + actual MCP + Chromium viewer + PNG + restart bằng data root tạm, report findings cho server/root, không tự sửa file ngoài scope. Không tái sử dụng worker M3 đã xong.

Review root phát hiện và chuyển worker sửa: cache job failed không cho Retry cùng request; input default font thiếu manifest (storage thực tế yêu cầu fontId tường minh, fallback chỉ phòng dữ liệu cũ); bundle validation mới đòi sortKey liên tục dù delete/reparent hợp lệ có thể để khoảng trống. M4 đã báo targeted11 pass trước regression sortKey này; chưa chạy full storage cho tới fix. Cho phép M4 sửa inventory orphan assets để giữ bản bundled font trong bundle, không thêm GC. Server đang chạy actual MCP/HTTP/SSE/real-worker integration tests; live_e2e đang dựng harness production browser.

Nhận hoàn tất `resume_hardening`: targeted **12/12 pass**, migration rollback, concurrent writer/export, import/restore/replay/hash/archive/symlink guards và regression sortKey/orphan. API/deps không đổi; source lớn nhất213 dòng. docs/BACKUP.md + checkpoint đã cập nhật.

Bằng chứng độc lập phiên tiếp tục:
- Root `npm run test:pipeline` (escalation được duyệt vì loopback) **11/11 pass**, 2.37s; gồm nested constraints và overflow mới. Không skip/cancel.
- Root `cargo test -p opencanvas-storage --offline --features fault-injection -- --test-threads=2` **38 passed / 13 suites**, 12.66s. Có cả M3/M4 và recovery cũ; chưa sửa storage sau kết quả này.
- Server worker Chromium render **2/2 pass** sau fix frame scene root phải bỏ parent page (authored giữ nguyên). Đang hoàn thiện kiểm process crash/timeout/queue và SSE durable gap.
- E2E đã đi qua MCP→viewer→PNG→SSE token update→restart; đang sửa riêng harness inspector responsive và chạy lại, chưa ghi final pass.

Nhận hoàn tất `live_e2e`: production `npm run test:e2e` và lượt Node cuối pass; scene/PNG RGBA 0 sai khác desktop/mobile/restart, semantic readback/inspect, retry/rollback và GET-only viewer. Root đã xem ảnh desktop/mobile thật; source lớn nhất206 dòng. Root sẽ chạy độc lập sau server chốt.

Lint cuối server phát hiện ba warning `-D warnings` trong storage bundle (int_plus_one/manual_is_multiple_of/type_complexity). Giao fresh task `storage_lint` chỉ xử cảnh báo library không đổi hành vi, không tái sử dụng M4 đã xong. Root chưa coi clippy strict đạt cho tới khi kiểm lại. Server actual integration từng suite đã pass 9 tests theo worker.

Đóng phiên tiếp tục: `storage_lint` đã xử đúng ba warning bằng biểu thức tương đương/private type alias; root review raw delta và Clippy cả storage/server production pass. Root full server9/9 và final Node E2E12 nhóm pass; không có source implementation delta sau các lượt kiểm cuối. Tất cả worker đã hoàn tất, không tái sử dụng. Các trạng thái tạm thời “đang chạy/chưa kiểm” trong nhật ký bên dưới được thay bằng mục trạng thái cuối phiên ở trên và `docs/ACCEPTANCE.md`.

| Task | Phạm vi sở hữu | Trạng thái | Kiểm chứng/đầu ra |
| --- | --- | --- | --- |
| M0 pipeline (`/root/m0_pipeline`) | `crates/layout/`, `renderer/`, `worker/`, `spikes/pipeline/`, root Node config | Hoàn tất; không tái sử dụng agent | Main xác minh 8/8 Chromium; worker Rust 3 + CLI 1 + typecheck/fmt. Xem spikes/pipeline/README.md và RESULTS.md |
| M0 storage/core (`/root/m0_storage`) | `crates/storage/`, `spikes/storage/` | Hoàn tất; không tái sử dụng agent | Worker cuối 17 tests pass và benchmark đạt; main xác minh bản 15 tests trước regression bổ sung. Có guard style alias và regression atomic/corrupt cycle |
| M1 viewer (`/root/m1_viewer`) | `web/`, `docs/VIEWER.md`, artifacts/viewer | Hoàn tất; không tái sử dụng agent | Build/typecheck, 4 state tests, 9 nhóm browser smoke pass theo worker; main xem ảnh thật. Chưa chạy với backend thật |
| M1/M2 server (`/root/m1_server`) | `crates/server/`, root Cargo member/lock | Đã dừng, chưa hoàn tất | rmcp 3.4.0 + HTTP/SSE/jobs đã lưu; chưa nghiệm thu compile/E2E. Xem docs/checkpoints/m1-server.md |
| M3 resolution (`/root/m3_resolution`) | storage validate.rs/validate/**, tests/resolution.rs | Đã dừng, chưa hoàn tất | Alias/theme và 5 tests đã viết; tests chưa thực thi. Xem docs/checkpoints/m3-resolution.md |
| M4 hardening (`/root/m4_storage_hardening`) | storage db/store/lib/model, backup/bundle/migration modules | Đã dừng, chưa hoàn tất | DB schema 2 + bundle code compile; chưa có runtime tests. Xem docs/checkpoints/m4-storage.md |

## Thứ tự khôi phục hiện tại

1. Đọc trạng thái cuối phiên và `docs/ACCEPTANCE.md`; các bước triển khai dang dở ở checkpoint cũ đã được giải quyết.
2. Kiểm working tree và yêu cầu mới trước khi sửa; không tạo lại dữ liệu demo, không chạy lại mọi suite khi không có thay đổi liên quan.
3. Để chạy app, dùng lệnh trong README. Test E2E dùng root tạm và dọn process/data; server thật không được để chạy bởi lượt nghiệm thu.
4. Khi sửa mới, DB schema2 và scene/API schema1 là hai định dạng riêng. Giữ public contracts và giao fresh worker, không hồi sinh worker cũ đã hoàn tất.

## Contract đang khóa ở M0

- Storage: `opencanvas-storage`, Store giữ root lock; Project cung cấp snapshot/apply_batch/commits_after/import_asset/read_asset/checkpoint. Snapshot camelCase, nodes có id/parentId/sortKey/kind/props, resource lists và retention. Chi tiết thực tế phải đối chiếu public API khi worker hoàn tất.
- Layout: `opencanvas-layout`, `compute(LayoutInput, MeasureCache)` đồng bộ trả pending measurement requests hoặc ready scene; driver gom misses thành một browser batch mỗi pass, tối đa 6 pass.
- Shared renderer: Canvas2D `drawScene` vẽ line list đã tính. Viewer tích hợp canvas bằng Konva.Image + overlay selection, không dùng Konva.Text để wrap lại. PNG dùng cùng drawScene. Flat scene pre-order giữ paint order; parentId phục vụ ancestor clips.
- Bundled font: `font_bundled`, family `OpenCanvas Noto Sans`, weight 400 normal; SHA-256 `bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d`, license trong `renderer/fonts/OFL.txt`. Kiểm hash thật trước khi nghiệm thu.
- Storage benchmark protocol đã xuất tại `spikes/storage/README.md` trước khi đo; small 100 nodes, large 5.000 nodes; budget single client p95 100/500 ms, 8 clients p95 2 s, queue p95 1,5 s, large snapshot p95 500 ms. Kết quả ghi bên dưới.
- Contract tích hợp HTTP/viewer tại `docs/CONTRACT.md`: backend mặc định 127.0.0.1:3847, snapshot/events/scene/jobs/assets, raw success JSON và `{error}` khi lỗi. Mọi worker tích hợp phải đọc file này.
- Worker báo: layout Rust 3 unit tests và renderer typecheck/build đã qua; storage compile với bundled SQLite 3.54.1 (`rusqlite=0.41.0`, libsqlite3-sys 0.39.1). Bằng chứng độc lập và benchmark ghi bên dưới.
- M3 contract: Token.value giữ literal, thêm `{aliasId}` hoặc `{default:<literal|alias>, themes:{name:<literal|alias>}}`; named themes tối đa 32, tokens 4.096, alias depth 32, chặn missing/type/cycle ở mọi theme. APIs `resolved_nodes_for_theme(&Snapshot,&str)` và `resolved_tokens_for_theme(&Snapshot,&str)`; `resolved_nodes` giữ wrapper default. Không có style aliases. Unknown theme → UNSUPPORTED_THEME. Xem checkpoint để biết phần nào thực sự đã viết/kiểm.
- M4 contract dự kiến: `Project::export_bundle()` → manifest + bounded TAR bytes; `Store::import_bundle(slug,bytes)` tạo ID mới/revision 0; `restore_bundle` giữ source ID/revision khi identity chưa tồn tại. Migration v1→v2 thêm retention_metadata, transactional. APIs đang viết, không coi là đã kiểm.

## Bằng chứng đã chạy bởi phiên chính

- `npm run test:pipeline`: sandbox mặc định chặn bind loopback (`EPERM`); chạy lại cùng lệnh với sandbox escalation đã được công cụ duyệt → **6/6 pass**, ~1.15 s. Không có auto-review rejection. Kiểm hai widths 420/240, cache repeat 0 batch RPC, shared viewer/PNG byte-identical, delayed decode, missing font/hash/image strict failure, bounded passes/pixels, paint order/clip/crop. PNG thật đã được xem tại `spikes/pipeline/output/fixture.png`.
- `cargo test -p opencanvas-storage --features fault-injection -- --test-threads=2` → **15 tests / 7 suites pass**, 2,16 s trước M3/M4. Có transaction, typed refs, asset/containment, abrupt-exit recovery. Worker bổ sung hai regression sau đó báo 17 pass; main chưa chạy lại vì M3/M4 đang thay storage.
- Hash font thật đã đối chiếu trùng contract; license OFL đọc trực tiếp.
- Dữ liệu test đều root tạm, chưa tạo project người dùng ở `~/.opencanvas`.
- Giới hạn storage/pipeline đã thống nhất: text 64 KiB, position/dimension 100.000, fontSize/lineHeight >=1; public fit resolve sang renderer fill; crop chuẩn hóa và radius được hỗ trợ.
- Viewer cuối dùng fixture API port ngẫu nhiên, đã dừng toàn bộ fixture servers. Port 3847 chưa được dùng cho server thật trong nghiệm thu.
- Sau thay đổi resource timeout/cancellation/nonstrict asset placeholders, phiên chính chạy lại `npm run test:pipeline` escalated → **8/8 pass**, 1.65 s. Worker terminal sau cancel phải khởi động lại; font thiếu vẫn fail rõ, chưa có declared fallback.
- Review phát hiện viewer coi sortKey là string; worker đã sửa thành integer và thêm test numeric. `docs/CONTRACT.md` ghi rõ kiểu.
- Đã đọc benchmark: large 5.000 / 8 clients successful-batch p95 45,421 ms, queue p95 27,633 ms; checkpoint 728/1128 → 1128/1128 sau reader. Số đo worker, không gồm end-to-end retry latency hoặc physical disk writes.
- CPU/RAM bằng run bổ sung riêng, không thay latency gốc: 13,61 s wall, 19,47 s user + 0,96 s sys, RSS 91.144.192 B (~86,9 MiB). Xem spikes/storage/README.md và các results*.json/resource-usage.txt.
- Review rủi ro style recursion: worker xác nhận validation đã chặn alias, bổ sung defense ở resolver và regression ba upserts/corrupt mutual cycle. Final storage 17 pass theo worker; không còn blocker M0 này.
- Đã xem ảnh desktop viewer: đúng shell sáng, canvas/panel/inspector read-only; đây là fixture QA, chưa phải MCP → browser thật. Worker báo đã sửa toolbar hẹp, build và smoke cuối pass; mọi source/test viewer <250 dòng. Bundle ~574 kB có warning chunk-size, không lỗi build.

## Lịch sử checkpoint trước phiên tiếp tục

Checkpoint cuối: **2026-10-28, khoảng 12:00 Asia/Ho_Chi_Minh**. Phiên chính đã đọc cả ba checkpoint và `collaboration.list_agents` xác nhận mọi worker còn trong danh sách đều completed. Không có task mới được giao sau lệnh dừng; không có server/process nền theo handoff.

- Storage mới: worker M4 báo `cargo check -p opencanvas-storage --offline` pass sau sửa lỗi SQL u64. Đây là compile, không thay bằng chứng runtime/test. Main không chạy thêm build/test sau chỉ đạo checkpoint.
- M3: 5 tests trong `crates/storage/tests/resolution.rs` chưa chạy vì compile vướng M4 đang sửa lúc đó. File test 303 dòng đã được đánh giá giữ chung vì một contract suite; đây là ngoại lệ >250, không quá350. Các source M3 <200 dòng.
- Server: formatting pass, file lớn nhất206 dòng; chưa biết lỗi compile riêng vì check trước dừng bị lỗi storage/DNS. Cần compile offline lại khi được tiếp tục, rồi sửa theme-aware resolution, token schemas, outline cursor, body caps và integration tests. Chưa có docs/SERVER.md hoặc root README vận hành.
- M4: code export/import/restore và migration compile, source lớn nhất196 dòng; chưa viết M4 tests, docs/BACKUP.md chưa có. Cần kiểm migration rollback, bundle round-trip/concurrent backup/malformed TAR/containment.
- Memory chung đã lưu tại `/Users/luis/.codex/memories/opencanvas-progress.md`; global AGENTS có đường dẫn nhắc đọc. Một số checkpoint worker vẫn ghi giả định ban đầu “no Git”; trạng thái Git hiện tại trong file này là mốc đã kiểm mới hơn.

- Đọc file này và báo cáo dưới `docs/`/`spikes/` nếu đã xuất hiện; liệt kê file hiện có bằng `rtk ls` và tìm hẹp bằng `rg`.
- Dùng `collaboration.list_agents` để biết worker nào còn chạy trong phiên hiện tại. Agent ID phiên cũ chỉ là bằng chứng bàn giao; nếu không còn agent sống, tạo agent mới cho task chưa hoàn tất, không giả định task đã xong.
- Đọc trạng thái thực tế, không chạy lại mọi test hoặc cài lại dependency không cần thiết. Kiểm phần còn dang dở trước khi nhận lại quyền sở hữu file.
