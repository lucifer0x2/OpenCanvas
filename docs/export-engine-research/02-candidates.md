# 02 — Sàng lọc engine

Nguồn chính thức được mở **2026-10-29**; thông tin `main`/`latest` chưa phải phiên
bản được pin trong OpenCanvas. Mọi nhận định phù hợp/ranking là suy luận, chưa đo.

## A. resvg/tiny-skia + text native — spike ưu tiên cho bỏ browser

[resvg](https://github.com/linebender/resvg/blob/main/README.md) render static SVG
qua usvg và tiny-skia, có API Rust/C/CLI; không chạy script/animation. Upstream
hướng đến reproducibility giữa platform, nhưng điều đó không chứng minh khớp
Chromium của OpenCanvas. License resvg: MIT OR Apache-2.0.

[tiny-skia](https://github.com/linebender/tiny-skia) là raster CPU subset, có
fill/stroke/clip/image blend/PNG; thiếu text và không xuất PDF/format ngoài PNG.
BSD-3-Clause; có AArch64 NEON. Không dùng con số benchmark trong README để dự báo
workload repo. Không phải binding tới Skia C++ và không phải API Canvas2D thay thế.

Hai cách adapter đáng thử:

1. Scene → **SVG nội bộ đầy đủ** → usvg/resvg → RGBA/PNG. Tái dùng tree/group/
   image crop concepts, thêm stroke semantics và load exact font bytes. Không lấy
   SVG public đang bỏ stroke làm chuẩn đủ. Không để resvg tự wrap lại scene lines.
2. Scene → tiny-skia trực tiếp + glyph outlines. Tránh serialize/parse SVG nhưng
   phải viết painter, font shape/outline/compositing, image sampling nhiều hơn.
   Chỉ chuyển cách 2 nếu profiler chứng minh intermediate là bottleneck.

Đo text trước Taffy vẫn cần adapter riêng. `usvg` preprocess text thành paths
không tự cung cấp measurement protocol tương đương `measureBatch` hiện tại.
[Manifest usvg hiện tại](https://github.com/linebender/resvg/blob/main/crates/usvg/Cargo.toml)
có `fontdb`, `harfrust`, `skrifa`, Unicode bidi/script; mặc định bật system-fonts.
Spike cần tắt scan system fonts, nạp face từ bytes đã hash, pin graph đã build.

### Text stack: đánh giá tên thư viện theo trách nhiệm

| Thành phần | Khả năng và giới hạn đã tra | Quyết định |
| --- | --- | --- |
| [RustyBuzz](https://github.com/harfbuzz/rustybuzz) | Port HarfBuzz shaping, MIT; repo archived 2026-07-26, khuyên HarfRust. | Khảo sát để hiểu stack cũ; không chọn mới làm mặc định. |
| [ttf-parser](https://github.com/harfbuzz/ttf-parser) | Parser TTF/OTF, đọc tables/outlines; MIT OR Apache-2.0; maintenance mode, khuyên fontations. Không wrap/raster. | Có thể dùng khi dependency pin cần, không tự thêm làm nền mới. |
| [HarfRust](https://github.com/harfbuzz/harfrust) | Shaping MIT, dùng read-fonts; output units-per-em cần scale; upstream nêu còn conformance gaps. Không layout/fallback hệ thống. | Ưu tiên kiểm cùng graph usvg/cosmic hiện tại; không coi ngang Chrome tự động. |
| [fontdb](https://github.com/RazrFalcon/fontdb) | MIT, lưu/chọn font face, load raw bytes; **không implement fallback**. | Registry manifest; ứng dụng vẫn phải chốt thứ tự fallback/face index. |
| [cosmic-text](https://github.com/pop-os/cosmic-text) | MIT OR Apache-2.0; HarfRust shaping, swash raster, custom bidi/layout/fallback, color emoji; upstream hỗ trợ Linux/macOS/Windows. | Đối thủ cho adapter measurement; phải override/port wrap semantics, tránh hai shaper khác graph. |

Không ghép mọi crate vào cùng engine theo danh sách: resvg/usvg đã có shaper/parser.
Nếu cosmic-text đo nhưng usvg shape lại, phải so glyph advances/runs; việc cả hai
dùng HarfRust chưa đủ khi phiên bản/options/font selection khác. Cân nhắc một
canonical glyph-run nội bộ cho measure và paint; public scene hiện chưa chứa nó.

## B. Skia C++ qua skia-safe CPU — đối chứng bắt buộc

[rust-skia](https://github.com/rust-skia/rust-skia) cung cấp Rust wrappers quanh
Skia C++, license wrapper MIT. README hiện theo chrome/m156; build script có thể
tải prebuilts. Linux arm64/x64 cần glibc ≥2.31, libstdc++ ≥GLIBCXX_3.4.28;
macOS arm64/x64 có trong matrix. WASM được liệt kê là **emscripten**, không phải
cam kết `wasm32-unknown-unknown`. Pin binary/source/feature hash; không tự tải lúc chạy.

[Skia license](https://github.com/google/skia/blob/main/LICENSE) BSD-3-Clause;
license của codecs/fonts/dependencies vẫn cần inventory riêng. Chọn raster surface
CPU, không bắt buộc GPU context. Drawing path rộng hơn subset tiny-skia là lý do
giữ ứng viên, chưa phải bằng chứng nhẹ hơn/nhanh hơn browser.

Điểm cần làm: scene painter; exact face loading; shaper như SkShaper/SkParagraph
hoặc HarfRust với positioned glyphs; custom wrap giống repo; PNG encode và bounds.
Không dùng text API đơn giản rồi giả định đã có complex shaping. Tài liệu
[Skia text overview](https://skia.org/docs/dev/design/text_overview/) tách shaping
khỏi drawing, mô tả positioned glyphs/fallback; không phải lời hứa CSS layout.

Skia cùng họ với renderer browser không bảo đảm cùng Blink text metrics,
font fallback, hinting, antialiasing, color space hoặc sampler. Ưu điểm cần kiểm
là drawing/effects coverage; giá phải trả là C++ build/link/runtime và binary
provenance, xem [bridge](03-bridges-packaging.md).

## C. Rust CDP giữ Chromium — thực tế nhất nếu chỉ bỏ Node

[chromiumoxide](https://github.com/mattsse/chromiumoxide/blob/main/README.md)
MIT OR Apache-2.0, Tokio async, generated CDP commands và API launch/execute;
có browser fetcher nhưng production nên cấp Chromium đã pin. API còn gaps;
không coi driver là replacement toàn bộ lifecycle Playwright miễn phí.

Rust worker có thể giữ layout loop/resources protocol, serve bundle bằng route
allowlist, dùng CDP gọi `renderer.measureBatch`/`drawScene` như trước.
[CDP Runtime](https://chromedevtools.github.io/devtools-protocol/tot/Runtime/)
có evaluate/callFunctionOn; không biên dịch JS sang Rust.

Phải làm lại launch flags, page setup, promise/error conversion, CDP version
compatibility, disconnect/cancel/deadline/process-tree cleanup. Giữ capture
Canvas2D PNG, không đổi sang screenshot. Cùng browser/font/flags mới có cơ hội
giữ RGBA cũ. Chromium OS libs/sandbox/update vẫn tồn tại, Node build tools/E2E
vẫn tồn tại. Chưa đo phần footprint hoặc latency chỉ do Node đóng góp.

## Các lựa chọn đã sàng lọc

| Lựa chọn | Nguồn/khả năng | Lý do chưa ưu tiên production |
| --- | --- | --- |
| Vello GPU/wgpu | [Vello](https://github.com/linebender/vello) hiện có GPU sparse strips và compute renderer experimental ở research; [wgpu](https://github.com/gfx-rs/wgpu) dùng Vulkan/Metal/D3D12/GL, web WebGPU/WebGL2; cả hai MIT OR Apache-2.0. | Không hợp baseline Docker CPU; software Vulkan vẫn thêm driver/adapter và cần đo. GPU startup/readback và glyph shaping chưa được giải quyết bằng wgpu. |
| Vello CPU | [README riêng](https://github.com/linebender/vello/blob/main/vello_cpu/README.md), CPU/SIMD, còn phát triển. | Không loại vì GPU; challenger hợp lý nếu hai spike chính thất bại/coverage tốt hơn. Cần scene/text/encode adapter mới; chưa chứng minh reusable SVG path lợi hơn resvg. |
| Cairo + Pango + HarfBuzz + FreeType qua Rust | [Cairo](https://www.cairographics.org/) có image/SVG/PDF surfaces; [Pango](https://docs.gtk.org/Pango/) international layout; [pangocairo](https://gtk-rs.org/gtk-rs-core/stable/latest/docs/pangocairo/) wrapper. | Khả thi CPU Linux; nhiều system libs/fontconfig/GLib, macOS packaging và parity custom wrapper cần đầu tư. Không cần full GTK GUI, nhưng binding không bỏ C dependencies. |
| Qt QPainter/QTextLayout | [QPainter](https://doc.qt.io/qt-6/qpainter.html) paint lên QImage; [QTextLayout](https://doc.qt.io/qt-6/qtextlayout.html) layout/render text. | Đáng cân nhắc nếu sản phẩm chọn native Qt UI; repo hiện Rust/web, thêm Qt runtime/plugin/cross build không có reuse sẵn. Không chọn QML Canvas Painter chỉ do tên giống Canvas2D. |
| Python Pillow | [ImageFont](https://pillow.readthedocs.io/en/stable/reference/ImageFont.html) FreeType, RAQM advanced layout; [build dependencies](https://pillow.readthedocs.io/en/stable/installation/building-from-source.html) gồm Raqm/HarfBuzz/FriBiDi và codecs theo cấu hình. | Hợp prototype/corpus converter; font wrapping/fallback/vector effects cần tự làm. Python wrapper không sửa parity hoặc loại native libs. |
| Python Pycairo | [Pycairo](https://pycairo.readthedocs.io/en/latest/) bọc Cairo, Python ≥3.11, Cairo ≥1.15.10. | Cần Pango/HarfBuzz cho text, không chỉ Cairo toy text; nếu dùng Rust rồi nên gọi Cairo/Pango trực tiếp tránh thêm interpreter. |
| Python skia-python | [Official repo](https://github.com/skia-python/skia-python), pybind11, wheels Linux/macOS arm64/x64; Linux yêu cầu EGL/GL/fontconfig theo README. | Vẫn Skia C++ + Python; có ích khi đã có Python renderer, repo chưa có. CPU raster không tự bảo đảm package wheel không link GL libs. |

### C stack: vai trò không hoán đổi

[HarfBuzz](https://harfbuzz.github.io/what-does-harfbuzz-do.html) shape codepoints
thành glyphs/positions, có model Latin/complex scripts/emoji; không tự thay line
breaker hoặc whole scene painter. [FreeType](https://freetype.org/) đọc/raster font;
Pango itemize/layout/fallback; Cairo composite/paths/surfaces. Rust wrappers gọi C
API, không port các implementation đó sang Rust. Dùng Pango layout nguyên trạng
sẽ đổi whitespace/break/baseline repo trừ khi adapter giữ semantics.

## Tình trạng duy trì/license đã kiểm và giới hạn xác minh

| Nhóm | Tín hiệu official ngày tra cứu; license |
| --- | --- |
| resvg/tiny-skia/cosmic/fontdb/HarfRust | Repos/docs truy cập được, không thấy thông báo archive ở trang đã mở; không coi là SLA. License như các đoạn trên; [resvg changelog](https://github.com/linebender/resvg/blob/main/CHANGELOG.md) cần đối chiếu tag chọn ở spike. |
| RustyBuzz/ttf-parser | Archive / bug-fix-only được README nói rõ; không suy từ số sao. |
| rust-skia/Vello/wgpu/CDP | Upstream có hướng build/API hiện hành; chưa kiểm release artifact cho mọi target/feature tổ hợp. |
| Cairo/Pango | Cairo site ghi 1.18.6 ngày 2026-09-20; Pango docs 1.58.2. Cairo LGPL-2.1 OR MPL-1.1; Pango LGPL-2.1-or-later; pangocairo wrapper MIT. |
| HarfBuzz/FreeType | FreeType site ghi 2.15.0 ngày 2026-10-12; [FreeType license](https://freetype.org/license.html) FTL OR GPLv2; [HarfBuzz COPYING](https://github.com/harfbuzz/harfbuzz/blob/main/COPYING) MIT-style notices. |
| Qt | [Qt licensing](https://doc.qt.io/qt-6/licensing.html): commercial hoặc LGPLv3/GPLv3 theo module; Qt GUI khác modules GPL-only. Cần kiểm module/third-party licenses thực tế, không gán chung MIT. |
| Python graphics | Pillow docs 13.0.0 (2026-10-16), [MIT-CMU](https://pillow.readthedocs.io/en/stable/about.html#license); Pycairo LGPL-2.1-only OR MPL-1.1; skia-python BSD-3-Clause. Không xác minh wheel hashes hoặc free-threaded compatibility thực. |

Các license là metadata nguồn chính thức; quyết định phân phối cần inventory
binary/dependency/font thực tế. Chưa chạy build hay package bất cứ ứng viên nào.
