# P09b — Prop `shadow`: nhiều lớp outer/inset, spread, như CSS `box-shadow`
Ưu tiên: P1 · Kích cỡ: ~90 phút dev · Phụ thuộc: B02 (đã merge); dùng `PropScope::Box` của P04a nếu đã merge (chưa thì tạo); nên **sau P01**, tuần tự với P04a/P05/P09a (vòng vẽ, version scene). Không phụ thuộc P09a. · Nguồn: ghi chú người dùng mục "Thiếu tính năng → Shadow" (`/Users/luis/Documents/opencanvas.md:67`), note #10 (keycap có shadow)

Phase b của P09. Phase a (`P09a-gradient-fill.md`): gradient. Phase c (`P09c-blur-backdrop.md`): blur/backdrop. P09b tự đủ giá trị: shadow card/modal, ring focus `0 0 0 1px`, highlight inset `0 1px` vẽ đúng thay vì giả bằng rect.

## Vấn đề

- Không có shadow. Người dùng phải vẽ highlight inset `0 1px` và ring `0 0 0 1px` bằng rect, còn shadow mờ thì không giả được (card, menu, modal phẳng).
- Rect giả không có blur, không theo góc bo, chồng lên nội dung và làm sai group opacity (note #10: shadow keycap lộ qua nền).
- AI coder đọc `get_design_context` thấy rect thừa thay vì `box-shadow`, nên dựng thêm phần tử và sai hình.

## Bằng chứng

Code ở `night/integration@c3ab012`:

- `crates/storage/src/props_catalog.rs:73-96`: không có `shadow`/`boxShadow`/`effects`. Key lạ bị `check_keys` (`validate/prop_keys.rs:11-39`) trả `field:"props.boxShadow"`.
- `crates/layout/src/model.rs:53-77` (`Props`, không `deny_unknown_fields`) và `:145-163` (`SceneNode`) không có field shadow. Prop mới optional không làm vỡ scene cũ.
- `renderer/drawing.ts:136-156`: mỗi node `save()`, áp clip mọi tổ tiên, `globalAlpha`, rồi `paintNode` (`:15-74`: fill → text → image). Không có bước vẽ dưới fill hay trên fill/dưới con. Sau P01 là `renderer/paint.ts` (`paintFill/paintText/paintImage`, `night/P01@06ae746`).
- `renderer/drawing.ts:133`: `setTransform(scale…)`; viewer P01 vẽ dưới transform Konva (zoom × DPR). Canvas **không áp transform cho `shadowBlur`/`shadowOffset`** ("not affected by the current transformation matrix", https://html.spec.whatwg.org/multipage/canvas.html#shadows), nên phải nhân tay theo scale thiết bị.
- Token: chỉ `color | dimension | number` (`validate/tokens.rs:83`, `schemas/operations.rs:36`); style chỉ `fill | text` (`schemas/operations.rs:41`). Shadow dùng token cho từng trường (màu, số), không cần loại token/style mới.

**Prototype đo trên Chromium pinned của repo (chromium-1248)**

Script scratch (không commit): `/private/tmp/claude-501/-Volumes-MySpace-Workspace-OpenCanvas/a47874fe-c598-4ad4-a1cc-0f72b40e19ca/scratchpad/p09/` (`proto.js`, `probe.mjs`, `oracle.mjs`, `perf2.mjs`, `perf3.mjs`, `nine.mjs`).

So canvas prototype với `div` CSS `box-shadow` cùng hình, nền trắng, toàn vùng 480×300, mọi kênh RGB:

| Ca | Shadow | Fill | dsf 1 max | dsf 2 max |
| --- | --- | --- | --- | --- |
| S1 | `0 4px 12px #00000040`, radius 12 | trắng | 0 | 1 |
| S2 | ring `0 0 0 1px #6366f1`, radius 8 | trắng | 1 | 1 |
| S3 | `inset 0 1px 0 #ffffff66`, radius 6 | `#1f2937` | 0 | 1 |
| S4 | `inset 0 0 0 1px #ffffff1a` | `#1f2937` | 0 | 1 |
| S5 | `0 1px 2px #0000000d, 0 10px 30px -5px #00000033` | trắng | 0 | 1 |
| S6 | modal `0 24px 48px #00000059`, 300×160 r16 | trắng | 1 | 1 |
| S7 | `inset 0 2px 8px #00000040`, r12 | `#f8fafc` | 2 | 2 |
| S8 | `0 4px 12px #000`, fill trong mờ | `#ffffff80` | 8 (33 kênh, mép AA) | 7 |
| S9 | spread 8 trên radius 4 | `#e2e8f0` | 1 | 1 |
| S10 | gradient + inset + outer (cần P09a) | linear | 1 | 4 |

Ba phát hiện quyết định thuật toán:

1. **`shadowBlur` = blur CSS × scale thiết bị** (không phải blur/2): cả hai đều có σ = giá trị/2 (CSS Backgrounds 3 "standard deviation equal to half the blur radius"; HTML canvas "σ be half the value of shadowBlur"). Đo khớp ≤ 1 ở dsf 1 và 2 khi nhân scale.
2. **Luật clip-out kiểu Blink:** cắt bóng khỏi phần dưới hộp (CSS: bóng ngoài không lộ dưới hộp) **chỉ khi fill không phải màu đặc alpha FF**. Luôn cắt thì mép AA lệch 11–40 (S1 11, S2 40, S9 27, S6 14 do AA bị nhân hai lần); bỏ cắt với mọi fill thì S10 (nền gradient) lệch 16. Theo luật trên: mọi ca ≤ 1–2, trừ S8 lệch ≤ 8 ở mép AA (Chrome cũng clip, chênh do cách AA).
3. **Chi phí:** blur theo diện tích hình, không theo bán kính. Headless (software canvas, trung vị 15 lần, đã flush):

| Cảnh 1440×900 | @1 | @2 |
| --- | --- | --- |
| 40 shadow blur 2 (card 160×160) | 2,6 ms | 16 ms |
| 40 shadow blur 30 | 6,1 ms | 25,4 ms |
| 40 nút inset `0 1px` + ring (blur 0, đường tô thường) | 0,5 ms | 1,2 ms |
| modal blur 48 (600×400) | 1,2 ms | 5,3 ms |
| cảnh kiểu QueryNest: 19 node, 28 lớp bóng, glow radial, 10 nút gradient | 3,7 ms | 13,7 ms |
| cùng cảnh, màu phẳng không bóng | 0,3 ms | 1,7 ms |

- Inset vẽ bằng khung rect cỡ 100 000 px tốn như blur cả khung; khung vừa đủ (hộp + offset + spread + 1,5·blur) cho cùng pixel.
- Nine-slice (sprite góc + cạnh kéo giãn như Skia làm cho `box-shadow`) khớp ≤ 1 nhưng chỉ nhanh ~2× với blur lớn, không nhanh hơn với blur nhỏ (headless bị giới hạn fill-rate): **không đưa vào**.
- Thử Chromium kênh `chromium` với ANGLE Metal: WebGL dùng GPU M3 nhưng canvas 2D vẫn cùng số đo, tức số trên là trường hợp xấu (software).

**Tham khảo**

- CSS Backgrounds 3 `box-shadow` (https://www.w3.org/TR/css-backgrounds-3/#box-shadow): lớp đầu nằm trên cùng; bóng ngoài chỉ vẽ ngoài border box, bóng trong chỉ trong padding box; spread mở rộng hình và bán kính góc theo công thức `r + s·(1 + (r/s − 1)³)` khi `r < s` (đo S9); spread âm thu bán kính `max(0, r + s)`; thứ tự vẽ: bóng ngoài dưới nền, bóng trong trên nền, dưới border và nội dung.
- Figma REST Effect (https://developers.figma.com/docs/rest-api/file-node-types/): `DROP_SHADOW/INNER_SHADOW {color, offset{x,y}, radius, spread, visible, blendMode, showShadowBehindNode}`; `radius` Figma xuất ra CSS bằng đúng blur. Figma áp effect lên toàn bộ nội dung layer; CSS áp lên hộp. Chọn CSS.
- Penpot Shadow (https://doc.plugins.penpot.app/interfaces/Shadow): `{style:'drop-shadow'|'inner-shadow', offsetX, offsetY, blur, spread, color, hidden}`.

## Giải pháp

### 1. Prop authored (một key, mảng lớp như CSS)

```json
{"shadow": [{"y": 4, "blur": 12, "color": "#00000040"}]}
{"shadow": [{"spread": 1, "color": "#6366f1"}]}
{"shadow": [{"inset": true, "y": 1, "color": "#ffffff66"}]}
{"shadow": [{"inset": true, "y": 1, "color": "#ffffff33"}, {"y": 1, "blur": 2, "color": "#0000004d"}]}
{"shadow": [{"y": 10, "blur": 30, "spread": -5, "color": {"tokenId": "shadow-ink"}}]}
{"shadow": []}
```

Lần lượt: card, ring focus `0 0 0 1px`, highlight inset `0 1px`, nút chính, card theo token, bỏ shadow.

| Key | Mặc định | Giá trị | CSS |
| --- | --- | --- | --- |
| `x`, `y` | 0 | số −10000..10000 hoặc `{tokenId}` number | offset-x, offset-y |
| `blur` | 0 | số 0..1000 hoặc `{tokenId}` dimension; **bán kính blur của CSS** (σ = blur/2) | blur-radius |
| `spread` | 0 | số −1000..1000 hoặc `{tokenId}` number | spread-radius |
| `color` | bắt buộc | hex `#RGB/#RGBA/#RRGGBB/#RRGGBBAA`, `{tokenId}` color, `{styleId}` fill style màu đặc | color |
| `inset` | false | boolean | `inset` |

**Luật chung**

- `shadow` là mảng 0..8 lớp; **lớp đầu nằm trên cùng** như CSS. Mảng rỗng = không có bóng (dùng để "xoá" vì `update_node` merge props, `candidate.rs:124-135`).
- Kind hợp lệ: frame, group, rectangle, image (`PropScope::Box` của P04a). Không hợp lệ trên text (CSS `box-shadow` của text là hộp, còn Figma là glyph; `text-shadow` để phase sau) và page.
- Authored giữ nguyên; `resolvedProps.shadow` cùng shape, chỉ thay ref bằng literal. Không chèn mặc định.

**Vì sao chọn dạng này**

| Phương án | Đánh giá |
| --- | --- |
| **Mảng `{x,y,blur,spread,color,inset}`** (chọn) | Tên và thứ tự khớp CSS `box-shadow` 1:1 (coder ghép chuỗi); field bỏ qua = 0; token được ở từng field; ~48 byte cho một card. |
| Chuỗi CSS `"0 4px 12px rgba(0,0,0,.25)"` | Ngắn hơn ~10 byte và LLM thuộc, nhưng cần parser 2 phía (rgba, hsl, tên màu, thứ tự tự do của `inset`/màu, nhiều lớp phân tách bằng phẩy nằm trong `rgba(…)`), không gắn được token màu. |
| `effects:[{type:"DROP_SHADOW"…}]` kiểu Figma | Gom chung blur vào cùng mảng (khác ngữ nghĩa CSS: blur là `filter`), tên `offset:{x,y}`/`radius` thêm tầng lồng và lệch tên CSS. |
| Object đơn khi chỉ có một lớp | Hai shape cho cùng prop làm coder phải xử lý cả hai khi đọc. Chỉ nhận mảng; lỗi chỉ cách bọc mảng. |

**Alias catalog** (B02 gợi ý `shadow`): `boxShadow`, `box-shadow`, `shadows`, `dropShadow`, `innerShadow`, `effects`, `elevation`.

**Chi phí token:** ring/highlight hiện là 1–2 `create_node` rect (~300 byte, 2 node trong mọi lần đọc) → ~40 byte.

### 2. Hình học và thứ tự vẽ (như trình duyệt)

**Ký hiệu:** `b` = bounds node (border box), `r` = radius node, `k = √|det M|` với `M = context.getTransform()` (scale thiết bị: scale PNG, hoặc zoom × DPR ở viewer). Mọi số shadow tính theo đơn vị scene; chỉ `shadowBlur`/`shadowOffset` nhân `k`.

**Thứ tự cho một node:** bóng ngoài (lớp cuối vẽ trước) → fill → text/ảnh → bóng trong (lớp cuối vẽ trước) → các con → stroke (P04a).

**Bóng ngoài** (mỗi lớp):
1. Nếu fill **không** phải màu đặc alpha FF: clip vào "ngoài hộp" = `rect` rất lớn + `roundRect(b, r)`, `clip("evenodd")`.
2. Hình bóng = `b` nới `spread` mỗi phía; bán kính `R = spreadRadius(r, spread)`:
   - `r = 0` → 0; `spread ≥ 0`: `r ≥ spread ? r + spread : r + spread·(1 + (r/spread − 1)³)`; `spread < 0`: `max(0, r + spread)`.
   - Hình rỗng (`w ≤ 0` hoặc `h ≤ 0`) → bỏ lớp.
3. `blur = 0`: tô thẳng hình dời `(x, y)` bằng `color` (không dùng máy shadow).
4. `blur > 0`: dựng hình ở `x − D`, `shadowColor = color`, `shadowBlur = blur·k`, `shadowOffsetX = (x + D)·k`, `shadowOffsetY = y·k`, tô bằng màu đặc bất kỳ. `D` đủ lớn để chính hình nằm ngoài canvas: `D = max(0, b.x + b.width + spread + M.e/k) + 1.5·blur + 2`.

**Bóng trong** (mỗi lớp, vẽ sau fill/ảnh, trước con):
1. Padding box `p` = `b`, hoặc mép trong của stroke `inside` khi P04a đã merge (`strokeEdges(...).i/iR`), giống CSS (bóng trong nằm trong padding box). Clip `roundRect(p, rp)`.
2. Lỗ = `p` dời `(x, y)` thu `spread` mỗi phía, bán kính `max(0, rp − spread)`.
3. Khung = `p` nới `m = |x| + |y| + |spread| + 1.5·blur + 2` (không dùng khung cỡ 100 000 px: cùng pixel, rẻ hơn).
4. `blur = 0`: tô `khung − lỗ` (`fill("evenodd")`) bằng `color`. `blur > 0`: dựng `khung − lỗ` ở `x − D'` với `D' = p.width + 2m + 10`, `shadowOffsetX = D'·k`, `shadowBlur = blur·k`.

**Clip và opacity**

- Bóng dùng clip và opacity của tổ tiên như fill (vòng vẽ hiện có). **Clip của chính node không cắt bóng ngoài của nó** (CSS `overflow:hidden` không cắt `box-shadow` của chính phần tử); bóng trong nằm trong padding box nên clip không ảnh hưởng.
- Group không fill có bóng: bóng theo hộp như CSS (div nền trong suốt), không theo nội dung con (khác Figma).
- Ảnh: bóng trong vẽ **trên** ảnh, như `div` có `background-image` (với `<img>` thật CSS che bóng trong). Ghi vào `implement-frame.md`.

**Phác thảo `renderer/shadow.ts`** (đã chạy trong prototype `proto.js`):

```ts
export function spreadRadius(r: number, s: number): number;
export function paintOuterShadows(c: CanvasRenderingContext2D, node: SceneNode): void; // trước fill
export function paintInsetShadows(c: CanvasRenderingContext2D, node: SceneNode, pad?: PaddingBox): void; // sau fill/ảnh
```

`paintNode` (P01 `paint.ts`) gọi `paintOuterShadows` trước `paintFill` và `paintInsetShadows` sau `paintImage`; node không có `shadow` không đổi lệnh vẽ nào (byte-identical).

**P05 (group opacity layer):** nếu P05 đã merge, `extent` của node có bóng ngoài trong `renderer/scene-tree.ts` nới thành hợp của `bounds` với, cho mỗi lớp ngoài, `bounds` dời `(x,y)` nới `spread + 1.5·blur + 1`. Nới cả node `clip` (clip của chính node không cắt bóng). Nếu P09b merge trước P05 thì P05 thêm đúng luật này.

### 3. Scene và pipeline

- `crates/layout/src/model/shadow.rs` (mới): `ShadowInput` (authored, field optional, đã resolve) và `SceneShadow {x, y, blur, spread, color, inset}` đầy đủ; `scene_shadows()` chuẩn hoá; kiểm hữu hạn.
- `Props.shadow: Option<Vec<ShadowInput>>`; `SceneNode.shadow: Vec<SceneShadow>` với `#[serde(default, skip_serializing_if = "Vec::is_empty")]`: node không bóng serialize byte-identical.
- `renderer/types.ts`: `shadow?: SceneShadow[]`.
- Bump `SCENE_VERSION`/`RENDERER_VERSION` lên số kế tiếp chưa dùng (sau P04a/P09a) ở Rust và TS, sửa 2 chuỗi hard-code trong `web/tests`. Lý do: renderer cũ gặp scene có bóng sẽ bỏ bóng im lặng.

### 4. Resolve và validate (storage)

- `crates/storage/src/validate/shadow.rs` (mới):
  - `Candidate::resolve_shadow(v)`: với mỗi lớp, `color` → `resolve_value(_, "color")` (fill style gradient bị chặn với lỗi P09a nếu có), `x/y/spread` → `"number"`, `blur` → `"dimension"`; shape sai giữ nguyên để validator báo;
  - `shadow(v)` trên giá trị đã resolve: mảng ≤ 8, key hợp lệ, khoảng số, `inset` boolean.
- `refs.rs:95-118`: nhánh `"shadow"` gọi `resolve_shadow`. `properties.rs`: `"shadow" => shadow(v)?`. `props_catalog.rs`: entry `shadow` scope `Box`, mô tả một dòng, alias.

**Lỗi** (format B02; lỗi giá trị dùng đường dẫn trong prop, không dùng `props.shadow`):

| Ca | field | message |
| --- | --- | --- |
| trên text | `props.shadow` (lỗi key, `nodeKind:"text"`) | `Shadow requires a frame, group, rectangle or image node; wrap text in a frame to give it a box shadow` |
| object thay vì mảng, hoặc > 8 lớp | `shadow` | `Shadow is an array of up to 8 layers [{x, y, blur, spread, color, inset}], first on top like CSS box-shadow` |
| thiếu màu | `shadow[0].color` | `Shadow layer requires color` |
| key lạ (`offsetX`, `radius`, `type`) | `shadow[0].offsetX` | `Unsupported shadow key 'offsetX'; a layer takes x, y, blur, spread, color and inset` |
| số sai khoảng | `shadow[1].blur` | `Shadow blur must be 0..1000 (the CSS blur radius) or a dimension token` |
| `inset` không boolean | `shadow[0].inset` | `Shadow inset must be true or false` |

### 5. `get_design_context` cho AI coder

- `props.shadow` nguyên văn, `resolvedProps.shadow` literal; compact walker gom token trong `shadow[i].color` (`crates/server/src/reads/compact.rs:75-96`).
- `implement-frame.md` (bảng `:25-36`) thêm: `shadow` → `box-shadow: [inset ]<x>px <y>px <blur>px <spread>px <color>`, giữ thứ tự lớp; trên ảnh thì dùng `background-image` hoặc lớp phủ nếu cần bóng trong.
- `author-frame.md`: "dùng `shadow` cho bóng, ring `0 0 0 1px` (`spread:1`) và highlight inset `0 1px`; không giả bằng rect".

## Phạm vi file

| File | Việc | Dòng (hiện → ước) |
| --- | --- | --- |
| `crates/storage/src/validate/shadow.rs` (mới) | resolve + validate + message | ~130 |
| `crates/storage/src/validate/refs.rs`, `properties.rs`, `validate.rs` | nhánh `shadow`, `mod shadow` | +5 |
| `crates/storage/src/props_catalog.rs` | entry `shadow` (+ `PropScope::Box` nếu P04a chưa có) | 126 → ~128 (~143 nếu thêm Box) |
| `crates/layout/src/model/shadow.rs` (mới) | kiểu input/scene, chuẩn hoá, kiểm hữu hạn | ~70 |
| `crates/layout/src/model.rs`, `engine.rs:115-154`, `validation.rs`, `lib.rs:13-14` | field, chiếu scene, gọi kiểm, bump version | +2, +1, +2, ±2 |
| `renderer/shadow.ts` (mới) | `spreadRadius`, bóng ngoài/trong, fast path blur 0, `D` | ~120 |
| `renderer/paint.ts` (sau P01; hoặc `drawing.ts`) | 2 lời gọi trong `paintNode` | +3 |
| `renderer/scene-tree.ts` (nếu P05 đã merge) | nới `extent` cho bóng | +8 |
| `renderer/types.ts` | `SceneShadow`, bump | +10 |
| `web/tests/state.test.ts`, `web/tests/fixture-data.mjs` | chuỗi version | 2 dòng mỗi file |
| `crates/storage/tests/shadow_props.rs` (mới) | AC1–AC3 | ~150 |
| `crates/layout/src/shadow_tests.rs` (mới) | AC4 | ~60 |
| `spikes/pipeline/shadow.test.mjs` (mới) | AC6–AC13 pixel, so CSS | ~190 |
| `spikes/pipeline/css-oracle.mjs` | dùng lại (P05/P09a) hoặc tạo, thêm `box-shadow` | ±10 |
| `crates/server/tests/shadow_context.rs` (mới) | AC5 | ~80 |
| `web/tests/zoom.mjs` (của P01) | biến thể fixture có bóng cho AC15 | +20 |
| skill `implement-frame.md`, `author-frame.md`; `docs/CONTRACT.md`, `docs/VIEWER.md`, `spikes/pipeline/README.md` | map CSS, thứ tự vẽ, version | vài dòng |

Không đụng `mcp.rs`, `schemas.rs`, `tools/list`. Không file mã nào vượt 200 dòng.

## Tương thích MCP

- `tools/list` byte-identical (`props` là object tự do); `crates/server/tests/tool_schemas.rs` pass không sửa.
- Prop mới optional; dữ liệu cũ validate như trước. `allowedKeys` của frame/group/rectangle/image thêm `shadow` (chỉ thêm).
- Scene node không bóng byte-identical trừ `provenance`. `sceneKey` đổi một lần do bump; tab cũ báo `PIPELINE_MISMATCH`.
- Tài liệu: props.md (tự sinh), 2 skill, CONTRACT, VIEWER, README pipeline.

## AC

Toạ độ frame-local. Root frame 480×300 `fill:"#ffffff"`, layout `free`, node tại `x:40, y:40`. Giá trị kỳ vọng là pixel CSS Chrome cùng phiên bản (`oracle.mjs`), dung sai ±2 mỗi kênh RGB trừ khi ghi khác.

**AC1 — Validator nhận** (revision +1): 6 ví dụ ở mục 1 trên rectangle; `color:{styleId:<fill style màu>}`; `x:{tokenId:<number>}`, `blur:{tokenId:<dimension>}`; `shadow` trên frame, group, image.

**AC2 — Validator từ chối** (đúng `code`, `field`, message chứa cụm in nghiêng): text → `props.shadow`, `nodeKind:"text"`, *wrap text in a frame*; `shadow:{y:4,color:"#000"}` → `shadow`, *array of up to 8 layers*; 9 lớp → `shadow`; `[{y:4}]` → `shadow[0].color`; `[{offsetX:1,color:"#000"}]` → `shadow[0].offsetX`; `[{blur:-1,color:"#000"}]` và `blur:1001` → `shadow[0].blur`; `inset:"yes"` → `shadow[0].inset`; `color:"black"` → `shadow[0].color`; `boxShadow:[…]` → lỗi key, `suggestion:"shadow"`; `effects` → `suggestion:"shadow"`.

**AC3 — Catalog.** Test B02 `validator_accepts_exactly_the_catalog_keys_per_kind` pass không sửa logic; dòng props.md của frame, group, rectangle, image chứa `` `shadow` ``, của text và page thì không.

**AC4 — Scene và tương thích.**
- Test hiện có pass sau khi chỉ đổi chuỗi version.
- Scene fixture `spikes/pipeline/fixtures.mjs` trước/sau bằng nhau sau khi bỏ `provenance`, `sourceSceneKey`; không node nào có key `shadow`.
- `[{y:4,color:"#0004"}]` thành `{"x":0,"y":4,"blur":0,"spread":0,"color":"#0004","inset":false}` trong scene; `shadow:[]` không tạo key.

**AC5 — `get_design_context`.** Token `shadow-ink` (color, có theme `night`): `props.shadow[0].color` = `{tokenId:"shadow-ink"}`; `resolvedProps` là hex theme default, khác ở `theme:"night"`; `compact:true` có `shadow-ink` trong `tokens`.

**AC6 — Bóng ngoài.**
- S1 (160×80, radius 12, `[{y:4,blur:12,color:"#00000040"}]`): (120,120) = `209,209,209`; (120,124) = `225,225,225`; (120,130) = `247,247,247`; (120,140) trắng; (34,80) = `245,245,245`; (120,36) = `249,249,249`.
- S5 (200×120, r12, `[{y:1,blur:2,color:"#0000000d"},{y:10,blur:30,spread:-5,color:"#00000033"}]`): (140,161) = `222,222,222`; (140,175) = `243,243,243`; (36,100) = `241,241,241`.

**AC7 — Ring và bán kính spread.**
- S2 (160×40, r8, `[{spread:1,color:"#6366f1"}]`): (120,39) và (39,60) = `99,102,241`; (120,38) và (120,40) trắng.
- S9 (120×60, r4, fill `#e2e8f0`, `[{spread:8,color:"#94a3b8"}]`): (100,33) = `148,163,184`; (33,33) trắng (bán kính bóng 11 theo công thức CSS; dùng `r` = 4 thì điểm này có màu).

**AC8 — Bóng trong.**
- S3 (160×40, r6, fill `#1f2937`, `[{inset:true,y:1,color:"#ffffff66"}]`): (120,40) = `120,126,135`; (120,41) và (120,79) = `31,41,55`.
- S4 (cùng hộp, `[{inset:true,spread:1,color:"#ffffff1a"}]`): (120,40), (40,60), (120,79) = `53,62,75`; (120,41) = `31,41,55`.
- S7 (200×100, r12, fill `#f8fafc`, `[{inset:true,y:2,blur:8,color:"#00000040"}]`): (140,42) = `219,221,223`; (140,46) = `239,241,243`; (140,90) = `248,250,252`.

**AC9 — Fill trong mờ không lộ bóng bên dưới.** S8 (160×80, r12, fill `#ffffff80`, `[{y:4,blur:12,color:"#000000"}]`): (120,80) trắng `255,255,255`; (120,124) = `136,136,136`.

**AC10 — Khớp CSS toàn vùng.** S1–S9 (và S10 nếu P09a đã merge), `worker.render` scale 1 và 2 so CSS `deviceScaleFactor` 1 và 2: max lệch ≤ 2 mỗi kênh trên toàn vùng 480×300, trừ S8: ≤ 8 và ≤ 60 kênh lệch > 2.

**AC11 — Thứ tự và clip.**
- Frame `clip:true`, r12, fill trắng, shadow S1, đặt ở (40,40) trong root: (120,124) = `225,225,225` (clip của chính frame không cắt bóng).
- Frame `clip:true` 160×82 không fill tại (40,40), con rect S1 ở `x:0, y:0`: (120,120) = `209,209,209`; (120,124) trắng (tổ tiên clip cắt bóng).
- Frame 160×80 fill trắng, `[{inset:true,spread:4,color:"#ff0000"}]`, con rect xanh `#2563eb` tại (0,0) 20×20: (42,42) = `37,99,235` (con trên bóng trong); (120,42) = `255,0,0`.

**AC12 — Padding box với stroke inside** (chỉ khi P04a đã merge). Rect 160×40 fill trắng, `stroke:{color:"#000000"}`, `shadow:[{inset:true,y:1,color:"#ff0000"}]`: (120,40) = `0,0,0`; (120,41) = `255,0,0`; (120,42) trắng. (Dùng border box thì (120,41) trắng.)

**AC13 — Group opacity** (chỉ khi P05 đã merge). Group `opacity:0.5` chứa rect S1: pixel (120,124) = CSS cùng cấu trúc ±2 (đo đối chiếu: không phải trắng), `opacityLayers === 0` vì group chỉ có một lệnh vẽ; thêm rect thứ hai chồng lên thì `opacityLayers === 1` và (120,124) vẫn khớp CSS ±2 (extent không cắt bóng).

**AC14 — Transform.** `paintScene` (P01) trên canvas `setTransform(2,0,0,2,0,0)` cho RGBA giống từng byte `drawScene` scale 2 cùng scene S1+S3+S7. Không nhân `k` thì AC này trượt (bóng mờ còn một nửa).

**AC15 — Hiệu năng** (đo tay, ghi `.night/dev/P09b.md`, không đưa vào CI). Cảnh kiểu QueryNest (bảng Bằng chứng, `perf3.mjs`):
- `drawScene` 1440×900: trung vị ≤ 8 ms @1, ≤ 25 ms @2 (đo prototype 3,7 / 13,7).
- Viewer (harness `web/tests/zoom.mjs` của P01, DPR 2, cử chỉ ×4/1000 ms rồi ×0,25/600 ms): `Konva.Layer.drawScene` p95 ≤ 20 ms, 0 long task ≥ 50 ms. AC5 của P01 (cảnh stress không bóng) vẫn pass.

**AC16 — Không regression.** `fixture(420)` scale 1 và 2, `geometry.test.mjs`: RGBA giống từng byte renderer `night/integration` (cách so của P05). Viewer/PNG worker byte-identical (`pipeline.test.mjs`) với fixture có thêm node bóng.

**AC17 — Live.** Server tạm: `apply_batch` tạo nút S10 (hoặc S3 nếu chưa có P09a) và card S1 → `render_frame` `ready`, `diagnostics:[]`; PNG có (120,124) ≈ `225,225,225`.

**AC18 — Modular, kiểm tĩnh.** File mới ≤ 200 dòng; `paint.ts`/`drawing.ts` tăng ≤ 5 dòng; `cargo fmt --check`, clippy, `npm run check:renderer`, `npm --prefix web run build` pass.

## Cách kiểm

```bash
W=<worktree>; cd $W && export CARGO_BUILD_JOBS=4
cargo test --offline -p opencanvas-layout -p opencanvas-storage -p opencanvas-server
cargo clippy --offline -p opencanvas-layout -p opencanvas-storage -p opencanvas-server --lib --bin opencanvas-server -- -D warnings
cargo fmt --all --check && npm run check:renderer
PLAYWRIGHT_BROWSERS_PATH=$W/.runtime/browsers npm run test:pipeline        # gồm shadow.test.mjs
npm --prefix web test && npm --prefix web run build
PLAYWRIGHT_BROWSERS_PATH=$W/.runtime/browsers npm --prefix web run test:zoom   # AC15 viewer (P01)
```

**Test mới:** `shadow_props.rs` (AC1–3), `shadow_tests.rs` (AC4), `shadow.test.mjs` (AC6–AC14, AC16, oracle CSS), `shadow_context.rs` (AC5).

**Prototype tham chiếu** (scratch, có thể bị dọn): `node probe.mjs S`, `node oracle.mjs`, `node perf3.mjs` với `PLAYWRIGHT_BROWSERS_PATH=/Volumes/MySpace/Workspace/OpenCanvas/.runtime/browsers`.

**Live (AC17):** `target/debug/opencanvas-server --port 0 --data-root $(mktemp -d)`; script mẫu `.night/qa/mcp-tools.mjs`; dọn process và data. Không đụng 3847.

## Rủi ro

- **Vòng vẽ và version:** P01/P04a/P05/P09a cùng sửa `paintNode` hoặc version. P09b chỉ thêm 2 lời gọi và file mới; version lấy số kế tiếp lúc merge.
- **P05 extent:** quên nới thì layer cắt bóng trong group mờ (AC13 bắt).
- **Hiệu năng viewer trên software canvas:** bóng mờ tốn theo diện tích (bảng Bằng chứng). Trên Mac thật canvas 2D thường được GPU tăng tốc, nhưng chưa đo được trong headless. Nếu AC15 trượt trên design thật: P01b (snapshot bitmap trong lúc cử chỉ) hoặc cache bitmap bóng theo (hình, lớp, `k`) cho lúc pan.
- **Clip-out kiểu Blink:** fill trong mờ cho lệch ≤ 8 ở 1 hàng pixel mép (S8); trình duyệt khác (Firefox) có thể khác Chrome ở đây.
- **Ảnh có bóng trong:** khác `<img>` thật của CSS (bóng trong bị ảnh che). Đã chọn hành vi như `background-image`, ghi trong skill.
- **Export (P03a/P03b):** PNG crop theo `bounds` sẽ cắt bóng ngoài; SVG cần `<filter>` (`feDropShadow` cho ngoài; bóng trong cần `feComposite`). Follow-up cho P03a/P03b như P04a đã ghi cho stroke.

## Ngoài phạm vi / phase sau

- `text-shadow` trên text (glyph), drop shadow theo alpha nội dung kiểu Figma (`filter: drop-shadow`).
- Loại style "effect" (đặt tên bộ shadow `shadow-md` trong design system): cần thêm `kind` vào enum `upsert_style` (`schemas/operations.rs:41`), tức đổi `tools/list`. Hiện dùng token cho từng field.
- Blend mode của bóng, `visible:false` từng lớp.
- Hit-test và bounds đọc được của phần bóng (`paintBounds`) trong `inspect_layout`/`get_design_context`.
- Cache bitmap bóng cho viewer (chỉ khi AC15 trượt trên design thật).
