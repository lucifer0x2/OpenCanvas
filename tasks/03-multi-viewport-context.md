# Task 03 — Đọc nhiều viewport trong một lần gọi

**Trạng thái:** hoàn tất qua Codex CLI và root kiểm độc lập18/18 server tests (2026-10-28); xem `docs/TASK03-REPORT.md`.
**Phụ thuộc:** Task 01 (dùng chung hạ tầng diagnostics). Làm sau Task 01.
**Phạm vi sở hữu:** `crates/server/src/schemas.rs`, `crates/server/src/reads/`,
`crates/server/src/instructions.rs` (phần mô tả tool), `docs/SERVER.md`, `docs/CONTRACT.md`.
Không sửa `crates/layout`, `crates/storage`.
**Workspace:** `/Users/luis/Workspace/OpenCanvas`, branch `main`.

## Bối cảnh

`instructions::CONNECT` dặn agent "re-read at another viewport to see how the design reflows
instead of inventing breakpoints". Lời khuyên đúng, nhưng hôm nay nó đắt và mơ hồ:

- Agent phải tự nghĩ ra nên thử viewport nào.
- Mỗi lần thử là một vòng `render_frame` → poll job → `inspect_layout`.
- So sánh hai kết quả là việc của agent, và nó phải tự kết luận "giống nhau nghĩa là gì".
- Với frame `free`, hai lần đọc cho kết quả **y hệt** (xem Task 02), nên agent tốn hai vòng gọi
  để học được con số không.

`get_design_context` hiện đã nhận `viewport` nhưng chỉ **echo lại** nó vào response
(`crates/server/src/reads/tree.rs:89`), không dùng để tính gì.

## Mục tiêu

Biến "đọc lại ở viewport khác" từ một lời khuyên thành một trường dữ liệu. Agent gọi một lần,
nhận về câu trả lời trực tiếp cho câu hỏi "thiết kế này có reflow không, và reflow thế nào".

## Việc phải làm

Mở rộng `get_design_context` (hoặc thêm tool mới nếu thiết kế schema gọn hơn — người làm quyết
định và ghi lý do):

1. Nhận `viewports: [{width, height}, ...]` thay cho một `viewport` đơn. Giới hạn số lượng rõ
   ràng, nằm trong budget bytes hiện có; `viewport` đơn vẫn phải chạy để không phá client cũ.
2. Trả về bounds đã tính cho từng viewport, không chỉ authored props.
3. Trả về cờ `reflow` ở cấp response: `false` khi bounds của mọi node con trùng nhau qua các
   viewport. Đây là câu trả lời mà agent đang phải tự suy ra.
4. Khi `reflow: false`, phát diagnostic của Task 01 — hai cơ chế phải nhất quán, không mâu thuẫn.

Lưu ý thiết kế: tính bounds cần qua layout engine và có thể cần measure text, nghĩa là đụng
đường đi của job/worker. Khảo sát trước xem tái dùng `SceneRequest` được tới đâu, và nêu rõ chi
phí trong báo cáo. Nếu chi phí quá lớn cho một read đồng bộ, đề xuất phương án (ví dụ chỉ trả
bounds khi đã có job cache cho viewport đó) thay vì âm thầm làm read chậm đi.

## Acceptance Criteria

- Gọi với `viewports` nhiều phần tử trả về bounds cho từng viewport trong một response.
- `reflow: false` đúng với frame toàn `free`; `reflow: true` đúng với frame stacks của Task 02.
- `viewport` đơn và client không truyền gì vẫn hoạt động như trước.
- Vượt giới hạn số viewport hoặc budget bytes → lỗi client-safe, có `truncated` tường minh theo
  đúng quy ước phân trang hiện có.
- Mọi file mã viết tay ≤250 dòng.
- `docs/SERVER.md` và `docs/CONTRACT.md` ghi hình dạng mới.

## Cách kiểm

```bash
cargo test -p opencanvas-server --offline
cargo clippy -p opencanvas-server --offline -- -D warnings
```

Integration test: frame `free` → `reflow:false`; frame stacks → `reflow:true` kèm bounds khác
nhau; quá giới hạn → lỗi đúng; client cũ truyền `viewport` đơn → response tương thích.

Kiểm qua MCP thật trên data root tạm với cả hai frame mẫu.

## Đầu ra

- Delta mã + tests.
- Docs cập nhật.
- Báo cáo: hình dạng request/response cuối cùng, chi phí tính bounds, và quyết định thiết kế
  (mở rộng tool cũ hay thêm tool mới) kèm lý do.

## Ngoài phạm vi

- Không thêm breakpoint vào mô hình design (xem Task 04). Task này chỉ **đọc** thiết kế ở nhiều
  viewport, không cho phép thiết kế **khai báo** hành vi theo breakpoint.
