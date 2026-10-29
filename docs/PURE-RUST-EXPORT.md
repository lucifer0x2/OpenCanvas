# Xuất ảnh thuần Rust — đánh giá khả thi và spike đo

Ngày: **2026-10-29**. Checkout `codex/p24-components-instances` @ `535d7e1`.
Bổ sung cho nghiên cứu rộng ở [EXPORT-ENGINE-RESEARCH.md](EXPORT-ENGINE-RESEARCH.md)
(phiên khác, gồm cả C++/Python/CDP). Tài liệu này chỉ trả lời một câu:
**bỏ hẳn Node + Chromium khỏi đường xuất ảnh, thay bằng Rust thuần, có giữ được chất lượng không?**
Chưa thay engine, chưa sửa mã sản phẩm. Số đo và cách chạy lại ở [spikes/rust-export](../spikes/rust-export/README.md).

**Trạng thái 2026-10-29: gác lại.** Người dùng chốt giữ nguyên đường Node + Chromium (phương án A). Lộ trình Task 09–13
và các tối ưu đường Chromium đề xuất sau benchmark (xem `docs/agents/LOG.md`) đều chưa làm; tài liệu và spike giữ để
tham khảo khi xem lại.

## Kết luận

**Có, khả thi, nhưng phải tách hai mức chất lượng.** Đo trên macOS:

1. **Hình học: đạt giống Chrome tuyệt đối.** HarfRust đo 1 232 cặp chuỗi×cỡ, lệch tối đa 0.000061 px so `measureText`,
   0 ca lệch sau làm tròn 1/64; wrap 120/120 ca trùng cả text dòng, width và baseline. Điều kiện: mô phỏng cách Chrome
   cắt advance từng glyph về lưới 1/65536 px. Không mô phỏng thì 11/1 232 width lệch 1/64 px, đủ lật một lần xuống dòng.
2. **Pixel: gần, không bao giờ trùng.** Nền, viền, clip, opacity: mean Δ 0.16/255, Δ>8 chỉ ở mép antialias.
   Ảnh: mean Δ ~2. Text lệch ở mép glyph vì Chrome raster glyph bằng engine font của OS (CoreText trên mac,
   FreeType trên Linux). Bằng mắt không phân biệt được ([ảnh 2×](../spikes/rust-export/out/side-snap-g1.45-s2.png)).

Không engine nào ngoài chính Chrome cho được pixel giống Chrome; ngay Chrome mac và Chrome Linux trong Docker cũng khác nhau
(hinting làm width thành số nguyên, note #13 ở P10a). Output Rust thì như nhau trên mọi máy: đó là điểm cộng về chất lượng,
không phải điểm trừ.

**Phạm vi đã chốt 2026-10-29: chỉ macOS (chạy native).** Linux/Docker để sau; spike đã đo đúng môi trường này.
Còn mở: mới một font LTR. Benchmark end-to-end (mục tốc độ bên dưới), sau khi tối ưu painter spike: Rust nhanh hơn ở cold
mọi cấu hình và ở warm 7/8 cấu hình. Riêng grid 2080 ở 2× warm còn chậm hơn ~1,1× (1,4× nếu 1 thread), do nén PNG và tô
hình; layer opacity, ảnh và viền chưa có trong frame đo.

## Phương án (phạm vi macOS)

| | Phương án | Bỏ khỏi runtime | Chất lượng trên mac | Kết luận |
| --- | --- | --- | --- | --- |
| A | Giữ Node + Chromium, chỉ tối ưu (cache resource, layout process sống lâu) | Không | Pixel trùng viewer | Giữ làm đối chứng và rollback |
| B | Rust điều khiển Chromium qua CDP | Node | Pixel trùng viewer | Loại: không thuần Rust, vẫn ôm Chromium |
| **C** | **Rust thuần: HarfRust + skrifa + tiny-skia** | **Node + Chromium** | Hình học trùng (đã đo); hình gần trùng; text gần (CoreText) | **Chọn** |
| C+ | C + raster glyph diện tích chính xác (kiểu PhotoCraft) | Như C | Có thể giảm lệch text, chưa đo | Thử trong bước 2, chỉ giữ nếu số đo tốt hơn |
| D | Skia C++ (skia-safe), Cairo/Pango, Python | — | — | Loại: không thuần Rust |
| E | vello_cpu thay tiny-skia | Như C | Chưa đo | Dự phòng nếu tiny-skia thiếu tính năng (P09) |

Ba quyết định đi kèm C:

1. **Tiêu chí chất lượng:** hình học (width, line break, baseline, bounds) trùng tuyệt đối với Chrome mac; pixel theo ngưỡng,
   text không trùng pixel vì Chrome mac vẽ glyph bằng CoreText.
2. **Glyph thiếu trong font:** diagnostic + fallback khai báo trong manifest, không mượn font hệ thống như Chrome.
   Text có ký tự như "⌘" (không có trong NotoSans) sẽ đổi hình.
3. **Thứ tự:** chứng minh parity với semantics hiện tại (`web-2`) trước, P10a sau như một lần bump version riêng.

## "Chất lượng" ở OpenCanvas nghĩa là gì

Chuẩn của dự án là **khớp Chrome**, không phải "đẹp" chung chung:

- P10a lấy DOM Chrome làm chuẩn cho width/baseline text (`docs/recommend-features/tasks/P10a-text-measure-fidelity.md`);
  agent port UI từ DOM rồi so lại bằng mắt hoặc P13a `compare`.
- Viewer vẽ bằng **cùng** `paintScene` Canvas2D trong trình duyệt người dùng. Đổi engine export là có hai engine.

| Mức | Nội dung | Yêu cầu đề xuất |
| --- | --- | --- |
| Hình học (bắt buộc) | width text → line breaks → bounds Taffy → vị trí mọi node | Giống tuyệt đối với Chrome trên corpus (sau lượng tử 1/64). Lệch ở đây là lệch layout. |
| Pixel (ngưỡng) | antialias glyph, mép path, lấy mẫu ảnh | Không đòi 0 px; thống nhất ngưỡng và lưu ảnh diff cho người duyệt. |

## Renderer hiện tại nhỏ hơn tưởng

Toàn bộ `renderer/*.ts` là ~1 070 dòng. Thứ Canvas2D thật sự vẽ (`paint.ts`, `stroke.ts`, `layers.ts`):

| Tính năng | Canvas2D đang dùng | Rust thuần tương ứng (spike) |
| --- | --- | --- |
| Fill nền | `roundRect` + màu đặc | tiny-skia `PathBuilder` (cubic) + `fill_path` |
| Viền từng cạnh | ring `evenodd` hai `roundRect`, wedge clip khi màu khác nhau | `FillRule::EvenOdd` + `Mask` (spike mới port viền một màu) |
| Clip con | `roundRect` + `clip()` có AA | `Mask` anti-aliased |
| Group opacity | canvas tạm + `globalAlpha` | `Pixmap` tạm + `draw_pixmap` với opacity |
| Ảnh | `drawImage` crop/fit, smoothing mặc định | decode + pattern bilinear trong clip |
| Text | `fillText` từng dòng, baseline alphabetic | HarfRust shape + skrifa outline → path |
| Đo/wrap text | `measureText` + wrap tự viết | HarfRust advance + port nguyên thuật toán (64 dòng Rust) |

Scene hiện không có gradient, shadow, blur, blend mode hay transform xoay. Spike phủ gần hết bảng trên trong 647 dòng Rust.

## Stack đề xuất

| Việc | Lựa chọn thuần Rust | Ghi chú |
| --- | --- | --- |
| Shaping, đo | `harfrust` 0.15 (port HarfBuzz, khớp 15.0.0) | Thêm mô hình làm tròn advance kiểu Chrome; RustyBuzz đã archived. |
| Font tables, outline, variable font | `skrifa`/`read-fonts` (fontations) | Ascent/descent cho baseline P10a, trục `wght` cho P12b, cmap cho P11a. |
| Wrap | Port `measurement.ts` + `unicode-segmentation` | Giữ nguyên semantics hiện tại. `parley` 0.11 dùng UAX#14, chỉ xét khi chủ động đổi semantics (P10b). |
| Raster | `tiny-skia` 0.12 | Bản 0.12 thêm sweep và two-point conical gradient, đủ cho P09a. Blur/shadow (P09b/c) phải tự viết. `vello_cpu` (0.0.x) là đối thủ dự phòng. |
| Ảnh vào | `png`, decoder JPEG thuần Rust, `image-webp` (decode lossy + lossless) | Cần quản lý màu ICC (`moxcms` hoặc `qcms`): ảnh chụp màn hình mac thường mang Display P3. |
| Font vào | TTF/OTF đọc trực tiếp; WOFF2 giải nén một lần lúc import | Phải chọn và kiểm một decoder WOFF2 thuần Rust. |
| Ảnh ra | `png` (tuỳ chọn `oxipng`); JPEG encoder thuần Rust | WebP lossy (P03d): `image-webp` chỉ encode lossless; `zenwebp` có lossy thuần Rust nhưng còn non. |
| PDF/SVG | `krilla` cho PDF vector + subset font; SVG outline text bằng skrifa | Sửa luôn `SVG_TEXT_NOT_OUTLINED` hiện tại. |

Tiền lệ cùng dạng bài toán: **Takumi** render JSX/HTML thành ảnh không cần browser bằng taffy + parley + resvg
(OpenCanvas cũng dùng Taffy); `typst-render` raster bằng tiny-skia.

## Lợi ích kiến trúc ngoài việc bỏ Chromium

- **Đo text đồng bộ trong Taffy.** Closure measure gọi thẳng shaper, bỏ vòng tối đa 6 pass, `spawnSync(layout-step)`
  mỗi pass, trạng thái "pending" và round-trip `measureBatch` (`worker/pipeline.mjs`, `crates/layout/src/engine.rs`).
- **Bỏ một nửa hệ thống chạy.** Không còn worker Node, NDJSON base64 có trần 48 MiB, Playwright, Chromium và OS libs
  trong image (hiện 1,29 GB). Binary spike 2,1 MB (chưa gồm font). Chưa đo image sau khi gỡ.
- **Tất định giữa máy.** Cùng font bytes cho cùng pixel trên mac/Linux; cần kiểm thêm x86_64 so arm64 (SIMD).
- **Thấy được glyph thiếu.** Không có fallback ngầm sang font hệ thống; P11a `MISSING_GLYPH` thành một lần tra cmap.
- **Song song an toàn.** Render trên thread Rust, không nhân bộ nhớ browser theo worker.

**Tốc độ end-to-end đã đo** ([Phần D](../spikes/rust-export/README.md#phần-d--benchmark-end-to-end-2026-10-29)). Đường
Chromium được đo qua MCP `render_frame` tới khi có bytes PNG, trên server release. Đường Rust là prototype in-process: chạy
`opencanvas-layout` với đúng input của server, trả lời đo bằng HarfRust, vẽ bằng painter spike đã tối ưu (mask text theo
vùng bao, mượn mask clip thay vì clone, song song bằng rayon). Frame đo: welcome-stacks 1040/390, welcome gốc và một grid
727 node, mỗi frame ở 1× và 2×. Phía Rust đo ở 11 thread và 1 thread.

- **Đúng đắn:** bounds và dòng text của Rust trùng Chromium tuyệt đối ở 4 frame × 25 viewport (29 000 node, 7 450 dòng,
  0 lệch). Painter tối ưu cho pixel trùng từng byte với painter trước đó, ở 1, 2 và 11 thread, trên 200 ca của 4 frame và
  trên scene có ảnh, viền, clip, nhóm opacity.
- **Cold (job đầu sau khi server khởi động):** Rust nhanh hơn 6,4–18× ở 1× và 2,8–12× ở 2×, kể cả grid (1 thread: 5,2–17×
  và 2,1–10×). Phía Chromium mất ~300 ms chỉ để khởi worker; Rust khởi trong 5 ms.
- **Warm:** ở 11 thread, Rust nhanh hơn 2,0–3,5× ở 1× và 1,2–2,2× ở 2× trên frame cỡ welcome. Grid 2× chậm hơn 1,1× với
  PNG `Balanced`, nhanh hơn 1,3× với PNG `Fast`. Ở 1 thread: 1,6–3,1× ở 1×, từ hòa tới 1,8× ở 2×, grid 2× chậm hơn 1,4×.
  Trước khi tối ưu, painter làm ba lượt toàn canvas cho mỗi text node và Rust chậm hơn 1,3–11× ở 2× và ở grid.
- **Stage chi phối phía Rust bây giờ:**
  - Nén PNG `Balanced`, một luồng: 48–69% thời gian warm.
  - Tô hình qua mask clip toàn canvas: 56–85% thời gian paint.
  - Text chỉ còn ≤ 6 ms/job.
  - 11 thread nhanh hơn 1 thread chỉ 1,25–1,3× (hai stage trên vẫn tuần tự) và tốn thêm 1,2–1,5× CPU.
- **CPU:** Chromium cũng chạy 5 process và ~109 thread mỗi worker, nhưng CPU mỗi job chỉ bằng 1,0–1,2× wall. Rust 1 thread
  dùng ít CPU hơn 1,25–5,9×; riêng grid 2× thì nhiều hơn 27%.
- **Phía Chromium khi warm:** IPC Playwright (8–62 ms), hai lần tải lại font mỗi job (12,5 ms) và spawn `layout-step`
  (3–13 ms) chiếm phần lớn; `drawScene` chỉ 2,5–7,3 ms.
- **Layout + đo khi cache trống:** Rust nhanh hơn 7–15× (0,8–11,5 ms so với 11–85 ms).

Kết luận:

- **Task 10** tiết kiệm 20–29% thời gian warm ở frame cỡ welcome (12–14% ở grid), nhiều hơn khi cache đo trượt, nhưng
  không đổi cold.
- **Task 11** bỏ được ~300 ms cold. Số đo cho thấy warm cũng nhanh hơn Chromium ở 7/8 cấu hình với 11 thread, và ở 6/8 với
  1 thread (welcome 2× hòa, grid 2× chậm hơn). Điều kiện: painter mask text theo vùng bao và không clone mask clip; cả hai
  đã chứng minh trùng byte.
- `crates/render` không dùng thread, nên Task 11 nên nhắm số 1 thread. Để grid 2× cũng thắng, việc còn lại là PNG (`Fast`
  hoặc nén song song ở server) và bỏ mask clip khi node nằm trọn trong vùng clip.

## Rủi ro chất lượng và cách chặn

| Rủi ro | Cách chặn |
| --- | --- |
| Viewer vẫn là Canvas2D nên có hai engine | Hình học dùng chung vì dòng text đến từ scene server. Export và `compare` lấy Rust làm chuẩn (oracle); viewer dùng chính painter đó qua WASM (Task 12), test so native↔WASM trong dung sai cố định, như PhotoCraft làm với GPU. |
| Mô hình width kiểu Chrome là suy ngược cho một build | Giữ Chromium **chỉ trong test**: CI so line list Rust với Chrome đã pin trên corpus; lệch thì bump version có chủ đích. |
| Linux chưa đo | Chạy lại phần Chrome của spike trong image `opencanvas:local` (mặc định và `--font-render-hinting=none`); phía Rust không cần chạy lại vì không phụ thuộc OS. |
| Chrome đang fallback ngầm sang font hệ thống (ví dụ "⌘" không có trong NotoSans) | Danh sách fallback khai báo trong manifest (font bundled) + diagnostic; output những text này sẽ đổi, phải bump `RENDERER_VERSION`. |
| Glyph nhạt hơn Chrome | Tăng contrast coverage 1.45 + snap baseline giảm 38% pixel Δ>32 vùng text ở 1×; chỉnh lại sau khi đo Linux. |
| Tính năng trong roadmap | Blur/shadow tự viết (resvg có mã filter để tham khảo); RTL/complex script cần bidi/itemize; emoji màu cần COLR/bitmap; WebP lossy còn non. |
| Hai implementation renderer (TS + Rust) | Chấp nhận trong giai đoạn chuyển; hướng dài hạn là một painter Rust, WASM cho viewer. |

## Tham khảo: PhotoCraft render thế nào

[storytold/photocraft](https://github.com/storytold/photocraft) @ `5f9b439` (2026-10-28), v0.17.0, MIT OR Apache-2.0,
"early alpha": bản viết lại Photoshop bằng Rust thuần, 24 crate, UI egui, chạy native và WASM, có CLI headless và MCP.
Chỉ đọc mã, không chạy.

**Cách họ render:**

| Phần | Cách làm | Nguồn |
| --- | --- | --- |
| Text | Parley 0.11 (HarfRust, bidi, line break) + skrifa, outline **không hinting**; không bật backend fontconfig, tự quét thư mục font, luôn đăng ký font bundled | `crates/text/Cargo.toml`, `src/lib.rs` |
| Raster glyph | Tự viết: cộng dồn diện tích có dấu chính xác (kỹ thuật font-rs), coverage f32, không supersample | `crates/text/src/raster.rs` |
| Đường antialias | Coverage thô → `powf` theo chế độ AA (Strong 0.55, Crisp 1.35, Sharp 2.2), hằng số khớp theo output Photoshop | `crates/text/src/render.rs:62` |
| Path/vector | Rasterizer tự viết, diện tích chính xác theo scanline (như gray rasterizer của FreeType), nonzero/evenodd và boolean op ngay khi quét | `crates/vector/src/raster.rs` |
| Compositor | Compositor **CPU là oracle tham chiếu**; compositor wgpu phải khớp nó trong 2/255 (premultiplied) cho mọi blend/adjustment/group/clip/mask | `crates/gpu/tests/it/parity.rs` |
| Effects | Blur shadow là hai lượt box (tent) như Photoshop, khớp trên file mẫu psd-tools; sai số coverage ghi trong comment | `crates/compose/src/effects/blur.rs` |
| Màu | Tự viết CMS ICC v2/v4 thuần Rust (6 270 dòng, không phụ thuộc crate khác) | `crates/cms/src/lib.rs` |
| Ảnh | `image`, `png`, `zune-jpeg`, `image-webp`, `jpeg-encoder`, `tiff`, `exr`; resvg chỉ dùng cho import SVG | `crates/codecs`, `crates/io` |
| Web | Toàn bộ engine dịch sang WASM; font phục vụ theo manifest có trường `scripts` cho fallback | `apps/photocraft-web`, `crates/text/src/served.rs` |

**Áp dụng được cho OpenCanvas** (phương pháp, không phải hằng số):

1. **Oracle CPU + engine thứ hai đo theo dung sai.** Đây là lời giải cho rủi ro "hai engine": painter Rust là chuẩn;
   viewer Canvas2D (hoặc WASM sau này) chạy cùng bộ fixture, so pixel premultiplied với ngưỡng cố định trong CI.
2. **Hiệu chỉnh theo output của app đích, ghi hằng số và sai số dư.** PhotoCraft phải dò Photoshop đóng mã;
   OpenCanvas có lợi thế: Chromium mở và chạy headless được, nên sinh corpus tham chiếu không giới hạn (glyph gamma,
   box-shadow, gradient) rồi khớp và khóa bằng test.
3. **Raster glyph diện tích chính xác thay vì supersample.** Gray rasterizer của FreeType (glyph Chrome trên Linux) cũng tính
   diện tích chính xác; tiny-skia 0.12 supersample hệ số 4 (`SUPERSAMPLE_SHIFT = 2`, `src/scan/path_aa.rs`). Đáng thử cho phần lệch text còn lại trong spike, đo trên Linux.
4. **Đường cong coverage là tham số có chủ đích.** Khớp với spike (contrast 1.45 giảm 38% pixel Δ>32);
   nên khớp lại trên Linux thay vì đoán.
5. **Không dùng fontconfig/system font; fallback khai báo trong manifest theo script.** Hợp với rủi ro "⌘" ở trên.
6. **Ngân sách hiệu năng có baseline và phân rã theo stage.** Với text, hình học glyph chỉ chiếm dưới 1/4;
   phần lớn là các lượt per-pixel, chạy song song được mà pixel vẫn trùng từng byte (`docs/text-raster-performance.md`).
   Spike Phần D đã làm thử: raster text theo vùng bao và các lượt per-pixel chạy rayon, pixel trùng từng byte với bản
   tuần tự (gate `paint-gate`).

**Không nên lấy:**

- Hằng số khớp Photoshop: tent blur, số mũ AA, optical kerning. Đích của OpenCanvas là CSS/Chrome, ví dụ box-shadow
  dùng Gaussian sigma = blur/2.
- Compositor GPU wgpu: server xuất ảnh chạy Docker CPU; GPU chỉ đáng xét cho viewer.
- Dùng crate PhotoCraft làm dependency: không publish lên crates.io, gắn chặt với mô hình tài liệu PSD (`photocraft-doc`),
  project còn alpha. Nếu thực nghiệm chứng minh lợi ích, chép riêng module nhỏ (ví dụ `vector/src/raster.rs` 584 dòng)
  kèm ghi công theo MIT/Apache.

## Lộ trình đề xuất

Phạm vi macOS, phương án C, **một engine Rust cho hai đích**: native để server xuất PNG, WASM để viewer hiển thị.
Viewer chỉ để xem, agent tự code lại UI theo frontend dự án đích, nên viewer phải giống ảnh export, không cần giống
Chrome từng pixel; hình học text vẫn phải khớp Chrome. Mỗi bước là một task handoff trong `tasks/`, sau cờ, đường
Chromium giữ nguyên làm đối chứng tới Task 13.

| Task | Việc | Phụ thuộc | Gate |
| --- | --- | --- | --- |
| [09](../tasks/09-render-wasm-spike.md) | Spike WASM: kích thước, thời gian khung lớn, native↔WASM | — | Go/no-go cho 11–12 |
| [10](../tasks/10-native-text-measure.md) | Đo text native, Taffy một lượt, Chromium vẫn vẽ | — | Scene trùng 100% Chromium |
| [11](../tasks/11-render-crate-native-png.md) | Crate `crates/render` thuần + xuất PNG không Chromium | 10, 09 go | Ngưỡng pixel, native↔WASM ≤ 1/255 |
| [12](../tasks/12-viewer-wasm-render.md) | Viewer vẽ bằng WASM trong Worker, vẽ lại khi zoom dừng | 11 | Bitmap = PNG export, không long task |
| [13](../tasks/13-rust-render-default.md) | Đổi mặc định khi chạy native mac, bump version | 10–12 | Chạy không cần Chromium, rollback được |

Task 09 và 10 chạy song song được. Preview motion (`/preview`, DOM) giữ riêng. Để sau: semantics P10a, Linux/Docker
(container 3847 ghim engine Chromium), tính năng P09 (gradient/shadow/blur), gỡ hẳn đường Node/Chromium cũ.

## Nguồn

- [harfrust 0.8.0 docs (mức conformance với HarfBuzz)](https://docs.rs/crate/harfrust/0.8.0), [RustyBuzz](https://github.com/harfbuzz/rustybuzz)
- [tiny-skia 0.12.0](https://docs.rs/crate/tiny-skia/0.12.0/source/Cargo.toml), [changelog 0.12.0](https://zff.dev/notgull/piet-tiny-skia/pulls/4)
- [Parley](https://github.com/linebender/parley), [parley 0.11.1](https://docs.rs/crate/parley/latest)
- [vello_cpu](https://lib.rs/crates/vello_cpu), [anyrender_vello_cpu 0.13.0](https://docs.rs/crate/anyrender_vello_cpu/0.13.0)
- [Takumi](https://docs.rs/takumi), [image-webp 0.2.4](https://docs.rs/crate/image-webp/latest), [zenwebp](https://lib.rs/crates/zenwebp)
- [krilla](https://github.com/LaurenzV/krilla), [Chromium font_render_params.h (gamma/contrast Skia)](https://sources.debian.org/src/chromium/139.0.7258.127-2/ui/gfx/font_render_params.h)
