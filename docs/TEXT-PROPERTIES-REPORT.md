# Text properties P10a/P11a/P10b — 2026-10-30

Nhóm1/2 của [đợt 5 nhóm thuộc tính](DESIGN-PROPERTIES-IMPLEMENTATION.md). AC gốc ở
[roadmap](design-properties-figma-research/roadmap.md) dòng A (P10a+P11a) và B (P10b).
Chạy trên working tree `main`@`6ccf191` chưa commit, **đã có paint lane tích hợp**
(corners/shadow/gradient). Pipeline version giữ `scene-3`/`web-4` (aggregate chưa phát
hành, không bump). Môi trường: macOS, Node 24.21.0, Playwright 1.64.1, Chromium
156.0.8078.4, rustc 1.99.1; font ghim theo hash: `renderer/fonts/NotoSans.ttf`,
`fixtures/fonts/Lato-Regular.ttf`.

## Đã triển khai theo AC

**P10a — metrics, width, wrap, whitespace**

- Baseline = `fontBoundingBoxAscent + floor((lineHeight − ascent − descent)/2)`, tức
  half-leading theo metrics font như CSS; lineHeight nhỏ hơn font (âm leading) và
  lẻ đều khớp DOM marker.
- Width báo cáo và phép fit dùng cùng lưới `ceil64`; text đo lại ở width của chính nó
  không vỡ dòng (`Prefer` hug ở W giữ 1 dòng, W−1/64 thành 2 dòng).
- `textWrap: wrap|nowrap`; LF luôn ngắt; whitespace ngang co lại (tương đương
  `white-space: pre-line`). NBSP U+00A0, narrow NBSP U+202F, word joiner U+2060 không
  là điểm ngắt; từ dài ngắt khẩn cấp theo grapheme (không cắt cluster dấu/ZWJ/cờ).
  Nowrap tràn ngang căn start như Chrome.
- Module: `renderer/text-wrap.ts`, `renderer/measurement.ts` (điều phối),
  `crates/layout/src/model/text.rs` (wire contract), `crates/layout/src/text_lines.rs`
  (validate/align/decoration/diagnostics, không đo lại, không sửa source).

**P11a — glyph thiếu**

- `renderer/glyph-coverage.ts` đọc cmap SFNT format 4/12 từ bytes đã ghim (không suy từ
  "font load thành công"). Text NFC và NFD tiếng Việt không cảnh báo giả.
- Thiếu mapping → `MISSING_GLYPH node=… font=…: cmap has no mapping for U+…` (tối đa 8
  code point mỗi node, tổng tối đa 20 cảnh báo + `GLYPH_DIAGNOSTICS_LIMIT`).
- WOFF/WOFF2/container lạ/cmap hỏng → không có coverage →
  `GLYPH_CHECK_UNAVAILABLE`, không báo pass bịa. Geometry giống hệt khi có/không coverage.

**P10b — tracking, align, decoration, transform**

- `letterSpacing`: px có dấu, hữu hạn −1000..1000; vào measure request/cache key; scene
  mang `letterSpacing`. Tracking đổi điểm wrap ở biên và cache miss đúng.
- `textAlign: left|center|right` tính sau đo trong `text_lines::align` (center làm tròn
  xuống 1/64 như Chrome).
- `textDecoration: none|underline|line-through` → scene `decoration{line,offset,thickness}`;
  vẽ liền, không skip-ink (tương đương CSS `text-decoration-skip-ink:none`, đã ghi ở
  props guide và capability).
- `textTransform: none|uppercase|lowercase|capitalize` (`renderer/text-transform.ts`)
  chỉ áp vào chuỗi hiển thị/đo; `props.text` gốc giữ nguyên ở context, snapshot, bundle
  (`Straße`→`STRASSE`, `ﬁ`→`FI`, `o'neil`→`O'neil`).
- Consumers dùng cùng line origins của scene: canvas/PNG `renderer/paint-text.ts`,
  preview DOM `web/src/preview/textContent.ts`, SVG production
  `crates/server/src/svg/text.rs` (font nhúng, `letter-spacing`, decoration là `<rect>`).
- Storage: `crates/storage/src/validate/text_props.rs` cho node và text style, rollback
  nguyên tử khi giá trị sai hoặc token theme sai; giữ qua style/theme/instance/
  snapshot/bundle.
- Extent: `renderer/scene-tree.ts` union `line.inkBounds` (đo từ `actualBoundingBox*`
  có tracking) vào extent text, nên opacity layer của group không cắt ink tracking âm
  lớn/overhang.

## Lệnh đã chạy (trên tree hiện tại, kết quả cuối)

| Lệnh | Kết quả |
| --- | --- |
| `cargo test -p opencanvas-layout --lib` | 27 pass |
| `npm run check:renderer` | tsc pass (exit 0) |
| `node --test renderer/glyph-coverage.test.mjs renderer/text-extent.test.mjs` | 7 pass (3 + 4) |
| `npm run build:renderer` + `cargo build -p opencanvas-layout --bin layout-step` | build OK |
| `node --test --test-concurrency=1 spikes/pipeline/text-measure.test.mjs` | 5 pass |
| `node --test --test-concurrency=1 spikes/pipeline/text-pixels.test.mjs` | 4 pass |
| `cargo test -p opencanvas-storage --test text_props` | 2 pass |
| `cargo test -p opencanvas-server --test text_rendering` (gọi `spikes/pipeline/text-svg-offline.mjs`) | 1 pass |
| `cargo test -p opencanvas-server --test font_face_context` | 2 pass |
| `rustfmt --check` các file Rust của nhóm text (edition theo crate) | sạch |

Hai pipeline suite cuối cùng chạy chung một lệnh sau khi build lại renderer: 9/9 pass.
Server tests dùng data root tạm + `127.0.0.1:0`; không chạm `~/.opencanvas`, cổng 3847,
container hay volume. Không còn process Chromium/layout-step/server sau khi chạy.

**text-measure** (so DOM cùng Chromium, không dung sai): 2 font × 6 size × 40 chuỗi ×
tracking {−0.5, 0, 1.25} = 1.440 tổ hợp, width/baseline bằng DOM chính xác và đo lại ở
width của chính nó vẫn 1 dòng; baseline 14 cặp font×(size,lineHeight) gồm 11/13, 15/15,
13/16; NBSP/U+202F/U+2060/grapheme; tracking {−1,0,2} biên wrap + cache hit; coverage;
transform giữ source.

## Pixel (dung sai hẹp)

`spikes/pipeline/text-pixels.test.mjs`, so canvas với CSS oracle và preview DOM production:

- Ma trận 1: 2 font × {14/20, 24/32} × tracking {−0.5,0,1.25} × {left, center, right,
  center+underline, right+line-through} = 60 CSS + 60 preview.
  - Preview: **60/60 khớp 0 pixel**.
  - CSS line-through: **12/12 khớp 0 pixel**; không dung sai cho strike.
  - CSS underline: **12/12 lệch 5–15 pixel, maxDelta 1 kênh, `unexpected` 0** (Noto:
    6/5/6 ở 14px, 12/15/15 ở 24px; Lato: 6/7/7 và 12/12/12). Dung sai chỉ cho pixel bên
    trong hình chữ nhật underline (bỏ hai pixel đầu mút), nơi canvas là màu mực đặc và
    lệch ≤1; đầu mút và mọi pixel khác phải chính xác.
- Ma trận 2: 8 size × 2 tỉ lệ lineHeight × {underline, line-through} + đoạn 3 dòng
  uppercase/center/tracking/strike có so line list: **66 so sánh, 0 pixel lệch**.
- Opacity layer: group opacity 0.5 chứa text `letterSpacing −30` (ink lệch trái tới
  x≈89, extent dự phòng cũ chỉ tới x=218) + một rect để buộc layer. 806 pixel mực nằm
  ngoài extent cũ; |darkness(0.5) − darkness(1)/2| tối đa **1.5** (hai lần làm tròn 8 bit).
  Đối chứng âm (xoá `inkBounds` khỏi scene) cho **127.5**, tức layer bị cắt — test phân
  biệt được.

## Bằng chứng ảnh

`docs/text-properties-evidence/canvas.png` và `production-svg.png` (400×240) được sinh lại
bởi `text_rendering` trên tree hiện tại; test khẳng định 0 pixel lệch giữa canvas và SVG
production mở offline (không request HTTP, font data URL), và tọa độ `<text>` bằng scene.
Hai file trùng byte (SHA-1 `1540eca…`).

Đã mở và xem: dòng Lato 24/32 center, capitalize, tracking −0.5, underline
"Straße Sao Chép / Hello-World O'neil" — hai dòng căn giữa, underline liền và đi qua
descender `p` (không skip-ink, đúng contract). Khối Noto 14/20 right, tracking 1.25,
line-through "Sao chép Ấp / second line ỹ" — mép phải thẳng hàng, dấu tiếng Việt
đúng, strike ở giữa x-height. Hai dòng `customers` nowrap trong box rộng 20 căn
center vẫn bắt đầu ở x=0 và tràn phải như Chrome. Không thấy khác biệt giữa hai ảnh.
Điểm cần biết: đầu mút underline/strike gồm cả tracking sau ký tự cuối (giống CSS).

## Giới hạn phải giữ rõ

- cmap chỉ chứng minh **scalar mapping**; không chứng minh complex shaping, emoji ZWJ/
  skin-tone, variation sequences hay ligature/cluster đúng.
- WOFF/WOFF2/format không đọc được → `GLYPH_CHECK_UNAVAILABLE`, không kiểm coverage.
- Glyph thiếu chỉ là **advisory**; chưa có fallback chain ghim, không fail render.
- Pixel chỉ cam kết trên runtime/font ghim ở trên (macOS + Chromium 156); **không hứa
  pixel giữa các OS**, Docker/Linux, Safari/Firefox chưa đo.
- Tracking chỉ px; không rich runs, RTL, ellipsis/maxLines, vertical align, variable axes.
- SVG giữ text + font nhúng, không outline (`SVG_TEXT_NOT_OUTLINED`); hiển thị phụ thuộc
  trình xem hỗ trợ `@font-face` data URL.
- Underline CSS có lệch 1 kênh ở vùng chồng glyph/underline như số liệu trên.

**Opacity của text tự chồng (R2).** Text có decoration, `letterSpacing` âm, hoặc các dòng
có ink chạm nhau (ink rect nới 1 đơn vị) dưới opacity của chính node được composite một
lần như CSS: Canvas/PNG đếm hai paint unit để buộc opacity layer (`renderer/text-overlap.ts`,
extent gồm cả rect decoration); SVG bọc `<text>`/`<rect>` trong một `<g opacity>`
(`svg/text.rs::self_overlaps`; tracking âm trong một `<text>` vốn đã composite một lần).
Còn lại: glyph tự chồng ở tracking ≥ 0 (font script, kerning chặt) không được phát hiện nên
Canvas vẫn composite hai lần; khi vượt `MAX_LAYER_PIXELS`, nhóm rơi về gập alpha vào từng
paint (`opacityLayerFallbacks`) và vùng chồng lại đậm hơn CSS.

## Thay đổi của phiên hoàn tất này

- `renderer/text-extent.test.mjs`: test cũ không chạy được (Node không resolve import
  không đuôi của `scene-tree.ts`, `ERR_MODULE_NOT_FOUND`); nay bundle bằng esbuild trong
  RAM và có 4 ca (ink trái, ink phải/dưới, fallback không inkBounds, group).
- `spikes/pipeline/text-pixels.test.mjs`: in số so sánh của ma trận 2; thêm test opacity
  layer qua pipeline thật (kèm đối chứng âm đã chạy tay).
- `rustfmt` cho `text_lines.rs`, `svg/text.rs`, `tests/text_rendering.rs`,
  `tests/text_props.rs` (chỉ định dạng).
- Không sửa file dùng chung với paint lane; union inkBounds trong `scene-tree.ts` đã có.

Kích thước: mọi file text ≤155 dòng. `cargo fmt --all --check` còn lệch ở các file
paint/dùng chung (engine, svg/mod, svg/elements, validate/*, paint_tests…), không thuộc
nhóm này; `git diff --check` báo dòng trống cuối `crates/storage/src/validate/stroke.rs`
(paint lane).
