# P29 — Lint thiết kế: diagnostics trong render_frame / inspect_layout

Ưu tiên P2 · Kích cỡ 150 phút → tách P29a (90: engine lint + tương phản + cỡ chữ + ngoài frame) / P29b (60: vùng chạm + tràn/clip text + lọc `lint`) · Phụ thuộc: không (dùng scene hiện có; P10a đo text chính xác hơn giúp rule tràn text bớt nhiễu)

## Vấn đề
Agent bàn giao frame mà không có tín hiệu máy đọc nào về lỗi thiết kế cơ bản: chữ tương phản thấp, chữ quá nhỏ, nút quá bé, text bị cắt, node nằm ngoài frame. Hiện agent phải tự soi PNG hoặc tự tính từ `inspect_layout`. Server đã có sẵn đủ dữ liệu đo trong scene để tự phát hiện.

## Bằng chứng (nhánh `night/integration`)
- `crates/layout/src/model.rs:151-169`: `SceneNode` đã có `bounds`, `lines`, `font_size`, `fill` (màu resolved), `opacity`, `clip`, `parent_id`, `kind`, đủ cho cả năm rule.
- `crates/layout/src/model.rs:194` + `crates/layout/src/engine.rs:193`: scene có `diagnostics: Vec<String>` nhưng luôn rỗng.
- `crates/server/src/jobs/record.rs:35`: job chép `scene.diagnostics` thành `diagnostics` của job; `crates/server/src/inspect.rs:74` trả lại nguyên mảng đó. Đường ống đã có, chỉ thiếu nội dung.
- `crates/server/src/reads/diagnostics.rs:5-37`: mẫu diagnostic có cấu trúc (`code`, `frameId`, `message`) đã dùng cho `NO_REFLOW_INFORMATION`, nên lint theo đúng dạng đó.
- `crates/server/src/schemas/render.rs:8`: `render_frame` đã giữ `diagnostics` trong response gọn, nên lint tới agent mà không cần `includeScene`.

## Giải pháp
Thêm module thuần `crates/layout/src/lint.rs`, chạy sau khi layout trên `&[SceneNode]` và ghi vào `scene.diagnostics` (đổi kiểu sang `Vec<Value>`, hoặc thêm `lint: Vec<LintIssue>` cạnh mảng cũ để giữ schema). Đây không phải tool mới. `render_frame` và `inspect_layout` trả về như hiện có; `inspect_layout` thêm tham số tùy chọn `lint: "off"|"warn"` (mặc định `warn`) và lọc theo `nodeIds`.

Năm rule, mỗi node tối đa một issue cho mỗi rule:
| code | điều kiện |
| --- | --- |
| `LOW_CONTRAST` | text: tỉ lệ WCAG giữa `fill` (đã nhân opacity) và nền ước lượng < 4.5 (< 3.0 nếu `fontSize ≥ 24`, hoặc ≥ 18.66 khi đậm). Nền là node tổ tiên/anh em đứng trước gần nhất có fill đặc và bounds phủ tâm text; không có thì lấy fill của frame; gặp ảnh hoặc gradient thì `confidence:"low"`. |
| `TEXT_TOO_SMALL` | text `fontSize < 11` |
| `TAP_TARGET_SMALL` | node có `interaction`/`link` (P06), hoặc tên khớp `/button|btn|link|tab/i` khi chưa có P06: `min(w,h) < 24` |
| `TEXT_CLIPPED` | tổng chiều cao `lines` > `bounds.height` + 0.5, hoặc một dòng rộng hơn bounds; hoặc text vượt bounds của tổ tiên có `clip:true` |
| `OUTSIDE_FRAME` | bounds giao với frame gốc = 0 (`partial:true` nếu chỉ chạm một phần và frame có clip) |

Ví dụ issue tối thiểu:
```json
{"code":"LOW_CONTRAST","severity":"warn","nodeId":"t1","ratio":2.9,"required":4.5,
 "fg":"#9aa0a6","bg":"#ffffff","bgNodeId":"card","confidence":"high"}
```
Mảng giới hạn 50 issue, sắp theo severity rồi thứ tự cây; vượt thì thêm `{"code":"LINT_TRUNCATED","omitted":n}`.

## Phạm vi file
- Mới: `crates/layout/src/lint.rs` (≤200 dòng, chỉ các rule), `crates/layout/src/contrast.rs` (luminance, ratio, parse màu, ~60 dòng), `crates/layout/src/lint_tests.rs`.
- Sửa: `crates/layout/src/engine.rs` (gọi lint, khoảng dòng 193), `crates/layout/src/model.rs` (kiểu issue), `crates/server/src/inspect.rs` (tham số `lint`, lọc theo `nodeIds`), `crates/server/src/schemas/render.rs` (mô tả), `docs/SERVER.md`, `skill://opencanvas/author-frame.md` (bước "đọc diagnostics trước khi bàn giao").
- Bump `scene_version` vì scene key đổi.

## AC
1. Text `#9aa0a6` trên frame `#ffffff`, 14px → đúng một `LOW_CONTRAST` với `ratio` sai lệch ≤0.05 so với công thức WCAG 2.x; đổi thành `#5f6368` thì không còn issue.
2. Text 24px với ratio 3.2 → không cảnh báo; ở 14px → có cảnh báo.
3. Text 10px → `TEXT_TOO_SMALL`; 11px → không.
4. Node `button` 20×40 → `TAP_TARGET_SMALL`; 24×24 → không.
5. Text 3 dòng trong box cao bằng 1 dòng → `TEXT_CLIPPED`; con của tổ tiên `clip:true` vượt bounds tổ tiên → `TEXT_CLIPPED`.
6. Node đặt ở x = frame.width + 10 → `OUTSIDE_FRAME`.
7. Frame mẫu sạch hiện có trong tests (export_support) → `diagnostics` không có issue lint nào (0 false positive).
8. `render_frame` (response gọn) và `inspect_layout` trả cùng mảng issue cho cùng sceneKey; `lint:"off"` trả mảng không chứa issue lint; `NO_REFLOW_INFORMATION` vẫn giữ nguyên.
9. Lint 500 node thêm < 5 ms (đo trong test); file mới ≤200 dòng.

## Cách kiểm
- `cargo test -p opencanvas-layout lint` (unit test cho từng rule và cho contrast).
- `cargo test -p opencanvas-server --test multi_viewport --test stack_context --test skill_layer` (không hồi quy diagnostics cũ).
- Server tạm `--port 0` + data root tạm: dựng frame AC1–6 bằng `apply_batch`, gọi `render_frame` rồi `inspect_layout`, đối chiếu mảng issue. Không đụng cổng 3847.

## Rủi ro
- Nền ước lượng sai khi nền là ảnh, gradient (P09a) hoặc chồng nhiều lớp bán trong suốt: đánh dấu `confidence:"low"`, không chặn bàn giao.
- Rule vùng chạm dựa trên tên dễ nhiễu cho tới khi P06 có `interaction`; giữ `severity:"info"` khi chỉ khớp theo tên.
- Đổi kiểu `diagnostics` từ string sang object có thể phá client cũ: phương án an toàn là thêm field `lint` riêng trong scene và gộp vào `diagnostics` ở tầng server.
- Đo text lệch (P10a) gây `TEXT_CLIPPED` giả: dung sai 0.5px, ghi rõ trong message.
