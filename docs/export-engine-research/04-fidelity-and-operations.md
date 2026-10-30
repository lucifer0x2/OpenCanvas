# 04 — Ma trận fidelity và vận hành

Ngày 2026-10-29. **C** = code hiện có đã đọc; **U** = upstream capability;
**G** = gate phải đo/kiểm trong spike. Capability không đồng nghĩa parity.

## Text: đúng line list vẫn có thể khác hình

| Ca | Chromium hiện tại (C) | resvg/HarfRust hoặc cosmic (U→G) | skia-safe CPU (U→G) | Cairo/Pango hoặc Python (U→G) |
| --- | --- | --- | --- | --- |
| Vietnamese NFC/NFD, combining marks | `measureText`/`fillText`; split grapheme bằng Intl.Segmenter | Shape nguyên run, giữ combining cluster; so glyph bounds/advances; không sum width từng scalar. | Shaper + exact face; kiểm hinting/baseline thay vì chỉ SkFont width. | Pango hoặc Pillow RAQM; không Cairo toy text/Pillow BASIC. |
| Grapheme ZWJ/variation selectors | Long word split grapheme, browser Unicode version | Unicode grapheme segmentation version phải pin; không cắt giữa emoji sequence. | Wrapper phải quyết break theo cùng policy. | Layout engine mặc định có thể khác word/grapheme breaking. |
| Kerning/ligature | `fontKerning="normal"`, whole line measure/paint | Shape cả line; options/feature set và size scaling nhất quán. | Không thay bằng draw từng character; compare AV/To/ffi. | RAQM/HarfBuzz options và raster FreeType phải pin. |
| Font fallback | Exact FontFace load nhưng thiếu glyph vẫn có thể dùng system fallback | fontdb không fallback; cosmic có policy khác browser; manifest explicit fallback cần thiết kế. | Kiểm font manager không scan host tùy ý; ghi exact fallback face/hash. | Pango/fontconfig và Pillow fallback không mặc nhiên cùng Chrome; kiểm từng cluster. |
| Emoji màu, CJK, ký hiệu ⌘↵✓ | Phụ thuộc font hệ thống khi bundled thiếu; chưa diagnostic glyph | cosmic/swash có color emoji; usvg/path support font format phải thử (COLR/bitmap/SVG). | Kiểm font format/color glyph paths đã bật; không suy text monochrome đủ. | FreeType/Pango/RAQM support khác nhau theo build/codecs; Python wrapper chưa chứng minh đủ. |
| Missing glyph/tofu | Font missing != glyph missing; P11a planned | Shape glyph ID 0/cluster coverage + diagnostic; không chỉ cmap presence cho sequence. | Cùng gate; export strict policy glyph cần chốt riêng. | Cùng gate; không báo ready “đủ font” chỉ vì file mở được. |
| LF/CRLF/space, empty text | Normalize CRLF, trim/collapse horizontal spaces, LF forced, empty paragraph vẫn line | Port current wrapper; không nhận CSS/SVG/Pango default như tương đương. | Tách breaker khỏi drawing để giữ contract. | Không lấy textwrap Python/word count thay browser grapheme/shaping. |
| Width round/baseline | Grid 1/64 round; heuristic baseline; P10a planned sửa | So baseline cả current và P10a desired; break gần width−1/64,width,width+1/64. | Same; unhinted fractional vs device-hinted metrics cần đo. | FreeType hinting/layout metrics theo options; native policy khác DOM có thể hợp lệ nhưng cần version. |
| Wrap/hug/fill | Taffy multi-pass, native worker phải trả exact measurements | Width null=max-content, finite width, min-content=0; repeat own-width không tự wrap lại. | Cùng tests; chuyển engine không sửa Taffy layout semantics. | Pango/cosmic tự layout không được chạy thành box engine thứ hai. |
| Variable fonts/letter spacing | Các P10b/c/P12b còn planned | Kiểm khả năng nhưng không quảng cáo hỗ trợ authored props chưa có. | Same; retain future axes/features trong key khi được thêm. | Same; library capability không là product feature. |

Nguồn capability: [HarfRust](https://github.com/harfbuzz/harfrust),
[COSMIC Text](https://github.com/pop-os/cosmic-text), [fontdb](https://github.com/RazrFalcon/fontdb),
[HarfBuzz](https://harfbuzz.github.io/what-does-harfbuzz-do.html),
[Pillow ImageFont](https://pillow.readthedocs.io/en/stable/reference/ImageFont.html).
Chi tiết source hiện tại ở [trace](01-current-pipeline.md); bảng trên là gate
thiết kế từ các khả năng đó, chưa có result ứng viên.

## Hình và formats

| Ca | Hành vi cần giữ | Gate native |
| --- | --- | --- |
| Fill/clip/radius | rounded fills, ancestor clips, image own clip | AA edges, fractional coordinates, nested clipping, transparent background; RGBA premultiplied/straight conversion. |
| fit/contain/cover + crop | crop normalized source, center placement; `fit` stretch | Portrait/landscape/checkerboard, subpixel source rect, minuscule crop, image edge outside viewport. |
| Opacity | CSS group composite, subtree overlap không nhân alpha sai | Isolated layers và existing fold-alpha fallback khi 16M layer budget hết; count/stats hoặc approved new behavior. |
| Stroke | uniform/per-side widths/colors, inside/center/outside, sau subtree, own clip không cắt stroke | Rounded border corners, adjacent sides, stroke over child, ancestor clip; không đơn giản offset toàn rect. |
| Image decode/color | Browser Image.decode hiện là authority raster | PNG alpha, JPEG/WebP inputs, EXIF orientation, ICC/sRGB/gamma, interpolation/downscale; compare masks/color separately. |
| PNG | Hiện có frame + node, scale≤4, transparent background, ceil dimensions | Native encoder có thể khác compressed bytes; decode so RGBA, signature/dimensions/errors giữ. |
| SVG | Hiện có live text/no fonts embedded và cảnh báo stroke omission | Giữ public semantics/diagnostics trừ task riêng; internal outlined SVG không phải lý do đổi public SVG ngầm. |
| JPG/WebP | P03d planned, chưa code hiện tại | Codec/quality/white JPEG background/alpha WebP là task format riêng; tiny-skia không tự encode hai format này. |
| PDF | Chưa hỗ trợ | Skia/Cairo/Chromium có hướng PDF; kiểm page bounds/font embed/text còn selectable/vector; raster PNG đặt vào PDF không tính vector export. |
| Animation/effects tương lai | Time-sampling/effects còn planned ngoài static export hiện tại | resvg static không chạy authored animation; sample scene trước renderer. Gradients/shadow/blur/backdrop cần feature gate riêng. |

Nguồn scope formats: `docs/SERVER.md:92`, `jobs/export_svg.rs:90`, P03d handoff;
nguồn library: [tiny-skia](https://github.com/linebender/tiny-skia),
[Cairo](https://www.cairographics.org/), [Skia modules](https://skia.org/docs/user/modules/).
Không có native format/parity nào được nghiệm thu trong nghiên cứu này.

## Deterministic fonts và provenance

Đề xuất: native worker chỉ nhận bytes đã validate/hash; disable implicit system
font discovery; fallback là ordered manifest có version và face-index/axes cụ thể.
Nếu chưa có glyph, trả diagnostic cluster/codepoints và face được chọn; không
âm thầm dùng host font làm kết quả tất định. Thay policy glyph strict/non-strict
là quyết định sản phẩm riêng, vì hiện strict export chỉ đòi resources sẵn sàng.

Storage hiện nhận **TTF/OTF/WOFF2** (`crates/storage/src/assets.rs:208`–`:210`).
Native parser stack không mặc nhiên đọc compressed WOFF2: phải kiểm hỗ trợ,
decode trong worker có budget và giữ hash **file gốc**, hoặc trả unsupported
rõ ràng trong thử nghiệm. Không thu hẹp formats đang hợp lệ khi rollout.
NotoSans bundled có license OFL trong repo; fallback/emoji fonts phải có license
phân phối, pinned bytes và attribution riêng. Không dùng SF Pro của host làm font
Docker mặc định hoặc corpus redistributable.

Cache key mới cần gồm engine build/version/features, shaper/parser/raster/encoder
versions/options, font original+decoded hashes/face/axes/fallback order, image
hash+decode/color policy, viewport/theme/state/scale và authored resolved input.
Public provenance hiện có rendererVersion/sceneVersion/fontManifestHash/
assetManifestHash/sourceSceneKey. Internal fingerprint có thể chứa manifest kỹ
thuật mới; không đổi response schema tùy tiện. Các runtime fields kiểu
browserVersion/runtimeVersion phải được xử lý compatibility rõ, không giả browser
version cho native. Fallback engine phải có identity thật, không trộn cache.

## Docker CPU, limits và lỗi

- Bắt buộc run trên Linux arm64 CPU-only, không GPU/display mount; native macOS
  arm64 là gate riêng. Có thể thêm Linux x64 sau; không ghi đã hỗ trợ vì crate
  README liệt kê target. Native C/Qt/Python libs vẫn có system packaging.
- Giữ 2048 nodes, depth128, 16M output pixels, scale≤4, 6 layout passes, 10k
  measurement-cache entries, per-resource timeout10s, worker op30s, parent job45s
  (`layout/lib.rs:22`, `validation.rs:103`, `pipeline.mjs:14`, `:48`, `:76`,
  `state.rs:27`). Export48MiB và NDJSON response48MiB là hai cap khác nhau.
- Decode image/font có thể nở vượt kích thước file. Cap decoded bytes/glyph cache,
  layer sum và output **trước** allocate, kiểm arithmetic overflow; output-pixel
  cap không giới hạn toàn RSS. Đo copies JSON/base64 + RGBA + compositor layers.
- Giữ requested/background queue/backpressure/retry policy, worker restart sau
  timeout/failure, cleanup subprocess tree. Cache memory cần bounded eviction;
  không tăng worker pool vì giả định Rust nhẹ hơn.
- Không hứa chạy engine native trên Tokio task rồi Future timeout có thể ngắt
  native call. Child process cho deadline có thể enforce, xem [bridges](03-bridges-packaging.md).

## Viewer parity, native và WASM

Giữ React shell và scene layout authority ở Rust không cản native server export.
Nhưng browser `fillText` sẽ shape lại strings; native font advances đúng mà browser
glyph width khác vẫn có thể tràn box. Ba mức quyết định:

1. **Giữ Canvas2D viewer**: native export phải đạt agreed tolerances với browser,
   hoặc chấp nhận/render diagnostic rõ. Không tự hạ tiêu chí RGBA0 đang dùng.
2. **Shared native/WASM painter**: có thể đưa CPU renderer/shaper chung vào browser,
   giữ shell/selection; phải đo WASM load size/init memory, pan/zoom/DPR, clipboard,
   font/resource loading. Đây là dự án riêng, không hệ quả miễn phí của viết Rust.
3. **Server PNG làm viewer**: giảm painter divergence nhưng mất vector zoom hiện có
   và tăng network/render latency; không đề xuất mặc định cho yêu cầu hiện tại.

[CanvasKit](https://skia.org/docs/user/modules/canvaskit/) là Skia WASM có API gần
Canvas, có thể là hướng chung C++/web; không phải Rust thuần. rust-skia WASM matrix
nêu emscripten. wgpu WebGPU/WebGL2 có thể mở đường GPU browser nhưng hỗ trợ backend
không chứng minh parity hoặc giảm footprint. Giữ native/WASM options mở qua
scene/renderer boundary nhỏ, không rewrite viewer trước khi CPU export qua gates.
