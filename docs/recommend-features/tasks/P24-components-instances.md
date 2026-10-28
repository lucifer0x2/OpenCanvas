# P24 — Component + instance tối thiểu (giảm token ghi, nền cho Code Connect)

Ưu tiên P1 · Kích cỡ ~165 phút → P24a storage/validate/expand (~90) + P24b reads/MCP (~75) · Phụ thuộc: không cứng; P20 (Code Connect, khoảng trống #2) phụ thuộc P24; nên làm sau P19 (tools-list diet) để tính byte chung.

## Vấn đề

- Agent phải ghi lại toàn bộ cây cho mỗi cụm lặp. QueryNest (18 frame, ~6.900 node, ghi chú `/Users/luis/Documents/opencanvas.md`) có hàng bảng, nút, keycap lặp hàng chục lần: mỗi lần là 1 frame + N text với props giống hệt, chỉ khác `text`.
- Sửa một chi tiết chung (padding hàng, màu viền) = sửa N node: nhiều op, dễ lệch.
- Coder không có cách map "1 cụm thiết kế ↔ 1 component code": `get_design_context` trả 20 cây hàng như 20 cây độc lập. P20 ghi rõ Code Connect cần component/instance trước (`P20-token-code-syntax.md:114`, `PLAN.md:399` "component/variant chỉ khi cần thật").

## Bằng chứng

- `crates/storage/src/model.rs:13-20`: `NodeKind` chỉ có `page, frame, group, rectangle, text, image`; `Node` (`model.rs:39-45`) = `id, parent_id, sort_key, kind, props` — không có khái niệm tham chiếu node khác.
- `crates/storage/src/model/requests.rs:5-31`: op ghi là `CreateNode/UpdateNode/ReparentNode/ReorderNode/DeleteSubtree`; không có op nào nhân bản hay tham chiếu.
- `crates/server/src/schemas.rs:104`: enum `kinds` của `find_nodes` cứng 6 kind — thêm kind phải sửa ở đây.
- `crates/layout/src/tree.rs:7` `build(...)`: layout dựng cây từ node đã authored; nếu mở rộng instance **trước** khi gọi `build`, engine không phải đổi.
- Đo thật (script tái tạo ở cuối file), 20 hàng bảng, mỗi hàng = frame row + 4 text cell (id, query, ms, status), props theo dạng `props.md` (layout row/gap/padding, fill/stroke token, styleId):

| Cách ghi | Số op | Byte JSON ops |
| --- | --- | --- |
| Hiện tại: 100 `createNode` | 100 | **22.191** |
| P24: 1 component (5 op, 1.108 B) + 20 instance (~246 B/instance) | 25 | **6.018** (−72,9%) |

Nhân ra QueryNest: nếu ~40% của 6.900 node nằm trong cụm lặp có hình dạng như trên, phần đó giảm ~70% byte ghi; sửa padding chung từ N op còn 1 op.

## Giải pháp

Không thêm tool. Mở rộng `apply_batch` (kind mới), `get_design_context`, `find_nodes`, `get_subtree`.

1. **Component = frame có prop `component`** (không thêm kind cho master, layout/render giữ nguyên):
   ```json
   {"op":"createNode","node":{"id":"cmpRow","parentId":"pgLib","kind":"frame",
     "props":{"name":"TableRow","component":{"name":"TableRow"},"layout":{"mode":"row","gap":12},"width":"fill","height":"hug"}}}
   ```
   Con của master authored bình thường (`cmpRow-q`, …). P20 sau này thêm `component.code: {source,name,props}` vào đúng chỗ này.
2. **Instance = kind mới `instance`, lá trong cây authored** (không có con authored):
   ```json
   {"op":"createNode","node":{"id":"row3","parentId":"tbl","kind":"instance",
     "props":{"componentId":"cmpRow","width":"fill",
              "overrides":{"cmpRow-q":{"text":"select * from t3"},"cmpRow-st":{"fill":{"token":"color.danger"}}}}}}
   ```
   - Props riêng của instance: `componentId`, `name`, sizing/position như frame (`x,y,width,height,min/max`), `visible`.
   - `overrides` **phẳng**: khóa = id node con trong master; giá trị = props một phần, chỉ whitelist: `text, fill, visible, styleId, asset` (image) và `name`. Không override layout/cấu trúc ở bản tối thiểu (tránh kết hợp phức tạp; variant để sau).
3. **Mở rộng (expand) ở một chỗ duy nhất** — module mới `crates/storage/src/instances.rs`: `expand(nodes) -> Vec<Node>` sinh node ảo id `<instanceId>/<masterChildId>` (ví dụ `row3/cmpRow-q`), merge `overrides` lên props master, root instance mang layout của master + sizing của instance. Layout (`layout/src/tree.rs:7`), renderer, scene, export HTML gọi trên cây đã expand → không sửa engine. Node ảo **chỉ đọc**: ghi vào `row3/cmpRow-q` trả `INSTANCE_CHILD_READONLY` kèm gợi ý `updateNode row3 props.overrides`.
4. **Validation** (`storage/src/validate/` thêm `instances.rs`): `componentId` phải là frame có `component`; cấm vòng (master chứa instance của chính nó, trực tiếp hoặc gián tiếp, giới hạn độ sâu 8); khóa `overrides` phải là hậu duệ của master; key ngoài whitelist → lỗi `props.overrides.<id>.<key>` + `allowedKeys` như lỗi props hiện tại; `deleteSubtree` master còn instance → `COMPONENT_IN_USE {instanceIds (≤20), count}`.
5. **`get_design_context` trả một lần định nghĩa + tham chiếu**:
   ```json
   {"components":{"cmpRow":{"name":"TableRow","rootId":"cmpRow","nodes":[/* cây master 1 lần, như node thường */]}},
    "nodes":[{"id":"row3","kind":"instance","componentId":"cmpRow",
              "overrides":{"cmpRow-q":{"text":"select * from t3"}},
              "computed":{"bounds":{...},"children":{"row3/cmpRow-q":{"bounds":{...},"lines":[...]}}}}]}
   ```
   - Mặc định `instances:"reference"`; tham số mới `instances:"expand"` trả cây ảo đầy đủ (cho agent cần bounds từng cell như cũ).
   - `components` chỉ chứa master mà frame tham chiếu (giống `resources referenced` của `compact:true`).
   - `computed` vẫn có bounds của node ảo (dùng để kiểm cài đặt), key theo id ảo.
6. **`find_nodes`**: thêm `"instance"` vào enum `kinds` (`schemas.rs:104`); `properties:["componentId"]` hoạt động sẵn; `textContains` khớp cả `overrides.*.text` và trả id ảo. `get_outline/get_subtree`: instance hiển thị như lá với `componentId`, `childCount` = số node master.
7. **Byte `tools/list`**: +`"instance"` trong 1 enum (~11 B), tham số `instances` của `get_design_context` (~90 B gồm mô tả), 1 câu trong instructions server (~160 B). Tổng < 300 B, không có tool mới. Thêm mục `instance` + `component` vào `skill://opencanvas/props.md`.

**Migration**: `kind` đã lưu dạng text, `props` là JSON → không đổi cột SQL. Tăng schema version (bản deploy hiện ở schema 2 → 3) để server cũ từ chối bundle có `instance` thay vì đọc sai; migration chỉ ghi version, dữ liệu cũ hợp lệ nguyên trạng. Bundle export/import (`storage/src/bundle*.rs`) giữ node như authored (không expand). Không tự động gom cụm có sẵn thành component (để task sau: "extract component" bằng gợi ý từ fingerprint).

## Phạm vi file

- P24a: `crates/storage/src/model.rs` (thêm `Instance`, `as_str`), `crates/storage/src/instances.rs` (mới, expand ≤200 dòng), `crates/storage/src/validate/instances.rs` (mới), `validate/prop_keys.rs` (key `component`, `componentId`, `overrides`), `migration.rs` (schema 3), `props_catalog.rs`, tests `crates/storage/tests/instances.rs` (mới).
- P24b: `crates/server/src/reads/` (design context, `find.rs`, `tree.rs`, mới `reads/instances.rs` cho khối `components`), `crates/server/src/schemas.rs:104` + schema `get_design_context`, `instructions.rs`, `props_doc/` (mục instance), chỗ gọi layout/render/export chuyển sang cây đã expand, tests `crates/server/tests/instances_mcp.rs` (mới), `docs/SERVER.md`, `docs/CONTRACT.md`.
- Không đụng `crates/layout/src/engine.rs`, `stack.rs`.

## AC

1. `apply_batch` tạo master (frame + `component`) và 20 `instance` cho ví dụ hàng bảng; tổng byte ops ≤ 30% bản 100 `createNode` tương đương (đo bằng test in byte, kỳ vọng ~27%).
2. `render_frame` của frame chứa 20 instance cho PNG giống pixel-identical với frame dựng tay 100 node cùng props (so sánh hash hoặc diff = 0).
3. `updateNode cmpRow props.layout.padding` một op → `inspect_layout` cho thấy cả 20 hàng đổi bounds ở revision mới.
4. Override `text`/`fill` chỉ ảnh hưởng instance đó; key ngoài whitelist trả lỗi có `field: props.overrides.<id>.<key>` và `allowedKeys`.
5. Vòng component (A chứa instance của B, B chứa instance của A) bị từ chối với lỗi có cấu trúc; `deleteSubtree` master còn instance trả `COMPONENT_IN_USE` với `count`.
6. `get_design_context` mặc định trả master đúng một lần trong `components`, 20 node `instance` không lặp cây con; byte phản hồi giảm ≥ 50% so với `instances:"expand"` cho frame ví dụ. `instances:"expand"` trả cây ảo với id `row3/cmpRow-q` và bounds khớp AC2.
7. `find_nodes {kinds:["instance"]}` trả 20; `find_nodes {textContains:"t3"}` trả `row3/cmpRow-q`.
8. Project/bundle cũ (schema 2) mở được và qua migration không đổi dữ liệu; bundle có instance export→import round-trip giữ `componentId/overrides`.
9. `tools/list` tăng < 300 B (đo trước/sau); không có tool mới. File mới ≤ 250 dòng.

## Cách kiểm

- `cargo test -p opencanvas-storage instances` và `cargo test -p opencanvas-server instances_mcp` (tên crate theo `Cargo.toml`), `cargo clippy` hai crate.
- Live: server tạm `--port 0` + data root tạm (không chạm cổng 3847, không `~/.opencanvas`), chạy kịch bản AC1–7 qua MCP, đo byte `tools/list` trước/sau, dọn process/data.
- Tái tạo số đo (Node, không phụ thuộc repo):
  ```js
  // 20 hàng × (frame row + 4 text): before = 100 createNode; after = master 5 op + 20 instance
  // kết quả đo 02:45 29/10: before 22191 B, after 6018 B, master 1108 B, ~246 B/instance, −72,9%
  ```

## Rủi ro

- **Id ảo** `<instanceId>/<childId>` phải ổn định cho scene key (`crates/server/src/inspect.rs`), `inspect_layout`, links/notes (P02*, P06*): id authored chỉ cho `[A-Za-z0-9_-]` (`crates/storage/src/validate/values.rs:9`) nên `/` không thể va chạm; node ảo phải đi qua mọi chỗ đang gọi validate id mà không bị từ chối.
- **Invalidation**: sửa master đổi kết quả mọi frame chứa instance — cache render/scene key theo frame phải tính cả revision/hash của master; thiếu thì render cũ (stale). Kiểm trong AC3 bằng `render_frame` sau khi sửa master.
- **Override cấu trúc** (ẩn/đổi thứ tự con, variant) bị cắt khỏi bản tối thiểu; agent có thể cần `visible:false` override làm tạm — đã có trong whitelist.
- **Animation/links** (P06b, P07*) tham chiếu node master: quy ước áp cho mọi instance hay cấm — chọn "cấm ở bản tối thiểu", lỗi có cấu trúc.
- Chi phí expand trên frame ~6.900 node: expand O(n), nhưng phải đo `get_design_context` p95 trước/sau (không vượt +10%).
