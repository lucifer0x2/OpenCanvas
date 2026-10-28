# Task 06 — Tìm phần tử theo tên, loại và nội dung chữ: tool `find_nodes`

**Trạng thái:** HOÀN TẤT 2026-10-28 (người dùng duyệt `find_nodes`; root nghiệm thu độc lập, xem `docs/TASK06-REPORT.md`). Kế hoạch gốc: chưa giao; chờ người dùng duyệt bổ sung một MCP tool vào contract (xem Cảnh báo
phạm vi). Kế hoạch lập 2026-10-28, mọi `file:line` dưới đây đã mở kiểm lại cùng ngày.
**Phụ thuộc:** không phụ thuộc logic vào Task 01–05. Về workspace: chỉ bắt đầu khi phiên Task 04
(row wrap) không còn sửa `crates/server/`; kiểm `git status crates/server` trước khi nhận file.
**Phạm vi sở hữu:** `crates/server/src/schemas.rs` (thêm schema tool), `crates/server/src/reads.rs`,
`crates/server/src/reads/tree.rs` (chỉ thay traversal, giữ hành vi), file mới
`crates/server/src/reads/index.rs` và `crates/server/src/reads/find.rs`, một dòng dispatch trong
`crates/server/src/mcp.rs`, `crates/server/src/instructions.rs`,
`crates/server/skills/implement-frame.md`, `crates/server/skills/author-frame.md`,
test mới `crates/server/tests/find_nodes.rs`, `docs/SERVER.md`.
Không sửa `crates/storage`, `crates/layout`, `web/`, `PLAN.md`, `AGENTS.md`.
**Workspace:** `/Users/luis/Workspace/OpenCanvas`, branch `main`. Không reset/checkout, không
commit/push khi chưa được yêu cầu.

## Cảnh báo phạm vi

Bảng MCP tool ở `PLAN.md` §6 liệt kê 13 tool đã cam kết; `find_nodes` không có trong bảng. Đây là
bổ sung contract chỉ đọc: không đổi mô hình dữ liệu, storage, viewer hay tool cũ. Vẫn phải được
người dùng duyệt trước khi triển khai. Sau khi duyệt, phiên điều phối (không phải worker) thêm
một dòng vào bảng §6. Worker không sửa `PLAN.md` vì file này đang thuộc Task 04.

## Bối cảnh

Hôm nay agent muốn tìm "cái card giá bên phải" chỉ có một đường: `get_outline` cả project hoặc
cả frame rồi tự grep trong context.

- `get_outline` trả đúng `id/parentId/kind/name/sortKey` mỗi node
  (`crates/server/src/reads/tree.rs:56`). Nó không có nội dung chữ, nên "card có chữ *Giá*"
  không tìm được bằng outline; agent phải gọi thêm `get_subtree`.
- Schema `tree` (`crates/server/src/schemas.rs:46`) chỉ có `nodeId`/`frameId`, `depth`,
  `limit` ≤200, `budget` ≤65536, `properties[]`, `cursor`. `properties[]` lọc **key của props**
  (`tree.rs:60`), không lọc node. `ids[]` ở `schemas.rs:74` chỉ thuộc bốn tool `list_*` và là
  exact match.
- Đo trên frame mẫu `welcome-dashboard-stacks` (`artifacts/sample-stacks/authored-snapshot.json`):
  182 node (122 group, 41 text, 17 rectangle); dữ liệu 5 trường kiểu outline ≈17,7 KB. Tên node
  phần lớn là cấu trúc (`space-29`, `brand-row-content`…), nội dung người dùng nhìn thấy nằm ở
  `props.text` của 41 node text. Một project vượt 200 node buộc agent đi nhiều trang.
- `depth` **không** cắt cây: nó dừng trang tại node đầu tiên sâu hơn
  `base_depth + depth`, với `base_depth` là độ sâu của entry đầu trang
  (`crates/server/src/reads/pagination.rs:64`, `:71`; `docs/SERVER.md:144`). Gọi
  `get_outline depth:1` trên page có frame chứa con chỉ nhận page + frame đầu tiên rồi
  `truncated:true`. Muốn liệt kê mọi frame vẫn phải duyệt hết cây.
- `visit()` (`tree.rs:142`) lọc lại toàn bộ `s.nodes` cho từng node: traversal O(n²). Mỗi read
  còn mở project và đọc cả snapshot (`crates/server/src/state.rs:73`). Tìm kiếm phải quét cả
  scope nên kế thừa chi phí này.

### Phản biện hướng rẻ: thêm `kinds[]`/`nameContains` vào schema `tree`

Ưu điểm có thật: không thêm tool (danh sách tool là token cố định trong mọi phiên agent), tái
dùng nguyên `pagination::page` và `truncated`. Cursor cũng không phải vấn đề: scope hash băm toàn
bộ query trừ `cursor` (`pagination.rs:15`–`:25`), nên filter mới tự động gắn vào cursor.

Không chọn, vì bốn lý do:

1. **Depth cắt trang sai.** Danh sách đã lọc có độ sâu nhảy cóc. Match sâu hơn
   `base_depth + depth` sẽ dừng trang và trả `truncated:true` dù chưa chạm `limit`/`budget`.
   Agent không phân biệt được "dừng vì depth", "cắt vì budget" và "còn kết quả".
2. **`parentId` treo.** Outline là cây; outline đã lọc trả node có cha không nằm trong response.
   Với 182 node mà đa số tên cấu trúc, agent không định vị được match nếu không kéo lại outline.
   Đúng cái chi phí mà task muốn bỏ.
3. **Schema `tree` dùng chung cho `get_design_context`** (vòng lặp `schemas.rs:49`–`:73`). Lọc
   node trong context phá bất biến "diagnostics/reflow tính trên toàn scope, node là subtree tự
   đủ" (`docs/SERVER.md:105`). Không lọc thì phải tách schema theo tool, tức là vẫn đổi
   contract ba tool thay vì thêm một.
4. **Không có chỗ cho số đếm.** Muốn trả lời "không tìm thấy" một cách tường minh cần
   `totalMatches` trước phân trang. Thêm nó vào outline nghĩa là đổi hình dạng response mà mọi
   client đang đọc.

Một tool riêng có hình dạng response riêng: danh sách match phẳng, mỗi match mang `path` tổ
tiên, `totalMatches` tách khỏi `truncated`. Đổi lại có thêm một mô tả tool (~1 KB trong
`tools/list`); phải đo con số thật.

## Mục tiêu

Agent tìm được node bằng một lần gọi, theo tên, loại, nội dung chữ hoặc danh sách ID. Kết quả
đủ ngữ cảnh để chọn đúng node mà không cần outline. Response luôn phân biệt được ba trường hợp:
không có kết quả, đã trả hết, và còn trang sau.

## Việc phải làm

1. **Index dùng chung (`reads/index.rs`, mới, mục tiêu <80 dòng).** Dựng một lần
   `HashMap<parent_id, Vec<&Node>>`. Thứ tự sibling phải trùng hành vi hiện tại: duyệt `s.nodes`
   theo thứ tự storage, rồi `sort_by_key(sort_key)` ổn định. Cung cấp preorder `(node, depth)`
   cho một root hoặc mọi root, và chuỗi tổ tiên của một node. `tree.rs` thay `visit()` bằng
   index này; output của `get_outline`/`get_subtree`/`get_design_context` phải giữ nguyên từng
   byte. Thay luôn lookup `resolved.iter().find` (`tree.rs:71`, cũng O(n²)) bằng map theo id.
2. **Schema `find_nodes` (`schemas.rs`).** Input, `additionalProperties:false`:
   - `projectId` (bắt buộc); `nodeId` hoặc `frameId` (tùy chọn, loại trừ nhau, mặc định cả project).
   - Tiêu chí, **ít nhất một**, AND giữa các tiêu chí: `kinds[]` (enum 6 kind, OR trong mảng),
     `nameContains` (1–128), `textContains` (1–256, chỉ khớp `props.text` dạng string của node
     text), `ids[]` (≤200, exact; dùng để đọc lô sau `get_changes` của Task 07).
   - `properties[]` (≤20 key): authored props kèm theo mỗi match, cùng ngữ nghĩa lọc cột như
     `tree.rs:60`. Không truyền thì match không mang props.
   - `limit` 1–200 (mặc định 50), `budget` 1024–65536 (mặc định 65536), `cursor`, `revision`.
   - Không có `depth`, `theme`, `viewport`.
   Không có tiêu chí nào thì trả `INVALID_REQUEST`, để tool không thành đường dump cả cây.
3. **So khớp.** Không phân biệt hoa thường bằng `str::to_lowercase` cả hai vế, khớp chuỗi con.
   Không gộp dấu, không chuẩn hoá NFC/NFD: repo chưa có crate `unicode-normalization` và task
   này không thêm dependency. Ghi giới hạn này vào mô tả tool và `docs/SERVER.md`. Node không có
   `props.name` không khớp `nameContains`.
4. **Response (`reads/find.rs`, mới, mục tiêu <180 dòng).**
   `{schemaVersion, projectId, revision, scope, criteria, scannedNodes, totalMatches, matches[],
   truncated, nextCursor, limits}`. Mỗi match:
   `{id, kind, name, parentId, depth, path:[{id,kind,name}], matchedOn:["name"|"text"|"kind"|"id"],
   textPreview?, props?}`.
   - `path` đi từ gốc scope tới cha, tối đa 8 phần tử gần nhất kèm `pathTruncated:true` khi bị cắt.
   - `textPreview` ≤120 ký tự (cắt theo `char`, không theo byte), chỉ khi khớp `textContains`.
   - Thứ tự match là preorder, trùng thứ tự outline.
   - `totalMatches` và `scannedNodes` tính trên toàn scope, trước phân trang. Ngữ nghĩa: không có
     kết quả là `totalMatches:0, matches:[], truncated:false`; còn trang là `truncated:true` +
     `nextCursor`; budget nhỏ không đủ cho một match là `CONTEXT_ITEM_TOO_LARGE` như hiện tại.
5. **Phân trang.** Tái dùng `pagination::scope` và `pagination::page` nguyên trạng. Mọi entry
   mang depth 0 và gọi `page(..., depth = 0, ...)` để luật dừng theo depth (`pagination.rs:71`)
   không bao giờ kích hoạt. Không truyền `usize::MAX`, vì `base_depth + depth` sẽ tràn số. Field
   mới của `ReadRequest` (`crates/server/src/reads.rs:11`) phải có
   `#[serde(skip_serializing_if = "Option::is_none")]`, để JSON băm scope của các tool cũ không
   đổi.
6. **Dispatch.** Thêm `find_nodes` vào nhánh read ở `mcp.rs:111`.
7. **Hướng dẫn agent.** `instructions::CONNECT` (`crates/server/src/instructions.rs:6`, `:13`)
   và hai skill: khi đã biết tên, chữ hoặc loại thì dùng `find_nodes`; dùng `get_outline` khi cần
   cấu trúc. Nói rõ: không có kết quả là `totalMatches:0`, không phải lỗi; tìm theo vị trí ("bên
   phải") chưa hỗ trợ, khi đó lấy bounds bằng `get_design_context viewports` hoặc
   `inspect_layout nodeIds`.
8. **Docs.** Thêm mục `find_nodes` vào `docs/SERVER.md`: request, response, ngữ nghĩa ba trường
   hợp, giới hạn so khớp, cursor bị vô hiệu khi có mutation đồng thời (cùng luật
   `docs/SERVER.md:145`).

## Nghiệm thu

### Acceptance Criteria

- `textContains:"Tổng quan"` trên frame stacks mẫu trả đúng node text đó, kèm `path` tới frame,
  trong một lần gọi. Ghi bytes thật so với outline đầy đủ của cùng scope.
- `kinds:["frame"]` liệt kê mọi frame của project đa page dù frame có con, không bị depth cắt.
- Ba trường hợp tách bạch, mỗi trường hợp có test riêng: 0 kết quả; vừa đủ một trang; nhiều
  trang với `limit:1` cho đủ các match, không trùng, không thiếu, đúng thứ tự preorder.
- Thiếu tiêu chí, `nodeId`+`frameId` cùng lúc, scope không tồn tại, kind sai enum, cursor của
  query khác hoặc revision khác: mỗi trường hợp trả lỗi client-safe đúng mã hiện có
  (`INVALID_REQUEST`, `INVALID_CURSOR`, `REVISION_UNAVAILABLE`).
- `properties:["text","fill"]` kèm đúng hai key đó cho mỗi match, không kèm key khác.
- Output `get_outline`/`get_subtree`/`get_design_context` không đổi byte nào sau khi thay
  traversal. Test so sánh trên fixture có sibling trùng `sortKey` và cha khai báo sau con trong
  thứ tự storage.
- Hiệu năng: fixture 5.000 node (cùng cỡ "large" ở `spikes/storage/README.md`) đo `get_outline`
  trước/sau và `find_nodes` sau. Báo số thật. Xử lý server-side của read phải nằm trong budget
  large snapshot p95 500 ms đã công bố; không tự đặt ngưỡng khác.
- Ghi delta byte của `tools/list` do mô tả tool mới.
- Mọi file mã viết tay ≤250 dòng; `tree.rs` không được dài hơn hiện tại (152 dòng).

### Cách kiểm

```bash
cargo test -p opencanvas-server --offline
cargo clippy -p opencanvas-server --offline -- -D warnings
cargo fmt --all --check
git diff --check
```

Kiểm qua MCP thật trên data root tạm bằng frame stacks (`node samples/welcome-stacks/verify.mjs`
dựng project; script tự dọn root và process). Không dùng cổng 3847, không ghi `~/.opencanvas`.

### Đầu ra

- Delta mã + `crates/server/tests/find_nodes.rs`.
- `docs/SERVER.md`, instructions và hai skill đã cập nhật.
- Báo cáo `docs/TASK06-REPORT.md`: hình dạng request/response cuối cùng, bytes outline vs find
  trên mẫu, latency 5.000 node trước/sau, delta `tools/list`, lệnh đã chạy kèm kết quả.

## Ràng buộc

- Không lọc node trong `get_outline`/`get_subtree`/`get_design_context`; mọi schema cũ giữ nguyên.
- Không tìm theo bounds/vị trí. Việc đó cần scene đã tính và sẽ kéo read đồng bộ vào job cache,
  đúng đánh đổi mà Task 03 đã tránh. Nếu cần, đề xuất thành task riêng sau khi có số liệu sử dụng.
- Không thêm dependency, không full-text index, không thay storage. Quét tuyến tính trên snapshot
  là đủ ở quy mô đã công bố (5.000 node).
- Không gộp dấu tiếng Việt trong task này; ghi rõ là giới hạn đã biết.
- Không sửa `PLAN.md`, `AGENTS.md`; không chạm cổng 3847; không commit/push.
