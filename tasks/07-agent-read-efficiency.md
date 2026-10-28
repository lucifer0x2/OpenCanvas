# Task 07 — Giảm vòng gọi, byte và phỏng đoán khi agent đọc design qua MCP

**Trạng thái:** HOÀN TẤT 2026-10-28, đủ 07A–07F (07D được người dùng duyệt; root nghiệm thu độc lập từng lát, xem `docs/TASK07-REPORT.md`, `artifacts/agent-cost/`). Kế hoạch gốc: chưa giao; kế hoạch lập 2026-10-28, mọi `file:line` dưới đây đã mở kiểm lại cùng
ngày. Gồm sáu lát độc lập 07A–07F; mỗi lát là một handoff riêng cho một worker mới. 07D cần
người dùng duyệt trước (xem Cảnh báo phạm vi).
**Phụ thuộc:** 07A làm trước mọi lát khác, vì nó tạo số đo nền. 07F mục 4 và 07D dùng index của
Task 06. Lát nào đụng `crates/server/` thì chờ Task 04 (row wrap) và Task 06 rời các file đó.
**Phạm vi sở hữu:** ghi riêng cho từng lát ở mục Việc phải làm. Hai lát chạm cùng một file thì
không chạy song song. Không lát nào sửa `crates/storage`, `crates/layout`, `web/`, `PLAN.md`,
`AGENTS.md`.
**Workspace:** `/Users/luis/Workspace/OpenCanvas`, branch `main`. Không reset/checkout, không
commit/push khi chưa được yêu cầu.

## Cảnh báo phạm vi

- **07D (`get_changes` + làm giàu `REVISION_CONFLICT`) vượt bảng tool `PLAN.md` §6.** Giá trị
  của nó chủ yếu xuất hiện khi nhiều agent ghi cùng một project. PLAN §2 ghi "Ngoài phạm vi: cộng
  tác nhiều người". Không triển khai khi chưa được duyệt.
- **Khóa lạc quan theo node/frame: đã cân nhắc và không đề xuất.** PLAN §5 chốt "Mỗi batch áp
  dụng cho một project và một expected revision", và "backend không tự merge thay đổi ngầm".
  Revision theo node đổi chính nghĩa của `expectedRevision`, idempotency hash và SSE cursor
  `projectId:revision`. 07D cho phép agent tự rebase phía client với chi phí một lần gọi, và
  server vẫn validate lại toàn batch, nên không cần đổi mô hình revision.
- 07A, 07B, 07C, 07E, 07F nằm trong phạm vi PLAN: đo đạc; M3 "pagination/tối ưu context
  budgets"; "truncation có cursor"; tham số tùy chọn trên tool đã có; hiệu năng nội bộ không đổi
  contract.

## Bối cảnh — cái gì đang tốn và bằng chứng

Số liệu dưới đây đo từ artifact mẫu stacks (`artifacts/sample-stacks/`, frame 182 node):

1. **Design context lặp dữ liệu.** `context-pages.json`: 181 node vừa một trang 60,7 KB, sát trần
   64 KiB. 137/181 node (76 %) có `resolvedProps` trùng `props` (bỏ qua `name`). Tổng
   `resolvedProps` ≈20,2 KB, khoảng 1/3 response. Context còn đẩy **mọi** token/style/asset/font
   của project, không chỉ những cái scope tham chiếu (`crates/server/src/reads/tree.rs:106`).
2. **`inspect_layout` cắt mà không có cursor.** Nhiều hơn 200 node thì trả `truncated:true` rồi bỏ
   phần còn lại (`crates/server/src/mcp.rs:166`). Agent chỉ lấy lại được bằng cách tự gom
   `nodeIds` từ outline. Mỗi lần inspect luôn kèm `lines` của mọi node: `stacks-*-inspect.json`
   ≈35 KB mỗi viewport.
3. **Polling render.** Bounds đa viewport cần compute + strict PNG trên một worker
   (`docs/SERVER.md:129`). Helper của Task 03 poll mỗi 50 ms và vẫn mất thêm 270–350 ms để settle
   (`docs/TASK03-REPORT.md:141`, `:143`). Agent thật poll bằng vòng gọi LLM, mỗi vòng vài giây và
   tốn context, vì `render_frame {jobId}` trả ngay, không chờ.
4. **Thua conflict thì mù.** `REVISION_CONFLICT` chỉ mang `currentRevision`
   (`crates/storage/src/project.rs:94`). `commits_after` có sẵn (`project.rs:140`) nhưng chỉ phục
   vụ SSE HTTP (`crates/server/src/http.rs:39`). MCP resources chỉ có 2 skill
   (`crates/server/src/resources.rs:3`). Agent thua phải kéo lại outline/context.
5. **Mở project mỗi lần đọc.** `state.snapshot` gọi `open_project` mỗi read
   (`crates/server/src/state.rs:73`). Registry chỉ giữ `Weak` (`crates/storage/src/store.rs:170`),
   còn server không giữ `Arc`. Project ID dạng `p_<uuid>` (`store.rs:74`) không qua được kiểm
   slug, nên mỗi read quét `list_projects()` (`store.rs:135`): mở mọi DB trong data root, rồi mở
   thêm connection RW và chạy migrate.
6. **Phân trang serialize lại cả response sau mỗi entry** (`crates/server/src/reads/pagination.rs:87`):
   O(entries × bytes) mỗi trang. `schemas::validate` dựng lại toàn bộ tool schema mỗi lần gọi
   (`crates/server/src/schemas.rs:5`).
7. **Prerender sau commit nhắm sai frame.** Sau mutation, server render 4 frame **đầu tiên** theo
   thứ tự storage, ở kích thước authored, bất kể `changedIds` (`crates/server/src/domain.rs:67`).
   Hàng đợi có 8 chỗ (`crates/server/src/jobs.rs:53`). Project có hơn 4 frame thì sửa frame thứ 6
   không prerender nó, nhưng vẫn chiếm queue mà context read cần.
8. **Cursor dễ gãy khi có người ghi.** Mọi mutation vô hiệu cursor đang dùng (`docs/SERVER.md:145`).
   Đây là rủi ro đã ghi nhận, không có lát riêng: đọc ghim revision cũ cần lịch sử snapshot, mà PLAN
   §7 nói v1 không hứa. `find_nodes` (Task 06) và 07C giảm số trang, nên giảm luôn mức phơi nhiễm.

## Mục tiêu

Với ba luồng chính (tìm node, implement một frame, kiểm implementation ở nhiều viewport), giảm
số vòng gọi và byte vào context. Mọi thay đổi đo được bằng cùng một harness trước và sau. Không
lát nào được báo "nhanh hơn" khi chưa có số.

## Việc phải làm

### 07A — Harness đo chi phí agent (làm trước, chỉ thêm file)

Sở hữu: `spikes/agent-cost/` (mới), `artifacts/agent-cost/` (mới). Import, không sửa,
`spikes/e2e/server.mjs` và `spikes/e2e/protocol.mjs`.

- Dựng backend trên data root tạm, cổng ngẫu nhiên, project mẫu stacks. Mỗi lần gọi MCP ghi
  `{tool, requestBytes, responseBytes, ms}`; mỗi kịch bản ghi tổng số gọi, tổng bytes và wall time.
- Kịch bản: **S1** tìm node có chữ "Tổng quan" và đọc props của nó; **S2** đọc hết design context
  frame stacks; **S3** render + inspect ở 1040/800/390, poll cố định mỗi 250 ms và ghi số poll;
  **S4** hai client cùng base revision, client thua tự phục hồi tới khi commit được; **S5**
  latency `get_outline` khi data root có 1 và 20 project; **S6** project 5.000 node sinh tự động
  cùng cỡ "large" ở `spikes/storage/README.md`.
- Xuất `artifacts/agent-cost/baseline.json`. Mỗi lát sau chạy lại và nộp `after-07X.json`.
  Process và root tạm phải dọn sạch trong `finally`.

### 07B — `inspect_layout` có cursor và chọn trường (trong phạm vi, ưu tiên 1)

Sở hữu: file mới `crates/server/src/inspect.rs` (chuyển `inspect` ra khỏi `mcp.rs`), `mcp.rs`,
`schemas.rs` (schema tool này), `reads/pagination.rs` (thêm helper băm scope từ một `Value`, giữ
nguyên `scope()`/`page()` cũ), `docs/SERVER.md`.

- Thêm `cursor`, `limit` (1–200), `budget`, `fields:["bounds"|"lines"]` (mặc định cả hai, giữ hành
  vi cũ). Cursor gắn `jobId`, `sceneKey`, `nodeIds`, `fields`. Phân trang bằng `pagination::page`.
- `truncated:true` luôn đi kèm `nextCursor`.
- Đo: S3 bytes với `fields:["bounds"]`; scene 250 node đọc đủ qua cursor, không trùng, không thiếu.

### 07C — Design context gọn, tùy chọn (trong phạm vi M3, ưu tiên 2)

Sở hữu: file mới `crates/server/src/reads/compact.rs`, `reads/tree.rs` (chỉ gọi vào compact),
`reads.rs`, `schemas.rs` (thêm property chỉ cho `get_design_context`), `instructions.rs`, skill
`implement-frame.md`, `docs/SERVER.md`.

- `compact:true` (mặc định false, response cũ giữ nguyên từng byte):
  - `resolvedProps` chỉ giữ key có giá trị resolved khác authored; response ghi
    `resolvedPropsMode:"differing"`.
  - `tokens`/`styles`/`assets`/`fonts` chỉ giữ những cái scope tham chiếu, kèm bao đóng
    style→token và alias/theme→token; response ghi `resources:"referenced"`. Lấy đúng tập key
    tham chiếu (`tokenId`, `styleId`, font, asset) từ `crates/storage/src/validate/refs.rs`,
    không tự đoán.
- Diagnostics/reflow vẫn tính trên toàn scope như cũ.
- Field mới của `ReadRequest` dùng `skip_serializing_if`, để scope hash của request cũ không đổi.
- Đo S2 trên mẫu, kỳ vọng giảm tới ~20 KB; thêm fixture 300 token để đo phần resources.

### 07D — `get_changes` và conflict có ngữ cảnh (CẦN DUYỆT, ưu tiên 5)

Sở hữu: file mới `crates/server/src/reads/changes.rs`, `domain.rs`, `error.rs`, `schemas.rs`,
`mcp.rs`, `instructions.rs`, `docs/SERVER.md`.

- `get_changes {projectId, sinceRevision, limit≤100, includeOperations=false, budget}` →
  `{fromRevision, currentRevision, commits:[{revision, changedIds}], changed:{nodes, tokens,
  styles, assets, fonts, removed}, truncated, nextRevision}`. `changedIds` của storage trộn node,
  token và style (`crates/storage/src/validate/candidate.rs:92`–`:194`), nên phân loại bằng
  snapshot hiện tại; id không còn tồn tại vào `removed`. Cursor vượt revision hiện tại thì trả
  `RESET_REQUIRED` như storage.
- `REVISION_CONFLICT` từ `apply_batch`/`import_asset` được server gắn thêm
  `changedSince:{fromRevision, toRevision, changedIds≤200, truncated}` vào object `error`. Giữ
  nguyên code, HTTP status và `currentRevision`. Làm ở server qua trường chi tiết tùy chọn của
  `ApiError`, không đổi `StorageError`.
- Instructions: khi thua, đọc `changedSince`. Nếu không giao với các node mình sửa và tổ tiên của
  chúng, gửi lại batch với `expectedRevision` mới và **idempotencyKey mới** (body đã đổi). Nếu
  giao, đọc lại đúng các id đó bằng `find_nodes ids[]` (Task 06). Server vẫn validate lại toàn
  bộ; rebase sai thì nhận `VALIDATION_FAILED`, không bao giờ ra dữ liệu hỏng.
- Đo S4: số gọi và bytes để phục hồi, trước/sau.

### 07E — Chờ job có giới hạn khi poll (trong phạm vi, ưu tiên 4)

Sở hữu: file mới `crates/server/src/jobs/wait.rs`, `jobs.rs` (chỉ thêm tín hiệu hoàn tất; file
đang 233 dòng và phải ≤250), `mcp.rs` (nhánh render), `schemas.rs`, `docs/SERVER.md`.

- `render_frame {projectId, jobId, waitMs 0–10000}`: trả ngay khi job rời `pending`, hết hạn thì
  trả trạng thái hiện tại. Không giữ DB handle hay read transaction khi chờ. Không thêm chờ vào
  `get_design_context`, giữ quyết định của Task 03; agent dùng `waitMs` trên các `jobId` mà
  context trả về.
- Đo S3: số poll trước/sau ở cùng nhịp 250 ms.

### 07F — Hiệu năng nội bộ, không đổi contract (ưu tiên 3)

Sở hữu: `state.rs`, `reads/pagination.rs`, `schemas.rs` (cache), `domain.rs`, `reads/viewports.rs`.
Chạy sau 07B nếu cùng chạm `pagination.rs`.

1. Giữ `Arc<Project>` theo id và slug trong `AppState`, để `open_project` đi đường registry nhanh
   thay vì quét mọi project. Trước khi làm, đọc `crates/storage/src/bundle_import.rs` và
   `store.rs` để chắc rằng giữ handle không cản import/restore. Có rủi ro thì dừng và báo.
2. Phân trang cộng dồn byte theo entry, không serialize lại cả response. Test chứng minh ranh giới
   trang và bytes trùng hệt cách cũ trên nhiều budget.
3. Cache tool list (`OnceLock`) cho `validate`, `list_tools`, `get_tool`.
4. Prerender nhắm các frame chứa `changedIds` (đi tổ tiên bằng index của Task 06), tối đa 4. Thay
   đổi chỉ ở token/style thì giữ hành vi cũ.
5. `scope_ids` của `viewports.rs` (vòng lặp điểm bất động) chuyển sang index.

- Đo S5, S6 và S2 latency; queue sau khi sửa frame thứ 6 trong project 6 frame.

## Nghiệm thu

### Acceptance Criteria

- 07A có trước; mọi lát nộp `after-07X.json` cạnh baseline. Báo cáo nêu delta gọi/bytes/ms và
  không làm tròn có lợi.
- Mặc định của mọi tool cũ giữ nguyên response từng byte, trừ khi lát ghi rõ là sửa lỗi (07B:
  `truncated:true` nay có `nextCursor`). Test hiện có pass không sửa kỳ vọng.
- Mọi giới hạn mới có lỗi client-safe theo mã hiện có. Không lát nào cắt âm thầm.
- Mọi file mã viết tay ≤250 dòng; `jobs.rs`, `mcp.rs`, `tree.rs` không vượt độ dài hiện tại cộng 10.
- `docs/SERVER.md` ghi hình dạng mới; `instructions.rs` và skill chỉ đổi ở lát có hành vi mới cho
  agent.

### Cách kiểm

```bash
cargo test -p opencanvas-server --offline
cargo clippy -p opencanvas-server --offline -- -D warnings
cargo fmt --all --check
git diff --check
node spikes/agent-cost/run.mjs --label after-07X
```

Harness và test dùng data root tạm, cổng ngẫu nhiên; không dùng cổng 3847, không ghi
`~/.opencanvas`. Lát nào đổi luồng MCP→viewer thì chạy thêm `npm run test:e2e`.

### Đầu ra

- Mỗi lát: delta mã + test, `artifacts/agent-cost/after-07X.json`, và một đoạn trong
  `docs/TASK07-REPORT.md` gồm lệnh thật, số trước/sau và quyết định thiết kế kèm lý do.

## Ràng buộc

- Một worker một lát; không gộp lát để "tiện tay". 07D chỉ bắt đầu khi có câu duyệt của người dùng.
- Không đổi mô hình revision, idempotency hash, SSE cursor hay schema DB/scene/API version.
- Không thêm dependency. Không subscription MCP resource cho thay đổi: hỗ trợ của client chưa đồng
  đều, và `get_changes` đủ cho agent kéo theo nhu cầu.
- Không thêm đường bounds-only cho worker trong task này. Nếu số đo 07A cho thấy cold render là nút
  thắt chính, đề xuất thành task riêng có duyệt.
- Không sửa `PLAN.md`, `AGENTS.md`, `crates/storage`, `crates/layout`, `web/`; không commit/push.
