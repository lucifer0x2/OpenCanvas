# P14a — Kind `vector`: một path SVG (`path` + `viewBox` + `fillRule`), màu theo token, vẽ bằng `Path2D`, đọc qua MCP
Ưu tiên: P1 · Kích cỡ: ~85 phút dev · Phụ thuộc: B02 (đã merge, `props_catalog`); xếp tuần tự với task đang sửa `renderer/paint.ts`, `props_catalog.rs`, `crates/layout/src/{model,engine}.rs` (P04a, P09a, P08a); P05 đã merge, không cần đổi `extent` · Nguồn: note "Thiếu tính năng → Vector/SVG node" (40 icon Material Symbols phải rasterize PNG 2x rồi `import_asset`), BACKLOG "vector/SVG node"

Phase a của P14. Phase b (`P14b-import-svg.md`) thêm op `import_svg` (SVG text → node `vector`). P14a tự đủ giá trị: agent viết icon/hình bằng path data (chép thẳng từ bộ icon), nét ở mọi mức zoom, đổi màu theo token, AI coder đọc lại path để dựng `<svg>` inline.

## Vấn đề

- OpenCanvas chỉ có `page/frame/group/rectangle/text/image` (`crates/storage/src/model.rs:13-20`). Hình không phải hộp chữ nhật (icon, logo, mũi tên, chevron) chỉ vẽ được bằng ảnh raster.
- Phiên vẽ lại QueryNest phải rasterize 40 icon Material Symbols thành PNG 2x rồi `import_asset` từng cái:
  - mờ khi zoom > 2x (đi ngược P01 viewer vector);
  - màu nướng cứng vào ảnh: không gắn token, đổi theme không đổi màu icon;
  - AI coder chỉ thấy `assetId` nên không biết đó là icon nào, cũng không lấy lại được hình học để dựng `<svg>`.
- Ghi chú người dùng: "Vector/SVG node — 40 icon Material Symbols phải rasterize thành PNG 2x rồi `import_asset`" (`/Users/luis/Documents/opencanvas.md:78`).

## Bằng chứng

Code ở `night/integration@565b12b` (sau P05, P02a).

**Storage / catalog**

| Vị trí | Hiện trạng |
| --- | --- |
| `crates/storage/src/model.rs:13-35` | `NodeKind` 6 giá trị; `container()` = page/frame/group. |
| `crates/storage/src/props_catalog.rs:8-41` | `PropScope` có `Any/Container/Text/Image`; `fit`, `crop`, `assetId` thuộc `Image` (`:93-95`). |
| `crates/storage/src/props_catalog.rs:89` | `fill` mô tả "background of shapes and containers, glyph color of text". |
| `crates/storage/src/props_catalog.rs:119-126` | `NODE_KINDS: [NodeKind; 6]` dùng cho props.md (`crates/server/src/props_doc.rs:29,68`) và test B02. |
| `crates/storage/src/validate/prop_keys.rs:42-55` | `check_required` chỉ có hai message (image, text). |
| `crates/storage/src/validate/properties.rs:14-90` | kiểm giá trị từng key; lỗi giá trị dùng field **không** có tiền tố `props.` (ví dụ `"crop"`, `"fontId"`). |
| `crates/storage/tests/prop_catalog.rs:17-52` | test B02 gửi `key: null` cho mọi kind × key và coi `field == "props.<key>"` là lỗi key, nên lỗi giá trị của `path/viewBox/fillRule` phải dùng field `path`/`viewBox`/`fillRule`. |
| `crates/storage/src/db.rs:5,64` | `nodes.kind TEXT` không có CHECK: thêm kind không cần migration; nhưng binary cũ deserialize `"vector"` sẽ lỗi (rủi ro rollback). |
| `crates/storage/src/image_fit.rs:7-24` | từ vựng `fit/contain/cover` dùng chung storage–renderer (B05). |

**Layout / scene / renderer**

- `crates/layout/src/model.rs:53-77`: `Props` không `deny_unknown_fields`; `SceneNode` (`:145-163`) serialize mọi field.
- `crates/layout/src/validation.rs:24`: danh sách kind cứng; `:62-68` ngân sách text 64 KiB/node, 1 MiB/frame (mẫu cho ngân sách path).
- `crates/layout/src/engine.rs:139-151`: `fill` mặc định `transparent` (trừ text), `fit` mặc định `contain`, `clip` mặc định theo kind.
- `crates/layout/src/tree.rs:86-90`: node không phải text là `new_with_children` → leaf không có kích thước nội tại.
- `renderer/paint.ts:8-9`: `paintFill` tô nền mọi kind trừ text → **phải bỏ qua `vector`**, vì `fill` của vector là màu path chứ không phải nền.
- `renderer/paint.ts:77-85`: `paintNode` rẽ nhánh text/image. `renderer/image-fit.ts:9-29` `placeImage(fit, box, source)` dùng lại nguyên cho viewBox.
- `renderer/scene-tree.ts:68-76`: `extent` của node không phải text = bounds. Vector clip theo bounds nên không cần nới `extent` (P05).
- `renderer/types.ts:1-2`, `crates/layout/src/lib.rs:13-14`: `scene-1`/`web-1`; chuỗi cứng ở `web/tests/state.test.ts:69-70`, `web/tests/fixture-data.mjs:79-80`.
- `web/src/canvasHitTest.ts:3-17`: hit test theo bounds, dùng được cho vector.
- `web/src/components/Inspector.tsx:45-46`: in mọi authored prop nguyên văn (path 1,6 KB sẽ tràn panel). `Icon.tsx:2-20`, `Layers.tsx:45`: icon theo `node.kind`.

**MCP**

- `crates/server/src/schemas/operations.rs:15`: `create_node.kind` enum 6 giá trị. `crates/server/src/schemas.rs:101`: `find_nodes.kinds` enum 6 giá trị, `maxItems: 6`.
- `crates/server/src/reads/tree.rs:66-77`: `get_design_context` trả `props` và `resolvedProps` (đầy đủ, hoặc chỉ key khác khi `compact`), nên path sẽ bị lặp hai lần ở chế độ thường.
- `PLAN.md:106`: "vector path editor" ngoài phạm vi. Đề xuất này **không** có editor: path chỉ được viết qua MCP và vẽ ra.

**Dữ liệu thật: 33 icon QueryNest**

- Nguồn: `/Volumes/MySpace/Workspace/table-plus/frontend/src/components/ui/icons.tsx:2-34` (33 Material Symbols Rounded, weight 500 từ `@material-symbols-svg/react`, `vite.config.ts:16-21`). Mỗi icon là **một** `<path d fill="currentColor">` trong `viewBox="0 -960 960 960"` (`icons.tsx:48-58`).
- Độ dài `d`: min 237 B, median 581 B, p90 1 093 B, max 1 661 B, tổng 20 330 B; 16–108 segment.
- Phương án PNG 2x (36×36) hiện tại, base64: min 572 B, median 1 080 B, max 2 424 B, tổng 40 488 B. Tức **gấp 2 lần** path mà vẫn mờ khi zoom và mất token.

**Prototype (Chromium 156.0.8078.4, Playwright build 1248)**

Script ở scratchpad, không commit: `/private/tmp/claude-501/-Volumes-MySpace-Workspace-OpenCanvas/0c1758af-4597-4d58-b91f-4496d5b95b83/scratchpad/p14/` (`compare2.mjs`, `icons.json`) và `.../scratchpad/p14proto/` (Rust, `svgtypes 0.15.3` offline).

Thuật toán vẽ ở mục Giải pháp 3, so với hai oracle:
- **SVG image:** cùng markup `<svg>` (vector = `<svg x y width height viewBox preserveAspectRatio><path/></svg>` lồng trong một svg gốc) nạp làm `Image`, `drawImage` ở cùng DPR. Đây là renderer SVG của Chrome, không qua compositor.
- **DOM:** cùng markup inline trong trang, chụp màn hình.

| Ca | DPR | vs SVG image (max kênh / px > 1) | vs DOM (max / px > 16 / px mực) |
| --- | --- | --- | --- |
| 33 icon 18 px, toạ độ nguyên | 1 / 2 | **0 / 0** · **0 / 0** | 0 / 0 / 3 002 · 15 / 0 / 9 718 |
| 33 icon 24 px | 1 / 2 | **0 / 0** · **0 / 0** | 0 / 0 / 4 934 · 59 / 33 / 16 121 |
| 33 icon 18 px lệch +0,37 px | 1 / 2 | **0 / 0** · **0 / 0** | 0 / 0 · 12 / 0 |
| 11 icon zoom 1,7 | 1 / 2 | **0 / 0** · **0 / 0** | 0 / 0 · 23 / 4 |
| `fit` contain/fit/cover trên hộp 48×24 | 1 / 2 | **0 / 0** · **0 / 0** | 0 · 0 |
| `fillRule` nonzero/evenodd (vòng vuông) | 1 / 2 | **0 / 0** · **0 / 0** | 0 · 0 |

- Lệch DOM ở DPR 2 (≤ 59 trên ≤ 0,2 % pixel mực > 16) là AA của raster compositor. Cùng path đó qua SVG image ra 0, nên đây không phải lệch hình học. AC dùng oracle SVG image.
- Bẫy khi viết test: `<svg>` inline đặt bằng CSS `left: 8.37px` bị Chrome snap về pixel nguyên (lần đo đầu lệch tới 156). Oracle phải lồng `<svg x y>` trong một svg gốc.
- Token: pixel đặc của icon = đúng màu fill (`#344054` → `52,64,84,255`; đổi `#ef4444` → `239,68,68,255`).
- Chi phí: vẽ 1 000 icon 18 px @2x với `Path2D` cache là 4 ms; parse 1 000 `Path2D` là 2,1 ms. P01 vẽ lại mỗi frame vẫn trong ngân sách.
- Validate `d` bằng `svgtypes::PathParser` (crate có sẵn trong registry offline, build 4,9 s): 33/33 icon hợp lệ. Lỗi có vị trí ký tự, ví dụ `"M10 10 L20 20 30"` → end of stream (vị trí 17), `"L10 10"` → vị trí 1, `"M10 10 X 5"` → vị trí 8. Message gốc của svgtypes quá chung chung nên cần bọc lại (mục 2).

**Tham khảo**

- Figma REST `VECTOR`: `fillGeometry`/`strokeGeometry: Path[]`, mỗi `Path {path: <SVG path data>, windingRule: "NONZERO"|"EVENODD"}` khi gọi `geometry=paths` (https://developers.figma.com/docs/rest-api/file-node-types/). Plugin API `VectorPath {windingRule, data}`, `data` là tập con path SVG (https://developers.figma.com/docs/plugins/api/VectorPath/).
- Penpot: shape `path` giữ `content`/`d` là path data (https://doc.plugins.penpot.app/interfaces/Path). Phần tử SVG không chuyển được thì giữ thành `svg-raw` (https://doc.plugins.penpot.app/interfaces/SvgRaw).
- URL tham khảo kiểm lại lúc 01:55 29/10: tất cả trả HTTP 200, anchor `PathDataBNF`, `FillRuleProperty` và `ArcImplementationNotes` có thật.
- SVG 2 path data grammar và `fill-rule` (https://www.w3.org/TR/SVG2/paths.html#PathDataBNF, https://www.w3.org/TR/SVG2/painting.html#FillRuleProperty). `preserveAspectRatio` mặc định `xMidYMid meet`; `<svg>` không phải gốc mặc định `overflow:hidden` (clip theo viewport).
- Canvas `new Path2D(d)` nhận SVG path data; `fill(path, "nonzero"|"evenodd")` (https://developer.mozilla.org/en-US/docs/Web/API/Path2D/Path2D).

## Giải pháp

### 1. Kind và props authored

```json
{"op":"create_node","id":"ic-search","kind":"vector","parentId":"toolbar",
 "props":{"width":18,"height":18,"viewBox":[0,-960,960,960],
          "path":"M376.57-324.93q-109.84 0-185.96-76.2…Z","fill":{"tokenId":"icon-muted"},
          "name":"Material Symbols Rounded 500/search"}}
```

`kind: "vector"` là leaf như `rectangle`/`image`, không nhận con.

| Key | Bắt buộc | Giá trị | Mặc định | Tương đương SVG/Figma |
| --- | --- | --- | --- | --- |
| `path` | có | chuỗi SVG path data (`d`), 1 B..32 KiB, lưu nguyên văn | — | `<path d>` / `VectorPath.data` |
| `viewBox` | không | `[minX, minY, width, height]`, số hữu hạn \|v\| ≤ 1e6, width/height > 0 | không có: toạ độ path là pixel của chính node | `viewBox` |
| `fillRule` | không | `"nonzero"` \| `"evenodd"` | `nonzero` | `fill-rule` / `windingRule` |
| `fit` | không | `"contain"` \| `"fit"` \| `"cover"` (từ vựng ảnh B05) | `contain` | `preserveAspectRatio` = `xMidYMid meet` / `none` / `xMidYMid slice` |
| `fill` | không | như hiện nay (hex, `{tokenId}` màu, `{styleId}` fill style); là **màu path** | `#000000` (giá trị khởi đầu của SVG) | `fill` |

**Luật**

- Key chung vẫn dùng như cũ: `name, styleId, x, y, width, height, min/max*, opacity, clip, radius`. `radius` bo góc vùng clip như ảnh; `clip` không có tác dụng (leaf).
- **Kích thước:** vector không có kích thước nội tại. Thiếu `width`/`height`, hoặc dùng `"hug"`, sẽ bị từ chối (field `width`/`height`): `Vector has no content size; give width and height (number, dimension token or "fill"), e.g. 24`.
- `fill` gradient (khi P09a merge) trên vector bị từ chối như trên text: `Gradient fill is not supported on vector yet; use a solid color`. Task nào merge sau thì thêm luật này.
- Catalog thêm alias để B02 gợi ý:
  - `path` ← `d`, `pathData`, `svgPath`, `data`, `vectorPaths`, `fillGeometry`;
  - `viewBox` ← `viewbox`, `view_box`;
  - `fillRule` ← `windingRule`, `fill-rule`, `clipRule`.

**Vì sao chọn dạng này** (A trong brief) thay vì chỉ giữ asset SVG (B, `drawImage`):

- AI viết và đọc path trực tiếp: đó chính là `d` của bộ icon (ít token, không base64).
- `fill` gắn token nên đổi theme đổi màu.
- AI coder dựng `<svg viewBox><path d fill="currentColor"/></svg>` 1:1.
- Không đưa XML/script vào storage hay renderer.
- Asset SVG vẽ bằng `drawImage` vẫn nét, nhưng: màu nướng trong file (không token); phải cho phép `image/svg+xml` trong `import_asset` (`assets.rs:200-211` đang chặn, có test `crates/storage/tests/assets.rs:44`), tức mở bề mặt tấn công XML/`<script>`/external ref; AI coder nhận lại một blob thay vì cấu trúc.

**Một path mỗi node** (Figma cho nhiều path chung một fill):

- Icon đơn sắc là 1 path (33/33 icon QueryNest).
- Icon nhiều màu là một `group` chứa nhiều `vector` cùng `viewBox`, `width/height:"fill"` (P14b sinh đúng dạng này).
- Mỗi node giữ một màu và một token, nên không cần mảng lồng.

### 2. Validate (storage)

- File mới `crates/storage/src/validate/vector.rs`:
  - `check_path(&Value)`: chuỗi, 1..32 768 byte, không rỗng sau trim. Duyệt `svgtypes::PathParser`; mọi số hữu hạn, \|v\| ≤ 1e6.
  - `check_view_box`, `check_fill_rule`, `check_vector_size(props)`.
- Message lỗi path (field `path`), một dòng, chỉ cách sửa:
  - `Invalid path data at character 17 (near "L20 20 30"): 'L' takes x y, numbers come in pairs; SVG d syntax: M x y, L x y, H x, V y, C x1 y1 x2 y2 x y, S x2 y2 x y, Q x1 y1 x y, T x y, A rx ry rotation large-arc sweep x y, Z (lowercase = relative)`.
  - Lỗi ở ký tự 1: `Path data must start with M or m`.
  - Lệnh lạ: `Unknown path command 'X' at character 8`.
  - Vị trí lấy từ lỗi svgtypes (`UnexpectedData/InvalidNumber/InvalidChar(pos)`), hoặc độ dài + 1 khi `UnexpectedEndOfStream`. Lệnh và số tham số lấy từ bảng `M/L/T:2, H/V:1, S/Q:4, C:6, A:7`, áp cho chữ lệnh gần nhất trước vị trí lỗi.
- `properties.rs:14-90` thêm `"path" | "viewBox" | "fillRule" => vector::…`. Sau `check_required`, nếu `NodeKind::Vector` thì gọi `check_vector_size`.
- `prop_keys.rs:45-50` thêm nhánh `PropScope::Vector`: `Vector requires path (SVG path data, e.g. "M4 4h16v16H4z")`.
- `props_catalog.rs`:
  - `PropScope::Vector` (label `vector`) cho `path/viewBox/fillRule`;
  - `PropScope::Fitted` (label `image, vector`) cho `fit`;
  - `NODE_KINDS` thêm `Vector` (7);
  - mô tả `fill` thêm "path color of vectors (default black, as in SVG)".
- `Cargo.toml` storage: `svgtypes = "0.15"` (0.15.3 cùng `kurbo 0.11.3`, `siphasher 1.0.3` đã có trong `~/.cargo/registry`, resolve `--offline`).

### 3. Scene và renderer

**Rust**

- `crates/layout/src/model.rs`: `Props` và `SceneNode` thêm `path: Option<String>`, `view_box: Option<[f32;4]>`, `fill_rule: Option<String>`. Trên `SceneNode` đặt `#[serde(skip_serializing_if = "Option::is_none")]`, nên scene không có vector giữ nguyên shape.
- `engine.rs`: chiếu ba field. Với kind vector:
  - `fill` mặc định `#000000`;
  - `fit` như ảnh;
  - `clip` mặc định false.
- `validation.rs:24`: thêm `"vector"`. Kiểm `viewBox` hữu hạn, w/h > 0; ngân sách path 32 KiB/node và 1 MiB/frame (cộng chung với text, như `:62-68`). Không parse lại `d`.
- Bump `SCENE_VERSION`/`RENDERER_VERSION` lên số kế tiếp chưa dùng lúc merge, ở cả hai phía. Sửa hai test hard-code.

**TS: `renderer/vector.ts` (mới, ~50 dòng)**

```ts
/** Fills the node's SVG path inside its bounds: viewBox placed by fit (as preserveAspectRatio), clipped like an inline <svg>. */
export function paintVector(ctx: CanvasRenderingContext2D, node: SceneNode): void {
  const b = node.bounds, [vx, vy, vw, vh] = node.viewBox ?? [0, 0, b.width, b.height];
  ctx.save(); ctx.beginPath(); ctx.roundRect(b.x, b.y, b.width, b.height, node.radius ?? 0); ctx.clip();
  const at = placeImage(node.fit, b, { width: vw, height: vh });
  ctx.translate(b.x + at.x, b.y + at.y); ctx.scale(at.width / vw, at.height / vh); ctx.translate(-vx, -vy);
  ctx.fillStyle = node.fill; ctx.fill(cachedPath(node.path!), node.fillRule ?? "nonzero"); ctx.restore();
}
```

- `cachedPath`: `Map<string, Path2D>` giới hạn 1 024 mục (xoá hết khi đầy). Viewer P01 vẽ mỗi frame không phải parse lại.
- Bounds 0 hoặc viewBox 0 thì không vẽ.
- `paint.ts:9`: `paintFill` bỏ qua `node.kind === "vector"`. `paint.ts:83-84` thêm `if (node.kind === "vector") paintVector(context, node)`.
- `types.ts`: `SceneNode` thêm `path?: string; viewBox?: [number, number, number, number]; fillRule?: "nonzero" | "evenodd"`.
- `drawing.ts`: không thêm resource (vector không cần tải gì).

**Viewer**

- `Icon.tsx`: icon `vector` (hình bút/đường cong).
- `Inspector.tsx`: chuỗi authored > 120 ký tự hiển thị `…(N bytes)`, `title` giữ đầy đủ.
- Hit test giữ theo bounds.

### 4. MCP

- `schemas/operations.rs:15` và `schemas.rs:101`: enum kind thêm `"vector"`; `find_nodes.kinds.maxItems` 6 → 7.
- `get_design_context`: `props` giữ nguyên văn (`path`, `viewBox`, `fill: {tokenId}`).
  - **`resolvedProps` không lặp `path`** ở cả hai chế độ, vì path không chứa tham chiếu. Sửa ở `reads/tree.rs:74` hoặc `compact.rs::resolved_props`: bỏ key `path` khi node là vector.
  - Thêm một câu vào mô tả tool: "vector path appears once, in props".
  - Chỉ áp cho key mới của kind mới, nên không đổi nghĩa field cũ.
- `props.md` (sinh tự động) có dòng `vector` và 3 key mới. Mục Notes thêm:
  > Icons and other non-rectangular shapes: kind `vector` with SVG path data and `viewBox`; color with `fill` (token), never rasterize icons to PNG.
- `crates/server/skills/author-frame.md`: "Icon: `vector` + path/viewBox của bộ icon (Material Symbols `[0,-960,960,960]`), `fill` token, `name` ghi tên icon gốc (`Material Symbols Rounded 500/search`) để AI coder import đúng icon".
- `implement-frame.md` (bảng `:25-36`) thêm dòng:
  > `vector` → `<svg width height viewBox="minX minY w h" preserveAspectRatio(fit)><path d=path fill=token|currentColor fill-rule/></svg>`; dùng `name` nếu codebase có sẵn thư viện icon.

## Phạm vi file

| File | Việc | Dòng (hiện → ước) |
| --- | --- | --- |
| `crates/storage/src/validate/vector.rs` (mới) | path/viewBox/fillRule/size, message có vị trí | ~120 |
| `crates/storage/src/validate.rs` | `mod vector` | +1 |
| `crates/storage/src/validate/properties.rs` | dispatch 3 key, kiểm size vector | 124 → ~131 |
| `crates/storage/src/validate/prop_keys.rs` | message required vector | 63 → ~66 |
| `crates/storage/src/props_catalog.rs` | scope `Vector`, `Fitted`, 3 entry, `NODE_KINDS` 7, mô tả fill/fit | 126 → ~146 |
| `crates/storage/src/model.rs` | `NodeKind::Vector` | 128 → ~130 |
| `crates/storage/Cargo.toml`, `Cargo.lock` | `svgtypes 0.15` | +1 / lock |
| `crates/layout/src/model.rs`, `engine.rs`, `validation.rs`, `lib.rs` | field, chiếu, mặc định, ngân sách, bump version | 210→~218, 192→~198, 130→~138 |
| `renderer/vector.ts` (mới) | `paintVector`, cache `Path2D` | ~50 |
| `renderer/paint.ts`, `renderer/types.ts`, `renderer/index.ts` | dispatch, kiểu, export, version | 98→~101, 115→~119 |
| `web/src/components/Icon.tsx`, `Inspector.tsx` | icon vector, cắt chuỗi dài | +6, +6 |
| `web/tests/state.test.ts`, `web/tests/fixture-data.mjs` | chuỗi version | 2 + 2 |
| `crates/server/src/schemas.rs`, `schemas/operations.rs` | enum kind | ±2 |
| `crates/server/src/reads/tree.rs` (hoặc `compact.rs`) | bỏ `path` khỏi resolvedProps | +4 |
| `crates/server/src/props_doc.rs` | dòng Notes | +2 |
| `crates/storage/tests/vector_props.rs` (mới) | AC1–AC3 | ~150 |
| `crates/layout/src/vector_tests.rs` (mới, `#[cfg(test)]`) | AC4 | ~70 |
| `spikes/pipeline/vector.test.mjs` (mới) + `spikes/pipeline/vector-fixture.mjs` (mới, 33 path icon chép từ `icons.json` của prototype, ~22 KB data) | AC5 | ~150 + data |
| `crates/server/tests/vector_context.rs` (mới) | AC6–AC8 | ~150 |
| `crates/server/skills/{author,implement}-frame.md`, `docs/{SERVER,CONTRACT,VIEWER}.md` | tài liệu | ~15 |

`vector-fixture.mjs` là data (ngoại lệ Modular, ghi rõ). Không file viết tay nào vượt 200 dòng mới / 250 dòng hiện có.

## Tương thích MCP

- `tools/list` chỉ **thêm**:
  - giá trị `"vector"` trong enum kind của `create_node` và `find_nodes`;
  - `maxItems` của `kinds` 6 → 7.
- Không tool mới, không field cũ đổi nghĩa.
- Client cũ không tạo vector thì không thấy gì khác. Client đọc project có vector sẽ thấy `kind:"vector"` và các key mới.
- Scene: version bump. Scene không có vector giữ nguyên shape JSON nhờ `skip_serializing_if`.
- Tài liệu cần sửa: props.md (tự sinh), `author-frame.md`, `implement-frame.md`, `docs/SERVER.md` (kinds), `docs/CONTRACT.md` (`SceneNode`), `docs/VIEWER.md` (vector paint), `spikes/pipeline/README.md` (version).

## AC

1. **Tạo và đọc.** `apply_batch` tạo `vector` với 24×24, `viewBox:[0,-960,960,960]`, path `search` của QueryNest, `fill:"#344054"` → commit. `get_outline` trả `kind:"vector"`. Test B02 `validator_accepts_exactly_the_catalog_keys_per_kind` pass mà không sửa logic. Allowed keys của vector = 14 key scope `Any` cộng `path, viewBox, fillRule, fit`, tổng 18 ở `565b12b`, theo thứ tự bảng. Nếu P04a/P09a… đã thêm key `Any` thì cộng theo.
2. **Lỗi path có vị trí** (field `path`, `nodeId`, `nodeKind:"vector"`):

   | Input | Message chứa |
   | --- | --- |
   | `"M10 10 L20 20 30"` | `character 17` và `'L' takes x y` |
   | `"L10 10"` | `character 1` và `must start with M or m` |
   | `"M10 10 X 5"` | `character 8` và `'X'` |
   | `"   "` | `empty` |
   | 32 769 byte | `32768` |
   | `"M1e400 0"` | lỗi số không hữu hạn |

   Batch lỗi không để lại commit (revision không đổi).
3. **Lỗi khác:**

   | Input | Field | Message chứa |
   | --- | --- | --- |
   | `viewBox:[0,0,24]` | `viewBox` | `[minX, minY, width, height]` |
   | `viewBox:[0,0,0,24]` | `viewBox` | `width` và `height` phải > 0 |
   | `fillRule:"even-odd"` | `fillRule` | `nonzero` và `evenodd` |
   | vector thiếu `path` | `path` | — |
   | vector `width:"hug"`, hoặc thiếu `height` | `width` / `height` | `no content size` |
   | `path` trên rectangle | `props.path` | `allowedKeys` của rectangle, không có `path` |
   | `d` trên vector | `props.d` | `suggestion:"path"` |
   | `assetId` / `crop` trên vector | lỗi key | — |
4. **Scene** (unit trong `crates/layout`):
   - vector có `path`, `viewBox`, `fit:"contain"`, `fill:"#000000"` khi không authored fill; `fillRule` chỉ có khi authored;
   - fixture `spikes/pipeline/fixtures.mjs` (không vector): `scene.nodes` serialize **byte-identical** với trước (so chuỗi JSON `nodes`; `provenance` khác do version).
5. **Pixel** (`spikes/pipeline/vector.test.mjs`, Chromium pinned). Frame 420×300 gồm:
   - 33 icon 18 px `#344054` (toạ độ nguyên và lệch +0,37);
   - 3 vector 48×24 `fit` contain/fit/cover;
   - 2 vector `fillRule` nonzero/evenodd.

   Render qua pipeline thật ở scale 1 và 2, so với oracle SVG image cùng scale (svg gốc 420×300, mỗi vector là `<svg x y width height viewBox preserveAspectRatio><path d fill fill-rule/></svg>`). Đạt khi **max kênh ≤ 1** (prototype đo 0).
6. **Token** (test server, worker thật):
   - token `icon-fg` `#344054` có theme `night` = `#e4e7ec`;
   - vector 48×48 path `M2 2h20v20H2z` viewBox 24 với `fill:{tokenId:"icon-fg"}`;
   - `render_frame` → pixel (24,24) = `52,64,84`;
   - `upsert_token` value `#ef4444` → render mới, cùng pixel = `239,68,68`;
   - `theme:"night"` → `228,231,236`.
7. **`get_design_context`:**
   - `props.path` nguyên văn, `props.fill = {tokenId:"icon-fg"}`;
   - `resolvedProps` không có key `path` (thường và compact), `resolvedProps.fill = "#344054"`;
   - compact: `tokens` có `icon-fg`;
   - frame 33 icon QueryNest: response compact ≤ 30 000 byte và mỗi chuỗi path xuất hiện đúng 1 lần (đếm substring).
8. **`tools/list`:** enum kind của `create_node` và `find_nodes` chứa `vector`; `find_nodes {kinds:["vector"]}` trên frame AC7 trả `totalMatches: 33`.
9. **props.md:**
   - có dòng `vector`;
   - có dòng key `path` (scope `vector`, required yes);
   - dòng `fit` scope `image, vector`;
   - `every_kind_line_and_key_is_published` pass.
10. **Viewer** (test web): `hitTest` tại tâm bounds trả node vector; `Inspector` hiển thị path dài 1 661 byte dưới dạng ≤ 120 ký tự + `(1661 bytes)`.
11. **Modular:** file mới ≤ 200 dòng (trừ data fixture); báo `wc -l` mọi file sửa; không file nào vượt 250 dòng vì task này.

## Cách kiểm

- `CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-storage --test vector_props --test prop_catalog` (AC1–3).
- `cargo test --offline -p opencanvas-layout` (AC4).
- `npm run test:pipeline` (AC5; tự build renderer và `layout-step`).
- `cargo test --offline -p opencanvas-server --test vector_context` và `cargo test --offline -p opencanvas-server props_doc` (AC6–9).
- `npm --prefix web test` (AC10, version chuỗi).
- Kiểm live: `target/debug/opencanvas-server --port 0 --data-root $(mktemp -d)`.
  - Qua MCP: tạo project, frame, 3 vector (một dùng token); `render_frame`; mở viewer ở zoom 400 % để nhìn icon nét; `get_design_context compact:true`.
  - Dọn process và data sau khi kiểm.

## Rủi ro

- **Parser lệch Chromium:** svgtypes nhận một chuỗi mà Blink coi là lỗi thì Chrome vẽ thiếu đoạn sau lỗi, và lỗi này im lặng. Giảm thiểu: test AC5 thêm 8 chuỗi khó (`a1 1 0 00.5.5` cờ dính, `1e1`, `-.5.5`, `M0 0 10 10` lineto ngầm, `z m2 2`, …). Mỗi chuỗi nối thêm `M0 0H1V1H0Z` ở cuối: nếu Chrome parse hết thì ô 1×1 xuất hiện.
- **Rollback binary:** sau khi có node `vector`, binary cũ không đọc được project đó (`db.rs:64` deserialize `NodeKind`). Không phải migration schema, nhưng orchestrator gốc cần biết trước khi deploy lên 3847 (giống P06).
- **Kích thước đọc:** icon nặng hơn image node (~600 B so với ~150 B mỗi node). Giảm thiểu: không lặp trong `resolvedProps`, compact mặc định trong skill, `find_nodes properties` lọc được. Icon lặp lại nhiều lần (chevron ×10) vẫn mang path đầy đủ mỗi lần (xem phase sau).
- **AA DOM ở DPR 2:** AI coder dựng inline `<svg>` sẽ lệch ≤ 59/255 ở < 0,2 % pixel mép so với render của OpenCanvas. Đây là AA của compositor, không sửa được; đã đo và ghi lại.
- **File nóng:** `paint.ts`, `props_catalog.rs`, `engine.rs`, `model.rs` cũng bị P04a, P08a, P09a sửa. Lấy số version kế tiếp tại lúc merge.
- **Icon nét (Lucide, Feather, Heroicons outline)** chưa biểu diễn được cho tới P14c (stroke cho vector).
- **Va chạm với P04a** (bổ sung 01:58): `PropScope::Box` của P04a cho mọi kind trừ page/text (`night/P04a:crates/storage/src/props_catalog.rs:24`), nên khi có kind `vector`, `stroke` sẽ lọt key check và bị vẽ như viền hộp. P14a phải đổi `Box.allows` thành loại trừ cả `Vector` (1 dòng). Test: `stroke` trên vector bị lỗi key `props.stroke` cho tới khi P14c merge.

## Ngoài phạm vi / phase sau

- **P14b:** op `import_svg` (SVG text → node `vector`/`group`, chặn script/external ref).
- **P14c (đề xuất kế tiếp, sau P04a):** `stroke` trên vector (center, `cap`/`join`, độ dày theo đơn vị viewBox) để biểu diễn bộ icon nét.
- Gradient trên vector (cần quy ước bbox của path như SVG `objectBoundingBox`).
- Nhiều path mỗi node, boolean ops, chỉnh path trên viewer (`PLAN.md:106` loại vector editor).
- Symbol/instance để icon lặp lại không mang path nhiều lần (cần mô hình component).
- Hit test chính xác bằng `isPointInPath`.
- Export SVG (P03b): mỗi vector ra `<svg x y width height viewBox preserveAspectRatio><path/></svg>` lồng. Prototype đã dùng đúng mapping này làm oracle (0/255). Task nào merge sau thì thêm nhánh `vector` (~15 dòng) cùng AC "export SVG của frame AC5 rasterize ≤ 1/255 so với PNG".
