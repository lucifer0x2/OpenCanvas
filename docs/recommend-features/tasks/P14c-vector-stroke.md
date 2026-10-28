# P14c — Stroke cho `vector`: dùng chung prop `stroke` của P04a (một màu, căn giữa path, `cap`/`join`/`miterLimit`), `fill:"none"`, và stroke trong `import_svg`
Ưu tiên: P2 · Kích cỡ: ~75 phút dev · Phụ thuộc: **P14a, P04a** (bắt buộc), **P14b** (cho mục 4; thứ tự a → b → c) · Nguồn: note "Vector/SVG node", note "Stroke/border"; phần ngoài phạm vi của P14a/P14b

Phase c của P14. P14a/b chỉ có fill, nên bộ icon nét (Lucide, Feather, Tabler, Heroicons outline: `fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"`) chưa biểu diễn được và `import_svg` từ chối chúng. P14c thêm stroke cho vector **bằng đúng key `stroke` của P04a**. Không thêm key top-level mới và không thêm tool.

## Vấn đề

- Icon nét là một nửa thế giới icon web. Chúng không quy về fill được, trừ khi outline nét thành path, và việc đó mất `stroke-width` khi AI coder dựng lại.
- Sau P14a, vector chỉ có `fill`, nên agent sẽ rasterize hoặc outline tay.
- Sau P14b, `import_svg` trả `stroke is not imported yet` cho mọi icon Lucide.
- **Va chạm giữa P04a và P14a:** `PropScope::Box` của P04a cho phép mọi kind trừ page/text (`!matches!(kind, Page | Text)`, `.night/wt/P04a/crates/storage/src/props_catalog.rs:24`). Khi có kind `vector`, `stroke` sẽ lọt qua key check và bị vẽ như **viền hộp** quanh bounds của vector: sai so với SVG mà không có lỗi nào báo. P14c định nghĩa nghĩa đúng. Trước khi P14c merge, P14a phải loại `Vector` khỏi `Box` (1 dòng, đã ghi vào Rủi ro của P14a).

## Bằng chứng

**P04a** (nhánh `night/P04a` @ `cde7f4f`, `.night/dev/P04a.md`, chưa merge lúc viết):

| Vị trí | Hiện trạng |
| --- | --- |
| `crates/storage/src/validate/stroke.rs:12-30` | `resolve_stroke` resolve ref trong `color` (`"color"`) và `width` (`"dimension"`), scalar hoặc mảng. |
| `crates/storage/src/validate/stroke.rs:34-70` | `validate_stroke`: chỉ nhận `color/width/align`; lỗi mang field `stroke.<key>` (không phải `props.stroke`); `align` ∈ inside/center/outside. |
| `crates/storage/src/props_catalog.rs:14,24,33,43,98` | `PropScope::Box` + entry `stroke` với 13 alias. |
| `crates/layout/src/model/stroke.rs:8-70` | `Sides<T>`, `StrokeInput {color, width, align}`, `SceneStroke` chuẩn hoá 4 cạnh, `align()` mặc định `"inside"`, `inside_insets()`. |
| `renderer/stroke.ts` (146 dòng), `renderer/paint.ts:98` | Viền hộp vẽ bằng vòng `evenodd`/nêm, sau subtree của node. |
| `renderer/types.ts:63-70` | `SceneNode.stroke?: SceneStroke`. |

**P14a** (`.night/proposals/P14a-vector-path-node.md`): `renderer/vector.ts` `paintVector` clip theo bounds, biến đổi viewBox → bounds bằng `placeImage`, rồi `fill(Path2D)`. Stroke chỉ cần thêm `ctx.stroke(path)` dưới **cùng** transform: độ dày tính theo đơn vị viewBox như SVG, và bị clip ở viewport như `<svg>` inline.

**Prototype** (Chromium 156.0.8078.4)

Script ở scratchpad, không commit: `/private/tmp/claude-501/-Volumes-MySpace-Workspace-OpenCanvas/0c1758af-4597-4d58-b91f-4496d5b95b83/scratchpad/p14/stroke.mjs`. Oracle là SVG rasterize như ảnh ở cùng DPR, như P14a.

| Ca | DPR 1 (max kênh / px > 16) | DPR 2 |
| --- | --- | --- |
| 4 icon kiểu Lucide (`search` = cung + đoạn, `chevron`, `check`, `house` có cung nhỏ), 16 px, width 2, cap/join round | 0 / 0 | 0 / 0 |
| cùng bộ 24 px | 0 / 0 | 0 / 0 |
| 24 px lệch +0,37 px | 0 / 0 | 0 / 0 |
| fill + stroke (`square`/`bevel`), `fit:"fit"` kéo giãn 96×32, đoạn thẳng cap `square` width 3 | 0 / 0 | 0 / 0 |
| góc nhọn ~22° join miter, **`ctx.miterLimit = 4`** | 0 / 0 | 0 / 0 |
| cùng ca, **để mặc định canvas (10)** | **238** / 15 | **238** / 54 |

- Thứ tự vẽ fill rồi stroke và độ dày theo đơn vị viewBox (kể cả khi kéo giãn không đều) khớp tuyệt đối với SVG.
- Bẫy duy nhất: `miterLimit` mặc định của canvas là **10**, của SVG là **4** (https://www.w3.org/TR/SVG2/painting.html#StrokeMiterlimitProperty, https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D/miterLimit). Không đặt thì góc nhọn lệch tới 238/255.

**Tham khảo tên gọi**

- SVG: `stroke`, `stroke-width`, `stroke-linecap` (butt|round|square), `stroke-linejoin` (miter|round|bevel), `stroke-miterlimit` (https://www.w3.org/TR/SVG2/painting.html#StrokeLinejoinProperty).
- Figma: `strokeCap`, `strokeJoin`, `strokeMiterAngle` (https://developers.figma.com/docs/plugins/api/properties/nodes-strokecap/).
- Penpot `Stroke`: `strokeAlignment` center|inner|outer, `strokeCapStart/End` (https://doc.plugins.penpot.app/interfaces/Stroke).
- Canvas: `lineCap`, `lineJoin`, `miterLimit`.

## Giải pháp

### 1. Prop authored: cùng key `stroke`, luật theo kind

```json
{"op":"create_node","id":"ic-search","kind":"vector","parentId":"bar",
 "props":{"width":16,"height":16,"viewBox":[0,0,24,24],"fill":"none",
          "path":"M21 21l-4.3-4.3M3 11a8 8 0 1 0 16 0a8 8 0 1 0-16 0",
          "stroke":{"color":{"tokenId":"icon-fg"},"width":2,"cap":"round","join":"round"}}}
```

| Key trong `stroke` | Hộp (P04a, giữ nguyên) | Vector (mới) |
| --- | --- | --- |
| `color` | màu, hoặc mảng đúng 4 | **một** màu: hex, `{tokenId}` hoặc `{styleId}` fill style |
| `width` | số/token, hoặc mảng 4, đơn vị px | **một** số/token (0..100000), **đơn vị viewBox** (px khi không có viewBox), như SVG `stroke-width` |
| `align` | inside (mặc định) / center / outside | chỉ `"center"`; bỏ trống = center |
| `cap` | từ chối | `"butt"` (mặc định) \| `"round"` \| `"square"` |
| `join` | từ chối | `"miter"` (mặc định) \| `"round"` \| `"bevel"` |
| `miterLimit` | từ chối | số 1..100, mặc định **4** (SVG) |

**`fill:"none"` trên vector**

- Nhận literal `"none"` (từ khoá SVG mà LLM quen). Scene ghi `"transparent"`, renderer bỏ bước fill.
- Fill bỏ trống vẫn là `#000000` như P14a/SVG. Quên `fill:"none"` thì ra icon đen đặc: lỗi thấy ngay trong render, và AI coder vẫn map 1:1 với SVG.
- `"none"` trên kind khác bị từ chối với message cũ cộng ý "`none` is only for vectors; use #0000 for a transparent box".

**Vì sao dùng chung key**

- Một khái niệm "stroke" cho agent: cùng alias B02 (`strokeWidth`, `strokeColor`, `border`… → `stroke`), cùng cách resolve token, cùng tên với Figma/SVG.
- Khác biệt theo kind là luật giá trị có lỗi rõ ràng, không phải key mới.

Phương án đã loại:
- **(a) key top-level `strokeCap/strokeJoin`:** thêm 3 key vào catalog cho mọi kind, trong khi chỉ vector dùng; `update_node` có thể để lại trạng thái lai.
- **(b) prop riêng `pathStroke`:** hai tên cho cùng một ý, agent phải chọn.
- **(c) cho vector inside/outside như Figma:** cần clip theo path (inside) hoặc vẽ đôi rồi khoét (outside). SVG/CSS không có, nên AI coder không dựng lại được bằng `<svg>` thuần.

**Lỗi** (field `stroke.<key>` như P04a, có `nodeId`/`nodeKind`):

| Trường hợp | Field | Message |
| --- | --- | --- |
| mảng màu trên vector | `stroke.color` | `A vector stroke has one color; per-side colors are for boxes` |
| mảng width trên vector | `stroke.width` | `A vector stroke has one width, in viewBox units` |
| `align:"inside"` trên vector | `stroke.align` | `Vector strokes are centered on the path as in SVG; omit align` |
| `cap:"rounded"` | `stroke.cap` | `cap takes butt, round or square` |
| sai `join` | `stroke.join` | `join takes miter, round or bevel` |
| `miterLimit` < 1 | `stroke.miterLimit` | `miterLimit is a number from 1 (default 4)` |
| `cap`/`join`/`miterLimit` trên hộp | `stroke.cap` (…) | `cap, join and miterLimit apply to vector strokes only` |

### 2. Storage

- `props_catalog.rs`:
  - `PropScope::Box` (P04a) đổi `allows` thành "mọi kind trừ page, text" **và tách nhãn**;
  - mô tả `stroke` thêm vế vector: `on vector: one color, width in viewBox units, centered; cap butt|round|square, join miter|round|bevel, miterLimit (default 4)`;
  - mô tả `fill` thêm `"none" on vectors`.
- `validate/stroke.rs` (P04a, 80 dòng): `validate_stroke(v, kind)`; nhánh vector tách sang file mới `validate/vector_stroke.rs` (~70 dòng) để không phình file của P04a. Key hợp lệ theo kind ở bảng trên.
- `validate/refs.rs` (`resolve_props`) và `properties.rs`: `fill == "none"` hợp lệ khi kind là vector (bỏ qua resolve/`color()`).

### 3. Scene và renderer

**Rust**

- `crates/layout/src/model/stroke.rs`:
  - `StrokeInput` thêm `cap`, `join`, `miter_limit: Option<…>`;
  - `SceneStroke` thêm 3 field, `skip_serializing_if = "Option::is_none"`, nên scene hộp **giữ nguyên byte** so với P04a;
  - vector: `scene()` ghi `align:"center"` và `cap/join/miterLimit` đã điền mặc định (`butt/miter/4`), để renderer không phải đoán.
- `engine.rs`: `fill:"none"` → `"transparent"` khi kind là vector. `inside_insets` không áp cho vector (leaf, không có con).
- `validation.rs`: kiểm `miterLimit` hữu hạn ≥ 1.
- Bump `SCENE_VERSION`/`RENDERER_VERSION` lên số kế tiếp, kèm các chuỗi hard-code trong test web (P04a đã liệt kê thêm `web/tests/fixture-dense.mjs`).

**TS**

- `renderer/vector.ts` (P14a, ~50 dòng → ~65): sau `fill`, nếu `node.stroke` thì
  ```ts
  ctx.strokeStyle = s.color[0]; ctx.lineWidth = s.width[0];
  ctx.lineCap = s.cap; ctx.lineJoin = s.join; ctx.miterLimit = s.miterLimit;
  ctx.stroke(path);
  ```
  Vẫn trong clip bounds và transform viewBox. `fill === "transparent"` thì bỏ fill.
- `renderer/paint.ts`: đường stroke hộp của P04a (`paint.ts:98`) **bỏ qua** `kind === "vector"`, vì vector tự vẽ stroke của nó.
- `extent` (P05) không đổi, vì mọi nét đều bị clip trong bounds như `<svg>` inline.
- `types.ts`: `SceneStroke` thêm `cap?`, `join?`, `miterLimit?`.

### 4. `import_svg` (P14b) nhận stroke

- **Đọc:** `stroke`, `stroke-width`, `stroke-linecap`, `stroke-linejoin`, `stroke-miterlimit`, `stroke-opacity`, kế thừa qua `<g>` và `style=""`.
  - `currentColor` và stroke chưa khai báo màu → `color` của op (như fill).
  - `fill="none"` → `fill:"none"`.
- **`<line>`:** nay được nhập (`M x1 y1 L x2 y2`).
- **Phần tử có cả fill và stroke:** một vector mang cả hai (thứ tự vẽ như SVG).
- **Độ dày dưới transform:**
  - ma trận tích luỹ có hai giá trị suy biến bằng nhau (sai lệch ≤ 1e-6, gồm xoay, scale đều, lật) → `width × s`, với `s = sqrt(|det M|)`;
  - scale không đều → lỗi `Stroked <path> at line N has a non-uniform transform; a vector stroke cannot be stretched per axis`.
- **`stroke-opacity < 1`:**
  - màu hex → gộp vào alpha (`#RRGGBBAA`);
  - màu là token (`color` của op) → lỗi `stroke-opacity with a token color is not supported`, vì node `opacity` sẽ làm mờ cả fill.
- **Từ chối:** `stroke-dasharray`, `stroke-dashoffset` ≠ 0, `vector-effect`, `paint-order` ≠ normal, `stroke="url(#…)"`. Mỗi lỗi nêu thuộc tính và dòng.
- **Thay message P14b:** `stroke is not imported yet` không còn. Mô tả `apply_batch` đổi "fills only" thành "fills and strokes".

### 5. MCP đọc và skill

- `get_design_context`: `props.stroke` nguyên văn; `resolvedProps.stroke` thay token bằng hex (đường P04a). Compact gom token trong `stroke.color`. Không đổi gì ở server reads.
- `implement-frame.md`, dòng vector:
  > `stroke` → `<path stroke=color stroke-width=width stroke-linecap=cap stroke-linejoin=join stroke-miterlimit=miterLimit>` (đơn vị viewBox); `fill:"none"` → `fill="none"`.
- `author-frame.md`:
  > Outline icons (Lucide…): vector `fill:"none"` + `stroke {color token, width 2, cap/join round}`, or `import_svg` the SVG.

## Phạm vi file

| File | Việc | Dòng (sau P04a/P14a/P14b → ước) |
| --- | --- | --- |
| `crates/storage/src/validate/vector_stroke.rs` (mới) | luật vector của `stroke` + message | ~70 |
| `crates/storage/src/validate/stroke.rs` | rẽ nhánh theo kind | 80 → ~86 |
| `crates/storage/src/validate/{refs,properties}.rs`, `props_catalog.rs` | `fill:"none"` vector; mô tả `stroke`/`fill` | +6, +4, ±2 |
| `crates/layout/src/model/stroke.rs`, `engine.rs`, `validation.rs`, `lib.rs` | cap/join/miterLimit, fill none, version | 74→~92, +3, +2, ±2 |
| `renderer/vector.ts`, `renderer/paint.ts`, `renderer/types.ts` | stroke path, bỏ qua hộp, kiểu | ~50→~65, +1, +3 |
| `crates/server/src/svg_import/{parse,geometry}.rs` | thuộc tính stroke, `<line>`, độ dày dưới transform. Nếu `parse.rs` > 200 dòng thì tách `svg_import/stroke.rs` (~80) | +60 |
| `crates/server/src/schemas.rs` | 1 câu mô tả `apply_batch` | ±1 |
| `crates/storage/tests/vector_stroke.rs` (mới) | AC1–AC3 | ~150 |
| `crates/layout/src/vector_tests.rs` (P14a) | AC4 | +30 |
| `spikes/pipeline/vector-stroke.test.mjs` (mới) | AC5, AC6 pixel (port `stroke.mjs` prototype) | ~140 |
| `crates/server/tests/import_svg.rs` (P14b) hoặc `import_svg_stroke.rs` (mới nếu > 200) | AC8–AC10 | ~120 |
| skills, `docs/SERVER.md`, `docs/CONTRACT.md` | tài liệu | ~10 |

## Tương thích MCP

- **Không tool mới, không key top-level mới.** `create_node.props` vẫn là object tự do, nên `tools/list` chỉ đổi đúng một câu mô tả `apply_batch` của P14b ("fills only" → "fills and strokes").
- **Byte thêm vào `tools/list`:** ≤ +40 (đo bằng `serde_json::to_vec(tools/list).len()` trước và sau).
- `stroke` trên hộp giữ nguyên nghĩa và mặc định của P04a. Scene hộp byte-identical với P04a.
- props.md (tự sinh) đổi mô tả `stroke`/`fill`. Tài liệu: `implement-frame.md`, `author-frame.md`, `docs/SERVER.md` (op `import_svg`), `docs/CONTRACT.md` (`SceneStroke`).

## AC

1. **Nhận:**
   - vector `fill:"none"`, `stroke:{color:"#344054",width:2,cap:"round",join:"round"}` commit;
   - `stroke.color {tokenId}` và `stroke.width {tokenId}` (dimension) commit;
   - `miterLimit: 10` commit.
2. **Lỗi**, mỗi dòng của bảng ở Giải pháp 1 cho đúng field `stroke.<key>` và message chứa cụm in nghiêng tương ứng. Thêm `fill:"none"` trên rectangle → field `fill`, message có `only for vectors`. Batch lỗi không commit.
3. **Không vỡ P04a:** `cargo test -p opencanvas-storage --test stroke_props` và `node --test spikes/pipeline/stroke.test.mjs` của P04a pass **không sửa test**.
4. **Scene:**
   - vector có stroke → `stroke = {color:[c,c,c,c], width:[w,w,w,w], align:"center", cap, join, miterLimit}`, mặc định `butt/miter/4` khi bỏ trống;
   - `fill:"none"` → `fill:"transparent"`;
   - fixture P04a (chỉ hộp) serialize `nodes` byte-identical trước/sau.
5. **Pixel** (`spikes/pipeline/vector-stroke.test.mjs`): các ca của bảng prototype trừ dòng cuối — 4 icon Lucide 16/24 px, lệch +0,37, fill + stroke square/bevel, `fit:"fit"` kéo giãn, góc nhọn miter. Render qua pipeline thật ở scale 1 và 2, so oracle SVG image cùng scale: **max kênh ≤ 1**.
6. **`miterLimit` mặc định 4:** ca góc nhọn (`M6 20L12 2L13 20`, width 2, viewBox 24 trên 48 px) không authored `miterLimit` → pixel (24, 2) (đỉnh miter khi limit 10) trắng (`255,255,255`), khớp SVG. Đo thật: SVG `255,255,255`; canvas limit 4 `255,255,255`; canvas mặc định 10 `17,24,39`. Script: `.../scratchpad/p14/ac6.mjs`.
7. **Token:**
   - vector `stroke.color {tokenId:"icon-fg"}` → pixel trên tâm nét = màu token;
   - `upsert_token` đổi màu → render mới đổi đúng màu đó;
   - `get_design_context compact:true` có `icon-fg` trong `tokens`; `resolvedProps.stroke.color` là hex.
8. **`import_svg` Lucide:** `search` dạng file Lucide gốc (`<svg … fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/></svg>`), `color:{tokenId:"icon-fg"}` →
   - `group` + 2 vector, mỗi vector `fill:"none"`, `stroke:{color:{tokenId:"icon-fg"},width:2,cap:"round",join:"round"}`;
   - render so oracle ≤ 1 / 255.
9. **Transform và thuộc tính nét:**
   - `<g transform="scale(2)">` bọc path `stroke-width="1.5"` → `width:3`;
   - `rotate(30)` → giữ `1.5`;
   - `scale(2 1)` → lỗi `non-uniform transform`;
   - `stroke-dasharray="4 2"` → lỗi nêu thuộc tính;
   - `stroke-opacity=".5"` + `stroke="#112233"` → `color:"#11223380"`; cùng thuộc tính với `currentColor` + `color` token → lỗi;
   - `<line x1="2" y1="12" x2="22" y2="12" stroke="#000">` → 1 vector `path:"M2 12L22 12"`.
10. **`tools/list`:** byte tăng ≤ 40 so với trước P14c; không có tool mới; `create_node`, `find_nodes` schema giữ nguyên.
11. **Modular:** file mới ≤ 200 dòng; `validate/stroke.rs` của P04a ≤ 100 dòng sau sửa; báo `wc -l`.

## Cách kiểm

- `CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-storage --test vector_stroke --test stroke_props` (AC1–AC3).
- `cargo test --offline -p opencanvas-layout` (AC4).
- `npm run test:pipeline` (AC5, AC6 và AC3 phần pixel của P04a).
- `cargo test --offline -p opencanvas-server --test import_svg` (AC8–AC9) và test đo `tools/list` (AC10, theo helper của F-TOOLS `900e0dc`).
- Test server cho AC7 (worker thật, như P14a AC6).
- Kiểm live trên server tạm `--port 0 --data-root $(mktemp -d)`:
  - nhập 5 icon Lucide bằng một `apply_batch`, xem viewer zoom 400 %, đổi token màu;
  - dọn process và data.

## Rủi ro

- **Phụ thuộc chuỗi P04a → P14a → P14b → P14c.** Dev board 01:54 ghi P14a/b chưa kịp đêm nay, nên P14c là việc của phiên sau.
  - Nếu P14a merge trước P14c mà chưa loại `Vector` khỏi `PropScope::Box`, `stroke` trên vector sẽ bị vẽ như viền hộp. P14a phải có dòng đó (đã ghi trong P14a).
- **Đơn vị độ dày khác nhau theo kind:** hộp tính px, vector tính đơn vị viewBox. Đây là ngữ nghĩa SVG mà AI coder cần. Catalog và skill ghi rõ; lỗi `stroke.width` trên vector nhắc "in viewBox units".
- **Nhớ đặt `miterLimit`:** quên đặt (canvas mặc định 10) thì góc nhọn lệch 238/255; AC6 khoá hành vi này.
- **Không có dash, gradient stroke, inside/outside cho vector:** icon dùng các thứ này bị `import_svg` từ chối rõ ràng.

## Ngoài phạm vi / phase sau

- `dash` (`stroke-dasharray`/`dashoffset`) cho vector và hộp.
- `vector-effect: non-scaling-stroke`.
- Stroke gradient (sau P09a).
- Align inside/outside cho path kín (kiểu Figma).
- Outline stroke thành fill (export).
- Export SVG (P03b) của vector có stroke: thuộc tính `stroke-*` 1:1, task nào merge sau thì thêm.
