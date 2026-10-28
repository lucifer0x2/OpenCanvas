# P28 — Responsive: kích thước `%` và breakpoint trên frame

Ưu tiên P2 · Kích cỡ ~150 phút → P28a `%` sizing (~70) + P28b breakpoints (~80) · Phụ thuộc: không (P28b sau P28a; nên sau P08b nếu đụng cùng `util.rs`)

## Vấn đề
Layout chỉ có fixed/fill/hug và row wrap (Task04). Không diễn đạt được "cột chiếm 50% bề rộng cha" hay "dưới 600px đổi row→column". Agent đọc `get_design_context viewports` chỉ thấy fill co giãn đều; responsive thật phải đoán.

## Bằng chứng
- `crates/layout/src/model/layout.rs:5-8`: `Sizing` = `Fixed(f32) | Rule(String)`; Rule chỉ có `"fill"`/`"hug"`.
- `crates/layout/src/util.rs:25-30`: `dim()` map Fixed→`length`, fill→`percent(1.0)`, còn lại→`auto`. Taffy đã hỗ trợ `percent(x)`, chỉ thiếu đường vào.
- `crates/layout/src/validation.rs:31` kiểm Fixed hữu hạn; `:123` chặn "Fill inside hug axis is ambiguous". `%` cần cùng quy tắc.
- `docs/TASK04-PROPOSAL.md:14,216` và `docs/ACCEPTANCE.md:19,180`: `%`/breakpoint bị loại khỏi Task04, "cần duyệt riêng".
- `crates/server/src/schemas.rs:95-100`: `get_design_context` đã nhận 1–4 viewports, có `reflow`, nên dùng làm thước đo AC.

## Giải pháp
**P28a — `%`:** `width`/`height`/`minWidth`/`maxWidth`/`minHeight`/`maxHeight` nhận chuỗi `"<n>%"` (0 < n ≤ 100), tính theo content box của cha (trừ padding), như CSS.
```json
{"op":"update_node","id":"col-a","props":{"width":"50%"}}
```
- `Sizing::Rule("50%")` giữ nguyên serde (không đổi schema lưu trữ); thêm `parse_percent()` trong `util.rs`, `dim()` trả `percent(n/100)`.
- Validation: `%` trên trục hug của cha → lỗi giống fill (`:123`); `%` trên root frame → theo viewport; n ngoài (0,100] → `Invalid dimension`.

**P28b — breakpoints (chỉ trên root frame):**
```json
{"op":"update_node","id":"frame","props":{"breakpoints":[{"maxWidth":600,"overrides":{"row-1":{"layout":{"mode":"column"}},"col-a":{"width":"fill"}}}]}}
```
- Áp breakpoint đầu tiên (theo thứ tự `maxWidth` tăng) có `viewport.width ≤ maxWidth`; overrides là merge nông props layout/size của node con theo ID, chỉ khoá: `layout`, `width`, `height`, `min*/max*`, `hidden`.
- Merge trước khi dựng tree Taffy (`crates/layout/src/tree.rs`), không ghi vào revision. `get_design_context` trả thêm `activeBreakpoint` mỗi viewport.
- Không thêm tool; mở rộng `update_node` props, `skill://opencanvas/props.md` và `get_design_context`.

## Phạm vi file
- P28a: `crates/layout/src/util.rs`, `validation.rs`, `validation/layout.rs`, test mới `crates/layout/src/percent_tests.rs`; props doc trong `crates/server/src/resources.rs`; export HTML ánh xạ `width:50%`.
- P28b: `crates/layout/src/model/layout.rs` (kiểu `Breakpoint`), `crates/layout/src/tree.rs` hoặc module mới `breakpoints.rs`, `crates/server/src/reads/viewports.rs`, `schemas.rs`, test `breakpoint_tests.rs`, E2E trong `spikes/e2e/`.

## AC
1. Frame row 1000×400, padding 20, con A `width:"50%"`: viewport 1000 → A.width = 480 (±0.5); viewport 600 → 280. Đo bằng `get_design_context viewports:[1000,600]` computedBounds.
2. `reflow` khác null và liệt kê A là changed giữa 2 viewport.
3. `width:"150%"`, `"0%"`, `"abc%"` → lỗi `Invalid dimension`, `field props.width`; `%` trong cha hug cùng trục → lỗi ambiguous.
4. Dữ liệu cũ (không `%`) cho bounds byte-identical trước/sau (chạy lại `wrap_tests`, `stack_tests`, `stretch_tests`).
5. P28b: breakpoint `maxWidth:600` đổi `row-1` sang column: viewport 1000 → con cùng y, x tăng dần; viewport 600 → cùng x, y tăng dần; `activeBreakpoint` = null / 0 tương ứng.
6. Revision và `get_subtree` không đổi sau khi render ở viewport có breakpoint (override không được ghi).
7. Mọi file sửa ≤ 250 dòng hoặc báo lý do.

## Cách kiểm
- `cargo test -p opencanvas-layout percent breakpoint` (tên crate theo `Cargo.toml`); `cargo test -p opencanvas-server tool_schemas`.
- Server tạm `--port 0` + data root tạm: dựng frame AC1/AC5 qua `apply_batch`, đọc `get_design_context` viewports [1000,600], so số; dọn process/data. Không chạm cổng 3847.

## Rủi ro
- `%` cạnh hug lồng nhau dễ ra vòng phụ thuộc; giữ chặn như fill thay vì đoán.
- Breakpoint làm bounds phụ thuộc viewport ⇒ khoá cache render/scene key phải gồm viewport (đã có) — kiểm `jobs/retain.rs` không tái dùng scene sai.
- Viewer `web/` vẽ theo computed bounds nên ổn; nhưng editor UI cho breakpoint nằm ngoài phạm vi.
- Override chỉ cho phép khoá layout/size; mở rộng sang màu/text cần task riêng (dễ thành "variants").
