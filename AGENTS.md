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

## Trạng thái hiện tại — 2026-10-28

- Yêu cầu: triển khai PLAN, phiên chính chỉ quản lý/kiểm chứng; lưu đủ tiến độ để phiên khác tiếp tục nếu limit.
- Điểm gốc: workspace ban đầu chỉ có `PLAN.md`, không có Git repository. Chưa tạo branch/commit.
- Runtime đã kiểm: cargo 1.99.1, rustc 1.99.1, Node 24.21.0, npm 11.19.0; pnpm và RTK có sẵn. Chưa cài dependency tại thời điểm bắt đầu.
- Phạm vi hoàn tất bắt buộc: M0 → M1 → M2 (MVP theo §10). M3/M4 triển khai sau khi MVP đạt, theo phần cần thiết và thời gian; không tuyên bố hoàn tất toàn PLAN nếu còn phần chưa làm.
- Chưa có test/build nào chạy và chưa có mốc nào đạt.

## Công việc đang giao

| Task | Phạm vi sở hữu | Trạng thái | Kiểm chứng/đầu ra |
| --- | --- | --- | --- |
| M0 pipeline (`/root/m0_pipeline`) | `crates/layout/`, `renderer/`, `worker/`, `spikes/pipeline/`, root Node config | Hoàn tất, không tái sử dụng agent | Main xác minh8/8Chromium; worker Rust3+CLI1+typecheck/fmt; báo cáo spikes/pipeline/README.md + RESULTS.md |
| M0 storage/core (`/root/m0_storage`) | `crates/storage/`, `spikes/storage/` | Chờ sửa lỗi cycle đã phát hiện trước khi chốt | Tests15 + benchmarkđạt; root phát hiện fillstyle refs có thể đệ quy cycle, yêu cầu regression atomic |
| M1 viewer (`/root/m1_viewer`) | `web/`, `docs/VIEWER.md`, product/design docs nếu cần | Đang chạy độc lập contract | React/TS/Konva shell và hành vi read-only; build/typecheck/browser smoke; không mở rộng layout engine |
| M1/M2 server (`/root/m1_server`) | `crates/server/`, root Cargo member/lock, README/docs/SERVER.md | Đang triển khai | HTTP/MCPStreamable+SSE+jobs thực; tích hợpstorage/pipeline, tests giao thức và runbook |

## Thứ tự tiếp tục

1. Chờ hai worker M0, đọc báo cáo và chạy kiểm tra độc lập. Không mở rộng layout nếu spike không đạt.
2. Khóa contract giữa Rust layout, browser worker và API; giao agent mới xây backend HTTP/MCP và agent mới xây viewer read-only (React/TypeScript/Konva), phạm vi không chồng nhau.
3. Giao agent mới kiểm E2E/consistency M1–M2, sửa theo task độc lập nếu có lỗi; mỗi agent hoàn tất chỉ dùng một lần.
4. Ghi lệnh thật/kết quả vào tài liệu milestone, README vận hành và nhật ký này. Báo rõ phần chưa kiểm hoặc chưa triển khai.

## Contract đang khóa ở M0

- Storage: `opencanvas-storage`, Store giữ root lock; Project cung cấp snapshot/apply_batch/commits_after/import_asset/read_asset/checkpoint. Snapshot camelCase, nodes có id/parentId/sortKey/kind/props, resource lists và retention. Chi tiết thực tế phải đối chiếu public API khi worker hoàn tất.
- Layout: `opencanvas-layout`, `compute(LayoutInput, MeasureCache)` đồng bộ trả pending measurement requests hoặc ready scene; driver gom misses thành một browser batch mỗi pass, tối đa 6 pass.
- Shared renderer: Canvas2D `drawScene` vẽ line list đã tính. Viewer tích hợp canvas bằng Konva.Image + overlay selection, không dùng Konva.Text để wrap lại. PNG dùng cùng drawScene. Flat scene pre-order giữ paint order; parentId phục vụ ancestor clips.
- Bundled font: `font_bundled`, family `OpenCanvas Noto Sans`, weight 400 normal; SHA-256 `bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d`, license trong `renderer/fonts/OFL.txt`. Kiểm hash thật trước khi nghiệm thu.
- Storage benchmark protocol đã xuất tại `spikes/storage/README.md` trước khi đo; small 100 nodes, large 5,000 nodes; single client p95 100/500 ms, 8 clients p95 2 s, queue p95 1.5 s, large snapshot p95 500 ms. Chưa có kết quả.
- Contract tích hợp HTTP/viewer tại `docs/CONTRACT.md`: backend mặc định 127.0.0.1:3847, snapshot/events/scene/jobs/assets, raw success JSON và `{error}` khi lỗi. Mọi worker tích hợp phải đọc file này.
- Worker báo: layout Rust 3 unit tests và renderer typecheck/build đã qua; storage compile với bundled SQLite 3.54.1 (`rusqlite=0.41.0`, libsqlite3-sys 0.39.1). Bằng chứng độc lập và benchmark ghi bên dưới.

## Bằng chứng đã chạy bởi phiên chính

- `npm run test:pipeline`: sandbox mặc định chặn bind loopback (`EPERM`); chạy lại cùng lệnh với sandbox escalation đã được công cụ duyệt → **6/6 pass**, ~1.15 s. Không có auto-review rejection. Kiểm hai widths 420/240, cache repeat 0 batch RPC, shared viewer/PNG byte-identical, delayed decode, missing font/hash/image strict failure, bounded passes/pixels, paint order/clip/crop. PNG thật đã được xem tại `spikes/pipeline/output/fixture.png`.
- `cargo test -p opencanvas-storage --features fault-injection -- --test-threads=2` → **15 tests / 7 suites pass**, 2.16 s. Có transaction, typed refs, asset/containment, abrupt-exit recovery. Benchmark M0 storage chưa xong.
- Hash font thật đã đối chiếu trùng contract; license OFL đọc trực tiếp.
- Dữ liệu test đều root tạm, chưa tạo project người dùng ở `~/.opencanvas`.
- Các giới hạn storage/pipeline đang thống nhất: text64KiB, position/dimension100000, fontsize/lineHeight>=1; public fit được resolve sang renderer fill; crop chuẩn hóa và radius được hỗ trợ. Sau thay đổi mới chỉ chạy lại kiểm liên quan.
- Worker viewer được dùng tạm port3847 để fixture QA khi backend chưa chạy; phải dừng fixture trước tích hợp server thật.
- Sau thay đổi resource timeout/cancellation/nonstrict asset placeholders, phiên chính chạy lại `npm run test:pipeline` escalated → **8/8 pass**, 1.65 s. Worker terminal sau cancel phải khởi động lại; font thiếu vẫn fail rõ, chưa có declared fallback.
- Review contract bắt lỗi viewer coi sortKey là string; storage thực trả integer. Đã gửi worker sửa và thêm fixture numeric, `docs/CONTRACT.md` cập nhật rõ kiểu.
- Đã đọc benchmark lần đầu: large5000/8clients successful-batch p95 45.421ms, queue p95 27.633ms; checkpoint728/1128→1128/1128 sau reader. Đây là số worker đo, chưa bao gồm end-to-end retry latency; CPU/RAM báo cáo đang hoàn thiện. Không chạy lại benchmark vô cớ.
- Storage đã hoàn tất CPU/RAM bằng run bổ sung riêng (không thay sốlatency gốc):13.61s wall,19.47s user+0.96s sys,91,144,192B RSS (~86.9MiB). Xem spikes/storage/README.md/results.json/results-resource-run.json/resource-usage.txt. Giới hạn retrylatency chưa đo ghi rõ.
- Root review phát hiện chu trình style-to-style fill có thể gây recursion overflow. Đã yêu cầu worker chặn ref loại này (alias ngoàiMVP) hoặc guardcycle, thêm regression; chưa chốt storage cho tới fix.
- Viewer ảnh desktop đã xem trực tiếp: đúng shell sáng, canvas/panel/inspectorread-only; vẫn là fixtureQA, chưa bằng chứngMCP→browserthật. Worker đang sửa toolbar hẹp và numeric sortKey.

## Cách khôi phục sau gián đoạn

- Đọc file này và báo cáo dưới `docs/`/`spikes/` nếu đã xuất hiện; liệt kê file hiện có bằng `rtk ls` và tìm hẹp bằng `rg`.
- Dùng `collaboration.list_agents` để biết worker nào còn chạy trong phiên hiện tại. Agent ID phiên cũ chỉ là bằng chứng bàn giao; nếu không còn agent sống, tạo agent mới cho task chưa hoàn tất, không giả định task đã xong.
- Đọc trạng thái thực tế, không chạy lại mọi test hoặc cài lại dependency không cần thiết. Kiểm phần còn dang dở trước khi nhận lại quyền sở hữu file.
