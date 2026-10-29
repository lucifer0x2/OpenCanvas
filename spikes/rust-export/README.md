# Spike: xuất ảnh thuần Rust so với Chromium Canvas2D

Đo ngày 2026-10-29 trên macOS arm64. Phân tích và khuyến nghị ở
[docs/PURE-RUST-EXPORT.md](../../docs/PURE-RUST-EXPORT.md); file này giữ cách chạy và bảng số.

- **Rust:** harfrust 0.15.0 (khớp HarfBuzz 15.0.0), skrifa 0.49.0 / read-fonts 0.46.0, tiny-skia 0.12.0, png 0.18.1.
- **Chrome:** Chromium 156.0.8078.4 (headless_shell-1248), Playwright 1.64.1, Node 24.21.0. Trang tạm import `renderer/dist/index.js` thật:
  `loadResources`, `measureBatch`, `drawScene` + `toDataURL`. Mỗi phần chạy hai lần: launch mặc định và `--font-render-hinting=none`.
- **Font:** `renderer/fonts/NotoSans.ttf` (variable, instance mặc định wght 400).

## Chạy lại

Đường dẫn font, Playwright và browser tính tương đối từ thư mục spike (`src/font.rs`, `js/chrome.mjs`), nên chạy được ở
mọi checkout/worktree có `node_modules` và `.runtime/browsers`. Package đứng riêng (`[workspace]` rỗng), không thuộc workspace gốc.

```bash
cargo build --release --manifest-path spikes/rust-export/Cargo.toml
```

```bash
node spikes/rust-export/js/chrome.mjs
```

Subcommand của binary: `measure`, `measure-model <variant>`, `wrap`, `make-image`, `render`, `compare` (xem `src/main.rs`).
`js/analyze_widths.mjs` và `js/analyze_wrap.mjs` tạo `data/widths_report.json`, `data/wrap_report.json`.
Ảnh render thô không commit; chỉ giữ hai ảnh ghép 2× trong `out/` (Chrome | Rust | diff ×8, trắng là trùng).

## Phần A — độ rộng text (154 chuỗi × 8 cỡ = 1 232 cặp)

| Mô hình width phía Rust | Bằng đúng Chrome | max \|Δ\| px | Lệch sau round 1/64 | Lệch sau ceil 1/64 |
| --- | --- | --- | --- | --- |
| Tổng advance × size / upem | 0 | 0.00106 | 11 | 2 |
| **Mô hình Chrome:** advance từng glyph cắt xuống lưới 1/65536 px, kerning từ HarfRust | **1 176** | **0.000061** | **0** | **0** |

- Chrome luôn hẹp hơn tổng thô chút ít vì cắt advance từng glyph về 16.16 fixed-point.
- 11 ca lệch của mô hình thô đều nằm sát điểm giữa hai bước 1/64 (ví dụ "Người dùng" 14px: 79.078125 so 79.09375). Ceil 1/64 (P10a) vẫn lật 2 ca.
- `--font-render-hinting=none` không đổi gì trên macOS: width và PNG trùng từng byte với launch mặc định.

**Wrap** (20 đoạn × width 120/200/320 × cỡ 14/20, 18/26 = 120 ca, ~46 dòng tách giữa từ):
mô hình thô cho text dòng trùng 120/120 nhưng 3 dòng lệch width 1/64 (một dòng 119.328 so 119.344 với giới hạn 120 px);
mô hình Chrome trùng 120/120 cả text, width và baseline.

## Phần B — pixel trên scene cố định 400×300

Scene: card bo 12 + viền 1px, text tiếng Việt/Anh 14/20 và 24/32, ảnh cover trong clip bo 10, parent clip bo 16 + viền 2px,
nhóm opacity 0.5 hai rect chồng. Δ là chênh lớn nhất giữa các kênh màu.

| Scale | Vùng (pixel) | Δ>0 | Δ>8 | Δ>32 | max Δ | mean Δ |
| --- | --- | --- | --- | --- | --- | --- |
| 1× | text (34 840) | 4 628 | 3 634 | 1 491 | 127 | 3.64 |
| 1× | hình (80 600) | 10 260 | 169 | 1 | 39 | 0.16 |
| 1× | ảnh (4 560) | 4 417 | 58 | 20 | 49 | 2.52 |
| 2× | text (139 360) | 10 490 | 9 070 | 6 003 | 195 | 3.34 |
| 2× | hình (322 400) | 40 283 | 498 | 91 | 72 | 0.16 |
| 2× | ảnh (18 240) | 13 635 | 103 | 9 | 60 | 2.05 |

- Hình: phần lớn là ±1 do làm tròn qua lớp opacity; Δ>8 chỉ ở mép AA của góc bo, viền và clip.
- Ảnh: lệch nhỏ do căn mẫu bilinear, lớn nhất ở mép ô caro.
- Text: Chrome trên macOS raster glyph bằng CoreText nên không thể về 0.

Tinh chỉnh text (chỉ vùng text):

| Biến thể | 1× Δ>32 | 1× mean | 2× Δ>32 | 2× mean |
| --- | --- | --- | --- | --- |
| Gốc (coverage tuyến tính) | 1 491 | 3.64 | 6 003 | 3.34 |
| Tăng contrast 1.45 | 1 182 | 3.02 | 4 735 | 2.62 |
| Snap (x 1/4 px, baseline px nguyên) + contrast 1.45 | **920** | **2.72** | 4 497 | 2.54 |
| Snap + contrast 1.8 | 1 148 | 2.99 | **4 084** | **2.28** |
| Giảm contrast 0.8 (đối chứng) | 2 127 | 4.61 | 6 812 | 3.94 |

## Phần C — thời gian (chỉ tham khảo, máy Mac, warm)

| | 1× | 2× |
| --- | --- | --- |
| Rust render + PNG encode | 2.40 ms | 7.10 ms |
| Chrome `drawScene` + `toDataURL` trong trang | 0.48 ms | 1.42 ms |

Số Chrome **không** gồm launch browser, Playwright IPC, base64 NDJSON, spawn `layout-step` mỗi pass.
Painter Rust chưa tối ưu: shape lại text mỗi frame, không cache glyph, layer full-canvas, nén PNG mặc định.
Binary release (strip, LTO): 2 107 408 byte, chưa nhúng font.

## Giới hạn

- Mới đo trên macOS. Docker Linux dùng FreeType cho glyph và mặc định hinting (width nguyên, note #13 trong P10a): phải chạy lại phía Chrome trong image.
- Một font, một instance, LTR Latin/tiếng Việt. Chưa có fallback font, bold/italic, RTL/complex script, emoji, letterSpacing.
- Mô hình width kiểu Chrome là suy ngược cho build Chromium này; cần test parity trong CI nếu dùng.
- Port wrap tách grapheme bằng `unicode-segmentation` thay `Intl.Segmenter("vi")`; whitespace xấp xỉ `\s` của JS. Trùng trên corpus này.
- Painter chỉ có viền một màu (chưa port wedge màu từng cạnh), cubic thay conic của Skia, layer opacity full-canvas.
