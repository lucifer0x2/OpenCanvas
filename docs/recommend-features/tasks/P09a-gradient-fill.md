# P09a — `fill` nhận gradient linear/radial (kể cả trong fill style, token màu trong stop)
Ưu tiên: P1 · Kích cỡ: ~80 phút dev · Phụ thuộc: B02 (đã merge `51f6368`); nên làm **sau P01** (vẽ trong `renderer/paint.ts`) và tuần tự với P04a/P05 vì cùng sửa vòng vẽ · Nguồn: ghi chú người dùng mục "Thiếu tính năng → Gradient" và "Kết quả" (`/Users/luis/Documents/opencanvas.md:68,95`)

Phase a của P09. Phase b (`P09b-box-shadow.md`) thêm shadow outer/inset; phase c (`P09c-blur-backdrop.md`) thêm blur và backdrop blur. P09a tự đủ giá trị: nút, icon tile và glow nền vẽ đúng gradient thay vì màu trung bình hay 8 vòng tròn.

## Vấn đề

- `fill` chỉ nhận một màu. Agent designer phải lấy màu trung bình cho nút và icon tile; glow nền màn Welcome phải giả bằng 8 vòng tròn alpha thấp ("glow là vòng tròn đồng tâm").
- AI coder đọc `get_design_context` thấy 8 node tròn thay vì một `radial-gradient`, nên dựng sai DOM và sai hình.
- Ảnh render lệch app thật ở mọi nút chính (QueryNest: "gradient thành màu phẳng").

## Bằng chứng

Code ở `night/integration@c3ab012`:

**Validator, catalog, resolver (storage)**

| Vị trí | Hiện trạng |
| --- | --- |
| `crates/storage/src/props_catalog.rs:89` | `fill`: "#RGB… a color token {tokenId} or a fill style {styleId}". Alias: `color, textColor, background, backgroundColor, fillColor, bg`. |
| `crates/storage/src/validate/properties.rs:10, 28` | `properties()` resolve trước (`:10`), rồi `"fill" => color(v)?` chỉ nhận chuỗi hex (`values.rs:48-64`). |
| `crates/storage/src/validate/refs.rs:96-107` | `fill` resolve theo `"color"`. Gradient object (≥ 2 key) bị `resolve_value` (`:6-12`) trả `field:"ref"`, "A typed reference must contain exactly one ID": agent không biết là gradient chưa được hỗ trợ. |
| `crates/storage/src/validate/refs.rs:28-49` | `{styleId}` của fill style resolve bằng `resolve_value(&style.value, "color")`, nên style chỉ chứa được màu. |
| `crates/storage/src/validate/node.rs:10-19, 34-42` | Fill style và `fill` trong text style đều validate là màu. |
| `crates/server/src/schemas/operations.rs:41` | `upsert_style.value`: `oneOf [string maxLength 16, object maxProperties 50]`. **Gradient object đã lọt schema MCP hiện tại**, không phải sửa `tools/list`. Chuỗi CSS `linear-gradient(...)` thì không lọt (> 16 ký tự). |
| `crates/server/src/reads/compact.rs:75-96` | Walker compact đệ quy qua object và mảng, nên `{tokenId}` trong `fill.stops[i][1]` tự vào danh sách `tokens`. |

**Layout và scene**

- `crates/layout/src/model.rs:70`: `Props.fill: Option<String>`. Layout input được deserialize từ resolved props (`crates/server/src/jobs/input.rs:10`), nên một fill object sẽ làm **hỏng cả frame** ở bước layout nếu chỉ nới validator.
- `model.rs:156`: `SceneNode.fill: String`; `engine.rs:139-145` mặc định `"transparent"` (text `#182230`).
- `lib.rs:13-14`, `renderer/types.ts:1-2`: `scene-1`/`web-1`; `drawScene` ném `PIPELINE_MISMATCH` khi lệch (`renderer/drawing.ts:83-90`).

**Renderer**

- `renderer/drawing.ts:21-26`: `context.fillStyle = node.fill` rồi `roundRect().fill()`. Sau P01 là `paintFill` trong `renderer/paint.ts` (nhánh `night/P01@06ae746`).
- Gán `fillStyle` bằng object sai kiểu thì canvas **bỏ qua im lặng** và giữ màu cũ. Vì vậy scene mới vẽ bằng renderer cũ sẽ sai màu mà không báo lỗi; cần bump version.
- Viewer không đọc `scene.fill` (`rg` trong `web/src` chỉ thấy `CanvasView.tsx:99` là prop Konva). Server cũng không đọc fill của scene.

**Prototype đo trên Chromium pinned của repo (chromium-1248)**

Script ở scratch, không commit: `/private/tmp/claude-501/-Volumes-MySpace-Workspace-OpenCanvas/a47874fe-c598-4ad4-a1cc-0f72b40e19ca/scratchpad/p09/` (`proto.js` thuật toán, `probe.mjs` so toàn vùng, `oracle.mjs` giá trị điểm AC, `perf2.mjs` hiệu năng).

So canvas prototype với `div` CSS cùng hình, nền trắng, toàn bộ vùng 480×300 (mọi kênh RGB):

| Ca | Mô tả | dsf 1: max lệch | dsf 2: max lệch |
| --- | --- | --- | --- |
| G1 | linear 180°, nút 160×48 radius 8 | 0 | 1 |
| G2 | linear 135° trên 240×100 (góc "magic corners") | 0 | 1 |
| G3 | linear 90° `#ef4444` → `#0000` | 0 | 1 |
| G4 | stop có alpha `#ff000080` → `#0000ffcc` | 1 | 1 |
| G5 | radial mặc định (ellipse farthest-corner) 300×150 | 0 | 1 |
| G6 | glow radial tâm `[0.5,0]`, `#6366f159` → trong suốt ở 0.7 | 1 | 1 |
| G7 | radial tròn 120px tâm `[0.3,0.4]`, 3 stop, radius 16 | 0 | 8 (≤ 9 kênh, chỉ ở mép AA góc bo) |
| G8 | linear 45°, 3 stop, 120×220 | 0 | 1 |

**Phát hiện quan trọng (premultiplied):** canvas nội suy stop **không premultiplied**, CSS thì premultiplied. Không sửa thì G3 lệch tới **60/255** (điểm giữa: CSS `247,162,162`, canvas thô `187,145,145`, viền tối kiểu "xám bẩn"); G4 lệch 19. Prototype viết lại stop (mục Giải pháp 3) cho lệch ≤ 1.

**Hiệu năng (headless, software canvas, trung vị 15 lần, đã flush):**
- 40 nút linear gradient: 0,7 ms @1, 2,0 ms @2 (màu phẳng cùng hình: 0,3 / 1,6 ms).
- Radial phủ 1440×900: 4,3 ms @1, 17 ms @2. Glow thật cỡ 600×400: ~3 ms @2. Chi phí theo diện tích tô, giống CSS.

**Tham khảo**

- CSS Images 3/4: `linear-gradient(<angle>, …)` với 0deg = lên trên, 90deg = sang phải, mặc định 180deg; độ dài đường gradient = `|w·sin θ| + |h·cos θ|` (https://www.w3.org/TR/css-images-3/#linear-gradients). `radial-gradient` mặc định `ellipse farthest-corner at center`; ellipse farthest-corner giữ tỉ lệ của farthest-side nên `rx = √2·max(cx, w−cx)`, `ry = √2·max(cy, h−cy)` (đo khớp G5, G6). Nội suy mặc định trong premultiplied sRGB (https://www.w3.org/TR/css-images-4/#coloring-gradient-line).
- Figma REST (https://developers.figma.com/docs/rest-api/file-node-types/): paint `GRADIENT_LINEAR/RADIAL` với `gradientHandlePositions` (3 điểm chuẩn hoá theo bounding box) và `gradientStops [{position, color{r,g,b,a}}]`. Handle chuẩn hoá theo hộp nên một góc 45° trên hộp không vuông bị lệch; coder phải tự đổi ra góc CSS bằng lượng giác.
- Penpot (https://doc.plugins.penpot.app/interfaces/Fill): `fillColorGradient {type: linear|radial, startX, startY, endX, endY, width, stops:[{color, opacity, offset}]}`, cũng dạng điểm đầu/cuối chuẩn hoá.
- Canvas `createLinearGradient`/`createRadialGradient` chỉ có hình tròn; ellipse cần scale transform lúc `fill()` (gradient diễn giải theo transform tại thời điểm tô, path theo transform lúc dựng).

## Giải pháp

### 1. Prop authored: `fill` nhận thêm một gradient object

```json
{"fill": {"type": "linear", "angle": 180, "stops": [[0, "#6366f1"], [1, "#8b5cf6"]]}}
{"fill": {"type": "linear", "stops": [[0, {"tokenId": "brand-500"}], [1, {"tokenId": "brand-700"}]]}}
{"fill": {"type": "radial", "center": [0.5, 0], "stops": [[0, "#6366f159"], [0.7, "#6366f100"]]}}
{"fill": {"type": "radial", "center": [0.3, 0.4], "radius": 120, "stops": [[0, "#22c55e"], [1, "#052e16"]]}}
{"fill": {"type": "radial", "radius": [300, 120], "stops": [[0, "#fff"], [1, "#0000"]]}}
```

Dạng 1 là nút của người dùng; dạng 3 là glow nền Welcome (thay 8 vòng tròn).

| Key | Bắt buộc | Giá trị | Map CSS |
| --- | --- | --- | --- |
| `type` | có | `"linear"` hoặc `"radial"` | `linear-gradient` / `radial-gradient` |
| `angle` | linear, mặc định 180 | số −360..360, độ; 0 = lên trên, 90 = sang phải | `<angle>deg` |
| `center` | radial, mặc định `[0.5,0.5]` | `[x, y]`, phân số của bounds, mỗi số −10..10 | `at X% Y%` |
| `radius` | radial, mặc định CSS | bỏ = ellipse farthest-corner; số > 0 = hình tròn bán kính px; `[rx, ry]` = ellipse px | `""` / `circle Rpx` / `Rxpx Rypx` |
| `stops` | có | mảng 2..16 phần tử `[position, color]`; `position` 0..1 không giảm; `color` hex `#RGB/#RGBA/#RRGGBB/#RRGGBBAA` hoặc `{tokenId}` loại color | `color P%` |

**Luật chung**

- Key khác bị từ chối (`linear` không nhận `center/radius`, `radial` không nhận `angle`).
- Gradient hợp lệ trên frame, group, rectangle, image (fill nền dưới ảnh) và page. **Không hợp lệ trên text** trong phase này: canvas không dùng glyph làm clip, CSS cần `background-clip:text`. Lời nhắn chỉ cách sửa (mục 5).
- Màu tràn ngoài stop đầu/cuối giữ màu stop đó (CSS "pad"). Nội suy premultiplied như CSS.
- Authored giữ nguyên, không chèn mặc định. `resolvedProps.fill` cùng shape, chỉ thay `{tokenId}`/`{styleId}` bằng literal.

**Fill style chứa gradient.** `upsert_style {kind:"fill", value:<gradient>}` hợp lệ (schema MCP đã cho phép object). Node dùng `fill:{styleId}` hoặc `styleId` (fill style) nhận gradient đó. Không lồng `styleId` trong stop (giữ luật "Style aliases are not supported"). Ngữ cảnh chỉ nhận màu đặc (stroke của P04a, `color` của shadow P09b) gặp fill style là gradient thì lỗi rõ ràng (mục 5).

**Vì sao chọn dạng này**

| Phương án | Đánh giá |
| --- | --- |
| **Object `{type, angle, stops:[[p, c]]}`** (chọn) | Gần CSS (góc độ, vị trí %, ellipse mặc định), token màu được trong stop, compact walker tự gom token, lọt schema `upsert_style` hiện có. ~72 byte cho một nút. |
| Chuỗi CSS `"linear-gradient(180deg, …)"` | LLM thuộc lòng, ~60 byte. Nhưng cần parser ở Rust và TS cho nhiều cú pháp (`to right`, `turn`, `rgba()`, tên màu, color hint, `closest-side`, nhiều lớp); không gắn được token; đổi theme không còn tham chiếu trong authored; không vừa `upsert_style.value` (maxLength 16). |
| Handle chuẩn hoá kiểu Figma/Penpot | Góc phụ thuộc tỉ lệ hộp; coder phải tự đổi; ~250 byte. |
| Stop dạng object `{position, color}` | Tốn gấp ~2,3 lần (`[0,"#6366f1"]` 14 byte so với 33 byte), không thêm thông tin. Thứ tự `[position, color]` khớp `addColorStop(offset, color)`; validator nhận ra thứ tự đảo và chỉ cách sửa. |

**Alias catalog** (B02 gợi ý `fill`): thêm `gradient`, `backgroundImage`, `fills`, `linearGradient`, `radialGradient`.

**Chi phí token:** glow Welcome hiện là 8 `create_node` (~1,2 KB op, 8 node trong mọi lần đọc) → một prop ~90 byte.

### 2. Resolve và validate (storage)

- File mới `crates/storage/src/validate/paint.rs`:
  - `Candidate::resolve_paint(&self, v) -> Result<Value>`: chuỗi → `resolve_value(v,"color")`; `{tokenId}` → như cũ; `{styleId}` → style fill, giá trị style đi qua `resolve_paint` (màu hoặc gradient); object có `type` → resolve từng `stops[i][1]` bằng `resolve_value(_, "color")`, giữ nguyên các key khác (shape sai để validator báo).
  - `paint(v: &Value, kind: &NodeKind) -> Result<()>` trên giá trị đã resolve: màu hex hoặc gradient đúng bảng ở mục 1.
- `refs.rs:97`: `"fill"` đi qua `resolve_paint` thay vì `resolve_value(_, "color")`. Nhánh `{styleId}` ở `refs.rs:28-49` (đường "chỉ màu") trả lỗi khi style là gradient.
- `properties.rs:28`: `"fill" => paint(v, &n.kind)?`.
- `node.rs:18, 41`: fill style và `fill` trong text style validate bằng `resolve_paint` (text style chứa gradient sẽ bị chặn khi áp lên text, ở `properties.rs`).
- `props_catalog.rs:89`: mô tả `fill` thêm một vế gradient (dòng props.md sinh tự động).

### 3. Scene và renderer

**Rust (`crates/layout`)**

- File mới `crates/layout/src/model/fill.rs` (P04a đã dùng tên `model/paint.rs` cho stroke):
  - `#[serde(untagged)] enum Paint { Solid(String), Gradient(Gradient) }`;
  - `#[serde(tag = "type", rename_all = "lowercase")] enum Gradient { Linear { angle: f32, stops: Vec<(f32, String)> }, Radial { center: [f32; 2], radius: Option<Radius>, stops } }`, `Radius` untagged số hoặc `[f32;2]`;
  - Props nhận dạng authored (`angle`, `center` optional) rồi chiếu ra scene **đã chuẩn hoá**: linear luôn có `angle`, radial luôn có `center` và `radius` (`null` = mặc định CSS).
- `Props.fill: Option<Paint>`, `SceneNode.fill: Paint`. Node màu đặc serialize y như cũ (chuỗi), nên scene không có gradient **byte-identical** trừ `provenance`.
- `validation.rs`: thêm kiểm số hữu hạn cho angle/center/radius/position (phòng thủ, như các prop số khác).
- Bump `SCENE_VERSION`/`RENDERER_VERSION` lên số kế tiếp ở cả Rust và `renderer/types.ts` (P04a lên `scene-2/web-2`; nếu P04a đã merge thì P09a dùng `scene-3/web-3`). Sửa chuỗi hard-code `web/tests/state.test.ts:69-70`, `web/tests/fixture-data.mjs:79-80`.

**TS (`renderer/fill.ts`, mới)**

```ts
export type Paint = string | LinearPaint | RadialPaint;
/** Sets fillStyle for bounds b and fills the current path; radial ellipses scale the context only for the fill. */
export function fillPath(context: CanvasRenderingContext2D, paint: Paint, b: Bounds): void;
```

1. **Linear:** `θ = angle·π/180`, `d = (sin θ, −cos θ)`, `L = |w·sin θ| + |h·cos θ|`, tâm `c` của bounds; `createLinearGradient(c − d·L/2, c + d·L/2)`.
2. **Radial:** tâm `(b.x + cx·w, b.y + cy·h)`; `[rx, ry]` = `radius` hoặc mặc định `√2·max(cx·w, w − cx·w)`, `√2·max(cy·h, h − cy·h)`; `createRadialGradient(0,0,0, 0,0, ry)`, rồi `translate(tâm)`, `scale(rx/ry, 1)` chỉ quanh lời gọi `fill()`. Dùng `scale(rx, ry)` với bán kính 1 thì lệch tới 9/255 ở mép AA góc bo (đo G7), nên không dùng.
3. **Premultiplied:** viết lại danh sách stop trước `addColorStop`:
   - stop alpha 0: thay bằng hai stop cùng vị trí mang RGB của stop trước và stop sau (alpha 0). Kết quả đúng tuyệt đối với nội suy premultiplied.
   - hai stop kề nhau cùng khác 0 và khác alpha: chèn 7 stop phụ tính premultiplied (lệch đo được ≤ 1).
4. Vị trí stop đã không giảm (validator), kẹp vào [0,1].

`paint.ts` (`paintFill`, sau P01) gọi `fillPath(context, node.fill, b)` sau `roundRect`; chuỗi màu giữ đúng đường cũ (`fillStyle = string`) để byte-identical. Text giữ `fillStyle = node.fill` (luôn là chuỗi vì validator chặn gradient trên text).

### 4. `get_design_context` cho AI coder

- `props.fill` trả nguyên văn authored (token trong stop còn nguyên); `resolvedProps.fill` cùng shape với hex.
- `compact:true`: token trong stop vào `tokens`; fill style chứa gradient vào `styles`.
- Thêm vào bảng `implement-frame.md:25-36`:
  - `fill {type:"linear", angle, stops}` → `background: linear-gradient(<angle>deg, <c0> <p0·100>%, …)`;
  - `fill {type:"radial", center, radius?, stops}` → `background: radial-gradient([circle R px | Rx px Ry px] at <cx·100>% <cy·100>%, …)`; bỏ `radius` = mặc định CSS;
  - ghi rõ: nội suy giống CSS mặc định, không cần `in srgb`.
- `author-frame.md`: một dòng "dùng `fill` gradient cho nút, tile, glow; không lấy màu trung bình hay chồng vòng tròn".

### 5. Lỗi validate (theo format B02)

`field` theo đường dẫn trong prop, kèm `nodeId/nodeKind` như B02 làm cho lỗi giá trị. Không dùng `props.fill` cho lỗi giá trị (B02 dành `props.<key>` cho lỗi key).

| Ca | field | message |
| --- | --- | --- |
| gradient trên text | `fill.type` | `Gradient fill is not supported on text yet; use a solid color, or put the gradient on a frame behind the text` |
| thiếu/sai `type` | `fill.type` | `Gradient type must be linear or radial, e.g. {"type":"linear","angle":180,"stops":[[0,"#6366f1"],[1,"#8b5cf6"]]}` |
| key lạ (`colors`, `angle` trên radial…) | `fill.<key>` | `Unsupported linear gradient key 'colors'; linear takes type, angle and stops` |
| stops < 2, > 16, không phải mảng | `fill.stops` | `Gradient stops must be 2 to 16 [position 0..1, color] pairs` |
| stop đảo `["#fff", 0]`, position ngoài 0..1 hoặc giảm | `fill.stops[1]` | `Each stop is [position 0..1, color] with non-decreasing positions, e.g. [0.5, "#8b5cf6"]` |
| màu stop sai | `fill.stops[1]` | `Stop color must be #RGB, #RGBA, #RRGGBB, #RRGGBBAA or a color token {"tokenId"}` |
| `center` sai | `fill.center` | `Radial center is [x, y] as fractions of the node bounds, e.g. [0.5, 0]` |
| `radius` sai | `fill.radius` | `Radial radius is a pixel number (circle) or [rx, ry] (ellipse); omit it for the CSS default` |
| style gradient dùng ở chỗ chỉ nhận màu | `<prop>.color` | `<Prop> color must be a solid color; fill style '<id>' is a gradient` |

## Phạm vi file

| File | Việc | Dòng (hiện → ước) |
| --- | --- | --- |
| `crates/storage/src/validate/paint.rs` (mới) | `resolve_paint`, `paint()` kiểm gradient, message | ~140 |
| `crates/storage/src/validate/refs.rs` | `fill` → `resolve_paint`; chặn gradient ở đường chỉ-màu | 156 → ~162 |
| `crates/storage/src/validate/properties.rs`, `node.rs`, `validate.rs` | gọi `paint`, `resolve_paint`; `mod paint` | +4 |
| `crates/storage/src/props_catalog.rs` | mô tả + alias `fill` | ±1 dòng |
| `crates/layout/src/model/fill.rs` (mới) | `Paint`, `Gradient`, `Radius`, chuẩn hoá, kiểm hữu hạn | ~90 |
| `crates/layout/src/model.rs`, `engine.rs:139-145`, `validation.rs`, `lib.rs:13-14` | kiểu `fill`, mặc định `Paint::Solid`, gọi kiểm, bump version | 210→~211, 185→~186, 130→~132 |
| `renderer/fill.ts` (mới) | `fillPath`, toạ độ linear/radial, viết lại stop premultiplied | ~100 |
| `renderer/paint.ts` (sau P01; nếu chưa có thì `drawing.ts`) | `paintFill` gọi `fillPath` | +2 |
| `renderer/types.ts`, `renderer/index.ts` | `Paint` types, bump version, export | +12 |
| `web/tests/state.test.ts`, `web/tests/fixture-data.mjs` | chuỗi version | 2 dòng mỗi file |
| `crates/storage/tests/gradient_props.rs` (mới) | AC1–AC3 | ~150 |
| `crates/layout/src/fill_tests.rs` (mới, `#[cfg(test)]`) | AC4 (shape scene, byte-identical) | ~70 |
| `spikes/pipeline/gradient.test.mjs` (mới) | AC6–AC10 pixel, so CSS | ~170 |
| `spikes/pipeline/css-oracle.mjs` | dùng lại của P05 nếu đã merge; nếu chưa thì tạo (dựng div tuyệt đối, chụp, đọc pixel) | ~40 |
| `crates/server/tests/gradient_context.rs` (mới) | AC5 | ~90 |
| `crates/server/skills/implement-frame.md`, `author-frame.md` | 2 dòng bảng + 1 dòng hướng dẫn | +3 |
| `docs/CONTRACT.md`, `docs/VIEWER.md`, `spikes/pipeline/README.md` | kiểu `SceneNode.fill`, version | vài dòng |

`model.rs` đã 210 dòng (ngưỡng 250): chỉ đổi kiểu một field, kiểu mới nằm ở `model/fill.rs`. Không file mã nào vượt 200 dòng mới.

## Tương thích MCP

- `tools/list` không đổi: `props` là object tự do, `upsert_style.value` đã nhận object (`schemas/operations.rs:41`). `crates/server/tests/tool_schemas.rs` phải pass không sửa.
- Mọi `fill` cũ (hex, token, fill style màu) giữ nguyên nghĩa; resolvedProps và scene của chúng không đổi.
- Thêm nghĩa: `fill` object có `type`, fill style giá trị gradient. Client từng đọc `resolvedProps.fill` như chuỗi sẽ gặp object chỉ khi design dùng gradient.
- `sceneKey` đổi một lần vì bump version; tab viewer cũ báo `PIPELINE_MISMATCH` tới khi reload.
- Tài liệu: props.md (tự sinh), 2 skill, CONTRACT, VIEWER, README pipeline.

## AC

Toạ độ frame-local, scale 1. Root frame 480×300, `fill:"#ffffff"`, layout `free`; node đặt tại `x:40, y:40`. Giá trị kỳ vọng lấy từ CSS Chrome cùng phiên bản (cột "css" của `oracle.mjs`); sai số mỗi kênh RGB ≤ 2 trừ khi ghi khác.

**AC1 — Validator nhận.** `apply_batch` commit được (revision +1):
- rectangle với 5 ví dụ ở mục 1 (gồm stop `{tokenId}` loại color);
- `upsert_style {id:"btn-primary", kind:"fill", value:<linear>}` và rectangle `fill:{styleId:"btn-primary"}`, frame có `styleId:"btn-primary"`;
- gradient trên frame, group, image.

**AC2 — Validator từ chối** (mỗi ca một test, đúng `code:"VALIDATION_FAILED"`, `field`, message chứa cụm in nghiêng):
- text `fill:<linear>` → `fill.type`, *on text yet*;
- `{type:"conic",…}` → `fill.type`; `{angle:90, stops:[…]}` → `fill.type`;
- linear có `center` → `fill.center`, *linear takes type, angle and stops*;
- `stops:[[0,"#fff"]]` → `fill.stops`; 17 stop → `fill.stops`;
- `stops:[["#fff",0],[1,"#000"]]` → `fill.stops[0]`, *[position 0..1, color]*;
- `stops:[[0.6,"#fff"],[0.4,"#000"]]` → `fill.stops[1]`;
- `stops:[[0,"red"],[1,"#000"]]` → `fill.stops[0]`;
- `radius:-1` → `fill.radius`; `center:[0.5]` → `fill.center`;
- `stops:[[0,{"styleId":"x"}],[1,"#000"]]` → `fill.stops[0]`;
- nếu P04a đã merge: `stroke:{color:{styleId:"btn-primary"}}` → `stroke.color`, *is a gradient*.

**AC3 — Catalog.** Test B02 `validator_accepts_exactly_the_catalog_keys_per_kind` pass không sửa logic. Dòng `fill` trong props.md chứa `"linear"` và `"radial"`. `fill` gõ thành `gradient` hoặc `backgroundImage` → lỗi key với `suggestion:"fill"`.

**AC4 — Scene và tương thích.**
- Toàn bộ test hiện có pass sau khi chỉ đổi chuỗi version.
- Scene của fixture `spikes/pipeline/fixtures.mjs` trước/sau bằng nhau sau khi bỏ `provenance` và `sourceSceneKey`; `fill` vẫn là chuỗi.
- Scene của rect linear có `fill` = `{"type":"linear","angle":180,"stops":[[0,"#6366f1"],[1,"#8b5cf6"]]}`; radial không ghi `center/radius` thì scene có `"center":[0.5,0.5],"radius":null`.
- `drawScene` với scene có version cũ ném `PIPELINE_MISMATCH`.

**AC5 — `get_design_context` qua MCP.** Token `brand-500` có nhánh theme `night`; rect `fill:{type:"linear",stops:[[0,{tokenId:"brand-500"}],[1,"#000000"]]}`:
- `props.fill` đúng nguyên văn authored, ref còn nguyên;
- `resolvedProps.fill.stops[0][1]` = giá trị default của token; khác giá trị đó ở `theme:"night"`;
- `compact:true`: `tokens` chứa `brand-500`. Rect `fill:{styleId:"btn-primary"}` → `styles` chứa `btn-primary`, `resolvedProps.fill.type == "linear"`.

**AC6 — Pixel linear** (nút G1: 160×48, radius 8, angle 180, `#6366f1`→`#8b5cf6`): (120,40) = `99,101,241`; (120,64) = `119,96,243`; (120,87) = `139,92,246`; (40,40) trắng (ngoài cung góc bo).
G2 (240×100, angle 135, `#0ea5e9`→`#f43f5e`, radius 0): (40,40) = `14,164,232`; (160,90) = `129,113,163`; (279,139) = `243,63,94`; (279,40) = `177,93,135`.

**AC7 — Premultiplied.** G3 (240×60, angle 90, `#ef4444`→`#0000`): (160,70) = `247,162,162` (canvas thô cho `187,145,145`); (40,70) = `239,68,68`; (279,70) trắng.

**AC8 — Pixel radial.**
- G5 (300×150, mặc định, `#fde68a`→`#7c3aed`): (190,115) = `252,229,139`; (115,115) = `208,169,173`; (40,40) = `124,58,236`.
- G6 glow (400×220, `center:[0.5,0]`, `#6366f159`@0 → `#6366f100`@0.7): (240,40) = `200,201,250`; (240,117) = `220,221,252`; (60,60) = `250,250,254`; (240,259) trắng.
- G7 (300×200, radius 16, `center:[0.3,0.4]`, `radius:120`, 3 stop `#22c55e/#16a34a/#052e16` ở 0/0.5/1): (130,120) = `34,196,93`; (190,120) = `22,162,73`; (250,120) = `5,46,22`.

**AC9 — Khớp CSS toàn vùng.** Với G1–G8 (bảng Bằng chứng), dựng `div` CSS cùng hình qua `worker.runtime.browser.newPage()`. So `worker.render` ở scale 1 và 2 (CSS `deviceScaleFactor` 1 và 2): mọi pixel nằm cách mép hộp ≥ 1 px lệch ≤ 2 mỗi kênh; ở scale 2 chấp nhận tối đa 16 kênh lệch ≤ 10 tại mép AA góc bo (đo: G7 9 kênh, max 8).

**AC10 — Byte-identical khi không có gradient.** `fixture(420)` ở scale 1 và 2 và cảnh `geometry.test.mjs`: RGBA giống từng byte renderer build từ `night/integration` (cách so của P05: bundle cũ qua `resourceHandler`). Viewer và PNG worker vẫn PNG byte-identical (`pipeline.test.mjs`) với fixture có thêm 1 node gradient.

**AC11 — Kiểm live** trên server tạm: `upsert_style` gradient + rect dùng style + rect glow radial → `render_frame` `status:"ready"`, `diagnostics:[]`; PNG tải về có pixel (120,64) của nút ≈ `119,96,243` (±2).

**AC12 — Modular và kiểm tĩnh.** File mã mới ≤ 200 dòng; `paint.ts`/`drawing.ts` tăng ≤ 5 dòng; `cargo fmt --check`, clippy (lệnh ACCEPTANCE), `npm run check:renderer`, `npm --prefix web run build` pass.

## Cách kiểm

```bash
W=<worktree>; cd $W && export CARGO_BUILD_JOBS=4
cargo test --offline -p opencanvas-layout -p opencanvas-storage -p opencanvas-server
cargo clippy --offline -p opencanvas-layout -p opencanvas-storage -p opencanvas-server --lib --bin opencanvas-server -- -D warnings
cargo fmt --all --check
npm run check:renderer
PLAYWRIGHT_BROWSERS_PATH=$W/.runtime/browsers npm run test:pipeline     # gồm gradient.test.mjs
npm --prefix web test && npm --prefix web run build
wc -l crates/storage/src/validate/paint.rs crates/layout/src/model/fill.rs renderer/fill.ts renderer/paint.ts
```

**Test mới:** `gradient_props.rs` (AC1–AC3), `fill_tests.rs` (AC4), `gradient.test.mjs` (AC6–AC10, oracle CSS), `gradient_context.rs` (AC5).

**Prototype tham chiếu:** chạy `PLAYWRIGHT_BROWSERS_PATH=/Volumes/MySpace/Workspace/OpenCanvas/.runtime/browsers node probe.mjs G` và `node oracle.mjs` trong thư mục scratch ở mục Bằng chứng (`proto.js` là thuật toán đề xuất bằng JS, có thể bị dọn).

**Live (AC11):** `target/debug/opencanvas-server --port 0 --data-root $(mktemp -d)`; script node gọi `create_project` → `apply_batch` → `render_frame` → đọc PNG (mẫu `.night/qa/mcp-tools.mjs`); dọn process và data. Không đụng 3847.

## Rủi ro

- **Xung đột vòng vẽ và version:** P01 (`paint.ts`), P04a (stroke, bump `scene-2`), P05 (layer). P09a chỉ thêm một lời gọi trong `paintFill` và file mới, nên rebase nhẹ. Version: lấy số kế tiếp chưa dùng tại lúc merge.
- **Đổi kiểu `SceneNode.fill`** thành `string | object`: client ngoài repo đọc scene (qua `render_frame includeScene` của B04) có thể giả định chuỗi. Giảm thiểu: màu đặc giữ nguyên chuỗi; CONTRACT ghi rõ.
- **Ellipse rất dẹt** (`ry` gần 0 khi `center` nằm trên mép và hộp rất thấp): kẹp `ry ≥ 0.001`. Tâm trên mép dưới/trên vẫn đúng như CSS (G6 đo khớp).
- **Banding:** Chrome CSS và canvas cho cùng giá trị (lệch ≤ 1), không có dithering khác biệt trong đo.
- **Chi phí:** radial phủ toàn frame tốn ~17 ms @2 trên software canvas; ảnh hưởng frame cost viewer (P01 AC5). Glow thật nhỏ hơn (~3 ms). Ghi số đo vào `.night/dev/P09a.md`.
- **Export (P03a/P03b):** SVG export phải map gradient sang `<linearGradient>/<radialGradient>` (`gradientUnits="userSpaceOnUse"` với toạ độ tính như mục 3). Nếu P03b merge trước thì P09a/P03b thêm, hoặc phát diagnostic.

## Ngoài phạm vi / phase sau

- Gradient trên text (`background-clip:text`): vẽ glyph vào layer rồi `source-in` gradient. Cần layer của P05.
- `repeating-*`, `conic-gradient`, color hint, nội suy ngoài sRGB (`in oklab`), nhiều lớp nền (`fills:[…]`).
- Stroke gradient (P04a chỉ nhận màu đặc).
- Token loại "gradient" hoặc kích thước bằng token trong `radius`/`angle`. Fill style đã đủ để đặt tên gradient trong design system.
- Viewer Inspector hiện swatch gradient (hiện in JSON authored).
