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

Subcommand của binary: `measure`, `measure-model <variant>`, `wrap`, `make-image`, `render`, `compare`, và cho Phần D:
`bench`, `ndjson`, `paint-gate` (xem `src/main.rs`, `src/bench.rs`, `src/ndjson.rs`, `src/gate.rs`). `render` và `bench` nhận
`--paint full|bbox` (mặc định `bbox`). Lệnh chạy lại Phần D ở cuối Phần D.
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
Painter Rust lúc đo Phần C chưa tối ưu: shape lại text mỗi frame, không cache glyph, mask text và layer full-canvas, nén
PNG mặc định. Phần D đo lại sau khi mask text theo vùng bao và song song hóa (mục "Painter tối ưu").
Binary release (strip, LTO): 2 107 408 byte, chưa nhúng font. Số end-to-end (gồm launch, IPC, spawn) ở Phần D.

## Phần D — benchmark end-to-end (2026-10-29)

Phần C chỉ so phần vẽ. Phần D đo cả đường người dùng thấy: từ lúc gọi `render_frame` tới khi có bytes PNG, rồi phân rã
từng stage của đường Node + Chromium hiện tại và của một prototype Rust in-process dùng **đúng** crate
`opencanvas-layout` với **đúng** layout input server gửi cho worker. Số máy đọc được: [`data/e2e/results.json`](data/e2e/results.json).

**Môi trường.** Mac15,6, Apple M3 Pro (11 nhân: 5 hiệu năng + 6 tiết kiệm), 18 GiB, macOS 15.7.2. Lượt đo hiện tại
(2026-10-29 20:49–20:54 +07) chạy bằng pin (94%, Low Power Mode tắt); lượt đầu cắm điện. Node v22.23.2, Playwright 1.64.1,
Chromium 156.0.8078.4 (headless_shell-1248), rustc 1.97.0. Commit `cd41aff` cộng thay đổi chưa commit của Phần D.
Mọi binary đều release: `opencanvas-server`, `layout-step` (qua `OPENCANVAS_LAYOUT_BIN` như `npm start`) và `rust-spike`
(LTO, 2 604 384 byte, có rayon). Các phase chạy tuần tự, không có build/test nặng nào khác của phiên này chạy song song.
Tải 1 phút của máy là 3,1–3,9 trong lúc đo (lượt đầu 4,8–6,0), do daemon hệ thống (`ApplicationsStorageExtension`,
WindowServer) và server tạm đang idle của một phiên khác. Lượt này chạy sau lượt đầu (painter cũ) ~2 giờ và chậm hơn nó
2–10% ở cả Chromium lẫn painter cũ, nên so sánh trước/sau của painter Rust (D8) lấy trong cùng một lượt.

**Frame.** Cả 4 frame đều được author qua MCP `create_project` + `apply_batch` thật, trên server tạm
(`--port 0`, data root tạm, xóa sau khi đo):

- `samples/welcome-stacks` ở 1040×720 và 390×844 (stack, wrap theo width).
- `samples/welcome` gốc (toàn `free`) ở 1040×720.
- `grid-2080`: lưới 2×2 bản sao dashboard stacks trong frame 2080×1440, 727 node, nhỏ hơn `MAX_NODES` 2048. Text của
  bản 2–4 được nối thêm " 2"/" 3"/" 4" để mỗi bản phải đo riêng, không dùng chung kết quả đo.

Mỗi frame được đo ở scale 1 và 2. Không frame nào có ảnh, viền hay opacity layer; scene chỉ có fill, bo góc, clip và text.

**Cách đo.**

1. **Chromium mức server** (`js/e2e/server.mjs`): `render_frame` enqueue → `render_frame {jobId, waitMs}` → `GET imageUrl`.
   - *Cold* là job đầu sau khi server khởi động (worker Node, Playwright và Chromium khởi lazy trong job này). Lặp 3 lần,
     mỗi lần restart server, lấy p50.
   - *Warm* là 24 job tiếp theo trong cùng phiên server. Job thứ k đổi viewport thành `(W+k)×H`, nên mỗi job có scene key
     mới. Bỏ 3 job warm-up (k=1–3), thống kê 21 job k=4–24. Cache đo text trong bộ nhớ worker được giữ, đúng như production;
     vì vậy ở 1040 mỗi job chỉ đo lại 1 text có width phụ thuộc viewport, còn ở 390 và ở `welcome` không đo lại gì.
   - API không trả thời lượng job, nên chỉ có thời gian phía client.
2. **Lấy đúng input của server** (`js/e2e/capture.mjs`). Server spawn `node worker/cli.mjs` qua `PATH`, nên một lần chạy
   riêng đặt trước `PATH` một wrapper `node` (trong thư mục scratch) để `tee` NDJSON stdin/stdout của worker. Từ đó lấy
   layout input của mỗi frame, PNG Chrome và scene của job. Lần chạy đo thời gian không dùng wrapper; mã sản phẩm không bị sửa.
3. **Chromium phân rã** (`js/e2e/stage-run.mjs`, mỗi frame × scale một tiến trình Node mới):
   - Phần khởi động chép từng bước của `startRuntime`, cùng option, có timer cho từng bước.
   - Job chạy `PipelineWorker` thật như `runner.rs`: `compute {strict:false}` rồi `render`.
   - Phía Node có timer quanh mỗi `spawnSync(layout-step)` (đếm byte JSON vào/ra) và mỗi round trip `page.evaluate`.
   - Phía trang, `performance.now()` đo `loadResources`, `measureBatch`, `drawScene` và `toDataURL`, qua wrapper đặt trên
     `globalThis.renderer`.
   - Font tải từ một server release idle, cùng route và header như production.
   - NDJSON phía worker đo trong script. NDJSON phía server (serialize request, parse response, decode base64 hai lần như
     `runner.rs` và `record.rs`) đo bằng `rust-spike ndjson` trên các dòng đã capture.
   - *IPC Playwright* = round trip trừ thời gian trong trang. Đây chủ yếu là chi phí serialize scene/request vào trang và
     đưa base64 ra.
4. **Rust in-process** (`src/bench.rs`, `pipeline.rs`, `measure.rs`):
   - Parse layout input, rồi lặp `opencanvas_layout::compute`.
   - Mỗi `Pending.requests` được trả lời bằng HarfRust theo đúng ngữ nghĩa `measureBatch`: fits trên lưới 1/64, các trường
     `Measurement`/`TextLine` như TS, width theo mô hình Chrome. Số f32 qua JSON được đổi sang f64 như JS nhìn thấy.
   - Sau `Ready.scene`, vẽ bằng painter tiny-skia của spike (contrast 1.45 + snap; bản tối ưu bbox + rayon, xem mục
     "Painter tối ưu" bên dưới), rồi encode PNG ở mức `png` mặc định (`Balanced`) và `Fast`.
   - Hai cấu hình: **N thread** (pool rayon mỗi nhân một thread, N = 11) và **1 thread** (`--threads 1`). Painter cũ
     toàn canvas (`--paint full`, 1 thread) được đo lại trong cùng lượt làm số "trước" (D8).
   - *Cold* là cả tiến trình, từ spawn tới khi exit với PNG đã ghi: đọc input, đọc font + SHA-256 + parse, tạo pool,
     layout, đo, vẽ, encode. Lặp 5 lần mỗi cấu hình, xen kẽ N và 1, lấy p50.
   - *Warm* chạy cùng chuỗi viewport với server, cùng ngữ nghĩa cache, trong một tiến trình. Có thêm một lượt mỗi job dùng
     cache trống (D6). CPU mỗi run (user + sys mọi thread) lấy bằng `getrusage`.
5. **Gate đúng đắn** (`js/e2e/gate.mjs`) chạy trước khi so thời gian:
   - So geometry scene Rust với scene Chromium cho cùng input, ở **cả 25 width** của chuỗi warm. So mọi `bounds` và mọi
     dòng (`text`, `x`, `y`, `width`) bằng so khớp chính xác, không cần lượng tử.
   - Kiểm script phân rã ra đúng scene và đúng PNG (trùng từng byte) với server.
   - Pixel diff toàn khung giữa PNG Rust và PNG Chrome.
6. **Gate pixel của painter tối ưu** (`rust-spike paint-gate`, phase `paintgate`, chạy trước phase `rust`; lệch là dừng):
   - (a) painter mới so painter cũ toàn canvas, (b) painter mới ở 1, 2 và 11 thread so nhau, (c) cả hai kiểm trên scene
     400×300 của Phần B (ảnh, viền, clip, nhóm opacity) với hai bộ tùy chọn text.
   - Mỗi frame chạy đủ 25 viewport của chuỗi warm, ở 1× và 2×. So RGBA premultiplied từng byte; (b) so thêm RGBA đã
     demultiply, tức đúng dữ liệu đưa vào nén PNG.
   - Kết quả ở D9.
7. **CPU của đường Chromium** (`engineCpu` trong `js/e2e/common.mjs`): quanh 24 job warm, cộng CPU của worker Node,
   Chromium và các process con của nó. Không gồm `layout-step` (sống quá ngắn để `ps` thấy) và server. Kết quả ở D10.

Tổng các stage giải thích được **96–98%** con số server warm (cột "Giải thích" của D3) và **95–97%** con số
server cold (D5): script phân rã + NDJSON hai phía + MCP enqueue/tải ảnh. Phần còn lại là chờ queue/notify của server.
"MCP" (4,9–14 ms) là `make_input`, SHA-256 scene key, JSON-RPC và tải ảnh. Đường Rust nằm trong server cũng phải trả
phần này, nên cột tỷ lệ ở D2 đã cộng nó vào phía Rust.

**Painter tối ưu.** Lượt đo đầu cho thấy painter spike tốn 73–95% thời gian warm, phần lớn ở ba lượt toàn canvas cho mỗi
text node. Bản hiện tại sửa ba chỗ, output trùng từng byte với bản cũ (D9). Bản cũ vẫn chọn được bằng `--paint full`.

1. **Text theo vùng bao** (`src/text_mask.rs`).
   - Path glyph dựng như cũ. Lấy bounds, làm tròn ra pixel nguyên, thêm lề 1 px, rồi raster vào mask cỡ vùng đó bằng
     cách dịch path một số pixel nguyên.
   - LUT gamma, nhân clip và tô màu chỉ chạy trong vùng. tiny-skia đòi mask cùng cỡ pixmap, nên lúc tô, vùng được chép
     vào một mask cỡ canvas dùng chung, `fill_rect` chỉ trên rect của vùng, rồi xóa vùng đó. Pipeline per-pixel của
     tiny-skia giữ nguyên.
   - Vì sao trùng byte: khi path nằm trong canvas, trừ một số nguyên là phép trừ f32 chính xác. tiny-skia đổi điểm sang
     fixed-point bằng cách cắt phần lẻ, nên edge của đoạn thẳng và của quad đơn điệu theo y cũng dịch đúng một số nguyên.
   - Không chứng minh được cho: quad có cực trị y bên trong (tiny-skia chặt nó bằng phép tính f32 phụ thuộc độ lớn tọa
     độ), cubic (font CFF), path chạm mép canvas, canvas trên 8 191 px. Các ca này raster trên mask cỡ canvas như cũ rồi
     cắt lấy vùng (cột "fallback" ở D9).
   - Với NotoSans trên 4 frame không gặp quad nào như vậy. Fallback chỉ xảy ra ở text tràn mép phải của `stacks-390`.
     `RUST_SPIKE_FORCE_FALLBACK=1` ép mọi node đi đường fallback để kiểm cả đường đó.
2. **Không clone mask clip** (`src/clip.rs`, `src/paint.rs`). Node cha không clip mượn mask của tổ tiên. Node clip tạo mask
   mới rồi nhân mask tổ tiên vào bằng đúng phép nhân của `Mask::intersect_path`.
3. **Song song bằng rayon** (`src/par.rs`, feature `parallel` bật mặc định).
   - Coverage của mọi text node không phụ thuộc pixmap, nên raster song song trước; composite chạy tuần tự đúng thứ tự vẽ.
   - Nhân mask clip và demultiply trước khi nén PNG chạy song song theo mảnh 64 KB. Nén vẫn một luồng trong crate `png`.
   - Tắt feature (`--no-default-features`, cho wasm32 không thread) thì cùng các hàm đó chạy tuần tự.
   - Số thread chọn bằng `--threads N` hoặc `RUST_SPIKE_THREADS`; 0 nghĩa là mỗi nhân một thread.

**D1. Frame**

| Frame | Viewport | Node | Text node | Dòng text | Pass cold | Mục đo cold | PNG Chrome 1×/2× (KB) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| stacks-1040 | 1040×720 | 181 | 41 | 41 | 3 | 75 | 101 / 252 |
| stacks-390 | 390×844 | 181 | 41 | 49 | 3 | 75 | 56 / 134 |
| welcome-1040 | 1040×720 | 71 | 41 | 41 | 2 | 41 | 101 / 252 |
| grid-2080 | 2080×1440 | 727 | 164 | 167 | 3 | 300 | 282 / 736 |

**D2. End-to-end (ms).** Chromium = server release, MCP `render_frame` tới khi có bytes PNG. Rust = prototype, painter bbox, N = 11 thread / 1 thread (cold: cả tiến trình; warm: trong tiến trình). Tỷ lệ = Chromium ÷ (Rust + MCP enqueue + tải ảnh đo ở đường server).

| Frame × scale | Chromium cold | Rust cold N / 1 | MCP | Tỷ lệ cold N / 1 | Chromium warm p50 / p95 | Rust warm p50 N / 1 | Rust warm p95 N / 1 | Rust warm Fast p50 N / 1 | Tỷ lệ warm N / 1 | Tỷ lệ warm Fast N / 1 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| stacks-1040-s1 | 428 | 23.5 / 27.7 | 6.3 | 14.4× / 12.6× | 52.4 / 53.5 | 12.2 / 15.4 | 12.4 / 15.6 | 6.5 / 9.8 | 2.8× / 2.4× | 4.1× / 3.3× |
| stacks-1040-s2 | 438 | 50.1 / 64.5 | 6.5 | 7.7× / 6.2× | 62.1 / 64.1 | 34.5 / 43.6 | 35.6 / 44.0 | 18.3 / 27.1 | 1.5× / 1.2× | 2.5× / 1.8× |
| stacks-390-s1 | 435 | 17.6 / 19.2 | 6.3 | 18.1× / 17.0× | 47.0 / 48.0 | 6.9 / 8.7 | 7.0 / 8.9 | 3.8 / 5.6 | 3.5× / 3.1× | 4.6× / 3.9× |
| stacks-390-s2 | 434 | 30.3 / 36.8 | 6.3 | 11.9× / 10.1× | 51.9 / 53.1 | 17.8 / 22.5 | 18.1 / 23.1 | 9.1 / 13.9 | 2.2× / 1.8× | 3.4× / 2.6× |
| welcome-1040-s1 | 394 | 21.5 / 25.6 | 5.0 | 14.8× / 12.9× | 37.2 / 39.5 | 11.1 / 14.3 | 11.3 / 14.5 | 5.4 / 8.6 | 2.3× / 1.9× | 3.6× / 2.7× |
| welcome-1040-s2 | 405 | 47.6 / 62.3 | 5.0 | 7.7× / 6.0× | 45.9 / 46.6 | 33.1 / 42.7 | 33.8 / 43.1 | 17.0 / 26.1 | 1.2× / ÷1.0 | 2.1× / 1.5× |
| grid-2080-s1 | 577 | 75.3 / 95.8 | 14.6 | 6.4× / 5.2× | 131.9 / 135.2 | 51.5 / 66.8 | 52.6 / 67.7 | 34.0 / 48.9 | 2.0× / 1.6× | 2.7× / 2.1× |
| grid-2080-s2 | 608 | 205.4 / 270.8 | 14.5 | 2.8× / 2.1× | 160.2 / 162.5 | 164.8 / 206.6 | 165.7 / 210.8 | 110.8 / 150.2 | ÷1.1 / ÷1.4 | 1.3× / ÷1.0 |

**D3. Chromium warm, p50 theo stage (ms)** (script stage, cùng input; `resources` = 2 lần `loadResources`).

| Frame × scale | Job | resources | layout-step (pass, KB JSON) | measure RTT (trong trang) | draw RTT | drawScene | toDataURL | IPC Playwright | NDJSON worker + server | MCP enqueue + tải ảnh | Server p50 | Giải thích |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| stacks-1040-s1 | 44.7 | 12.5 | 8.0 (2, 187) | 0.90 (0.30) | 23.0 | 2.80 | 4.90 | 17.0 | 1.57 | 6.3 | 52.4 | 100% |
| stacks-1040-s2 | 52.2 | 12.4 | 7.8 (2, 187) | 0.91 (0.30) | 31.2 | 3.40 | 11.1 | 18.2 | 2.08 | 6.5 | 62.1 | 98% |
| stacks-390-s1 | 38.3 | 12.6 | 4.9 (1, 117) | 0.00 (0.00) | 20.7 | 2.50 | 3.30 | 16.0 | 1.41 | 6.3 | 47.0 | 98% |
| stacks-390-s2 | 42.9 | 12.7 | 4.8 (1, 117) | 0.00 (0.00) | 25.4 | 2.90 | 6.60 | 17.0 | 1.70 | 6.3 | 51.9 | 98% |
| welcome-1040-s1 | 30.7 | 12.6 | 3.3 (1, 44) | 0.00 (0.00) | 14.7 | 2.60 | 4.90 | 8.3 | 0.91 | 5.0 | 37.2 | 98% |
| welcome-1040-s2 | 38.6 | 12.5 | 3.1 (1, 44) | 0.00 (0.00) | 22.9 | 3.30 | 11.1 | 9.6 | 1.45 | 5.0 | 45.9 | 98% |
| grid-2080-s1 | 98.5 | 12.7 | 12.6 (1, 477) | 0.00 (0.00) | 73.2 | 4.30 | 12.3 | 57.9 | 6.41 | 14.6 | 131.9 | 91% |
| grid-2080-s2 | 131.0 | 13.0 | 12.8 (1, 477) | 0.00 (0.00) | 105.1 | 7.30 | 36.4 | 62.3 | 8.36 | 14.5 | 160.2 | 96% |

**D4. Rust warm, p50 theo stage (ms)**, painter bbox. CPU = user + sys của mọi thread trong phần `Tổng` (không gồm PNG Fast).

| Frame × scale | Tổng N / 1 | CPU N / 1 | parse | layout (pass) | đo (mục) | paint N / 1 | text raster N / 1 | text composite | hình | clip mask (số) | PNG mặc định N / 1 | PNG Fast N / 1 | KB mặc định / Fast / Chrome |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| stacks-1040-s1 | 12.2 / 15.4 | 20.5 / 15.4 | 0.13 | 1.39 (2) | 0.01 (1) | 3.1 / 5.0 | 0.5 / 2.5 | 0.2 | 2.0 | 0.1 (1) | 7.6 / 8.9 | 1.9 / 3.2 | 56 / 95 / 101 |
| stacks-1040-s2 | 34.5 / 43.6 | 54.0 / 43.6 | 0.14 | 1.45 (2) | 0.01 (1) | 10.1 / 13.1 | 0.8 / 4.0 | 0.7 | 8.1 | 0.2 (1) | 22.9 / 28.8 | 6.6 / 12.4 | 133 / 243 / 252 |
| stacks-390-s1 | 6.9 / 8.7 | 12.9 / 8.8 | 0.13 | 0.85 (1) | 0.00 (0) | 1.9 / 3.2 | 0.5 / 1.8 | 0.1 | 1.1 | 0.1 (1) | 4.0 / 4.5 | 0.9 / 1.4 | 33 / 54 / 56 |
| stacks-390-s2 | 17.8 / 22.5 | 29.2 / 22.5 | 0.13 | 0.90 (1) | 0.00 (0) | 5.2 / 7.4 | 0.7 / 3.0 | 0.4 | 3.6 | 0.1 (1) | 11.5 / 14.1 | 2.9 / 5.4 | 75 / 136 / 134 |
| welcome-1040-s1 | 11.1 / 14.3 | 19.5 / 14.3 | 0.05 | 0.34 (1) | 0.00 (0) | 3.1 / 4.9 | 0.5 / 2.5 | 0.2 | 2.0 | 0.1 (1) | 7.7 / 9.0 | 1.9 / 3.3 | 56 / 95 / 101 |
| welcome-1040-s2 | 33.1 / 42.7 | 52.4 / 42.7 | 0.06 | 0.39 (1) | 0.00 (0) | 9.9 / 13.1 | 0.8 / 4.1 | 0.7 | 7.9 | 0.2 (1) | 22.7 / 29.2 | 6.6 / 12.5 | 133 / 242 / 252 |
| grid-2080-s1 | 51.5 / 66.8 | 84.0 / 66.8 | 0.48 | 3.73 (1) | 0.00 (0) | 22.7 / 31.8 | 1.7 / 10.7 | 0.9 | 17.6 | 1.5 (5) | 24.5 / 30.7 | 6.9 / 12.9 | 148 / 403 / 282 |
| grid-2080-s2 | 164.8 / 206.6 | 248.3 / 206.4 | 0.51 | 3.72 (1) | 0.00 (0) | 80.7 / 96.2 | 2.6 / 17.1 | 3.1 | 68.5 | 5.0 (5) | 79.8 / 105.8 | 25.9 / 49.8 | 355 / 1 022 / 736 |

**D5. Cold (ms).** Chromium: tiến trình Node mới (script stage); Rust: cả tiến trình, painter bbox.

| Frame × scale | Node boot | import playwright | launch | newPage + goto + import renderer | Job đầu | Cộng + MCP | Server cold | Rust: khởi (font đọc/SHA/parse/pool) | Rust: run đầu N / 1 | Rust: wall N / 1 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| stacks-1040-s1 | 24.1 | 162.0 | 86.3 | 48.5 | 87.9 | 420.3 | 428 | 4.8 (0.23/4.24/0.03/0.11) | 13.8 / 17.0 | 23.5 / 27.7 |
| stacks-1040-s2 | 23.7 | 160.5 | 87.5 | 46.1 | 95.9 | 424.1 | 438 | 5.0 (0.23/4.42/0.03/0.10) | 35.5 / 44.6 | 50.1 / 64.5 |
| stacks-390-s1 | 22.4 | 161.2 | 83.9 | 47.5 | 85.8 | 411.7 | 435 | 4.8 (0.23/4.25/0.03/0.11) | 8.8 / 10.5 | 17.6 / 19.2 |
| stacks-390-s2 | 23.6 | 162.2 | 90.4 | 47.5 | 89.5 | 423.2 | 434 | 4.8 (0.22/4.29/0.03/0.10) | 19.5 / 23.9 | 30.3 / 36.8 |
| welcome-1040-s1 | 23.2 | 162.0 | 92.3 | 48.0 | 53.6 | 388.3 | 394 | 4.7 (0.22/4.27/0.03/0.11) | 11.7 / 15.1 | 21.5 / 25.6 |
| welcome-1040-s2 | 24.6 | 161.1 | 81.7 | 47.7 | 60.7 | 384.6 | 405 | 4.8 (0.22/4.27/0.03/0.12) | 33.3 / 42.6 | 47.6 / 62.3 |
| grid-2080-s1 | 22.7 | 172.1 | 80.3 | 46.9 | 207.5 | 549.1 | 577 | 5.3 (0.22/4.29/0.03/0.11) | 59.1 / 74.3 | 75.3 / 95.8 |
| grid-2080-s2 | 23.8 | 161.4 | 93.3 | 47.7 | 242.0 | 587.2 | 608 | 5.2 (0.23/4.24/0.03/0.11) | 169.2 / 211.1 | 205.4 / 270.8 |

**D6. Layout + đo khi cache đo trống (warm process, p50, ms)** — phần Task 10 thay.

| Frame × scale | Mục đo | Chromium: layout-step spawn | Chromium: measure RTT | Chromium: cộng | Rust: layout | Rust: đo | Rust: cộng |
| --- | --- | --- | --- | --- | --- | --- | --- |
| stacks-1040 | 75 | 10.8 | 17.1 | 28.0 | 1.93 | 0.71 | 2.64 |
| stacks-390 | 75 | 11.5 | 17.2 | 28.7 | 1.88 | 0.70 | 2.57 |
| welcome-1040 | 41 | 5.7 | 5.6 | 11.4 | 0.52 | 0.25 | 0.77 |
| grid-2080 | 300 | 27.0 | 58.4 | 85.3 | 8.54 | 2.91 | 11.5 |

**D7. Gate hình học và pixel** (Rust so Chromium, cùng input; pixel: Δ là chênh lớn nhất giữa kênh RGBA).

| Frame | Viewport so | Node so / lệch | Dòng so / lệch | Script = server | 1× Δ>0 / Δ>32 / max / mean | 2× Δ>0 / Δ>32 / max / mean |
| --- | --- | --- | --- | --- | --- | --- |
| stacks-1040 | 25 | 4 525 / 0 | 1 025 / 0 | trùng, PNG trùng byte | 20 345 / 3 337 / 184 / 0.52 (2.7% px) | 46 455 / 11 683 / 195 / 0.37 (1.6% px) |
| stacks-390 | 25 | 4 525 / 0 | 1 225 / 0 | trùng, PNG trùng byte | 11 440 / 2 091 / 184 / 0.71 (3.5% px) | 26 156 / 6 742 / 186 / 0.48 (2.0% px) |
| welcome-1040 | 25 | 1 775 / 0 | 1 025 / 0 | trùng, PNG trùng byte | 20 345 / 3 337 / 184 / 0.52 (2.7% px) | 46 455 / 11 683 / 195 / 0.37 (1.6% px) |
| grid-2080 | 25 | 18 175 / 0 | 4 175 / 0 | trùng, PNG trùng byte | 87 296 / 14 625 / 184 / 0.57 (2.9% px) | 199 242 / 50 904 / 195 / 0.40 (1.7% px) |

**D8. Painter trước/sau, đo thực (warm p50, ms).** Cũ = painter toàn canvas (`--paint full`, 1 thread), đo lại cùng lượt; ước lượng cũ = công thức D8 trước đây áp lên lượt đo đó.

| Frame × scale | Paint cũ | Paint ước lượng cũ | Paint mới 1 / N | Cũ ÷ mới 1 / N | Rust warm Fast: ước lượng cũ / đo N | Rust warm cũ / mới N | Chromium warm p50 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| stacks-1040-s1 | 28.0 | 3.2 | 5.0 / 3.1 | 6× / 9× | 8.0 / 6.5 | 38.5 / 12.2 | 52.4 |
| stacks-1040-s2 | 115.2 | 14.5 | 13.1 / 10.1 | 9× / 11× | 28.6 / 18.3 | 145.7 / 34.5 | 62.1 |
| stacks-390-s1 | 13.3 | 1.7 | 3.2 / 1.9 | 4× / 7× | 4.1 / 3.8 | 18.8 / 6.9 | 47.0 |
| stacks-390-s2 | 49.1 | 5.6 | 7.4 / 5.2 | 7× / 10× | 12.1 / 9.1 | 64.6 / 17.8 | 51.9 |
| welcome-1040-s1 | 26.4 | 2.8 | 4.9 / 3.1 | 5× / 9× | 6.5 / 5.4 | 35.8 / 11.1 | 37.2 |
| welcome-1040-s2 | 102.9 | 10.1 | 13.1 / 9.9 | 8× / 10× | 22.9 / 17.0 | 132.3 / 33.1 | 45.9 |
| grid-2080-s1 | 436.6 | 42.2 | 31.8 / 22.7 | 14× / 19× | 59.6 / 34.0 | 472.1 / 51.5 | 131.9 |
| grid-2080-s2 | 1654.4 | 109.6 | 96.2 / 80.7 | 17× / 21× | 163.5 / 110.8 | 1764.4 / 164.8 | 160.2 |

So với D8 cũ (ước lượng từ lượt đầu, đã công bố; cùng thứ tự hàng): paint ước lượng 2,9 / 11,3 / 1,6 / 4,8 / 2,6 / 9,2 /
29,7 / 92,7 ms, đo thực 3,1 / 10,1 / 1,9 / 5,2 / 3,1 / 9,9 / 22,7 / 80,7 ms ở N thread và 5,0 / 13,1 / 3,2 / 7,4 / 4,9 /
13,1 / 31,8 / 96,2 ms ở 1 thread. Rust warm Fast ước lượng 6,2 / 19,7 / 3,3 / 8,6 / 4,8 / 16,4 / 41,1 / 124,4 ms, đo thực
6,5 / 18,3 / 3,8 / 9,1 / 5,4 / 17,0 / 34,0 / 110,8 ms ở N thread. Ở 1 thread, paint frame cỡ welcome cao hơn ước lượng
16–100%: ước lượng coi cả chi phí coverage tỷ lệ với diện tích, nhưng phần quét cạnh glyph (~2–4 ms) không giảm theo
diện tích. Ở grid, 1 thread khớp ước lượng trong 7%.

**D9. Gate pixel của painter mới** (`rust-spike paint-gate`, so từng byte RGBA premultiplied; (b) so cả RGBA đã demultiply đưa vào PNG; thread 1, 2, 11).

| Ca | Lượt | (a) bbox = full | (b) bằng nhau mọi số thread | Text node | Mask fallback toàn canvas |
| --- | --- | --- | --- | --- | --- |
| stacks-1040 s1 | 25 | 25/25 | 25/25 | 41 | 0 |
| stacks-1040 s2 | 25 | 25/25 | 25/25 | 41 | 0 |
| stacks-390 s1 | 25 | 25/25 | 25/25 | 41 | 183 |
| stacks-390 s2 | 25 | 25/25 | 25/25 | 41 | 182 |
| welcome-1040 s1 | 25 | 25/25 | 25/25 | 41 | 0 |
| welcome-1040 s2 | 25 | 25/25 | 25/25 | 41 | 0 |
| grid-2080 s1 | 25 | 25/25 | 25/25 | 164 | 0 |
| grid-2080 s2 | 25 | 25/25 | 25/25 | 164 | 0 |
| scene gamma 1 snap false s1 | 1 | 1/1 | 1/1 | 3 | 0 |
| scene gamma 1 snap false s2 | 1 | 1/1 | 1/1 | 3 | 0 |
| scene gamma 1.45 snap true s1 | 1 | 1/1 | 1/1 | 3 | 0 |
| scene gamma 1.45 snap true s2 | 1 | 1/1 | 1/1 | 3 | 0 |

Kiểm thêm ngoài `run.mjs`, cùng binary:

- PNG mốc do binary cũ ghi (viewport k = 0, 4 frame × 2 scale): RGBA trùng, bytes PNG cũng trùng.
- `RUST_SPIKE_FORCE_FALLBACK=1`, 3 viewport mỗi frame: trùng (mỗi ca 78–492 node đi đường fallback).
- Bản `--no-default-features` (không rayon): trùng; thời gian bằng bản rayon chạy 1 thread (lệch < 1%).
- Scene Phần B render bằng binary cũ, `--paint full` và `--paint bbox`, hai bộ tùy chọn text: PNG trùng byte.

**D10. CPU mỗi job warm (ms).** Chromium: CPU của worker Node + mọi process Chromium (không gồm `layout-step` và server) chia cho 24 job; Rust: getrusage quanh phần `Tổng`.

| Frame × scale | Chromium wall p50 | Chromium CPU/job | Chromium process / thread | Rust N wall / CPU | Rust 1 wall / CPU |
| --- | --- | --- | --- | --- | --- |
| stacks-1040-s1 | 52.4 | 55.0 | 5 / 110 | 12.2 / 20.5 | 15.4 / 15.4 |
| stacks-1040-s2 | 62.1 | 66.3 | 5 / 109 | 34.5 / 54.0 | 43.6 / 43.6 |
| stacks-390-s1 | 47.0 | 51.3 | 5 / 108 | 6.9 / 12.9 | 8.7 / 8.8 |
| stacks-390-s2 | 51.9 | 56.3 | 5 / 109 | 17.8 / 29.2 | 22.5 / 22.5 |
| welcome-1040-s1 | 37.2 | 44.2 | 5 / 108 | 11.1 / 19.5 | 14.3 / 14.3 |
| welcome-1040-s2 | 45.9 | 53.3 | 5 / 109 | 33.1 / 52.4 | 42.7 / 42.7 |
| grid-2080-s1 | 131.9 | 132.1 | 5 / 109 | 51.5 / 84.0 | 66.8 / 66.8 |
| grid-2080-s2 | 160.2 | 162.9 | 5 / 108 | 164.8 / 248.3 | 206.6 / 206.4 |

### Đọc kết quả

- **Đúng đắn trước, thời gian sau.**
  - Geometry Rust trùng Chromium tuyệt đối ở 4 frame × 25 width: 29 000 lượt so node và 7 450 lượt so dòng, **0 lệch**,
    không cần lượng tử. Script phân rã ra PNG trùng từng byte với server.
  - Painter tối ưu ra pixel trùng từng byte với painter cũ ở mọi ca, với mọi số thread (D9). Vì vậy pixel Rust so Chrome
    (D7) giữ nguyên: lệch ở 1,6–3,5% số pixel, mean Δ 0,37–0,71, toàn bộ ở mép glyph như Phần B.
- **Painter: nhanh hơn 4–17× (1 thread) và 7–21× (N thread) so bản cũ (D8).** Paint warm giảm từ 13–1 654 ms xuống
  1,9–81 ms ở N thread (3,2–96 ms ở 1 thread). Text giờ chỉ tốn ≤ 6 ms mỗi job ở N thread, gồm raster song song và
  composite.
- **Cold: Rust nhanh hơn rõ ở mọi frame.** Đã cộng MCP vào phía Rust:
  - N thread: 6,4–18× nhanh hơn ở 1×, 2,8–12× ở 2×. 1 thread: 5,2–17× và 2,1–10×.
  - Lượt đầu (painter cũ): grid 2× còn chậm hơn 3×.
  - Phía Chromium, khởi worker chiếm ~300 ms: Node 22–25 ms, `import("playwright")` 160–172 ms, launch 80–93 ms,
    page + `goto` + import renderer 46–48 ms.
  - Phía Rust, khởi mất 4,7–5,3 ms, trong đó 4,2–4,4 ms là SHA-256 của font 2 MB; tạo pool 11 thread 0,1 ms.
- **Warm: Rust nhanh hơn ở 7/8 cấu hình** (PNG `Balanced`, N thread):
  - 1×: 2,0–3,5× nhanh hơn. 2×: 1,2–2,2× ở frame cỡ welcome; grid 2× chậm hơn 1,1×.
  - Với PNG `Fast`: 2,7–4,6× ở 1×, 1,3–3,4× ở 2×, kể cả grid.
  - 1 thread: 1,6–3,1× ở 1×; ở 2× từ hòa (`welcome`) tới 1,8×; grid 2× chậm hơn 1,4× (hòa nếu dùng `Fast`).
  - Lượt đầu, painter cũ: frame cỡ welcome ở 1× từ chậm hơn 1,1× tới nhanh hơn 1,9×; 2× và grid chậm hơn 1,3–11×.
- **Stage chi phối bây giờ (D4, N thread).**
  - Nén PNG `Balanced` chiếm 48–69% thời gian warm. Nén một luồng trong crate `png`; phần demultiply đã song song.
  - Paint chiếm 25–49%. Trong paint, 56–85% là tô hình (`fill_path` của fill node qua mask clip của frame), tăng theo diện
    tích được tô: 1,1–68,5 ms.
  - Mask clip ≤ 5 ms (1–5 mask). Parse + layout + đo ≤ 4,3 ms khi cache đo ấm.
  - Với PNG `Fast`, paint chiếm 47–73% và thành stage lớn nhất.
- **Song song: N thread chỉ nhanh hơn 1 thread 1,25–1,30×.**
  - Hai stage lớn nhất (nén PNG, tô hình) vẫn tuần tự.
  - Phần đã song song thì được lợi rõ: raster text nhanh hơn 3,6–6,6× (2,5 → 0,5 ms; 17,1 → 2,6 ms). Demultiply song song bớt
    0,5–26 ms ở bước encode.
  - Đổi lại, CPU tăng 1,2–1,5× (D10).
- **CPU, cho công bằng (D10).**
  - Chromium headless cũng chạy nhiều tiến trình và thread: worker Node + 4 process Chromium (browser, GPU, network,
    renderer), 108–110 thread.
  - Dù vậy, CPU mỗi job chỉ bằng 1,0–1,2× wall: pipeline chủ yếu tuần tự qua IPC. Số này chưa gồm `layout-step` và server.
  - Rust 1 thread dùng CPU bằng wall và ít hơn Chromium 1,25–5,9×. Riêng grid 2× dùng nhiều hơn 27% (206 so 163 ms).
- **Chromium warm vẫn do IPC và resource chi phối, không do vẽ.**
  - Round trip `render` chiếm 48–80% job: Playwright IPC 8–62 ms, `toDataURL` 3–36 ms, `drawScene` chỉ 2,5–7,3 ms.
  - Hai lần `loadResources` mỗi job tốn ~12,5 ms; spawn `layout-step` 3–13 ms/job.
- **Layout + đo khi cache trống (D6):** Chromium 11–85 ms, Rust 0,8–11,5 ms, tức nhanh hơn 7–15×.
- **PNG.** Mức `Balanced` cho file nhỏ hơn Chrome 41–52% nhưng encode tốn 4–80 ms. Mức `Fast` nhanh hơn 3–4× và cho cỡ file
  gần bằng Chrome; riêng grid lớn hơn Chrome 39–43%.

### Ý nghĩa cho Task 10 và Task 11

- **Task 10 (đo native, Chromium vẫn vẽ).** Mỗi job bỏ được spawn `layout-step`, round trip `measureBatch` và lần
  `loadResources` của compute.
  - Khi cache đo đã ấm: tiết kiệm ~9–15 ms/job (20–29% thời gian warm) ở frame cỡ welcome, ~19 ms (12–14%) ở grid.
  - Khi cache trượt (text mới, width mới, worker vừa restart): tiết kiệm thêm 11–85 ms (D6).
  - **Cold gần như không đổi** vì bước `render` vẫn phải khởi Chromium (~300 ms).
  - Gate ở trên là bằng chứng sớm cho AC parity của Task 10, nhưng mới với một font và chỉ mac.
- **Task 11 (painter Rust, bỏ Chromium).**
  - Lợi ích cold chắc chắn: bỏ ~300 ms khởi Chromium cùng IPC/resource/NDJSON mỗi job (~20–80 ms).
  - Warm giờ cũng thắng ở 7/8 cấu hình với 11 thread, 6/8 với 1 thread, nhờ mask text theo vùng bao và mượn mask clip
    thay vì clone. Cả hai đã chứng minh trùng byte, nên Task 11 nên làm như vậy ngay từ đầu.
  - `crates/render` không dùng thread, nên số **1 thread** là số cần nhắm. Song song (nếu cần) đặt ở server: nhiều job
    cùng lúc, hoặc nén PNG.
  - Việc còn lại cho warm, theo thứ tự lợi ích:
    1. **PNG.** Dùng `Fast` (bớt 3–56 ms/job ở 1 thread), hoặc nén song song theo dải ở server. Ước lượng, chưa đo: nén
       `Balanced` một luồng tốn 4–78 ms; chia đều cho 5 nhân hiệu năng còn ~1–20 ms, tức bớt 3–60 ms/job (grid 2×,
       N thread: 165 → ~105 ms). Crate kiểu `mtpng` làm việc này; output vẫn là PNG hợp lệ nhưng bytes khác hiện tại.
    2. **Tô hình.** Mọi fill trong frame clip đều đi qua mask clip toàn canvas, nên tiny-skia không dùng được đường memset
       cho rect đục. Bỏ mask khi node nằm trọn trong vùng trong của clip; phải kiểm trùng byte. Chưa đo.
    3. **Layer opacity theo bounds**, như `layers.ts`. Frame đo không có layer nên chưa có số.
  - Text theo vùng bao: phép dịch path chỉ chứng minh được trùng byte với path không có quad cực trị y bên trong. Painter
    sản phẩm không cần trùng painter spike, nhưng nên giữ cùng ý: raster trong bounds và fallback có kiểm.
- **Phương án A (giữ Chromium) để đối chứng:** cache resource theo hash (−12 ms/job) và giữ layout process
  sống lâu (−3–13 ms) là các tối ưu rẻ. Phương án này không đụng được ~300 ms cold và IPC Playwright.

### Giới hạn của Phần D

- Painter đã tối ưu vẫn là painter spike: chưa có layer opacity theo bounds, viền nhiều màu, ảnh JPEG/WebP, ICC. Frame đo
  không có ảnh, viền, opacity layer hay font upload; grid là frame tổng hợp. Scene Phần B chỉ dùng cho gate pixel.
- Song song hóa dùng pool rayon trong một tiến trình chỉ chạy một job. Trong server có nhiều job cùng lúc, các job tranh
  nhau nhân, nên số 1 thread là cận dưới thực tế hơn khi tải cao. Máy có 5 nhân hiệu năng + 6 nhân tiết kiệm; N = 11.
- Đường Rust bỏ qua phần bookkeeping server: readiness/diagnostics, `states::apply` (no-op với state `default`),
  provenance, lưu PNG vào bảng job. Các phần này nhỏ nhưng chưa đo.
- Rust cold là một tiến trình CLI, đã gồm exec, nạp font và tạo pool. Trong server, font và pool tạo một lần nên cold ≈
  "run đầu" ở D5.
- CPU phía Chromium (D10) không gồm `layout-step` và server; CPU phía Rust gồm mọi thread của tiến trình.
- Chỉ đo một máy mac arm64, có tải nền. Server: 3 lần cold, 21 job warm. Rust: 5 tiến trình cold mỗi cấu hình, 21 run warm.
- Timer trong trang là wrapper `globalThis.renderer`, overhead < 0,1 ms mỗi lần gọi.

### Chạy lại Phần D

Từ gốc repo hoặc worktree; mất ~5 phút (phase `paintgate` ~1,3 phút, phase `rust` ~2 phút vì đo thêm painter cũ).
`.runtime/browsers` phải có `chromium_headless_shell-1248`. Trong worktree, đừng symlink cả thư mục `.runtime`:
`.gitignore` chỉ bỏ qua `.runtime/` khi nó là thư mục. Dùng `mkdir .runtime && ln -s <checkout>/.runtime/browsers .runtime/browsers`.

```bash
npm ci
npm run build:renderer
cargo build --release -p opencanvas-layout -p opencanvas-server
cargo build --release --manifest-path spikes/rust-export/Cargo.toml
BENCH_DIR=/tmp/opencanvas-e2e-bench node spikes/rust-export/js/e2e/run.mjs
```

- `run.mjs` chạy lần lượt các phase `author capture server stages paintgate rust gate report cleanup`. Có thể chạy riêng
  từng phase khi data root và capture còn trong `BENCH_DIR`, ví dụ `run.mjs paintgate rust gate report`.
- Sau mỗi phase, script kiểm không còn server/worker/Chromium nào của mình (`ps`). `cleanup` xóa data root đã author.
- Kết quả gộp ghi vào `data/e2e/results.json`; bảng D1–D10 in ra stdout.
- JSON từng phase, PNG Chrome/Rust/script và input đã capture nằm dưới `$BENCH_DIR` (không commit).
- Chạy một frame riêng:
  `rust-spike bench $BENCH_DIR/inputs/stacks-1040.json --scale 2 --iters 25 --json out.json`
  (thêm `--threads N`, `--paint full`, `--nocache`, `--png`, `--geometry`).

Gate pixel riêng, không cần server hay Chromium (input là layout input đã capture; ~1,3 phút):

```bash
rust-spike paint-gate $BENCH_DIR/inputs/{stacks-1040,stacks-390,welcome-1040,grid-2080}.json \
  --scene spikes/rust-export/data/scene.json --image spikes/rust-export/data/asset.png [--baseline DIR] [--json out.json]
RUST_SPIKE_FORCE_FALLBACK=1 rust-spike paint-gate ... --iters 3   # ép đường fallback toàn canvas
cargo build --release --no-default-features --manifest-path spikes/rust-export/Cargo.toml --target-dir spikes/rust-export/target/seq
spikes/rust-export/target/seq/release/rust-spike paint-gate ...   # bản không rayon
```

`--baseline DIR` so thêm PNG `<frame>-s<scale>.png` (viewport k = 0) do binary cũ ghi. Lệnh trả mã khác 0 nếu lệch.

Module trong `js/e2e/`:

| File | Việc |
| --- | --- |
| `common.mjs` | Đường dẫn, server tạm, kiểm process sót, CPU của worker + Chromium, đo `render_frame`, percentile |
| `frames.mjs` | Định nghĩa frame và author qua MCP |
| `capture.mjs` | Wrapper tee NDJSON |
| `server.mjs` | Đo mức server |
| `stages.mjs` + `stage-run.mjs` | Phân rã stage Chromium |
| `paint-gate.mjs` | Gate pixel painter mới (`rust-spike paint-gate`) |
| `rust.mjs` | Chạy `rust-spike bench`/`ndjson` (N thread, 1 thread, painter cũ) |
| `gate.mjs` + `geometry.mjs` | Gate geometry và pixel |
| `report.mjs` + `tables.mjs` | Gộp kết quả và in bảng |

Module painter trong `src/`:

| File | Việc |
| --- | --- |
| `paint.rs` | Duyệt cây, fill, ảnh, viền, layer opacity; chọn đường text theo `--paint` |
| `text_mask.rs` | Text theo vùng bao: raster song song, composite tuần tự |
| `text_full.rs` | Text toàn canvas (painter cũ, mốc so của gate) |
| `clip.rs` | Mask clip của node clip và ảnh |
| `par.rs` | Pool rayon và helper song song, bản tuần tự khi tắt feature |
| `shape.rs`, `mode.rs`, `profile.rs` | `roundRect`/paint chung, `--paint`, số liệu profile |
| `gate.rs` | `paint-gate` |

## Giới hạn (Phần A–C)

- Mới đo trên macOS. Docker Linux dùng FreeType cho glyph và mặc định hinting (width nguyên, note #13 trong P10a): phải chạy lại phía Chrome trong image.
- Một font, một instance, LTR Latin/tiếng Việt. Chưa có fallback font, bold/italic, RTL/complex script, emoji, letterSpacing.
- Mô hình width kiểu Chrome là suy ngược cho build Chromium này; cần test parity trong CI nếu dùng.
- Port wrap tách grapheme bằng `unicode-segmentation` thay `Intl.Segmenter("vi")`; whitespace xấp xỉ `\s` của JS. Trùng trên corpus này.
- Painter chỉ có viền một màu (chưa port wedge màu từng cạnh), cubic thay conic của Skia, layer opacity full-canvas.
