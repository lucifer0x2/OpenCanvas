# 01 — Pipeline hiện có và ranh giới dependency

Khảo sát source ngày 2026-10-29 tại HEAD ghi ở [bản chính](../EXPORT-ENGINE-RESEARCH.md).
Các `path:line` dưới đây là bằng chứng tĩnh, không phải trace profiler.

## Luồng từ request đến pixel và viewer

```text
MCP render_frame / export_nodes, HTTP scene / node exports
  → Jobs enqueue + cache key + requested/background queue (Rust)
  → resolve tokens/theme + expand components → frame input + manifests
  → Rust runner → Node worker NDJSON (persistent child)
  → resource bytes/hash/decode trong Chromium
  → [layout-step process → Taffy cache miss → browser measureBatch] tối đa 6 pass
  → computed scene: bounds + text lines + provenance
  → apply visual state (Rust)
  ├─ frame PNG / subtree PNG → Canvas2D → PNG → base64 NDJSON → Rust bytes
  └─ subtree SVG → Rust markup + embedded original image bytes → download
HTTP scene → React/Konva shell → shared paintScene trong browser người dùng
```

| Điểm vào/khâu | Bằng chứng source | Ý nghĩa khi thay engine |
| --- | --- | --- |
| Job dedup/cache | `crates/server/src/jobs.rs:115`, `:125`, `:130` | Hash input, scale, pipeline identity, export settings; giữ immutable revision. |
| Resolve/instance expansion | `crates/server/src/jobs/input.rs:9`, `:24`, `:33` | Dùng `resolved_expanded_nodes`, manifests chỉ font/asset được dùng; không chuyển domain sang engine. |
| Queue | `crates/server/src/jobs/queue.rs:9`, `:11`, `:82` | Requested ưu tiên background, mỗi tier 8 queued; runner một worker. |
| Child transport | `crates/server/src/worker.rs:19`, `:46`, `:53` | Chạy `node worker/cli.mjs`, ID NDJSON, đọc tối đa 48 MiB; kill/reap. |
| Compute cho mọi loại job | `crates/server/src/jobs/runner.rs:65`, `:70`, `:75` | SVG cũng compute qua worker; sau compute mới phân nhánh format. |
| Launch browser | `worker/runtime.mjs:12`, `:53`, `:56`, `:66` | Node server allowlist + Playwright Chromium + import ESM renderer. |
| Layout loop | `worker/pipeline.mjs:99`, `:127`, `:138`, `:159` | `spawnSync(layout-step)` **mỗi pass**; batch measure qua `page.evaluate`. |
| Taffy | `crates/layout/src/engine.rs:10`, `:36`, `:63`, `:83`, `:110` | Callback chỉ đọc cache; thiếu metric trả pending, không publish scene placeholder. |
| Measurement key | `crates/layout/src/util.rs:33`, `:56`, `:58` | Text/font hash/family/face/size/lineHeight/width + renderer version; grid 1/64. |
| Scene contract | `crates/layout/src/model.rs:128`, `:151`, `:172` | Lines chỉ text/x/y/width, không glyph IDs/runs, fallback face hoặc outlines. |
| Frame PNG | `worker/pipeline.mjs:181`, `:188`, `:193` | `drawScene` rồi `canvas.toDataURL("image/png")`; không phải Playwright page screenshot. |
| PNG decode phía Rust | `crates/server/src/jobs/png.rs:4` | Decode base64, kiểm signature/IHDR/dimensions; không phải rasterizer hoặc encoder. |
| Export subtree | `crates/server/src/jobs/subtree.rs:8`, `:39`, `:49` | Root về (0,0), viewport theo layout bounds, bỏ ancestor opacity/clip. |
| PNG settings | `crates/server/src/jobs/export.rs:47`, `:94`, `:115` | Render từng setting, scale/target size và ceil rounding phải giữ. |
| SVG | `crates/server/src/jobs/export_svg.rs:80`, `crates/server/src/svg/elements.rs:8` | Markup Rust từ line list; không đo/render lại bằng browser ở bước serialize. |
| Viewer | `web/src/hooks/useScene.ts:45`, `:49`, `:53`; `web/src/components/CanvasView.tsx:155` | Load exact manifests, parity canvas `drawScene`, Konva sceneFunc gọi `paintScene`. |

## Text không phải một việc

`renderer/measurement.ts:23` tạo Canvas2D context, chọn exact manifest face,
`fontKerning="normal"`, dùng browser `measureText`. JS tự normalize CRLF, collapse
horizontal whitespace, LF thành paragraph, wrap theo từ; từ quá dài tách bằng
`Intl.Segmenter("vi", {granularity:"grapheme"})` (`:61`, `:81`). Width/height và
baseline được round grid 1/64; baseline hiện là `fontSize + (lineHeight-fontSize)/2`
(`:54`). Native text cần giữ wrapper này hoặc version hóa thay đổi semantics.

`renderer/paint.ts:31` dùng alphabetic baseline và `fillText` mỗi scene line;
không gọi measure/wrap ở drawing. Tuy nhiên **fillText vẫn shape glyph** từ string
trong browser, nên một line list giống nhau chưa bảo đảm glyph positions giống
native shaper. Taffy lo box layout, không tự shape/đọc font/raster glyph.

`renderer/resources.ts:33` fetch bytes và check SHA; `:79` tạo FontFace; `:94`
load/check font readiness; `:117` decode image. Font file không sẵn sàng là lỗi
resource; glyph thiếu trong font đã load vẫn có thể được browser fallback. Comment
“no implicit system fallback” ở `:62` không đủ làm chứng minh coverage glyph.

## Drawing và encoding

- `renderer/drawing.ts:19`: kiểm scene/renderer versions, manifests, strict
  readiness; `:40`: scale và pixel budget; `:64`: canvas = ceil(viewport×scale),
  clear transparent rồi paint.
- `renderer/paint.ts:9`, `:40`, `:98`: fill → text/image; image normalized crop,
  fit/contain/cover, rounded image clip; stroke sau subtree, dưới ancestor clip.
- `renderer/layers.ts:55`, `:77`, `:100`: group opacity cần offscreen layer,
  budget tổng live layer 16M pixels, vượt budget thì fold alpha và đếm fallback.
  Đây là hành vi có điều kiện cần test, không chỉ phép nhân alpha từng node.
- PNG encoder hiện thuộc Canvas/Chromium. Rust chỉ nhận encoded bytes. Có thể
  thử encode RGBA bằng Rust nhưng phải đo transfer RGBA; ảnh 16M pixels cần
  ~64 MB decimal cho **một** RGBA buffer, chưa layers/copies/encoder.

## SVG hiện tại không đủ làm intermediate tương đương PNG

`crates/server/src/svg/mod.rs:128` viết tree, group opacity và ancestor clip;
`elements.rs:43` embed image data URI, crop bằng nested viewBox.
`elements.rs:32` viết live `<text>` từng line, chỉ font family/weight/style/size,
không font bytes hoặc glyph positions. `jobs/export_svg.rs:90` báo
`SVG_TEXT_NOT_OUTLINED`; `:94` báo `SVG_STROKE_NOT_EXPORTED`.

Vì vậy resvg có thể đọc SVG này nhưng không phục hồi stroke đã bỏ hoặc font chưa
embed. Spike dùng serializer nội bộ đầy đủ hoặc painter trực tiếp; không đổi
ngầm SVG public thành outlined text vì đó là thay đổi output có chủ đích.

## Dependency runtime, frontend và build là ba tập khác nhau

| Tập | Hiện có | Khi bỏ Node/Chromium server |
| --- | --- | --- |
| Backend runtime | Rust server, SQLite/assets, layout-step, Node, Playwright, Chromium + OS libs | Có thể còn Rust native worker/C++/C libs/CPython tùy lựa chọn. |
| Browser viewer runtime | Browser người dùng, JS React/Konva/shared renderer, FontFace/Image/Canvas2D | Vẫn tồn tại nếu giữ viewer; không cần Node trên máy client. |
| Build/dev/test | Cargo, Node/npm, TS/esbuild/Vite, browser E2E | Native export không tự loại Node dùng build web hoặc Playwright test. |
| Docker hiện tại | `Dockerfile:14` layout debug + server release; `:24` web build; `:32` Node runtime; `:39` browser shell | Phải sửa fingerprint/packaging/launchers cùng worker; chỉ xóa package sẽ hỏng startup. |

`crates/server/src/fingerprint.rs:6` hash lockfiles/worker/renderer/layout binary
và chạy Node/Playwright để lấy runtime identity (`:30`). Identity chụp tại startup;
không có hot replacement. `scripts/app.mjs`, README launch profiles vẫn dùng Node
ngay cả nếu core backend sau này bỏ Node. Cần phân biệt direct binary launch với
developer convenience launcher.

## Hiện trạng và kế hoạch không đồng nghĩa

Đã đọc tasks 01–08 (đều đóng), đặc biệt Task03 multi-viewport và Task07 read costs.
Đường context nhiều viewport cũng chịu compute/worker; giữ async/bounded read.
`docs/recommend-features/README.md:19` ghi P10a chưa làm, khớp source heuristic;
P10a handoff muốn ceil width, font baseline, Linux hinting flag và `textWrap`.
P11a missing-glyph cũng planned. P03a PNG và SVG có code; viewer export **hiện là
POST** (`crates/server/src/export/http.rs:14`), không dùng GET đề xuất cũ của P03c.
P03d JPG/WebP/withinId và PDF vẫn planned; không được quảng cáo là hiện hỗ trợ.

`docs/ACCEPTANCE.md:14` ghi RGBA=0 trên fixture/runtime đã nghiệm thu. Đó là bằng
chứng pipeline hiện tại trên phạm vi đó; không phải native parity hoặc mọi OS.
`spikes/pipeline/RESULTS.md` có timing/cache pass cũ; nghiên cứu này không đo lại.

## Tối ưu có thể kiểm trước migration — đều là giả thuyết

1. Instrument queue/launch/resources/layout/measure/paint/encode/IPC riêng; total
   render không cho biết browser startup hay text đang chiếm thời gian.
2. Cache decoded resources theo bytes hash thay vì nạp/xóa FontFace ở mỗi compute
   và render (`resources.ts:85`); giữ invalidation, bounded memory, strict errors.
3. Giữ layout process lâu dài hoặc Rust gọi library trong native worker: bỏ spawn
   mỗi pass, không đổi Taffy callback pending/cache contract.
4. Giữ browser warm đã có; cân nhắc resource reuse và reuse computed frame cho nhiều
   node exports. Job export key khác nhau hiện không bảo đảm reuse root scene.
5. Đo base64/JSON copies trước khi đổi binary IPC. Tránh đổi browser canvas capture
   sang page screenshot vì scale/crop/background semantics có thể khác.
6. Chưa tăng concurrency/pool: 16M canvas/layers và font cache nhân theo worker;
   phải có RSS budget và saturation benchmark trước.
