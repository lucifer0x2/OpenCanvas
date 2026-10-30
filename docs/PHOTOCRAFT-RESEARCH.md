# PhotoCraft có thể thay đường xuất ảnh OpenCanvas đến đâu?

Ngày nghiên cứu: **2026-10-29**. OpenCanvas checkout `codex/p24-components-instances`,
HEAD `535d7e1a7f2bb6017918ccbe1c0aa2b34f3745fa`. PhotoCraft ghim tại
[`5f9b4391874aabc9deb2fcb905267613e4d93ec8`](https://github.com/storytold/photocraft/commit/5f9b4391874aabc9deb2fcb905267613e4d93ec8).
Đây là nghiên cứu source và đề xuất thử nghiệm; **chưa thay engine/frontend, chưa
build hoặc benchmark PhotoCraft**. Các số spike Rust cũ là bằng chứng lịch sử,
không phải kết quả chạy trong nghiên cứu này.

## Khuyến nghị

**Có thể tái dùng phần render CPU của PhotoCraft, nhưng chưa nên lấy toàn engine
làm dependency production hoặc chọn nó làm chuẩn hình ảnh.** Hướng nên thử đầu
tiên là tái dùng **một module raster glyph diện tích chính xác** trong spike Rust
đang có, giữ nguyên shaper/wrap/scene để đo riêng tác dụng raster. Chỉ nhận module
đó nếu cải thiện có thể lặp lại trên cả Linux arm64 CPU và macOS arm64.

Nếu cần text stack đầy đủ hơn, thử **Parley + skrifa trực tiếp** với font registry
và wrapper của OpenCanvas. PhotoCraft cung cấp ví dụ canonical positioned glyphs
rất hữu ích, nhưng paragraph layout, baseline, optical kerning, fallback và màu
của nó phục vụ Photoshop. Phải port semantics OpenCanvas dù dùng crate có sẵn.
[Text layout](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/layout.rs#L1-L15),
[glyph raster](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/raster.rs#L1-L29).

Không bắt buộc loại direct-crate adapter từ đầu: `TextEngine::new()`,
`layout()`, `FontDb::register_font_data()` và `render::rasterize()` đều public.
Direct adapter là đối chứng để đo chi phí tái sử dụng; nó kéo theo document/PSD,
bundled fonts/build script và policy fallback. Không cần kéo UI/wgpu chỉ để CPU
compose. Chi tiết ở [pipeline và dependency](photocraft-research/01-pipeline-dependencies.md).

## Điều thực sự tái sử dụng được

| Phần | Bằng chứng source | Quyết định đề xuất |
| --- | --- | --- |
| Positioned glyphs dùng chung đo/vẽ | Public `TextLayout` có faces, glyph IDs, tọa độ, clusters và lines; raster nhận chính layout đó. | Học boundary này; tạo glyph runs nội bộ cho OpenCanvas, chưa đổi public scene. |
| Raster glyph exact-area | `text/raster.rs` chỉ cần segments/coverage, không cần compositor PSD. | Ứng viên vendor nhỏ nhất; giữ attribution, kiểm curve flatten và AA. |
| Vector coverage | Có fill rules, boolean components, stroke outline và masks. | Đối chứng nếu tiny-skia edge fidelity fail; chưa có lý do thay cả painter. |
| CPU compositor | Thư viện độc lập GPU/UI; straight-alpha RGBA f32, tile/band render. | Khả thi kỹ thuật nhưng nhiều semantics Photoshop và bộ nhớ cần đo. |
| Codecs/CMS | Có decode, orientation, ICC và PNG pipeline. | Tham khảo policy/limits; PNG đơn thuần dùng crate `png` tránh kéo tất cả formats. |
| CPU làm reference cho GPU | Upstream có parity tests và ngưỡng thực trong code. | Mượn cách tổ chức test; không tự đổi oracle hoặc tolerance OpenCanvas. |

Mọi khả năng trong bảng là **đã đọc source**, chưa nghiệm thu trên corpus
OpenCanvas. Đường file, API và phạm vi coupling có permalink tại
[phụ lục 01](photocraft-research/01-pipeline-dependencies.md).

## Ba phương án tích hợp

| Phương án | Phần việc OpenCanvas vẫn sở hữu | Chi phí duy trì và nhận định |
| --- | --- | --- |
| Dependency git ghim SHA vào `photocraft-text/vector/compose` | Map computed scene → layers/surfaces; resources/font IDs; wrap/baseline; stroke từng cạnh; clip; errors/jobs/budgets/provenance. | Ít chép code, nhưng phụ thuộc API alpha và graph PSD/text/fonts. Chỉ đáng chọn nếu spike chứng minh tổng effort ít hơn. |
| Vendor subset raster glyph/vector | Canonical shaping, font loading, whole scene painter và encode vẫn do OpenCanvas quản lý. | Phạm vi review nhỏ, hợp spike hiện có; OpenCanvas nhận trách nhiệm sửa bugs/merge upstream và notices. Không chép hằng số khớp Photoshop. |
| Parley + skrifa/HarfRust trực tiếp, painter độc lập | Font registry/fallback, current wrapper, canonical runs, painter/compositor và encode. | Chủ động semantics, không kéo doc PSD; nhiều integration cần kiểm. Giữ HarfRust spike cũ làm control trước khi thêm Parley. |

**Không phương án nào thay Taffy, HTTP/MCP, queue/jobs, SQLite/revision hoặc React
shell.** Native shaper có thể đo đồng bộ, nhưng bỏ bounded multi-pass/pending/cache
protocol là thay đổi kiến trúc riêng; không tự gộp vào thử raster.

Đối chứng **resvg/tiny-skia** và **skia-safe CPU** trong nghiên cứu rộng vẫn hợp
lý. PhotoCraft không làm chúng lỗi thời: resvg có scene SVG static; Skia có drawing
coverage rộng; PhotoCraft mạnh ở việc cho thấy một CPU pipeline Rust tự chủ có thể
tổ chức ra sao. Chưa có số đo xác định engine thắng.
[So sánh cập nhật](photocraft-research/03-semantics-operations.md#đối-chứng-cập-nhật).

## Typography là gate trước khi chọn renderer

- **Tiếng Việt NFC/NFD:** có shaper/cluster model không có nghĩa đã qua bộ dấu
  kết hợp của OpenCanvas. Không normalize authored strings ngầm; kiểm glyphs,
  advances, baseline, line breaks và bitmap riêng.
- **Wrap/baseline:** PhotoCraft dùng paragraph rules/leading và first baseline
  kiểu Photoshop. OpenCanvas hiện collapse whitespace, break từ dài theo grapheme,
  round grid 1/64 và baseline heuristic. P10a còn đề xuất ceil/font metrics/nowrap;
  một phần sửa own-width bằng rounded fits đã có trong source hiện tại.
- **Fonts:** `TextEngine::new()` tránh scan host, nhưng có Inter/JetBrains bundled
  và optional craft-fonts. Shared engine native mặc định có system discovery;
  Linux còn gọi `fc-list`, macOS có CoreText. “Không link fontconfig” không đồng
  nghĩa “không dùng font hệ thống”.
- **Fallback/missing glyph:** public flag tắt glyph protection vẫn giữ default
  family Inter. cmap chỉ là phép kiểm sơ bộ, không chứng minh cluster/sequence,
  raster outline hay color glyph được vẽ. TTF/OTF có public registration; **WOFF2
  và emoji màu chưa chứng minh**, phải có test hoặc unsupported rõ.

Source, mapping và corpus cụ thể ở [text/fonts](photocraft-research/02-text-fonts.md).

## Những kết luận cũ cần giới hạn lại

Giữ nguyên [PURE-RUST-EXPORT](PURE-RUST-EXPORT.md) và
[nghiên cứu rộng](EXPORT-ENGINE-RESEARCH.md) để bảo toàn lịch sử; đọc kèm các
điều chỉnh sau:

| Nhận định cũ | Kết luận có bằng chứng hơn |
| --- | --- |
| “Hình học đạt giống Chrome tuyệt đối” | Summary cũ ghi parity trên 1 font LTR/macOS; chưa phải mọi scene/font/OS. Raw v2 không đủ trong checkout để tái tính độc lập. |
| “Pixel không bao giờ trùng; chỉ Chrome mới trùng Chrome” | Spike có khác pixel trên fixture đã đo; không chứng minh mệnh đề bất khả thi cho mọi native implementation/case. Flat pixels có thể trùng. |
| “Rust cho cùng pixel mọi máy; không cần chạy lại Linux” | Đây là giả thuyết cần chạy hai bản build. Float/SIMD/libm, dictionaries, codec và font policy có thể khác. |
| “Missing glyph chỉ cần tra cmap” | Phải xét shaped cluster, selected fallback face, glyph ID0, dấu/variation/ZWJ và drawable glyph; cấm silent success. |
| “PhotoCraft không dùng system fonts” | Chỉ đúng khi chọn constructor/policy phù hợp; stock shared engine native mặc định scan. |
| “CPU oracle giải quyết hai engine” | CPU reference giúp regression nội bộ, không chứng minh đúng yêu cầu Chromium. Đổi oracle/tolerance là quyết định sản phẩm riêng. |
| “GPU parity 2/255” | Header ghi2/255; constant thực `1/255 + 4·f32::EPSILON`, so premultiplied. Mặc định có thể skip khi không adapter. |
| “Native đo đồng bộ → bỏ 6 passes/pending” | Có thể thiết kế lại, nhưng nghiên cứu raster chưa chứng minh thay protocol cần thiết hoặc giữ contract. |
| “Scene hiện không có transform/animation” | Static export painter là phạm vi đang xét; authored animation và viewer motion preview đã có, không được suy thành toàn sản phẩm không hỗ trợ motion. |

[Evidence cũ và giới hạn tái lập](photocraft-research/04-spike-evidence.md#bằng-chứng-hiện-có),
[ngưỡng upstream thật](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/gpu/tests/it/parity.rs#L1-L13).

## Bước tiếp theo nhỏ nhất

Giao một task spike riêng, chỉ scope `spikes/photocraft-export/` hoặc scratch:
copy/parameterize spike Rust cũ, giữ control tiny-skia, thêm exact-area glyph
raster; xuất raw metrics/runs/hashes và diff. Chưa cần adapter toàn document hoặc
production feature flag. Dừng nếu không giữ được geometry/whitespace hiện tại,
không có cải thiện raster rõ, hay chi phí dependency/maintenance vượt lợi ích.
Handoff gồm file boundaries, AC, corpus, cách đo và stop gates ở
[phụ lục 04](photocraft-research/04-spike-evidence.md).

## Mức độ xác minh

| Mức | Nội dung |
| --- | --- |
| Đã xác minh tĩnh | Connector đọc pinned tree/files; public CPU APIs/crate manifests/font constructors; glyph → coverage; compose cached surfaces → encode; upstream budgets; source OpenCanvas và artifacts cũ. |
| Suy luận kiến trúc | Vendor nhỏ dễ kiểm hơn toàn PSD adapter; Parley trực tiếp có thể giảm coupling; CPU child worker giữ failure isolation. |
| Chưa xác minh chạy | PhotoCraft compile/link; native Linux/macOS; font/emoji/WOFF2 coverage; pixel/metrics parity; deterministic cross-platform; cold/warm latency/RSS/size. |

Không dùng upstream test source hoặc số đo trên máy khác làm local pass. Không
chạy full suite, cài toolchain/deps, khởi động server hoặc chạm dữ liệu người dùng.
Lệnh kiểm tài liệu và cách lấy source ghi trong [evidence](photocraft-research/04-spike-evidence.md).
