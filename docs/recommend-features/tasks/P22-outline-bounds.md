# P22 — `get_outline bounds:true`: outline gọn kèm `[x,y,w,h]` để tìm theo vị trí
Ưu tiên: P2 · Kích cỡ: ~60′ dev · Phụ thuộc: không (chạm `schemas.rs` — xếp sau P19, xem "Tương thích MCP") · Nguồn: P20 khoảng trống #3 (so Figma `get_metadata`)

## Vấn đề
Agent nhận yêu cầu kiểu "sửa cái card bên phải" phải biết vị trí node. `get_outline` chỉ trả `id/parentId/kind/name/sortKey`, không có hình học. Muốn biết vị trí, agent phải gọi `get_design_context` kèm `viewports`, nhận toàn bộ props, resolvedProps, tokens và styles, hoặc gọi `inspect_layout` sau khi đã có job render. Tức là tốn hai bước, trong khi bước đọc nặng chỉ để lấy 4 số. Figma `get_metadata` trả outline XML gọn có sẵn `x/y/width/height` cho mỗi layer.

## Bằng chứng
- Entry outline được dựng ở `crates/server/src/reads/tree.rs:55-56` và không có trường hình học. Mô tả công bố ở `crates/server/src/schemas.rs:79` ghi rõ "It carries no props."
- Hướng dẫn cho agent thừa nhận khoảng trống này. `crates/server/src/instructions.rs:7-10` và mô tả `find_nodes` ở `crates/server/src/schemas.rs:107` đều ghi: "Position ('the card on the right') is not searchable: read bounds with get_design_context viewports or inspect_layout nodeIds."
- Cơ chế tính bounds đã có sẵn và dùng lại được. `crates/server/src/reads/viewports.rs:26-110` (`compute`) xếp job scene qua `state.jobs.for_context` (`crates/server/src/jobs.rs:85`, cache theo `sceneKey`, không bao giờ chờ Chromium trong lượt đọc, xem `viewports.rs:25`). Nó trả `bounds[id]` frame-local (`viewports.rs:107`), `status` là pending/ready/failed kèm `jobId`, và kiểm tra cảnh thiếu bounds (`INCOMPLETE_SCENE`, `viewports.rs:78-82`). Hiện chỉ `get_design_context` gọi tới (`crates/server/src/reads.rs:58-63`).
- `render_frame` bắt buộc có `viewport` (`crates/server/src/mcp.rs:157-159`), nên không có "kích thước mặc định" nào để dùng lại. P22 phải tự định nghĩa: lấy kích thước authored của frame.
- Số đo thật (TASK07 S1, `docs/TASK07-REPORT.md:41`): outline 182 node là 40 279 B response (2 trang). Tỉ lệ response/structured của S1 là 85 986/38 782 ≈ 2,2, suy ra structured ≈ 18,2 KB, khoảng 100 B/node, response khoảng 221 B/node.
- Đo từng mảnh (`printf … | wc -c`, ví dụ thật cỡ trung bình):
  - Entry outline `{"id":"card-pro","kind":"frame","name":"Card Pro","parentId":"pricing-grid","sortKey":2}` = **88 B**.
  - Thêm `,"bounds":[792,160,360,480]` = **+27 B** (+31 %).
  - Cách làm hiện nay qua `get_design_context` (`properties:[]`, 1 viewport) vẫn trả `"props":{}`, `"resolvedProps":{}` và `"computedBounds":[{"x":792.0,"y":160.0,"width":360.0,"height":480.0}]`, tức ≥ **99 B** chỉ riêng 3 trường này cho mỗi node. Chưa tính các trường node authored khác, `tokens/styles/fonts/assets`, `diagnostics`.
  - Ước tính cho frame 182 node: P22 tốn khoảng 182×115 ≈ 21 KB structured. Cách hiện nay tốn ≥ 182×(88+99) ≈ 34 KB cộng phần tài nguyên. **Chưa đo live**: AC1 bắt buộc đo trên server tạm.
- Figma MCP `get_metadata` trả outline XML thưa gồm id, tên, loại, vị trí và kích thước của từng layer. Nguồn là bảng P20 #3, tài liệu tại https://developers.figma.com/docs/figma-mcp-server/tools-and-prompts/ (lấy theo trích dẫn trong P20, phiên này chưa mở lại trang). REST đặt tên trường là `absoluteBoundingBox {x,y,width,height}`.

## Giải pháp
Thêm một tham số optional cho `get_outline`: `bounds: boolean`. Không thêm tool.

```json
// request
{"projectId":"p1","frameId":"pricing","bounds":true}
// ready: mỗi node trong scope có bounds = [x,y,w,h] số nguyên, frame-local
{"nodes":[{"id":"pricing","kind":"frame","name":"Pricing","parentId":"page-1","sortKey":0,"bounds":[0,0,1440,900]},
          {"id":"card-pro","kind":"frame","name":"Card Pro","parentId":"grid","sortKey":2,"bounds":[792,160,360,480]}],
 "computed":{"status":"ready","viewport":{"width":1440,"height":900},"jobId":"j_…","sceneKey":"…"}, …}
// pending: nodes không có bounds; chờ render_frame {jobId,waitMs} rồi gọi lại đúng request
{"nodes":[…],"computed":{"status":"pending","viewport":{"width":1440,"height":900},"jobId":"j_…","sceneKey":"…"}, …}
```

Quy tắc:
1. `bounds:true` bắt buộc có `frameId`, giống rule của `viewports` (`viewports.rs:34-49`). Thiếu frameId, hoặc dùng project/page scope, hoặc dùng `nodeId` thì trả lỗi `INVALID_REQUEST` với thông điệp "bounds requires frameId; use find_nodes kinds [frame] to pick one". Lý do: mỗi frame là một job render, quét cả project sẽ thành N job.
2. Viewport là `width/height` authored của frame, resolve theo theme mặc định, và phải là số cố định. Nếu là `fill`/`hug` hoặc nằm ngoài phạm vi `validate` (`viewports.rs:51`) thì trả `INVALID_REQUEST`: "bounds needs a fixed frame width/height; use get_design_context viewports".
3. Số được làm tròn bằng `f64::round` thành số nguyên, trong không gian frame-local như `computedBounds`. Đây là mảng 4 số, không phải object, để tiết kiệm token. Thứ tự giống CSS/Figma: x, y, width, height.
4. `false` được chuẩn hoá thành vắng mặt, giống `compact` (`reads.rs:51`), để hash scope và cursor của lượt đọc mặc định không đổi. `bounds:true` nằm trong hash scope (`pagination::scope`), nên cursor của lượt có bounds không dùng lẫn với lượt không có bounds.
5. Khi `depth` cắt bớt cây, node bị cắt không xuất hiện nên không có bounds, còn `omittedChildren` giữ nguyên. Khi `failed`, `computed.error` mang lỗi của job và không node nào có bounds.
6. Lượt đọc không có `bounds` giữ nguyên từng byte: golden `traversal_golden.jsonl` không đổi.

Phương án đã loại:
- **A. Chỉ lấy `x/y/width/height` authored, không chạy job.** Rẻ và đồng bộ, nhưng con của stack row/column (bố cục mặc định theo instructions) không có x/y, còn `fill`/`hug` không phải số. Hầu hết node sẽ thiếu hoặc sai, nên không giải được bài toán "card bên phải".
- **B. Luôn kèm bounds trong outline.** Làm mọi lượt outline tăng khoảng 31 % và kích một job render cho mỗi lượt đọc. Loại.
- **C. Tool mới `get_metadata`.** Tốn khoảng 400–600 B `tools/list` cố định, trùng chức năng outline. Vi phạm ngân sách `tools/list`.
- **D. Giữ nguyên, chỉ hướng dẫn dùng `get_design_context properties:[] viewports:[…]`.** Cần 0 B code nhưng mỗi node đắt ≥ 2× (xem Bằng chứng) và kéo theo tài nguyên. Phương án được chọn chỉ thêm khoảng 230 B `tools/list`.

## Phạm vi file
- Tạo `crates/server/src/reads/outline_bounds.rs` (~80–110 dòng):
  - `frame_viewport(snapshot, frameId) -> Result<Viewport>` lấy kích thước authored của frame.
  - `compute(state, snapshot, req) -> Result<Option<Computed>>` gọi lại `viewports::compute` với `viewports=[size]`, hoặc gọi một hàm tách `viewports::compute_one`.
  - `round(&Value) -> Value` chuyển bounds sang `[x,y,w,h]`.
  - `metadata(...)` dựng object `computed` gọn.
  - Unit test cho round và viewport.
- Sửa `crates/server/src/reads.rs` (97 dòng, +~8): thêm trường `bounds: Option<bool>` vào `ReadRequest`, chuẩn hoá false, rẽ nhánh outline ở dòng 58-63.
- Sửa `crates/server/src/reads/tree.rs` (143 dòng, +~6): ở dòng 55-56 thêm `bounds` vào entry outline; gắn `response["computed"]` cho outline.
- Sửa `crates/server/src/reads/viewports.rs` (159 dòng, ±10): nếu cần, mở `pub(super)` cho `validate` hoặc tách vòng lặp một viewport. Không đổi hành vi.
- Sửa `crates/server/src/schemas.rs` (163 dòng, +~4): thêm `bounds` riêng cho `get_outline` (schema `tree` ở dòng 71 đang dùng chung) và một câu mô tả. Sửa câu "Position…" ở dòng 107.
- Sửa `crates/server/src/instructions.rs` (±1 câu), `docs/SERVER.md` (mục get_outline), `skill://opencanvas/implement-frame.md` (nguồn trong `resources.rs`, nếu có nhắc vị trí).
- Tạo test `crates/server/tests/outline_bounds.rs` (~120–160 dòng).
- Không file nào vượt 250 dòng.

## Tương thích MCP
- Không thêm tool. `get_outline` có thêm một key optional `"bounds":{"type":"boolean"}` (28 B) cộng mô tả 202 B: "bounds:true (needs frameId) adds frame-local integer [x,y,w,h] per node from one render at the frame's authored size; while computed.status is pending, wait with render_frame {jobId,waitMs} and repeat." Tổng khoảng **+230 B** `tools/list`. Câu "Position … not searchable" trong `find_nodes` và trong instructions được viết lại cho ngắn hơn, để bù khoảng −40 B.
- Field cũ giữ nguyên nghĩa. Response không có `bounds` giữ nguyên từng byte.
- **P19:** P19 bỏ `viewport/theme` (không dùng) khỏi schema công bố của outline/subtree. P22 cố ý **không** dùng `viewport`/`theme` trên outline: viewport lấy từ frame, theme cố định là `default`, nên hai đề xuất không xung đột. Nếu P19 merge trước, dev P22 thêm `bounds` theo dạng `lean` của P19. Nếu P22 merge trước, P19 chỉ việc giữ key `bounds`. Test budget `tools/list` của P19 cần cộng 230 B.
- B01–B05: không chạm.

## AC
1. Test `outline_bounds_bytes` dùng frame fixture 20 node (stack row có 3 card). Đo `serde_json::to_vec(structured).len()` của `get_outline {frameId, bounds:true}` sau khi ready, so với `get_design_context {frameId, properties:[], viewports:[size]}`. Outline+bounds phải ≤ 60 % context. In ra cả hai con số.
2. Với frame có `width/height` cố định, lượt gọi đầu trả `computed.status` là `pending` hoặc `ready`, kèm `jobId` và `sceneKey`. Sau `render_frame {jobId, waitMs:10000}`, gọi lại cùng request thì được `ready`, và mỗi node trong scope có `bounds` gồm đúng 4 số nguyên.
3. `bounds` khớp `get_design_context viewports:[cùng size]` `computedBounds[0]` sau khi làm tròn, với mọi node (assert từng node).
4. Trong fixture stack row có 3 card, `bounds[0]` (x) tăng dần theo `sortKey`, nghĩa là xác định được "card bên phải" từ một lượt đọc.
5. `bounds:true` không có `frameId`, hoặc kèm `nodeId`, hoặc ở project scope, trả `INVALID_REQUEST` có chuỗi "bounds requires frameId". Frame có `width:"fill"`/`hug` trả `INVALID_REQUEST` có chuỗi "fixed frame width/height".
6. `bounds:false` và không truyền `bounds` cho response giống hệt nhau từng byte. `outline_subtree_and_context_bytes_match_golden` vẫn pass.
7. Cursor của lượt `bounds:true` dùng cho lượt không có bounds bị từ chối (scope mismatch), và ngược lại.
8. `tools/list` tăng ≤ 260 B so với commit nền, đo bằng `serde_json::to_vec(tools).len()` trong test budget hiện có. Số tool không đổi.
9. Gọi lại cùng revision không tạo job mới: `jobId` hoặc `sceneKey` trùng lần trước (dùng cache `for_context`).

## Cách kiểm
- `CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server --test outline_bounds --test tool_schemas --test outline_depth`
- `cargo test -p opencanvas-server reads::` (golden traversal và pagination)
- Các test cần Chromium (AC2–4, 9) theo cách của test viewports hiện có. Nếu môi trường CI không có Chromium thì đánh dấu giống các test `get_design_context viewports` đang làm.
- Kiểm live trên server tạm: `opencanvas-server --port 0 --data-root $(mktemp -d)`, tạo frame pricing 3 card qua MCP, gọi `get_outline {frameId,bounds:true}`, chờ job, gọi lại. Ghi byte response (JSON-RPC) so với `get_design_context` cùng scope vào report. Dọn process và data sau khi kiểm.

## Rủi ro
- Lượt đọc outline lần đầu có thể `pending`, nên agent cần thêm một vòng `render_frame waitMs`. Đã giảm nhờ cache `sceneKey`: frame chưa đổi revision thì ready ngay.
- Kích thước authored của frame có thể khác viewport mà người dùng định nhắm tới (frame responsive). `bounds` chỉ là số đo ở một kích thước. Mô tả phải nói rõ: "use get_design_context viewports to compare sizes."
- Frame rất lớn: trang outline tăng khoảng 31 % theo byte nên có thể tách thêm trang trong budget 64 KiB. Chấp nhận được vì cursor đã có sẵn.
- `schemas.rs` là file nóng (P19, P11b, P12a, P13a, P15): merge tuần tự.

## Ngoài phạm vi / phase sau
- `bounds` ở project/page scope (nhiều frame, nhiều job) và `bounds` theo viewport hoặc theme tuỳ chọn. Chỉ làm khi có nhu cầu thật, và cần tính lại xung đột với P19.
- Tìm theo vị trí trong `find_nodes` (ví dụ `sort:"x"` hoặc `within:[x,y,w,h]`).
- Định dạng XML hay "dòng gọn" kiểu Figma: JSON mảng 4 số đã đủ gọn.
