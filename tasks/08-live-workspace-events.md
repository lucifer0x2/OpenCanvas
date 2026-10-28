# Task 08 — Viewer cập nhật realtime ở cấp workspace: project mới và hoạt động của project khác

**Trạng thái:** HOÀN TẤT 2026-10-28 (công tắc "Follow agent" mặc định tắt; root nghiệm thu độc lập, xem `docs/TASK08-REPORT.md`). Kế hoạch gốc: chưa giao; kế hoạch lập 2026-10-28, mọi `file:line` dưới đây đã mở kiểm lại cùng
ngày. Có một quyết định mở cho người dùng (chế độ theo dõi, xem Việc phải làm mục 5).
**Phụ thuộc:** không phụ thuộc logic vào Task 06/07. Về workspace: chỉ bắt đầu khi phiên Task 04
(row wrap) không còn sửa `crates/server/`, `web/`, `spikes/e2e/`; kiểm `git status` trước khi nhận file.
**Phạm vi sở hữu:** `crates/server/src/events.rs`, file mới `crates/server/src/workspace_events.rs`,
`crates/server/src/http.rs` (một route), `crates/server/src/mcp.rs` (hàm `create`, chỉ phát event),
`crates/server/src/lib.rs` (khai báo module), test mới `crates/server/tests/workspace_events.rs`,
`web/src/hooks/` (tách `useProject.ts`), `web/src/state.ts`, `web/src/App.tsx` (chỉ project
selector), `web/tests/state.test.ts`, file mới `spikes/e2e/live-workspace.mjs` cùng một dòng gọi
trong `spikes/e2e/run.mjs`, `docs/CONTRACT.md`, `docs/VIEWER.md`.
Không sửa `crates/storage`, `crates/layout`, `renderer/`, `PLAN.md`, `AGENTS.md`.
**Workspace:** `/Users/luis/Workspace/OpenCanvas`, branch `main`. Không reset/checkout, không
commit/push khi chưa được yêu cầu.

## Bối cảnh

Viewer ở `http://localhost:3847` **đã** realtime cho project đang mở:

- Viewer mở `EventSource` tới `/api/v1/projects/{id}/events` (`web/src/hooks/useProject.ts:133`).
  Nhận `commit` thì đọc lại snapshot rồi scene; mất kết nối thì tự nối lại có backoff.
- E2E đã kiểm "production SSE token update without refresh" (`spikes/e2e/run.mjs:119`).
- Root đã probe chỉ-đọc trên server thật ngày 2026-10-28: stream trả `text/event-stream` kèm
  heartbeat; replay `cursor=…:0` ra đúng `event: commit` revision 1.

Ngoài project đang mở thì không có gì realtime:

1. **Danh sách project chỉ tải một lần** khi viewer mount (`useProject.ts:30`–`:53`, effect phụ
   thuộc `[reload]`). Agent gọi `create_project` thì project mới không hiện cho tới khi người dùng
   reload trang.
2. **`create_project` không phát event nào** (`crates/server/src/mcp.rs:96`). Hàm này dùng chung
   cho cả MCP và `POST /api/v1/projects`. Broadcast hiện chỉ có `commit` (`domain.rs:60`) và
   `scene_ready` (`jobs.rs:229`).
3. **Agent sửa project khác thì viewer im lặng.** Stream gắn với một project. Người dùng đang xem
   project A thì không biết agent vừa commit vào B.
4. **Stream của project này tốn DB read khi project khác commit.** Gặp broadcast của project khác,
   vòng lặp `continue` về đầu (`crates/server/src/events.rs:92`), tức gọi lại `commits_after`
   (`events.rs:54`). Gặp `scene_ready` của chính nó cũng rơi về replay. Mỗi commit ở B làm mọi
   stream đang mở của project khác chạy một lượt đọc SQLite vô ích.

**Không dùng WebSocket.** Viewer chỉ đọc, chỉ cần server → client. SSE đã có cursor, durable
replay, `reset_required`, heartbeat và tự nối lại; PLAN §1 chốt viewer dùng HTTP + SSE.

**Ràng buộc kết nối.** Server phục vụ HTTP/1.1 không TLS, nên browser giới hạn khoảng 6 kết nối
đồng thời mỗi origin. Mỗi tab mở **hai** `EventSource` thì 3 tab đã chiếm hết, và mọi request
khác (snapshot, job, asset) treo. Thiết kế phải giữ **tối đa một** `EventSource` mỗi tab.

## Mục tiêu

Mở viewer một lần là thấy mọi thay đổi agent tạo ra qua server này: project mới xuất hiện ngay,
project khác có commit thì được báo, project đang xem vẫn cập nhật như hiện tại. Không reload
trang, không thêm kết nối mỗi tab.

## Việc phải làm

1. **Event mới trên broadcast sẵn có** (`state.rs:40`, sức chứa 128):
   - `create` (`mcp.rs:96`) phát `{"event":"project_created",projectId,slug,name,revision}` sau
     khi tạo thành công. Không phát khi lỗi.
   - Commit giữ nguyên event `commit` hiện tại; stream của project khác dùng lại nó như tín hiệu
     hoạt động, như mục 2.
2. **Stream project (`events.rs`) mang thêm tín hiệu workspace, không thêm kết nối:**
   - Broadcast `project_created` → forward `event: project_created`.
   - `commit` của project **khác** → forward `event: project_activity` với data
     `{projectId,revision}`. **Không** replay DB.
   - `scene_ready` của chính nó → yield như cũ, không replay.
   - Chỉ `commit` của chính project này mới chạy replay durable như hiện tại.
   - Hai event mới là gợi ý không durable, không mang SSE `id`, nên `Last-Event-ID`/cursor của
     commit không đổi nghĩa. Lagged vẫn `reset_required` rồi kết thúc stream như hiện tại.
   - Giữ `events.rs` ≤250 dòng; tách phần forward ra hàm riêng nếu cần.
3. **Stream workspace (`workspace_events.rs`, route `GET /api/v1/events` ở `http.rs`)** chỉ dùng khi
   viewer chưa chọn được project nào (workspace rỗng). Data giống mục 2: `project_created`,
   `project_activity`, heartbeat 10 s, dừng theo `shutting_down()`. Không cursor, không replay;
   nguồn tin cậy là `GET /api/v1/projects`. Đi qua `guard` như mọi route GET.
4. **Viewer: tách `useProject.ts` (207 dòng) trước khi thêm logic:**
   - `useProjectList.ts`: danh sách, refetch.
   - `useProjectStream.ts`: một `EventSource` duy nhất, chọn URL project stream khi có project, URL
     workspace khi chưa có.
   - `useProject.ts`: ghép hai hook, giữ nguyên interface trả về cho `App.tsx`.
   - Mỗi lần stream `onopen`/`reset_required` thì refetch **cả** snapshot lẫn danh sách project,
     vì event mới không durable.
   - `project_created`: chèn/cập nhật danh sách theo `projectId`, giữ thứ tự slug như API.
     Workspace đang rỗng thì tự chọn project mới (đúng hành vi load đầu hiện tại).
   - `project_activity`: đánh dấu project khác "có thay đổi" (chấm trong selector + notice ngắn có
     tên project), bỏ dấu khi người dùng chọn nó.
   - Reducer thuần đặt trong `state.ts` và có unit test.
5. **Quyết định mở cho người dùng:** thêm công tắc "Theo dõi agent". Bật thì viewer tự chuyển sang
   project vừa có `project_activity`/`project_created`. Đề xuất mặc định **tắt**, lưu localStorage,
   vì tự nhảy project khi người dùng đang xem dễ gây mất ngữ cảnh. Worker làm công tắc với mặc định
   tắt; người dùng đổi mặc định thì chỉ sửa một hằng số.
6. **Docs:**
   - `docs/CONTRACT.md:12`: thêm `project_created`/`project_activity` vào mô tả events; thêm
     `GET /api/v1/events`; ghi rõ không durable và nguồn tin cậy là list API.
   - `docs/VIEWER.md`: hành vi danh sách live, dấu hoạt động, công tắc theo dõi, giới hạn một
     `EventSource` mỗi tab.

## Nghiệm thu

### Acceptance Criteria

- Viewer đang mở project A. Agent `create_project` B qua MCP thì B xuất hiện trong selector
  không cần reload, trong vòng 2 s ở máy local.
- Agent commit vào B khi viewer đang xem A: A không refetch snapshot, B hiện dấu hoạt động. Chọn
  B thì thấy revision mới nhất.
- Workspace rỗng: viewer mở stream workspace; tạo project đầu tiên thì viewer tự chọn nó và
  chuyển sang stream project.
- Mỗi tab có đúng **một** `EventSource` tại mọi thời điểm. E2E đếm bằng request listener của
  Playwright, kể cả qua chuyển project và reconnect.
- Commit vào B không làm stream của A gọi `commits_after`. Integration test chứng minh bằng
  revision và chuỗi event của A không đổi; nếu cần thì thêm counter test-only qua
  `cfg(test)`/feature sẵn có, không thêm API công khai.
- Ngữ nghĩa cũ giữ nguyên: durable replay, `id` `projectId:revision`, `reset_required`,
  `scene_ready`. Test transport hiện có pass mà không sửa kỳ vọng.
- Viewer vẫn chỉ gửi GET (`assertReadOnly`, `spikes/e2e/browser.mjs:143`).
- Mọi file mã viết tay ≤250 dòng, kể cả `events.rs`, `run.mjs` (đang 232) và các hook viewer.

### Cách kiểm

```bash
cargo test -p opencanvas-server --offline
cargo clippy -p opencanvas-server --offline -- -D warnings
cargo fmt --all --check
npm --prefix web run build
npm --prefix web test
npm --prefix web run test:browser
npm run test:e2e
git diff --check
```

Mọi kiểm dùng backend tạm, data root tạm, cổng ngẫu nhiên. Không dùng cổng 3847, không ghi
`~/.opencanvas`, không stop/restart server của người dùng. Sau khi merge, người dùng tự restart
server 3847 thì mới có hiệu lực.

### Đầu ra

- Delta mã + test server/viewer/E2E; nhóm E2E mới "live workspace" nằm trong `live-workspace.mjs`.
- Docs cập nhật.
- Báo cáo `docs/TASK08-REPORT.md`: chuỗi event thực tế, số `EventSource` mỗi tab, độ trễ đo được
  từ `create_project`/commit tới UI, lệnh đã chạy kèm kết quả.

## Ràng buộc

- Không WebSocket, không long-polling, không thêm dependency.
- Không thêm log durable cho project creation, không đổi DB schema hay snapshot/API schema version.
- Không dùng `scene_ready` để thay polling job của viewer trong task này (`useScene.ts` poll 500 ms).
  Nếu cần, đề xuất thành task riêng có số đo.
- Không thêm thao tác ghi nào cho viewer; viewer vẫn chỉ GET.
- Không sửa `PLAN.md`, `AGENTS.md`; không commit/push.
