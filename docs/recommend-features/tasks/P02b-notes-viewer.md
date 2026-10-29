# P02b — Viewer hiển thị note (chỉ đọc) và không render lại khi commit chỉ có note
Ưu tiên: P1 · Kích cỡ: ~85 phút dev · Phụ thuộc: **P02a** (đã merge vào `night/integration`) · Nguồn: U4

Phase hai của P02. Người dùng và designer cần thấy note trên viewer: pin trên canvas, thread trong Inspector, badge ở Layers. Phase này giữ nguyên nguyên tắc viewer chỉ đọc. Tạo, trả lời và resolve từ viewer cần người dùng duyệt (xem cuối file).

## Vấn đề

1. Sau P02a, note chỉ đọc được qua MCP. Người xem viewer không biết node nào có ý đồ hoặc câu hỏi đang mở.
2. Mỗi commit, kể cả commit chỉ có note, làm viewer:
   - refetch snapshot;
   - đổi `revision` của scene request;
   - xin render lại frame đang xem.
   `useScene` đặt `status:"pending"` và bỏ `scene` (`web/src/hooks/useScene.ts:43`), nên canvas trống trong lúc render (`CanvasView.tsx` chỉ vẽ khi `scene && bitmap`). Hậu quả: mỗi lần agent ghi note, canvas nháy, và tốn thêm một job render (liên quan note #7, B01).

## Bằng chứng

- Snapshot HTTP serialize nguyên `Snapshot` (`crates/server/src/http.rs:83-87`), nên sau P02a đã có `notes`. `web/src/types.ts:18-22` chưa khai báo field này.
- Scene request lấy `revision: snapshot.revision` (`web/src/App.tsx:36-46`). `useScene` chỉ refetch khi `JSON.stringify(request)` đổi (`useScene.ts:32`).
- Commit SSE (`useProject.ts:94-99`) gọi `snap.refresh()`, revision mới làm request đổi theo.
- Server từ chối scene ở revision cũ (`docs/CONTRACT.md:15`, `jobs.rs:111-116`). Vì vậy viewer không thể xin scene cũ, chỉ có thể **giữ** request cũ khi frame và viewport không đổi.
- SSE `commit` dựng từ log bền (`crates/server/src/events.rs:65`). `Commit.operations` có sẵn ops của commit, nên server biết được commit chỉ có note nhờ `is_note_batch` của P02a. `types.ts:54-57` `CommitEvent` chưa có `changedIds` hay cờ nào.
- Có sẵn chỗ để gắn UI:
  - overlay selection vẽ bằng Konva `Rect {...bounds}` trong cùng `Layer` (`CanvasView.tsx:109-116`): pin dùng cùng toạ độ frame-local;
  - Inspector có các `section.inspect-section` (`Inspector.tsx:41-90`);
  - Layers có nút từng node (`Layers.tsx:41`).
- Viewer chỉ đọc theo sản phẩm (`PLAN.md:19`, `:28`, mục 8 ở `PLAN.md:332`), và `web/src/api.ts:13` chỉ có GET.
- Test web: `node --test` trên TS (`web/package.json` script `test`, `web/tests/state.test.ts`).
- Tham khảo:
  - Figma và Penpot hiển thị comment bằng pin có số trên canvas, mở thread ở panel, ẩn thread đã resolve theo mặc định (Penpot `CommentThread.seqNumber/resolved`: https://doc.plugins.penpot.app/interfaces/CommentThread).
  - Dev Mode của Figma hiển thị annotation gắn node ngay cạnh node: https://developers.figma.com/docs/plugins/api/Annotation/

## Giải pháp

### Server (nhỏ)
SSE `commit` thêm `"notesOnly":true` **chỉ khi** `opencanvas_storage::is_note_batch(&commit.operations)` (`events.rs:65`, +3 dòng). Field vắng mặt nghĩa là commit có thay đổi thiết kế.
Cập nhật `docs/CONTRACT.md:12`: `{projectId,revision,changedIds,notesOnly?}`.

### Giữ scene khi commit chỉ có note
- `useProject` theo dõi `lastDesignCommit`: revision lớn nhất trong các event `commit` không có `notesOnly`. Khi `onOpen`, `onReset`, gap hoặc event không parse được, đặt "không rõ" (`Infinity`) để buộc render lại.
- Helper thuần `heldSceneRevision(held, key, snapshotRevision, lastDesignCommit)` trong `web/src/notes.ts`, với `key = frameId|width|height|retryKey`:
  - giữ `held.revision` khi `held.key === key` và `lastDesignCommit <= held.revision`;
  - các trường hợp còn lại trả `{key, revision: snapshotRevision}`.
- `App` dùng giá trị này cho `request.revision`. Kết quả: commit chỉ có note thì snapshot (và note) cập nhật, scene giữ nguyên, không có request `/scene` mới, không nháy. Đổi frame/viewport, bấm retry hay có commit thiết kế thì xin scene ở revision hiện tại như cũ.
- Status bar vẫn ghi `Scene r<held>`, đúng sự thật.

### Hiển thị note (chỉ đọc)

**Kiểu dữ liệu.** `types.ts` thêm:
- `Note {id,nodeId,text,author?,resolved?,replies?: {text,author?}[]}`;
- `Snapshot.notes?: Note[]`;
- `CommitEvent.changedIds?`, `notesOnly?`.

**Helper thuần** trong `web/src/notes.ts`:
- `openCountByNode(notes)`;
- `pinsFor(scene, notes)`: mỗi node trong scene có note open thì có một pin, gồm `{nodeId, count, x, y}`. `x = bounds.x + bounds.width` và `y = bounds.y`, kẹp trong viewport. Note của node không có trong scene bị bỏ qua;
- `threadsFor(target, frame, nodes, notes)` trả `{open, resolved, inherited}`. `inherited` là note của tổ tiên frame (page) và chỉ có khi target là frame.

**Canvas** (`components/NotePins.tsx`, Konva):
- Vòng tròn màu hổ phách `#d97706`, khác màu xanh của selection. Bán kính `9/scale`, chữ số đếm `11/scale`, nên kích thước trên màn hình không đổi khi zoom.
- Chỉ vẽ note open.
- Bấm pin → `select(nodeId)` với `cancelBubble`, để hit-test của Stage không chọn node bên dưới.
- `aria-label`/`title` của pin là "N open notes".

**Inspector** (`components/NoteThreads.tsx`):
- Section "Notes" cho node đang chọn; khi chưa chọn node thì cho frame.
- Thread open trước. Mỗi thread hiện `author`, `text` (giữ xuống dòng, `white-space: pre-wrap`) và replies thụt lề. Thread resolved gom trong `<details>` "N resolved".
- Khi target là frame, có thêm khối "On page <tên>" cho note kế thừa.
- Không có nút sửa. Text hiển thị dạng thuần, không render Markdown, để tránh rủi ro XSS.

**Layers.** Badge số note open bên phải tên node (`Layers.tsx`, +6 dòng).

**CSS.** File mới `web/src/notes.css`, vì `workspace.css` đã 229 dòng.

Lý do chọn: pin theo node, không theo toạ độ, vì note của P02a gắn node; layout reflow thì pin đi theo `bounds`. Một pin cho mỗi node (đếm số thread) để các thread cùng node không chồng nhau.

## Phạm vi file

| File | Việc | Ước lượng |
| --- | --- | --- |
| `crates/server/src/events.rs` | `notesOnly` | +3 |
| `crates/server/tests/notes_sse.rs` (mới) | SSE: note-only → `notesOnly:true`; design → vắng | ~70 |
| `web/src/types.ts` | `Note`, field mới | +14 |
| `web/src/notes.ts` (mới) | helper thuần ở trên | ~90 |
| `web/src/components/NotePins.tsx` (mới) | pin Konva | ~55 |
| `web/src/components/NoteThreads.tsx` (mới) | section Inspector | ~90 |
| `web/src/components/Inspector.tsx` | gắn section | 100 → ~110 |
| `web/src/components/CanvasView.tsx` | vẽ `NotePins` | 134 → ~142 |
| `web/src/components/Layers.tsx` | badge | 102 → ~110 |
| `web/src/hooks/useProject.ts` | `lastDesignCommit` | 138 → ~150 |
| `web/src/App.tsx` | revision giữ lại, truyền note | 231 → ≤245 |
| `web/src/notes.css` (mới), `web/src/main.tsx` | style, import | ~45, +1 |
| `web/tests/notes.test.ts` (mới), `web/package.json` | unit test helper; thêm vào script `test` | ~120, +1 |
| `docs/VIEWER.md`, `docs/CONTRACT.md` | mục Notes, `notesOnly` | +15, +1 |

Nếu `App.tsx` vượt 245 dòng thì tách phần dựng scene request thành `hooks/useSceneRequest.ts`.

## Tương thích MCP

- Không đổi tool, schema hay `tools/list`.
- SSE chỉ thêm field tuỳ chọn `notesOnly`; client cũ bỏ qua.
- Viewer không thêm route ghi: `web/src/api.ts` giữ GET-only.

## AC

1. **SSE.** Server tạm, batch chỉ gồm op note → event `commit` có `notesOnly:true`. Batch có `update_node` → không có key `notesOnly`. Replay khi kết nối lại bằng cursor cũ cho cùng kết quả (test `notes_sse.rs`).
2. **`heldSceneRevision`** (unit test), với `held = {key:k, revision:5}`:
   - snapshot r6, `lastDesignCommit` 5 → giữ 5;
   - commit thiết kế r7 (`lastDesignCommit` 7) → 7;
   - đổi key → revision snapshot;
   - `lastDesignCommit = Infinity` → revision snapshot.
3. **Helper hiển thị** (unit test):
   - `pinsFor` trả một pin cho mỗi node có note open, `count` đúng, toạ độ `(x+width, y)` kẹp trong viewport;
   - bỏ node không có trong scene và note resolved;
   - `threadsFor` tách open/resolved; `inherited` chỉ có khi target là frame.
4. **Live, không render lại** (Browser pane trên server tạm). Mở viewer ở frame có note, rồi `apply_batch` `create_note`:
   - ≤2 s sau có pin mới và Inspector cập nhật;
   - `read_network_requests` không có request `/api/v1/projects/*/scene` mới sau commit đó;
   - status bar vẫn `Scene r<cũ>`.
   Sau đó `update_node` cùng frame → có đúng 1 request `/scene` mới với revision mới.
5. **Pin và chọn node.**
   - Bấm pin → node gắn note được chọn (viền xanh, Inspector đổi).
   - Pin giữ ~18 px CSS khi zoom 25% và 400% (đo bằng `getClientRect`, sai số ±2 px).
   - Note resolved không có pin.
6. **Inspector.**
   - Node có 2 open và 1 resolved: hiện 2 thread, `<details>` "1 resolved" đóng sẵn.
   - Khi chưa chọn node: hiện note của frame và khối "On page …" cho note của page.
   - Text có `<script>` hiển thị nguyên văn, không thực thi.
7. **Layers.** Badge đúng số note open, mất khi resolve (live).
8. **Chỉ đọc.** `rg -n "method:\s*\"(POST|PUT|PATCH|DELETE)\"" web/src` không có kết quả; `api.ts` không đổi.
9. **Build.**
   - `npm --prefix web run build` (tsc + vite) pass.
   - `npm --prefix web test` pass, gồm `tests/notes.test.ts`.
   - `cargo test -p opencanvas-server --offline --test notes_sse --test workspace_events` pass.
10. **Modular.** File mới ≤ 200 dòng; `App.tsx` ≤ 245; báo số dòng các file đã chạm.

## Cách kiểm

```bash
export CARGO_BUILD_JOBS=4
cargo test -p opencanvas-server --offline --test notes_sse --test workspace_events -- --test-threads=2
npm --prefix web test && npm --prefix web run build
wc -l web/src/App.tsx web/src/notes.ts web/src/components/{NotePins,NoteThreads,Inspector,CanvasView,Layers}.tsx web/src/hooks/useProject.ts
```

Kiểm live (QA):
1. Chạy server tạm `--port 0 --data-root $(mktemp -d)`, phục vụ `web/dist`.
2. Tạo project, frame và note qua MCP (curl JSON-RPC).
3. Mở viewer bằng Browser pane, lưu ý Host phải là `127.0.0.1:<port>`.
4. Kiểm AC 4–7, chụp màn hình pin và Inspector vào `.night/qa/`.
5. Dọn process và data.

## Rủi ro

- **Giữ scene cũ khi lẽ ra phải render lại** (cờ sai, hoặc commit hỗn hợp bị coi là chỉ note). Giảm nhẹ:
  - `notesOnly` tính từ ops thật ở server;
  - mọi trường hợp không rõ (gap, reset, reconnect, parse lỗi) đều buộc render lại;
  - unit test cho các ca biên.
- **Hit-test pin và Stage.** Thiếu `cancelBubble` thì bấm pin sẽ chọn node bên dưới. AC 5 kiểm ca này.
- **Snapshot lớn.** Tối đa 5000 note × ~150 byte ≈ 750 KB thêm vào `GET /snapshot`, chấp nhận được. Nếu thành vấn đề, chuyển sang `GET /notes` (phase sau).
- **Xung đột file nóng** `web/src/App.tsx` và `CanvasView.tsx` với P01 (zoom luôn nét). Dev orchestrator xếp tuần tự hoặc rebase.

## Ngoài phạm vi / phase sau

- **Tạo, trả lời và resolve note từ viewer: CẦN NGƯỜI DÙNG DUYỆT, không triển khai trong đợt đêm.** Việc này mâu thuẫn với "viewer chỉ đọc, không có editing" (`PLAN.md:19`, `:28`, mục 8; `web/src/api.ts:13`; `docs/CONTRACT.md:10`). Phác thảo để người dùng quyết:
  - Route `POST /api/v1/projects/:id/notes` nhận `{expectedRevision, idempotencyKey, operations}`. Server từ chối nếu `!is_note_batch` (chỉ cho op note), rồi gọi lại `domain::mutate`.
  - Guard Host/Origin hiện có đã buộc mutation cùng origin (`crates/server/src/http.rs:132-159`), nên giảm được CSRF từ trang lạ.
  - UI: ô soạn trong `NoteThreads` (reply/resolve/mở lại), `author:"viewer"`, idempotency key dạng `crypto.randomUUID()`, tự retry khi `REVISION_CONFLICT`. Ước lượng ~80 phút.
- `GET /notes` riêng cho project rất lớn; lọc note theo page trong Layers; số thứ tự pin toàn project kiểu Penpot `seqNumber`.
