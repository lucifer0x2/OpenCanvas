# Task 04 — Mở rộng schema layout cho responsive thật

**Trạng thái:** hoàn tất row wrap đã duyệt; root nghiệm thu độc lập 2026-10-28. Xem `docs/TASK04-REPORT.md`. `%`/breakpoint variants chưa được duyệt; dashboard thật giữ nguyên.
**Phụ thuộc:** Task 02 (danh sách giới hạn gặp thật khi author bằng stacks).
**Phạm vi sở hữu:** `crates/layout/`, `crates/storage/src/validate*`, `PLAN.md`,
`docs/RESOLUTION.md`, `spikes/pipeline/`.
**Workspace:** `/Users/luis/Workspace/OpenCanvas`, branch `main`.

## Bối cảnh phạm vi trước khi duyệt

Các giới hạn dưới đây là PLAN trước Task04. Người dùng đã duyệt riêng row wrap,
PLAN đã cập nhật trước code; phần `%` và breakpoint vẫn ngoài phạm vi được duyệt.

`PLAN.md` đang công bố rõ:

- "Layout chỉ gồm row/column, padding, gap, fixed/fill/hug và giới hạn min/max rõ ràng."
- "Frame gốc có kích thước cố định; không hỗ trợ chu kỳ phần trăm hoặc fill trong trục hug mơ hồ."
- "Responsive v1 chỉ là agent chọn viewport hữu hạn và bố cục trong phạm vi row/column đã công bố."
- "Không cam kết tự động mô phỏng CSS media query hoặc mọi hành vi responsive của website."

Task này **vượt** ra ngoài những dòng đó. Không được tự ý làm. Task 01, 02, 03 và 05 đều nằm
trong phạm vi PLAN hiện tại và nên hoàn tất trước, vì chúng có thể đã đủ cho nhu cầu thật.

## Vấn đề còn lại sau Task 02

Stacks giải quyết được co giãn một mức, nhưng không giải quyết được đổi cấu trúc. Bằng chứng từ
probe trong Task 02, nhánh B ở viewport 390×844:

```
f     w=390        ← khung
side  w=176
body  w=464        ← tràn 250px ra ngoài khung
cards w=400        ← 3 card × minWidth 120 + 2 gap × 20
```

Ba card chạm `minWidth` nên không co thêm, mà engine không có `wrap`, nên chúng tràn ngang thay
vì xuống dòng. "Ba cột trên desktop, một cột trên mobile" là **bất khả** với schema hiện tại.

Ba hướng, không nhất thiết làm cả ba:

| Hướng | Được gì | Giá phải trả |
| --- | --- | --- |
| `wrap` cho `row` | 3 cột → 1 cột tự nhiên theo chiều rộng | Thêm trục chính trong taffy, ảnh hưởng `hug` theo trục chéo, và khái niệm "số dòng" trong scene |
| Sizing `%` | Tỉ lệ thay vì px cứng | PLAN loại trừ tường minh vì "chu kỳ phần trăm"; cần quy tắc chặn vòng lặp |
| Breakpoint variants | Khai báo bố cục khác hẳn theo viewport, như Figma | Thay đổi lớn nhất: node có nhiều bộ props, đụng revision, resolver, scene key, viewer và cả MCP schema |

## Việc phải làm

1. Đọc danh sách giới hạn thật từ báo cáo Task 02. Chọn hướng dựa trên cái đã gặp, không dựa
   trên cái có thể hình dung.
2. Viết đề xuất: schema thay đổi gì, validate thế nào, scene key và revision bị ảnh hưởng ra sao,
   viewer đọc thế nào, và điều gì **vẫn** không làm được sau thay đổi.
3. Trình đề xuất cho người dùng. **Dừng ở đây nếu chưa được duyệt.**
4. Sau khi duyệt: cập nhật `PLAN.md` trước, rồi mới triển khai. PLAN là nguồn yêu cầu, không
   phải thứ sửa sau cho khớp với mã.

## Acceptance Criteria

Giai đoạn đề xuất:
- Có tài liệu đề xuất nêu đủ ba thứ: schema, chi phí lan toả, và giới hạn còn lại.
- Có ít nhất một fixture chứng minh hướng được chọn giải quyết đúng ca "3 cột → 1 cột".

Giai đoạn triển khai (chỉ sau khi duyệt):
- `PLAN.md` cập nhật trước khi có delta mã.
- Fixture pipeline phủ ca mới, cả ca biên (wrap với `hug`, min/max xung đột).
- `npm run test:pipeline` pass toàn bộ, không hồi quy ca cũ.
- Mọi file mã viết tay ≤250 dòng; `crates/layout/src/tree.rs` và `engine.rs` không phình quá ngưỡng.

## Cách kiểm

```bash
npm run test:pipeline
cargo test -p opencanvas-storage --offline -- --test-threads=2
```

Pipeline hiện đang 11/11 — con số đó phải giữ hoặc tăng, không được giảm.

## Đầu ra

- Tài liệu đề xuất (giai đoạn 1).
- Delta mã + fixture + PLAN cập nhật (giai đoạn 2, nếu được duyệt).
- Báo cáo nêu rõ điều gì vẫn không làm được.

## Ngoài phạm vi

- Không mô phỏng toàn bộ CSS. PLAN đã loại trừ và việc đó không nên quay lại bàn.
- Không đụng viewer trong cùng task này; nếu viewer cần đổi, tách task riêng.

## Nghiệm thu — 2026-10-28

Root xác nhận PLAN-first và kiểm độc lập: layout7, pipeline16 (giữ11 cũ),
storage41 (fault-injection), server19, E2E16 (giữ15 cũ) đều pass. Strict Clippy
ba library, fmt, JS format/syntax, diff và size checks pass. Fixture1040/640/390
thể hiện3→2→1 cột, text/hug đúng, MCP/restart/viewer PNG sai khác0; root đã xem
PNG desktop/mobile production thật. Evidence `artifacts/e2e/wrap.json`,
`artifacts/e2e/evidence.json`, summary `artifacts/task04/summary.json`.
Không mã viết tay vượt250 dòng, không blocker. Không %/breakpoint, không sửa
dashboard thật hoặc restart3847; backend đang chạy vẫn cần nâng đồng bộ với
layout binary mới để dùng feature. Không commit/push.
