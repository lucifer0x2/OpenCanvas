# P30 — Tự gom giá trị lặp thành token/style: `repeatedValues` trong `get_design_context` + chế độ `tokenize` của `update_matching`
Ưu tiên: P2 · Kích cỡ: ~120′ (P30a phân tích read-only ~60′; P30b thay hàng loạt ~60′) · Phụ thuộc: P30b cần P26 (`update_matching`); P23 (lọc token/style) có ích nhưng không bắt buộc. Khác P29 (lint báo lỗi): P30 đề xuất token và thực hiện thay thế.

## Vấn đề
Agent vẽ lại UI (vd QueryNest ~6.900 node) thường ghi hex trần, `fontSize`, `padding`/`gap` số trần. Kết quả: không có design system, đổi theme hay đổi một màu thương hiệu phải sửa hàng nghìn node. Hiện không có cách đọc "giá trị nào lặp bao nhiêu lần" mà không tải toàn bộ cây, và không có cách thay `"#1F2937"` bằng `{tokenId}` hàng loạt mà không sinh N op `update_node`.

## Bằng chứng
- `crates/server/src/reads/compact.rs:80`: chế độ compact chỉ gom các tham chiếu `tokenId`/`aliasId` đã có; giá trị trần không được đếm hay gom.
- `crates/server/src/reads/tree.rs:143`: đọc cây chỉ nhận diện `value["tokenId"]`; hex/số trần đi qua như literal, không có thống kê.
- `crates/server/src/schemas/operations.rs:19`: op sửa duy nhất là `update_node` theo từng `id`.
- `crates/storage/src/validate/candidate.rs:137`: validate `UpdateNode { id, props }` từng node, nên thay hàng loạt = N op (giới hạn 500 op/batch).

## Giải pháp (mở rộng tool sẵn có, không thêm tool)
1. **P30a — `get_design_context` thêm `analyze: "repeatedValues"`** (read-only, cùng revision): trả `repeatedValues: [{prop, value, count, sampleNodeIds≤5, nearestToken?}]` cho các khóa `fill`, `stroke`, `fontSize`, `lineHeight`, `fontId`, `layout.gap`, `layout.padding`, `radius`; chỉ giá trị trần, `count ≥ minCount` (mặc định 3), sắp theo count, tối đa 50 mục. `nearestToken` = token đã có cùng giá trị đã resolve (khớp chính xác; màu so sau chuẩn hóa hex).
2. **P30b — `apply_batch` op `update_matching` (P26) thêm `replaceValue: {prop, from, to}`**: thay mọi literal `from` của `prop` trong scope bằng `to` (`{tokenId}` hoặc `styleId`), một op; kèm `dryRun` của P26 trả `matched`. Agent tạo token bằng `create_token` trong cùng batch rồi thay.
3. Tài liệu: thêm bước "tokenize" vào `skill://opencanvas/author-frame.md`.

## Phạm vi file
- `crates/server/src/reads/` (module mới `repeated_values.rs` ≤200 dòng + test), schema `get_design_context` trong `crates/server/src/schemas/`.
- `crates/storage/src/` matcher của P26 + nhánh `replaceValue`; `crates/storage/src/validate/candidate.rs` (gọi validate có sẵn cho từng node bị thay).
- `crates/server/src/props_doc.rs`, `docs/SERVER.md`.

## AC
1. Fixture 300 text, 280 dùng `fill:"#1F2937"`: `analyze` trả mục `{prop:"fill", value:"#1F2937", count:280}` đứng đầu; giá trị đã là `tokenId` không xuất hiện.
2. Mục có token cùng giá trị kèm `nearestToken`; không có thì vắng khóa.
3. Một batch `create_token` + `update_matching replaceValue` đổi đúng 280 node trong 1 revision; `get_design_context` sau đó trả `count` 0 cho giá trị đó; render pixel không đổi (cùng theme).
4. `dryRun` trả `matched:280`, không tăng revision.
5. `to` trỏ token không tồn tại/kiểu sai → lỗi validate, không đổi node nào.
6. Phân tích trên ~7.000 node trả trong <300 ms (đo bằng test release) và output ≤50 mục.

## Cách kiểm
`cargo test -p opencanvas-server repeated_values` và test storage cho `replaceValue`; test tích hợp MCP gọi `get_design_context {analyze}` rồi `apply_batch`, so `render_frame` trước/sau trên data root tạm.

## Rủi ro
- Gom theo giá trị chứ không theo ý nghĩa: hai màu giống nhau có thể khác vai trò (chữ vs viền) → giữ `prop` trong khóa gom, agent quyết định tên token.
- Màu gần giống (`#1F2937` vs `#1F2938`) không gom; gom gần đúng để sau.
- Phụ thuộc P26: nếu P26 chưa merge, P30a vẫn giao được độc lập.
