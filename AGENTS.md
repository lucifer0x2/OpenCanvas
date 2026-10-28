# OpenCanvas — quy tắc làm việc cho agent

File này được nạp vào mọi phiên (Codex, Claude Code), nên chỉ chứa quy tắc, trạng thái hiện tại và tham chiếu.
Nhật ký đang dùng ở `docs/agents/LOG.md`; lịch sử ở `docs/agents/history/`.

## Quy tắc làm việc

- Đọc `PLAN.md` trước khi triển khai. Đây là nguồn yêu cầu sản phẩm và AC.
- Task đã đóng gói chờ giao nằm ở `tasks/`; đọc trước khi tự định nghĩa việc mới. Mỗi file là một handoff đầy đủ, không phải ghi chú.
- Phiên chính chỉ điều phối, kiểm chứng và cập nhật tài liệu tiến độ; mọi triển khai giao cho subagent mới, một task mỗi agent. Không tái sử dụng agent đã hoàn thành, worker không tạo agent tiếp.
- Handoff phải có mục tiêu, phạm vi file, workspace/branch, AC, cách kiểm và đầu ra. Dùng `fork_turns="none"`, giữ model/effort mặc định.
- Dùng `rg`/`rg --files`, đọc bằng shell; dùng `apply_patch` sửa mã. Không dùng Python cho thao tác text thông thường. Loại node_modules, .git, .venv, dist, build khỏi tìm kiếm.
- Ưu tiên RTK tường minh khi hỗ trợ (`rtk ls`, `rtk git status`); dùng output gốc cho diff, điều tra lỗi và bằng chứng. Không tự bật hook/proxy/experimental. Xem `/Users/luis/.codex/RTK.md`.
- Ghi chú công cụ: `fork_turns`, `apply_patch` và hướng dẫn RTK ở `~/.codex/RTK.md` dành cho Codex. Trong Claude Code, tương đương là subagent mới không fork ngữ cảnh và sửa bằng Edit/Write.
- Chỉ chạy kiểm tra nhắm đúng vùng thay đổi, hạn chế song song; không ghi pass khi chưa chạy. Không tự tạo chat độc lập.
- Không ghi dữ liệu demo vào `~/.opencanvas`; tests dùng data root tạm. Chỉ API create_project thực mới khởi tạo project người dùng.
- Ghi `docs/agents/LOG.md` tại mỗi lần giao việc, nhận kết quả, phát hiện blocker và trước khi dừng (cách ghi ở mục "Nhật ký và memory"). Không ghi secret hoặc đường dẫn dữ liệu riêng của người dùng vào API.

## Modular Architecture Rules — bổ sung của người dùng 2026-10-28

- Luôn ưu tiên module nhỏ, mỗi file một trách nhiệm; theo kiến trúc hiện có, public API nhỏ và tường minh.
- Trước feature: khảo sát cấu trúc, xác định ranh giới trách nhiệm và các file tạo/sửa, rồi triển khai tách module.
- Mục tiêu 100–200 dòng/file; quá 250 dòng phải đánh giá tách; tránh quá 350 dòng mã viết tay. Ngoại lệ: generated/config hoặc tách làm giảm khả năng đọc, phải nêu lý do.
- Tách business logic thành services, hàm dùng lại thành utilities, types/interfaces khi hữu ích, phần UI thành components và stateful UI logic thành hooks phù hợp.
- Refactor file lớn khi thay đổi làm chúng phình thêm; giữ hành vi/public interface, tránh circular dependencies và abstraction không cần thiết; cập nhật imports/tests sau tách.
- Nghiệm thu phải kiểm số dòng, một trách nhiệm/module, chạy test/lint/type check liên quan và báo mọi file còn quá ngưỡng khuyến nghị.
- Áp dụng cho mọi handoff. Bản gốc của người dùng nằm trong memory chung (`$CLAUDE_CONFIG_DIR/CLAUDE.md` cho Claude Code, hướng dẫn chung của Codex).

## Nhật ký và memory

- `AGENTS.md` chỉ chứa quy tắc, trạng thái hiện tại và tham chiếu. Ngân sách ≤120 dòng / ≤10 KB; kiểm `wc -l -c AGENTS.md` trước khi dừng phiên.
- Mục "Trạng thái hiện tại" tối đa ~10 dòng, được **thay thế** khi trạng thái đổi, không nối thêm. Không ghi PID vào `AGENTS.md`.
- Nhật ký giao việc, nhận kết quả, blocker và điểm dừng ghi vào `docs/agents/LOG.md` (append-only): một heading có ngày cho mỗi task (`### <task> — YYYY-MM-DD`), mỗi lần ghi vài dòng, chỉ quyết định, kết quả cuối, blocker và link báo cáo.
- Không chép lại nội dung đã có trong `docs/TASK*-REPORT.md`, `tasks/`, output test hay git history; không ghi số liệu trung gian. Chi tiết kỹ thuật dài thuộc report/docs của task, không thuộc nhật ký.
- Khi `LOG.md` vượt ~300 dòng: chuyển các mục đã đóng sang `docs/agents/history/<ngày>.md`, giữ một dòng con trỏ trong `LOG.md`.
- Memory ngoài repo (thư mục memory của Claude Code, `~/.codex/memories`, file CLAUDE.md/AGENTS.md chung) chỉ lưu con trỏ hoặc sự thật cần dùng xuyên phiên, không chép nhật ký repo.

## Trạng thái hiện tại — 2026-10-28

- Task01–08 đã nghiệm thu (`215313c` trên `main`, đã push). Nhánh `night/integration` gồm `215313c` cộng một commit nền: schema `render_frame` phẳng, cache hints MCP 2026-07-28, bản Docker, `AGENTS.md` → `docs/agents/`. Nhánh này chưa push.
- **Đợt chạy đêm 2026-10-28:** đội R&D (nghiên cứu → dev → kiểm thử) làm theo `/Volumes/MySpace/Workspace/OpenCanvas/.night/TEAM.md`. Hạn chót tuyệt đối là 03:00 ngày 2026-10-29. Trong đợt này, quy tắc của TEAM.md (worktree riêng, commit trên nhánh `night/*`, không push) thay cho các quy tắc "phiên chính chỉ điều phối", "không tự tạo chat" và "không commit" ở trên.
- Cổng 3847 do container compose `opencanvas` phục vụ, volume `opencanvas-data` chứa dữ liệu thật của người dùng. Chỉ orchestrator gốc được restart. Kiểm live bằng server tạm `--port 0` + data root tạm, dọn process/data sau khi kiểm.
- Giới hạn đã công bố vẫn hiệu lực: default interaction state, không arbitrary authored history, deep layout explanation/diff, log pruning/GC, runtime hot replacement, backup UI/CLI/MCP; Task04 chỉ row wrap, `%`/breakpoint cần duyệt riêng. Chi tiết ở `docs/ACCEPTANCE.md`.
- Không reset/checkout/stash hay bỏ thay đổi của người khác; không push; không ghi `~/.opencanvas`.
- Khi có yêu cầu mới: đọc mục này, mục đang mở trong `docs/agents/LOG.md`, README/ACCEPTANCE và `git status`.

## Tham chiếu

| Đường dẫn | Đọc khi |
| --- | --- |
| `PLAN.md` | Trước mọi triển khai: nguồn yêu cầu sản phẩm và AC. |
| `README.md` | Cần setup, chạy app hoặc kết nối MCP. |
| `docs/ACCEPTANCE.md` | Nghiệm thu, cần AC, lệnh kiểm hoặc giới hạn đã công bố. |
| `tasks/` | Trước khi định nghĩa việc mới. 01–05 sinh từ điều tra "export HTML không căn theo màn hình" (frame mẫu author toàn `layout.mode:"free"` nên không có thông tin reflow); 06–08 là `find_nodes`, giảm chi phí đọc MCP, live workspace events. Trạng thái ghi ở đầu mỗi file. |
| `docs/TASK*-REPORT.md`, `docs/TASK04-PROPOSAL.md` | Cần chi tiết kỹ thuật, số đo hoặc rủi ro còn mở của Task03–08. |
| `docs/SERVER.md` | Sửa hoặc gọi backend: MCP, HTTP API, jobs, SSE. |
| `docs/CONTRACT.md` | Tích hợp giữa các module (viewer ↔ HTTP API, kiểu dữ liệu). |
| `docs/VIEWER.md` | Sửa viewer `web/` hoặc renderer dùng chung. |
| `docs/RESOLUTION.md` | Đụng token, alias, theme hoặc resolver M3. |
| `docs/BACKUP.md` | Đụng migration, export/import/restore bundle M4. |
| `docs/checkpoints/` | Cần bối cảnh bàn giao cũ của M1 server, M3, M4. |
| `spikes/e2e/README.md` | Chạy hoặc sửa E2E thật (`npm run test:e2e`). |
| `docs/agents/LOG.md` | Đầu phiên (mục đang mở) và mỗi lần ghi nhật ký. |
| `docs/agents/history/` | Cần lịch sử quyết định: contract M0, bằng chứng cũ, nhật ký Task01–08. |
