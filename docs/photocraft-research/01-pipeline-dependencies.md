# 01 — Pipeline thực và ranh giới tái sử dụng

Tất cả permalink PhotoCraft trong tài liệu này ghim SHA
`5f9b4391874aabc9deb2fcb905267613e4d93ec8`, version workspace0.17.0,
edition2024, MSRV1.95. Đây là implementation mở mô phỏng Photoshop, không phải
source Adobe. Semantics Photoshop được thể hiện ngay trong
[layout](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/layout.rs#L1-L15)
và [CPU compose](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/compose/src/lib.rs#L1-L15).

## Trace: model → layout/glyph → raster → compose → PNG

```text
Document.layers: TextLayer / ShapeLayer / Raster / Group ...
  ├─ text refresh: TextEngine::layout(TextLayer,dpi)
  │   → paragraph preparation / font selection / Parley shaping + line break
  │   → TextLayout {faces,glyph IDs,positions,clusters,lines,styles}
  │   → render::rasterize(layout,Affine,PixelFormat,AA)
  │   → skrifa unhinted outlines → flattened segments → Coverage f32
  │   → alpha accumulation → Surface + document-space rect → TextLayer.cache
  ├─ shape refresh: vector path/stroke raster → ShapeLayer.cache
  └─ image import/decode: codecs::Image → tiled Surface
Document (caches prepared) → compose::render / render_bands / flatten
  → CPU layered compositing/masks/effects → straight RGBA float Buffer
  → quantized codec Image / RGBA8 → codecs::encode PNG → png encoder → bytes
```

| Khâu | API/source thật | Điều cần giữ khi dùng cho OpenCanvas |
| --- | --- | --- |
| Text model | [TextLayer](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/doc/src/lib.rs#L393-L415) có points, transform, runs, paragraph shape và cache. | Không nhầm size_pt với logical px, cache với authored text. |
| Layout | [TextEngine API](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/lib.rs#L63-L76), [Layouter](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/layout.rs#L275-L320). | DPI/72 và Photoshop point/box rules là adapter work. |
| Shape & break | [RangedBuilder + styles](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/layout/shaping.rs#L292-L321), [break_all_lines](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/layout/shaping.rs#L353-L367). | Không để Parley thay current wrapper không được kiểm. |
| Canonical output | [GlyphFace/PlacedGlyph](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/layout.rs#L73-L97), [TextLayout](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/layout.rs#L109-L176). | FontData/index, variation coordinates và size thuộc run, đo/vẽ dùng chung. |
| Glyph → outline | [draw](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/render.rs#L167-L217). | Skrifa unhinted; glyph không outline bị bỏ; faux bold/italic là policy. |
| Outline → alpha | [Coverage](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/raster.rs#L71-L167), [rasterize entry](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/render.rs#L341-L383). | Bounded allocation, flatten tolerance, coverage transfer và alpha format. |
| Text cache refresh | [render/render_layer](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/lib.rs#L131-L143). | Refresh nhận mutable TextLayer, cập nhật surface; không dùng cache cũ để chứng minh native shaping. |
| Vector raster | [fill/stroke_rasterizer](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/vector/src/lib.rs#L41-L83). | Flatten path → polygons → scan coverage; không phải Canvas2D API. |
| Layer → pixels | [render_content](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/compose/src/content.rs#L63-L101). | Text/shape vào generic `layer.surface()`, không gọi TextEngine trong compositor. |
| Compose | [render](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/compose/src/render.rs#L22-L85), [Buffer/to_rgba8](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/compose/src/lib.rs#L86-L123). | CPU/native Rayon, WASM sequential; float straight alpha → RGBA8 copy. |
| Flat image | [document_to_image](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/io/src/flat.rs#L374-L496). | Native-depth single raster shortcut hoặc render bands; ICC/meta/export mode policy. |
| PNG | [codec dispatch](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/codecs/src/lib.rs), [PNG implementation](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/codecs/src/codecs/png.rs). | Encoder nhận pixels, không layout/shaping. Nén khác không đồng nghĩa RGBA khác. |

**Cache boundary là điểm quan trọng:** model cho phép raster appearance từ PSD
hoặc text engine. `Layer::surface()` trả TextLayer.cache; compose chỉ đọc surface.
Do đó convert PSD→PNG thành công có thể chỉ dùng pixel lưu sẵn của Photoshop.
Spike phải tạo/refresh text từ model bằng font bytes đã pin, ghi cache provenance,
thay text/font/scale và chứng minh cache invalidation. Đây là suy luận trực tiếp
từ các API trên, chưa chạy PhotoCraft.

## Crate graph tối thiểu từ manifests

```text
photocraft-text
  → geom, color→cms, raster→geom/color/serde, doc→geom/color/raster/serde_json
  → psd→thiserror/flate2; parley; skrifa; flate2; native rayon
photocraft-vector → geom/color/raster/doc; native rayon
photocraft-compose → geom/color/raster/doc/vector; native rayon
photocraft-codecs → image/png/tiff/exr/half/flate2/image-webp/zune-jpeg/
                    jpeg-encoder/craft-jpeg/...; optional avif/heif
photocraft-io → compose/text/vector/codecs + algo/raw/affinity/paint/format/PSD/...
photocraft-cli → engine/automation/io/format/codecs/...
```

Nguồn: [text manifest](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/Cargo.toml#L12-L45),
[compose](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/compose/Cargo.toml#L10-L19),
[vector](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/vector/Cargo.toml#L14-L21),
[codecs](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/codecs/Cargo.toml#L13-L53),
[io](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/io/Cargo.toml#L20-L62),
[CLI](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/apps/photocraft-cli/Cargo.toml).

Đây là graph khai báo, **không phải `cargo tree` resolved cho adapter mới**.
Chọn compose/text/vector không tự kéo gpu/ui-egui/eframe/wgpu. Chọn CLI hoặc io
kéo nhiều format/editor support hơn. Cargo git dependency ghim `rev` là hướng
khả thi; không kết luận “không publish nên không dùng được”. Git checkout toàn
repo khác với số crate thực compile/link, cần đo trong spike.

Đọc [Cargo.lock upstream](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/Cargo.lock)
cho thấy Parley0.11.1, fontique0.11.1, HarfRust0.12.0, skrifa0.44.0,
read-fonts0.41.0, Rayon1.12.0; resvg0.45.1/tiny-skia0.11.4 thuộc graph toàn app.
Spike OpenCanvas hiện dùng HarfRust0.15.0/skrifa0.49.0/tiny-skia0.12.0.
Khác version đủ để không giả định identical glyph positions hoặc copy type trực
tiếp giữa hai graph. Upstream lock **không tự khóa dependency graph** khi
PhotoCraft được thêm như library; adapter cần Cargo.lock và features riêng.

## Build/font/cache coupling

- Text build script luôn nén bốn TTF từ `../../assets/fonts`; không chỉ một thư
  mục `src` là build được. `CRAFT_FONTS_DIR/REQUIRED` là optional build input,
  phải ghi rõ unset hay pin manifest/hash. [build.rs](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/build.rs#L22-L61).
- Constructor `TextEngine::new()` có thể tránh host scan; shared singleton native
  mặc định scan, trừ `PHOTOCRAFT_NO_SYSTEM_FONTS`. Mac target bật CoreText backend;
  native bật complex-scripts/dictionaries, WASM không bật. Đây là **build
  features** khác với lựa chọn constructor lúc chạy. [API](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/lib.rs#L149-L166),
  [manifest](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/Cargo.toml#L18-L41).
- FontDb có source/face/PostScript/traits/alias caches; registration clear một số
  maps và tăng generation. Không thấy public byte-budget/eviction cho toàn
  FontDb trong API đã đọc. Không suy Parley internal cache là unbounded toàn bộ;
  cần đo đúng pinned graph. [FontDb](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/fonts.rs#L189-L204),
  [registration](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/text/src/fonts.rs#L293-L330).
- Compose có effect-map cache bounded theo bytes, public setter/purge; key có
  tile pointers/Weak và model state, thích hợp process-local. Không thay cho
  durable content hash/job key của OpenCanvas. [fx_cache](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/compose/src/fx_cache.rs#L9-L14),
  [budget API](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/crates/compose/src/fx_cache.rs#L84-L127).

## License, pin và maintenance

Workspace code MIT OR Apache-2.0; giữ copyright/license lựa chọn phù hợp khi
vendor, ghi upstream SHA, file origins, local modifications và upgrade notes.
Không mang brand/logo/trademark assets: NOTICE loại chúng khỏi open-source
license. Inter/JetBrains Mono là OFL1.1; craft-fonts có license từng file riêng.
[Cargo metadata](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/Cargo.toml#L6-L10),
[NOTICE](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/NOTICE#L1-L28),
[ATTRIBUTION](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/ATTRIBUTION.md#L1-L13).

License root không bao phủ mọi transitive dependency/font/corpus. Trước ship,
inventory resolved graph đúng features, licenses/notices và font hashes; nếu
copy riêng raster không cần mang fonts/UI assets. Đồ họa user import giữ quyền
riêng; không lấy system fonts macOS phân phối vào Docker theo tiện lợi.
`craft-jpeg` dùng git craft-libs pin khác, nên chọn codecs cũng thêm nguồn phải
pin/review. [workspace dependencies](https://github.com/storytold/photocraft/blob/5f9b4391874aabc9deb2fcb905267613e4d93ec8/Cargo.toml#L41-L55).

Với direct-crate: review API/behavior diff và regenerate corpus khi nâng SHA;
không tự theo `main`. Với vendor: OpenCanvas là người giữ bugfix/fuzz/portability,
không coi chép một file là xong maintenance. Với Parley/skrifa trực tiếp: update
cùng shaping/parser versions và run representation; không shape bằng một graph
rồi vẽ bằng graph khác chỉ vì type names giống nhau.
