# P25 — Quy trình agent end-to-end designer→coder: đo byte từng bước, đường token tối thiểu, skill `workflow.md`, compact mặc định
Ưu tiên: P1 · Kích cỡ: ~130′ dev → tách **P25a** harness đo (~60′) và **P25b** skill workflow + compact mặc định (~70′, sau P25a) · Phụ thuộc: không bắt buộc; số "sau đề xuất" giả định P19, P13a, P15, P22, P23, P20, P02c đã merge (P25a đo lại khi chúng vào) · Nguồn: hướng của root (đợt đêm 29/10), số đo TASK07

## Vấn đề

Chưa ai đo cả vòng: designer author → render/so → ghi note → coder đọc `get_design_context` → viết code → tự kiểm bằng ảnh. Mỗi đề xuất P13a/P15/P19/P20/P22/P23 tối ưu một bước riêng; không có con số tổng, không biết bước nào còn đắt nhất, và agent không có một chỗ nói "đường rẻ nhất là gì". Hậu quả: agent tự chọn đường đắt (outline toàn trang thay vì `find_nodes`, đọc context không `compact`, đọc PNG bằng mắt thay vì so ảnh, poll lặp), và mỗi phiên còn tự đọc hai skill 8–9 KB.

## Bằng chứng

Số đo fixture stacks 181 node (`docs/TASK07-REPORT.md:38-45`, `:300-336`). "Structured" là payload mô hình đọc; wire ≈ 2,2× vì `CallToolResult::structured` gửi JSON hai lần (text + structuredContent, `crates/server/src/mcp.rs:65`). Token ≈ structured B / 3,5 (ước lượng).

| # | Bước (vai) | Hôm nay (structured B) | Sau P19/P13a/P15/P22/P23/P20/P02c | Ghi chú |
| --- | --- | ---: | ---: | --- |
| 0 | Handshake mỗi phiên (×2: designer, coder) | `tools/list` 18 241 + `initialize` 3 331 (`TASK07-REPORT.md:338-339`); 24 525 @ `0847b4b` 16 tool (P19:6) | ~18 500 + ~3 300 (P19 AC1) | Cố định, nhân theo số phiên |
| 1 | Đọc skill (designer `author-frame.md`, coder `implement-frame.md`) | 9 032 + 8 557 (`crates/server/skills/*.md`) | như cũ | **Không đề xuất nào giảm** |
| 2 | Tìm frame | outline 2 trang 40 279 wire (S1) | `find_nodes` ~1 KB; P22 bounds khi cần vị trí | Đường rẻ đã có, agent không biết dùng mặc định |
| 3 | Author ~180 node qua `apply_batch` | ~36 KB request (ước, ~200 B/op, **chưa đo**) | P15: 0 B tool call (NDJSON qua HTTP) | `opencanvas.md:85` |
| 4 | Render + tự kiểm (3 viewport) | ready poll ~54 KB/job trước 07A; nay scene bị bỏ mặc định (`render_response.rs:1-4`); inspect 17,5 KB/vp, `fields:["bounds"]` 13,1 KB/vp (`TASK07-REPORT.md:200-203`) | P13a `compare` ≤ ~2 KB/vòng (P13a:25) | Đọc PNG 1440×900 ≈ 1 700 token ảnh/lần nếu không compare |
| 5 | Ghi note | — | P02c: field trong tool sẵn có | nhỏ |
| 6 | Coder đọc context | full 61 032; **`compact:true` 41 758** (−31,6 %, `resolvedProps` 20 397 → 1 066) | P20 thêm `codeSyntax` (+vài chục B/token), P23 lọc list_* | **Bước đắt nhất còn lại** |
| 7 | Coder tự kiểm bằng ảnh | như bước 4 | P13a compare | |

Tổng ước lượng một vòng (1 frame 181 node, 1 vòng kiểm 3 viewport mỗi vai), structured:
- Hôm nay, đường agent hay chọn: 2×21,6 K (handshake) + 17,6 K (skill) + 40 K (outline) + 36 K (author) + 2×52 K (inspect 3 vp) + 61 K (context full) ≈ **302 KB ≈ 86 K token**.
- Hôm nay, đường rẻ nhất đã có: 43 K + 17,6 K + 1 K + 36 K + 2×39 K (bounds) + 41,8 K ≈ **217 KB ≈ 62 K token**.
- Sau các đề xuất: 2×21,8 K + 17,6 K + 1 K + 0 (P15) + 2×~2 K (compare) + 41,8 K ≈ **108 KB ≈ 31 K token**. Còn lại: handshake 40 %, **context 39 %**, skill 16 %.

Kết luận: sau các đề xuất, bước đắt nhất là `get_design_context` (~230 B/node ngay cả compact), rồi handshake ×2 phiên, rồi skill. Hai thứ dưới cùng có thể giảm bằng hướng dẫn + mặc định, không cần tool mới.

## Giải pháp

**P25a — harness đo cả vòng (0 B `tools/list`).** Test `crates/server/tests/workflow_budget.rs` (theo mẫu `tests/tools_budget.rs:22-48`): dựng server với data root tạm, nạp fixture stacks 181 node, chạy đường tối thiểu bằng tool hiện có và in một dòng/bước:
```
workflow: step=<name> calls=<n> structured=<B> wire=<B>
```
Bước: `handshake`, `skill`, `find`, `author` (apply_batch chia 200 op), `render+inspect` (3 vp, `fields:["bounds"]`), `context` (`compact:true`), và nếu có `compare` (P13a) thì thay inspect. Thêm breakdown bước `context` theo khóa cấp một mỗi node (`props`, `resolvedProps`, `computed`, …) để quyết định bước kế (không thiết kế `fields` cho context khi chưa có số). Assert ngân sách tổng (AC2) để ngăn trượt.

**P25b — skill `workflow.md` + `compact` mặc định.**
1. Thêm `skill://opencanvas/workflow.md` (≤ 2 500 B) đăng ký trong `crates/server/src/resources.rs:6-13`: chuỗi gọi tối thiểu cho hai vai, mỗi bước một dòng với tham số rẻ nhất, ví dụ:
   ```
   coder: find_nodes {kinds:["frame"],query} → get_design_context {frameId, compact:true}
          → list_tokens {query} (P23) chỉ khi thiếu → code → render_frame {compare} (P13a)
   designer: get_outline {depth:1} → bulk NDJSON (P15) → render_frame {compare|fields:["bounds"]} → note (P02c)
   ```
   Hai skill dài giữ nguyên, `workflow.md` trỏ tới mục cần đọc khi gặp lỗi (đọc theo nhu cầu thay vì cả 17,6 KB).
2. Một câu trong server instructions (`crates/server/src/instructions.rs`) thay câu "Read skill://…implement-frame.md or author-frame.md": "Start with skill://opencanvas/workflow.md". Ròng ≤ +60 B `initialize`.
3. Đảo mặc định `compact` của `get_design_context` thành `true`; `compact:false` trả bản đầy đủ. Phản hồi đã tự mô tả (`resolvedPropsMode`, `resources`, `crates/server/src/reads/compact.rs:128-131`), nên client đọc được chế độ. Sửa mô tả ở `crates/server/src/schemas.rs:96-99` (ngắn lại, ≈ −150 B `tools/list`).

Không thêm tool. `tools/list` ròng ≤ 0 B; `resources/list` +~150 B.

## Phạm vi file

- P25a: `crates/server/tests/workflow_budget.rs` (mới, ≤ 200 dòng), có thể `tests/workflow_support/` cho fixture loader nếu > 200 dòng. Không sửa `src/`.
- P25b: `crates/server/skills/workflow.md` (mới), `src/resources.rs`, `src/instructions.rs`, `src/schemas.rs` (mô tả `compact`), `src/reads/compact.rs` hoặc chỗ chuẩn hóa `compact:false = absent` (`TASK07-REPORT.md:274`), `tests/compact_context.rs` (đổi kỳ vọng mặc định), `docs/SERVER.md`, `docs/CONTRACT.md`.
- Chạm `schemas.rs` → xếp sau P19, P22, P23 (cùng file).

## AC

1. `cargo test -p opencanvas-server --test workflow_budget -- --nocapture` in đủ 6 dòng `workflow: step=…` cho fixture stacks 181 node, data root tạm, không chạm `~/.opencanvas` hay cổng 3847.
2. Tổng structured đường tối thiểu (P25a, tool hiện có) ≤ **220 000 B**; test assert ngưỡng này (+5 % dung sai) và báo số thật trong report.
3. Breakdown bước `context` in byte theo khóa cấp một; tổng các khóa = structured của bước ± 1 %.
4. (P25b) `resources/list` có `skill://opencanvas/workflow.md`; file ≤ 2 500 B; mọi tool/tham số nhắc trong file tồn tại trong `tools/list` (test đối chiếu tên).
5. (P25b) `get_design_context {frameId}` không có `compact` trả `resolvedPropsMode:"differing"`, structured ≤ 42 500 B trên fixture (đo 41 758); `compact:false` trả lại đúng 61 032 B ± 1 % và `resolvedProps` đầy đủ.
6. (P25b) `tools/list` ròng ≤ 0 B, `initialize` ròng ≤ +60 B so với cùng HEAD trước P25b (`tests/tools_budget.rs`).
7. (P25b) Chạy lại AC1: tổng giảm ≥ 19 000 B so với P25a khi agent không truyền `compact` (đường "mặc định" của harness).

## Cách kiểm

- `cargo test -p opencanvas-server --test workflow_budget -- --nocapture` và `--test tools_budget -- --nocapture` (trước/sau), `--test compact_context`, `--test context`.
- Sau mỗi đề xuất P13a/P15/P19/P22 merge: chạy lại AC1, ghi bảng "hôm nay / sau" vào report để thay các ô ước lượng ở mục Bằng chứng.
- Kiểm live tùy chọn: server tạm `--port 0` + data root tạm, đọc `resources/read skill://opencanvas/workflow.md`.

## Rủi ro

- **Đảo mặc định `compact` là đổi hợp đồng**: client cũ đọc `resolvedProps` như bản đầy đủ sẽ thiếu khóa. Giảm: phản hồi đã mang `resolvedPropsMode`; ghi rõ trong `docs/CONTRACT.md`; viewer `web/` không dùng MCP nên không ảnh hưởng (cần xác nhận khi làm). Nếu người dùng không duyệt đổi mặc định, bỏ mục 3, giữ skill (AC5, AC7 bỏ).
- Token ước từ byte (/3,5) và việc client đưa text hay structuredContent cho mô hình khác nhau theo client; AC chỉ đo byte structured. Wire 2× (mcp.rs:65) là chi phí mạng, có thể không phải token.
- Số `author` (~36 KB) và token ảnh là ước lượng chưa đo; P25a phải thay bằng số thật.
- `workflow.md` có thể lệch khi tool đổi tên; AC4 đối chiếu tên để chặn.
- Context vẫn ~230 B/node sau compact; giảm tiếp (ví dụ `fields` cho context, bỏ `computed` khi chỉ cần cấu trúc) để task sau quyết dựa trên breakdown AC3, không làm trong P25.
