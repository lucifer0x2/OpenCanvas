# Triển khai 5 nhóm thuộc tính — 2026-10-30

Phạm vi người dùng đã duyệt: làm cả 5 mục ưu tiên trong
[báo cáo Figma](DESIGN-PROPERTIES-FIGMA-RESEARCH.md), trên pipeline Chromium hiện tại.

1. P10a/P11a: metrics, wrap/whitespace và diagnostic glyph thiếu.
2. P10b: letterSpacing px, textAlign, textDecoration, textTransform xuyên các consumer.
3. P19/P25/P29: capability theo tầng, hướng dẫn semantic text, fragmentation/overflow warnings và bằng chứng chi phí workflow.
4. P10c: chọn static face theo family/weight/style, giữ tương thích fontId.
5. P17/P04b/P09b/P09a: shorthand cạnh/góc, radii riêng, shadows và gradients.

Theo mặc định nghiên cứu: tracking px, glyph thiếu cảnh báo có bằng chứng,
SVG có diagnostic rõ cho giới hạn; không mở rich text, variable axes, blur,
vector, responsive %, hay engine Rust đã gác lại.

Thực hiện qua fresh worker, root điều phối và kiểm độc lập. Không chạm project
thật, port3847 hoặc restart server; không commit/push. Giữ nguyên thay đổi font
vừa hoàn tất và tài liệu phiên khác. Workers cập nhật bằng chứng thật trong
báo cáo riêng; không coi AC nghiên cứu là đã pass.

## Phân chia

- Text foundation: nhóm1/2; sở hữu integration text catalog/validation/contracts.
- Static faces: nhóm4; sở hữu face resolver và compact resource closure; phối hợp
  text worker để tích hợp các key chung, không sửa đè file đang làm.
- Discovery/diagnostics: nhóm3; capability sinh từ catalog, server scoped lint và skills.
- Paint: nhóm5, bắt đầu sau khi text contracts ổn định để tránh sửa đè model/paint.

## Trạng thái

**Đã nghiệm thu đủ 5 nhóm (2026-10-30), chưa commit/push.** Phiên đầu dừng ở
[checkpoint bàn giao](checkpoints/2026-10-30-design-properties-handoff.md); phiên sau hoàn tất.

- Nhóm1/2: [TEXT-PROPERTIES-REPORT](TEXT-PROPERTIES-REPORT.md).
- Nhóm3: [DESIGN-DISCOVERY-REPORT](DESIGN-DISCOVERY-REPORT.md); root đã review mã và chạy lại suites.
- Nhóm4: [FONT-FACE-REPORT](FONT-FACE-REPORT.md).
- Nhóm5: [PAINT-PROPERTIES-REPORT](PAINT-PROPERTIES-REPORT.md); sửa SVG alpha gradient
  premultiplied và inner radii/seam khi radii quá cỡ với stroke không đều.

Root kiểm độc lập trên một build debug đồng bộ (`npm run build`): Rust 29 suites,
renderer/web typecheck+test, pipeline 49/49, preview smoke 8/8, font UI smoke 3/3,
fmt/Clippy `-D warnings`/diff-check sạch, không file mã nào vượt 250 dòng.
Rủi ro mở: inset shadow + inside stroke khi inner radius tràn chưa có pixel test;
preview stroke center độ rộng lẻ lệch do snap nửa px; giới hạn text/SVG trong các report.
