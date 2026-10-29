# P32 — Sửa đồng thời nhiều agent: expectedRevision theo subtree

Ưu tiên: P2 · Kích cỡ: M · Phụ thuộc: không (P02c notes giúp phần claim, không bắt buộc)

## Vấn đề

Khi 2+ agent (2 designer, hoặc designer + coder ghi note) cùng sửa một project, mọi `apply_batch`
so `expectedRevision` với revision **toàn project**. Agent A sửa frame X, agent B sửa frame Y (không
liên quan), nhưng batch của B vẫn nhận `REVISION_CONFLICT`. Agent phải tự rebase: đọc `changedSince`,
kiểm từng ID có phải node mình sửa hoặc tổ tiên không, rồi gửi lại với idempotencyKey mới. Mỗi xung
đột giả tốn ít nhất một round-trip, và có thể phải gọi `find_nodes`/`get_changes` nếu `truncated` hoặc
agent không biết cây tổ tiên. Với N agent ghi liên tục, tỉ lệ conflict giả tăng theo N.

## Bằng chứng

- `crates/storage/src/project.rs:94`: `before.revision != request.expected_revision`, so sánh toàn project.
- `crates/server/src/reads/changes/conflict.rs:23`: conflict chỉ được *làm giàu* thêm `changedSince`
  (tối đa 200 ID / 100 commit, `MAX_IDS`/`MAX_COMMITS` ở dòng 10–12); server không tự rebase.
- `crates/server/src/instructions.rs:14`: hướng dẫn agent tự rebase (kiểm ID đã đổi so với node sửa và tổ tiên).
- `crates/server/src/schemas.rs:134`: schema `apply_batch` chỉ có `expectedRevision` dạng số nguyên.

## Giải pháp (mở rộng tool sẵn có, không thêm tool)

1. `apply_batch` nhận thêm tham số tuỳ chọn `conflictScope: "project" | "touched"` (mặc định `project`,
   giữ nguyên hành vi). Với `touched`, server lấy tập ID mà batch đụng tới (target, parent, và tổ tiên
   của chúng tới root page) rồi giao với `changedIds` của các commit trong `(expectedRevision, current]`.
   Giao rỗng và `truncated:false` thì server tự áp batch trên `current`. Validation đầy đủ vẫn chạy lại
   như hiện tại. Giao khác rỗng thì trả `REVISION_CONFLICT` cũ, thêm `overlapIds`.
2. Phản hồi thành công ghi `rebasedFrom: expectedRevision` để agent biết server đã rebase hộ.
   Idempotency giữ theo body gốc.
3. Khoá mềm (tuỳ chọn, giai đoạn 2): op `claim_note` qua hệ note sẵn có (`*_note`), là note có
   `kind:"claim"`, `owner`, `expiresAt` gắn vào frame. Claim chỉ để tư vấn: `apply_batch` với `touched`
   thêm cảnh báo `CLAIMED_BY_OTHER` vào kết quả, không chặn ghi. Chọn cách này để tránh deadlock khi
   agent chết.

## Phạm vi file

- `crates/server/src/schemas.rs` (thêm `conflictScope`), `crates/server/src/instructions.rs` (thêm 1 câu).
- Module mới `crates/server/src/reads/changes/rebase.rs` (tính tập touched + ancestors, giao changedIds),
  gọi từ đường `apply_batch`, cạnh `conflict.rs`.
- `crates/storage/src/project.rs` (nhánh commit tại `current` khi được phép; giữ khoá ghi đơn).
- Test mới `crates/server/tests/multi_agent_rebase.rs`.

## AC

1. Kịch bản 2 agent, 2 frame khác page/khác nhánh, mỗi agent 50 batch xen kẽ với `expectedRevision`
   cũ 1 bước: có `touched` thì **0** conflict giả, không có thì ~50 conflict (đo trong test, in số đếm).
2. Kịch bản 2 agent sửa cùng node, hoặc một agent sửa tổ tiên của node agent kia sửa: có `touched`
   vẫn nhận `REVISION_CONFLICT` với `overlapIds` chứa ID đó, 100% số lần.
3. `delete_subtree` hoặc reparent bởi agent A trên tổ tiên của node B đang sửa ⇒ B nhận conflict,
   không có node mồ côi. Snapshot sau cùng qua validate.
4. Mặc định (không gửi `conflictScope`): các test protocol/tool_schemas hiện có pass nguyên vẹn.
5. `truncated:true` (hơn 200 ID hoặc hơn 100 commit) ⇒ luôn conflict, không auto-rebase.
6. Số round-trip trung bình mỗi batch thành công trong AC1 ≤ 1.05 (so với ~2 khi không có `touched`).

## Cách kiểm

`cargo test -p opencanvas-server --test multi_agent_rebase` và `--test tool_schemas`, `--test protocol_2026`.
Chạy live bằng server tạm `--port 0` + data root tạm, 2 client MCP song song, đếm conflict trong log.

## Rủi ro

- Rebase ngữ nghĩa: batch B đọc giá trị (ví dụ token) mà A vừa đổi. Phạm vi touched không bắt được
  phụ thuộc qua token/style, nên phải tính token/style ID mà batch tham chiếu vào tập touched.
- Tính tổ tiên cần cây tại `current`, tốn thêm chi phí trong khoá ghi. Cần đo p95 latency apply_batch.
- Claim note có thể hết hạn hoặc bị bỏ rơi. Chỉ dùng để tư vấn, có TTL.
