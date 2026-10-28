# P27 — Trạng thái tương tác: `props.states` (hover/pressed/focus/disabled) ghi đè phẳng + `get_design_context`/`render_frame state`
Ưu tiên: P1 · Kích cỡ: ~150 phút dev → tách **P27a** (prop + validate + resolver + design context, ~80') và **P27b** (`render_frame state` + viewer/scene, ~70') · Phụ thuộc: không cứng; liên hệ **P07a** (animation `on: hover` nên tham chiếu `states.hover` làm đích), **P24** (instance override `states`), **P10b** (text style props trong state). File nóng: `props_catalog.rs`, `reads/tree.rs`, `jobs/input.rs`.

## Vấn đề
Design chỉ mô tả trạng thái `default`. AI coder không biết nút hover/pressed/disabled trông thế nào nên tự bịa hoặc hỏi lại; agent author không có chỗ ghi biến thể nên phải nhân bản frame (vd. `Button/Hover`) không liên kết với node gốc. Instructions MCP nói thẳng chỉ hỗ trợ `default`.

## Bằng chứng (nhánh `night/integration`)
- `crates/server/src/instructions.rs:44` — "Only interaction state 'default' is supported".
- `crates/server/src/schemas/render.rs:17` — `"state":{"enum":["default"]}`; `crates/server/src/jobs/input.rs:81` từ chối mọi state khác; `jobs/input.rs:42` đã đưa `state` vào input job (khoá scene key sẵn có).
- `docs/CONTRACT.md:64`, `docs/SERVER.md:439`, `docs/ACCEPTANCE.md` (giới hạn "default interaction state").
- `crates/storage/src/props_catalog.rs:120` — `links` đã có `on: hover` nhưng không có hình thái hover; `props_catalog.rs:111` `fill` là mẫu prop có thể ghi đè.
- `crates/server/src/reads/tree.rs:75` — `resolvedProps` tính một lần theo theme; chưa có chiều state.

## Giải pháp (tối thiểu; mở rộng tool sẵn có, không thêm tool)
1. **Prop mới `states`** (mọi kind trừ page) — ghi đè **phẳng** lên props default, không lồng, không kế thừa giữa các state:
```json
{"op":"update_node","id":"btn","props":{"states":{
  "hover":{"fill":{"tokenId":"color.primary.600"}},
  "pressed":{"fill":"#1E40AF","opacity":0.9},
  "focus":{"strokeColor":"#93C5FD","strokeWidth":2},
  "disabled":{"opacity":0.4}}}}
```
   Tên state cố định: `hover|pressed|focus|disabled`. Khoá trong state ⊆ allowlist "visual" (fill, opacity, stroke*, radius, text color/weight, shadow khi P09b có); cấm `layout`, `width/height`, `x/y`, `links`, `states`, `animations` → lỗi `field props.states.hover.<key>` + `allowedKeys` như cơ chế hiện có. `{}`/vắng = không có biến thể. Token/style ref hợp lệ như prop gốc (validate/refs.rs).
2. **Resolver**: `resolve(node, theme, state)` = `props` ⊕ `props.states[state]` (state con của cây: node nhận state khi chính nó hoặc tổ tiên gần nhất có khai báo state đó được "kích hoạt" — bản tối thiểu: chỉ node có `states[state]` đổi, con giữ default).
3. **`get_design_context`**: thêm `statesMode: "none"|"resolved"` (mặc định `"none"`, giữ payload cũ). `"resolved"` → mỗi node có `states` trả `resolvedStates:{hover:{fill:"#2563EB"}}` (chỉ khoá khác default); authored `props.states` luôn trả nguyên. `compact:true` giữ quy tắc "chỉ khoá khác". Coder map sang `:hover/:active/:focus-visible/[disabled]`.
4. **`render_frame state`**: enum `default|hover|pressed|focus|disabled`, áp state lên **mọi** node có khai báo state đó trong frame (render "toàn frame ở state X"); scene key đã gồm state (`jobs/input.rs:42`) nên cache tách đúng. Tuỳ chọn sau: `stateTargets:[nodeId]` giới hạn node.
5. Instructions/docs: thay câu `instructions.rs:44` bằng mô tả `states` + "map sang pseudo-class, đừng bịa state không khai báo"; P07a: animation `on: hover` không có `to` ⇒ đích = `states.hover`.

## Phạm vi file
- P27a: `crates/storage/src/props_catalog.rs`, `crates/storage/src/validate/` (file mới `states.rs`), resolver storage (theme resolve), `crates/server/src/reads/tree.rs`, `reads/compact.rs`, `crates/server/src/schemas.rs` (design context arg), `instructions.rs`, `docs/SERVER.md`, `docs/CONTRACT.md`, `docs/RESOLUTION.md`, skill `props.md`/`implement-frame.md`.
- P27b: `crates/server/src/schemas/render.rs`, `crates/server/src/jobs/input.rs`, renderer dùng chung (áp override trước layout), `web/` (bộ chọn state trong viewer, tuỳ chọn), `docs/ACCEPTANCE.md` (gỡ giới hạn).

## AC
1. `update_node` với `states.hover.fill` token hợp lệ → revision tăng; `get_subtree` trả nguyên `props.states`.
2. Khoá cấm (`states.hover.width`, `states.foo`) → lỗi field `props.states.hover.width` / `props.states.foo` kèm `allowedKeys`; không ghi revision.
3. `get_design_context statesMode:"resolved"` theme `night` → `resolvedStates.hover.fill` là hex của token ở theme night; node không có `states` không có trường `resolvedStates`; mặc định (không truyền) payload byte-identical so với trước P27 trên fixture có sẵn.
4. `render_frame state:"hover"` → PNG khác `default` đúng ở bbox node có `states.hover` (diff pixel ngoài bbox = 0); state `disabled` trên frame không node nào khai báo → ảnh giống `default`, kèm diagnostic `STATE_NOT_DECLARED`.
5. Hai job cùng revision/viewport khác state có scene key khác nhau; cùng state → tái dùng job.
6. State chỉ đổi visual → `inspect_layout` bounds giống hệt `default` (không layout shift).
7. `instructions.rs` không còn câu "Only interaction state 'default'"; tools/list tăng ≤ 400 byte (P19).
8. Mọi file sửa ≤ 350 dòng.

## Cách kiểm
- `cargo test -p opencanvas-storage states` (validate/resolver unit), `cargo test -p opencanvas-server --test tool_schemas` (enum state, arg mới), test tích hợp design context + render trên data root tạm, server `--port 0`.
- So ảnh: render default vs hover, đếm pixel khác ngoài bbox node = 0.
- `wc -l` các file sửa.

## Rủi ro
- Ghi đè phẳng không biểu diễn được "hover cha đổi màu con" — chấp nhận ở bản tối thiểu; ghi rõ trong docs, mở rộng sau bằng `stateTargets`/selector con.
- Xung đột với P24 instance override và P07a (đích animation) nếu hai task định nghĩa khác nhau → chốt hợp đồng `states` trước, P07a/P24 tham chiếu.
- Render "toàn frame một state" không giống thực tế (chỉ một phần tử hover) — chỉ dùng để xem biến thể; ghi rõ trong instructions.
- Tăng kích thước tools/list và payload design context — giữ mặc định `statesMode:"none"`.
