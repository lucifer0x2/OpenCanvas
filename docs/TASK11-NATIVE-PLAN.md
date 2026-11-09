# Task10 nền → Task11: native PNG

## Quyết định 2026-11-04

- Người dùng chọn nền Task10 rồi Task11, chưa làm viewer Task12.
- Gate tốc độ WASM Task09 không chặn triển khai native; không suy ra viewer đủ nhanh.
- Chromium vẫn mặc định, native opt-in; không deploy, push hoặc đổi dữ liệu thật.
- So Chromium: ngoài text, tỷ lệ pixel có chênh kênh >8/255 phải dưới 0,5%; text có ảnh đối chiếu.
- Glyph thiếu dùng `.notdef` và diagnostic `MISSING_GLYPH`, không mượn font hệ thống;
  liệt kê riêng ngoài parity text. Lỗi nạp/decode font không phải glyph thiếu.
- Giữ Task09 chưa commit trên nhánh `feat/task11-native-png`, không stash/discard.

## Ranh giới và thứ tự

1. `crates/text`: font bytes/hash/WOFF2, positioned runs dùng chung đo/vẽ,
   metrics/ink, transform/wrap/tracking theo renderer hiện tại. Thuần, không I/O/thread.
2. `crates/layout`: provider đo đồng bộ dùng chung thân engine; giữ `compute(input, cache)`
   và `layout-step`. Native một invocation engine, không vòng RPC giả đồng bộ.
3. `crates/render`: pure painter giữ đủ scene-3/web-4, RGBA premultiplied rõ ràng;
   encode PNG tách riêng. Không dùng adapter spike bỏ mất typography/paint.
4. Server: chọn backend ở startup/config, fingerprint/cache tách biệt; bytes từ store,
   compute → states → diagnostics → frame/export. Giữ SVG và error/readiness contracts.
5. Oracle/parity độc lập → sửa có bằng chứng → kiểm delta → báo cáo giới hạn.

Các worker có ownership riêng; worker text sở hữu root workspace manifest trong lát đầu.
Tích hợp server/manifest render chỉ giao khi contracts nền đã ổn định.

### Lát F3 sau bàn giao render crate

- Thay riêng paint dispatch của F2: backend Rust không khởi động `Worker`, kể cả PNG
  frame, export nhiều setting hoặc SVG-only. Backend Chromium giữ đường hiện tại.
- Tái sử dụng `subtree::scene`, quy tắc `ExportSetting::scales`, tên/metadata/file route,
  giới hạn 48 MiB và cách embed font/ảnh SVG; không viết lại crop/export contract.
- Giữ opaque diagnostics sau states/advisories. Provenance ghi engine native thật,
  không thêm browser version giả hoặc chép `null` vào metadata hợp lệ.
- Bytes/hash từ store; bundle font/ảnh giữ trong job để tránh decode lặp khi xuất nhiều scale.
  Scene vẫn ready nếu ảnh strict lỗi, nhưng không được cấp image URL/file khi render thất bại.
- Dùng cùng lane/drain/deadline cho CPU native; timeout ngoài không được bỏ rơi paint task.
- Kiểm Rust qua MCP/HTTP và binary trực tiếp với Node/layout worker không khả dụng:
  frame có text/ảnh, state preview, transparent crop, scale/target width/height,
  PNG+SVG, lỗi resource và pixel budget. Đây là kiểm tích hợp, không thay pixel parity.

## Ngữ nghĩa không được làm mất

- Width/fit ceil64, baseline font bounding metrics, CR/LF, horizontal whitespace,
  nonbreaking groups, grapheme emergency breaks, CSS case, static font faces.
- Tracking, alignment, decoration, ink/glyph advisories; không transform/wrap lần hai khi paint.
- Responsive, scoped themes/instances và state overrides qua input/scene hiện có.
- Gradient, corners, per-side stroke, outer/inset shadow, clip và group/text opacity.
- PNG/JPEG/WebP, ICC sang sRGB, crop/fit và giới hạn decode/output/layer.

## Seam server và rủi ro

- `jobs/input.rs` tiếp tục là nguồn input expanded/resolved, không dựng model native khác.
- `fingerprint.rs` hiện gọi Node khi tạo Config: đường Rust cuối phải không probe Node.
  Constructor native phải chọn backend trước fingerprint, không gọi `Config::new` rồi mới đổi backend.
- Diagnostic server có object, trong khi layout `Scene` dùng `Vec<String>`: adapter paint
  phải giữ diagnostic opaque, không deserialize mù scene sau append advisories.
- Scene ready và strict PNG lỗi là hai trạng thái riêng; giữ hành vi hiện có.
- Native CPU chạy ngoài reactor; timeout không được tạo dồn blocking jobs còn chạy.
  Chặn publish kết quả trễ và dùng cancellation/deadline checks ở pha có giới hạn.
- Font `trak`, shaping/tracking/metrics theo OS và ICC cần đối chứng, không giả định parity.

## Kiểm nghiệm thu

- Text oracle Chromium pin: corpus cũ và semantics mới, Noto + font upload + font `trak`;
  line list/width/height/baseline/bounds kiểm riêng với ink/raster. Không nới gate âm thầm.
- Render fixtures có đủ paint hiện tại, ảnh đối chiếu Chrome/Rust/diff và text mask chặt;
  báo denominator/tolerance/missing glyph, không che lỗi bằng mask cả text box rộng.
- Native frame/export sizing, resource/hash/version errors, layer counters và cache separation.
- Chứng minh binary native startup/render/export vẫn chạy khi Node/layout worker không khả dụng.
  Dùng binary Rust trực tiếp; launcher npm hiện tại không phải bằng chứng chạy không Node.
- Test/lint/fmt/size checks đúng vùng đổi; giữ default Chromium regression.
- WASM build/correctness nếu kiểm được ghi riêng, không điều kiện tốc độ native và không Task12.
- Chỉ version source/tests/report/lệnh; generated captures/font bytes/log/JSON/ảnh lớn ignored.

Đây là kế hoạch triển khai, chưa phải bằng chứng nghiệm thu. Trạng thái/kết quả ở
`docs/agents/LOG.md` và báo cáo các module sau mỗi bàn giao.
