# P31 — Khôi phục node/subtree về trạng thái ở revision R

Ưu tiên: P2 · Kích cỡ: M · Phụ thuộc: không (dùng log commit sẵn có; nên làm sau P24 nếu có instance)

## Vấn đề

Agent làm hỏng một frame/subtree (xoá nhầm, sửa props hàng loạt sai) thì không có đường quay lại ngoài việc tự dựng lại từng op. `docs/ACCEPTANCE.md` công bố giới hạn "không arbitrary authored history", nên hiện chỉ đọc được *những gì đã đổi*, không đọc/khôi phục được *trạng thái cũ*.

## Bằng chứng

- `crates/server/src/schemas.rs:127-128` — `get_changes` đọc log bền vững sau `sinceRevision`, có `includeOperations:true`; log đã lưu operation của từng commit, nhưng trả về trạng thái ở `currentRevision`, không ở revision cũ.
- `crates/server/src/mcp.rs:121` — chỉ route đọc `get_changes`; không có đường ghi nào dùng log.
- `crates/storage/src/project.rs:30` — `snapshot()` chỉ trả snapshot hiện tại; không có API "snapshot của subtree tại R".
- `crates/storage/src/assets.rs:76-80` — mẫu ghi chuẩn: `read_snapshot` → kiểm `expected_revision` → `Candidate::from_snapshot` → validate → commit; op mới đi qua đúng đường này.

## Giải pháp (mở rộng tool sẵn có, không thêm tool)

1. Thêm op `restore_subtree {nodeId, revision}` vào `apply_batch`. Server dựng trạng thái subtree tại `revision` bằng cách phát lại ngược (inverse) các operation trong log từ `currentRevision` xuống `revision+1`, chỉ cho các ID thuộc subtree (theo `changedIds`).
2. Kết quả ghi thành **một commit mới** (revision tăng), không viết lại lịch sử; `get_changes` thấy nó như mọi commit khác. Node bị xoá sau R được tạo lại với ID cũ; node tạo sau R trong subtree bị xoá.
3. Giới hạn rõ: chỉ trong cửa sổ log còn giữ (nếu thiếu → `RESET_REQUIRED`/`HISTORY_UNAVAILABLE`); tham chiếu token/style/asset/font đã bị xoá → lỗi `RESTORE_DANGLING_REF` kèm danh sách ID, không tự khôi phục tài nguyên; không khôi phục tokens/styles (chỉ node); tối đa 2000 node/op; `nodeId` là page/frame/node đều được, không cho root project.
4. Thêm `get_subtree {nodeId, revision}` đọc-chỉ (cùng bộ dựng) để agent xem trước trước khi restore — mở rộng tham số `revision` hiện có, không phải tool mới.

## Phạm vi file

- `crates/storage/src/` — module mới `history_rebuild.rs` (dựng subtree tại R từ log), op mới trong module batch.
- `crates/server/src/schemas.rs` (+ `schemas/` nếu đã tách) — schema op `restore_subtree`, mô tả giới hạn.
- `crates/server/src/reads/` — `get_subtree` nhận `revision` < current.
- `docs/SERVER.md`, `docs/ACCEPTANCE.md` — sửa dòng giới hạn thành "restore subtree trong cửa sổ log".
- Tests: `crates/storage/tests/restore_subtree.rs`, `crates/server/tests/` (MCP).

## AC

1. Tạo frame F (10 node) ở R1, sửa props 3 node + xoá 2 node + thêm 4 node ở R2..R5; `restore_subtree {F, R1}` → `get_subtree F` bằng byte-for-byte (JSON chuẩn hoá) với đọc tại R1.
2. Restore sinh đúng 1 commit mới (revision = current+1), `get_changes` liệt kê đủ ID đổi; các node ngoài subtree không đổi (so sánh snapshot).
3. Revision ngoài cửa sổ log → lỗi có mã, không ghi gì; tham chiếu tài nguyên đã xoá → `RESTORE_DANGLING_REF` liệt kê ID.
4. `expectedRevision`/`idempotencyKey` hoạt động như op khác (replay trả cùng kết quả, xung đột trả `REVISION_CONFLICT`).
5. Subtree 2000 node restore < 500 ms trên máy dev; > 2000 → lỗi `LIMIT_EXCEEDED`.
6. File mới ≤ 250 dòng.

## Cách kiểm

`cargo test -p opencanvas-storage restore_subtree` và `cargo test -p opencanvas-server` (test MCP mới); data root tạm, không chạm cổng 3847.

## Rủi ro

- Log có thể không đủ để đảo ngược (op không lưu giá trị cũ) → cần kiểm định dạng log trước; nếu thiếu, phải dựng tiến từ snapshot gần nhất/đầu log (tốn hơn).
- Khôi phục ID cũ có thể đụng ID đã tái dùng ở chỗ khác → validate trùng ID, lỗi rõ.
- Thứ tự sibling và parent đã bị xoá: nếu parent của F không còn → lỗi, không tự tạo parent.
