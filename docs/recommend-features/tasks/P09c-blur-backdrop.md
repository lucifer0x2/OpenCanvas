# P09c — Prop `blur` (CSS `filter: blur`) và `backdropBlur` (CSS `backdrop-filter: blur`)
Ưu tiên: P2 · Kích cỡ: ~85 phút dev · Phụ thuộc: **P05 bắt buộc** (layer offscreen `paintTree`); sau P01 (`renderer/paint.ts`); nếu P09b đã merge thì đặt backdrop trước bóng ngoài (mục 2) · Nguồn: ghi chú người dùng mục "Thiếu tính năng → Blur / backdrop-filter" và "Kết quả" (`/Users/luis/Documents/opencanvas.md:69,95`). Thay mục P16 trong hàng đợi nghiên cứu.

Phase c của P09. Phase a (`P09a-gradient-fill.md`) gradient, phase b (`P09b-box-shadow.md`) shadow. P09c tự đủ giá trị: modal Settings/Backup làm mờ nền như app, glow mờ (`filter: blur`) vẽ bằng một node.

## Vấn đề

- Modal Settings/Backup trong app làm mờ nền (`backdrop-filter`), trên canvas nền vẫn sắc nét: ảnh render khác app ở mọi màn có modal ("không có blur nền modal").
- Không có blur lớp: glow/đốm sáng trang trí kiểu `filter: blur(64px)` không vẽ được. P09a giải quyết phần lớn glow bằng radial gradient, nhưng hình mờ không tròn (blob, chữ mờ) vẫn thiếu.
- AI coder không biết overlay có `backdrop-filter`, nên code modal phẳng.

## Bằng chứng

Code ở `night/integration@c3ab012`:

- `crates/storage/src/props_catalog.rs:73-96`: không có `blur`, `backdropBlur`, `filter`. Key lạ bị trả `field:"props.<key>"` (B02).
- `renderer/drawing.ts:136-156`: vẽ thẳng từng node lên canvas đích, không có offscreen. P05 (`.night/proposals/P05-group-opacity-layers.md`) thêm `paintTree` với layer `HTMLCanvasElement` căn lưới thiết bị, `extent` từng subtree và ngân sách `MAX_LAYER_PIXELS`; P05 ghi sẵn "dùng lại hạ tầng layer cho blur/backdrop (P16)".
- Viewer P01 vẽ scene qua Konva `sceneFunc` lên canvas layer cỡ màn hình (`night/P01@06ae746`, `web/src/components/CanvasView.tsx`); canvas này đọc lại được bằng `drawImage(context.canvas, …)`.
- Canvas: `context.filter = "blur(Npx)"` có trong Chromium; giống `shadowBlur`, độ dài filter **không theo transform** (đo: phải nhân scale thiết bị).

**Prototype đo trên Chromium pinned của repo (chromium-1248)** — scratch, không commit: `/private/tmp/claude-501/-Volumes-MySpace-Workspace-OpenCanvas/a47874fe-c598-4ad4-a1cc-0f72b40e19ca/scratchpad/p09/` (`lblur.mjs`, `blur.mjs`, `perf4.mjs`).

*Blur lớp* (hình tròn 120×120 `#6366f1` tại (140,90), so CSS `filter: blur(Npx)`): layer cỡ `extent + 3σ`, composite với `filter = blur(N·k px)` dưới transform đơn vị.

| blur | dsf | max lệch toàn vùng | (200,150) CSS / canvas | (200,90) | (200,80) |
| --- | --- | --- | --- | --- | --- |
| 6 | 1 | **0** | 99,102,241 / = | 175,176,248 / = | 249,249,254 / = |
| 20 | 1 | **0** | 99,102,241 / = | 185,187,249 / = | 213,214,251 / = |
| 6 | 2 | 1 | | | |
| 20 | 2 | 1 | | | |

*Backdrop blur* (nền sọc đỏ/trắng 20 px + thanh xanh; overlay 200×120 radius 12, fill `rgba(15,23,42,.4)`, `backdrop-filter: blur(8px)`):

| Cách lấy mẫu mép | dsf | vùng trong (cách mép ≥ 3σ) max / mean | dải mép 3σ max |
| --- | --- | --- | --- |
| lấy cả nội dung ngoài hộp | 1 | 1 / 0,28 | **54** |
| gương (mirror) trong hộp | 1 | 1 / 0,28 | 2 |
| kẹp (clamp) mép | 1 | 1 / 0,28 | 2 |
| gương | 2 | 1 / 0,27 | 30 (1 điểm tại góc thanh xanh ở mép phải) |

Kết luận: Chrome **không** lấy mẫu ngoài hộp; nhân blur là σ = N (khác `box-shadow` có σ = blur/2).

*Chi phí* (headless, software canvas, trung vị 9 lần): backdrop phủ 1440×900 tốn ~8 ms @1 và ~38–43 ms @2 (blur 4–20). Giảm độ phân giải ×2 trước khi blur chỉ còn ~24–28 ms @2 và lệch tới 6/255, nên không dùng. Overlay 200×120: 0,6 ms @1, 1,6 ms @2.

**Tham khảo**

- CSS Filter Effects 1 `blur()`: tham số là độ lệch chuẩn (https://www.w3.org/TR/filter-effects-1/#funcdef-filter-blur).
- Filter Effects 2 `backdrop-filter` (https://drafts.fxtf.org/filter-effects-2/#BackdropFilterProperty): ảnh nền phía sau được lọc rồi vẽ trong border box (có `border-radius`); **Backdrop Root**: tổ tiên có `opacity < 1`, `filter`, `mask`… chặn phạm vi nền đọc được.
- Figma Effect `LAYER_BLUR`/`BACKGROUND_BLUR {radius}` (https://developers.figma.com/docs/rest-api/file-node-types/): `radius` của Figma = 2σ; Dev Mode xuất `blur(radius/2)`. Vì vậy OpenCanvas dùng số CSS, không dùng số Figma.
- Penpot Blur (https://doc.plugins.penpot.app/interfaces/Blur): `{type:'layer-blur', value, hidden}`, chỉ có layer blur.
- Tailwind: `blur-*`, `backdrop-blur-*` (tên agent coder quen).

## Giải pháp

### 1. Prop authored

```json
{"blur": 64}
{"fill": "#0f172a66", "backdropBlur": 4}
```

| Key | Kind | Giá trị | CSS |
| --- | --- | --- | --- |
| `blur` | frame, group, rectangle, image, text | số 0..250 hoặc `{tokenId}` dimension; px, σ như CSS | `filter: blur(<n>px)` |
| `backdropBlur` | frame, group, rectangle, image | số 0..250 hoặc `{tokenId}` dimension | `backdrop-filter: blur(<n>px)` |

- 0 hoặc bỏ = không blur. Không hợp lệ trên page; `backdropBlur` không hợp lệ trên text (CSS cho phép nhưng nền chữ không có hộp hiển thị, dễ hiểu nhầm; lời nhắn bảo bọc frame).
- Hai số phẳng, không lồng: một node hiếm khi cần nhiều filter; giữ ít token (`"backdropBlur":4` = 16 byte).
- Message và mô tả catalog nhắc: "CSS px (standard deviation); a Figma blur radius R is R/2 here".
- Alias catalog: `blur` ← `filter`, `layerBlur`, `blurRadius`; `backdropBlur` ← `backdropFilter`, `backdrop-filter`, `backgroundBlur`, `backdrop`.

**Phương án đã loại**

| Phương án | Lý do loại |
| --- | --- |
| `filter: "blur(4px)"` chuỗi CSS | Cần parser; agent sẽ thử `brightness()`, `drop-shadow()` mà renderer không có. |
| `effects:[{type:"LAYER_BLUR",radius}]` kiểu Figma | Số Figma là 2σ, lệch CSS; thêm tầng lồng. |
| Gộp vào `shadow` (P09b) | Khác ngữ nghĩa (bóng theo hộp; blur theo nội dung và nền). |

### 2. Vẽ

**Blur lớp (`blur`)** — trong `paintTree` của P05:
- Node có `blur > 0` **luôn** thành layer (kể cả `units = 1`), vẽ node và subtree với alpha 1.
- Kích thước layer = `extent` của subtree nới `3·blur` mỗi phía (vùng mờ lan ra), giao `clipRect` của đích, rồi làm tròn ra pixel như P05.
- Composite: `setTransform(1,0,0,1,0,0)`, `globalAlpha = a'`, `filter = "blur(" + blur·k + "px)"`, `drawImage(layer, x0, y0)`, rồi `filter = "none"`. `k = √|det M|`.
- `extent` của node có `blur` (và tổ tiên) trong `scene-tree.ts` nới `3·blur`. Clip của chính node vẫn cắt con trước khi blur (như CSS: `overflow` trong phần tử được lọc).

**Backdrop (`backdropBlur`)** — bước đầu tiên của `paintNode`, trước bóng ngoài (P09b) và fill:
1. `src` = `context.canvas` (canvas đích hoặc canvas layer P05 đang vẽ). Vùng `r` = bbox thiết bị của `roundRect(bounds)` qua `M`, làm tròn ra pixel, giao canvas; rỗng thì bỏ.
2. Canvas tạm cỡ `r + 2p` với `p = ceil(3·σ_dev)`, `σ_dev = backdropBlur·k`; vẽ `src[r]` vào giữa và **lật gương** 8 ô xung quanh (edge mode như Chrome; đo lệch mép ≤ 2 ở dsf 1).
3. Canvas thứ hai: `filter = blur(σ_dev px)`, `drawImage(tạm)`.
4. Trên đích: `save`, clip `roundRect(bounds, radius)` theo transform hiện tại, `setTransform(1,0,0,1,0,0)`, `drawImage(kết quả, p, p, r.w, r.h, r.x, r.y, r.w, r.h)`, `restore`.
5. Sau đó vẽ fill (thường trong mờ) như cũ.

**Backdrop Root như CSS:** tổ tiên có `opacity < 1` hoặc `blur > 0` chặn nền. Với P05, chỉ cần: node nào có con cháu dùng `backdropBlur` và `a' < 1` thì **luôn** thành layer (kể cả `units = 1`), để backdrop chỉ đọc nội dung trong group như CSS. `scene-tree.ts` tính cờ `hasBackdrop` cho subtree (post-order, cùng lượt với `units`).

**Viewer (P01):** `paintScene` chạy trong `sceneFunc` của Konva; `context.canvas` là canvas layer Konva, đọc được. Shape của P01 đặt `listening={false}` (`web/src/components/CanvasView.tsx:106-108` trên `night/P01@06ae746`), nên hit canvas không gọi `sceneFunc` và backdrop không chạy hai lần.

**Ngân sách:** canvas tạm của backdrop và layer blur tính vào `MAX_LAYER_PIXELS` của P05; vượt thì vẽ không blur (backdrop bỏ bước 1–4, layer blur composite không filter) và đếm `blurFallbacks` trong kết quả `drawScene`, không ném lỗi.

### 3. Scene, resolve, validate

- `crates/layout`: `Props.blur`, `Props.backdrop_blur: Option<f32>`; `SceneNode.blur`, `SceneNode.backdrop_blur: f32` với `#[serde(default, skip_serializing_if = "is_zero")]` (node cũ byte-identical); `validation.rs` kiểm hữu hạn 0..250; bump version lên số kế tiếp ở Rust và `renderer/types.ts`, sửa 2 chuỗi trong `web/tests`.
- `crates/storage`: `refs.rs:98-99` thêm `"blur" | "backdropBlur"` vào nhóm resolve `"dimension"`; `properties.rs` kiểm `number(v, 0., 250., k)` với message riêng; `props_catalog.rs` 2 entry (`blur` scope mới "mọi kind trừ page" hoặc dùng `Box` + text; `backdropBlur` scope `Box` của P04a).
- Lỗi (format B02): `blur:-1` → `field:"blur"`, `Blur must be 0..250 CSS px (standard deviation; a Figma blur radius R is R/2) or a dimension token`; `backdropBlur` trên text → `field:"props.backdropBlur"`, `nodeKind:"text"`, message chứa `wrap text in a frame`.

### 4. `get_design_context` cho AI coder

- `props.blur`/`backdropBlur` nguyên văn, `resolvedProps` literal (token dimension nếu có).
- `implement-frame.md` (bảng `:25-36`): `blur` → `filter: blur(<n>px)`; `backdropBlur` → `backdrop-filter: blur(<n>px)` (thêm `-webkit-backdrop-filter` cho Safari) cùng `background` trong mờ từ `fill`.
- `author-frame.md`: overlay modal = rect phủ màn hình `fill` trong mờ + `backdropBlur`; số là CSS px, không phải bán kính Figma.

## Phạm vi file

| File | Việc | Dòng (ước) |
| --- | --- | --- |
| `renderer/backdrop.ts` (mới) | `paintBackdrop(context, node, budget)`: rect thiết bị, gương, blur, vẽ lại có clip | ~110 |
| `renderer/layers.ts` (P05) | ép layer khi `blur > 0` hoặc (`hasBackdrop` và `a' < 1`); composite với `filter`; đếm fallback | +20 (≤ 200) |
| `renderer/scene-tree.ts` (P05) | `extent` nới `3·blur`; cờ `hasBackdrop` | +12 |
| `renderer/paint.ts` | gọi `paintBackdrop` đầu `paintNode` | +2 |
| `renderer/types.ts` | `blur?`, `backdropBlur?`, bump version | +4 |
| `crates/layout/src/model.rs`, `engine.rs`, `validation.rs`, `lib.rs` | 2 field Props + 2 field Scene, chiếu, kiểm, version | +4, +2, +3, ±2 |
| `crates/storage/src/props_catalog.rs`, `validate/refs.rs`, `validate/properties.rs` | 2 entry, resolve dimension, kiểm khoảng | +3, +1, +4 |
| `web/tests/state.test.ts`, `web/tests/fixture-data.mjs` | chuỗi version | 2 dòng mỗi file |
| `crates/storage/tests/blur_props.rs` (mới) | AC1–AC3 | ~110 |
| `spikes/pipeline/blur.test.mjs` (mới) | AC5–AC10, oracle CSS | ~180 |
| `crates/server/tests/blur_context.rs` (mới) | AC4 | ~70 |
| skill `implement-frame.md`, `author-frame.md`; `docs/VIEWER.md`, `docs/CONTRACT.md` | map CSS, backdrop root, ngân sách | vài dòng |

`crates/layout/src/model.rs` (210 dòng) lên ~214, vẫn dưới 250; nếu P04a/P09a/P09b đã đẩy nó quá 230 thì chuyển `SceneNode` sang `model/scene.rs` trong cùng task.

## Tương thích MCP

- `tools/list` không đổi (props tự do); `tool_schemas.rs` pass không sửa.
- 2 prop optional mới; `allowedKeys` thêm key (chỉ thêm). Scene node không blur byte-identical trừ `provenance`; `sceneKey` đổi một lần do bump.
- `drawScene` trả thêm `blurFallbacks` (worker trả trong `draw`, server bỏ qua), như `opacityLayers` của P05.

## AC

Toạ độ frame-local. Dung sai ±2 mỗi kênh RGB so CSS Chrome cùng phiên bản (oracle dựng `div` qua `worker.runtime.browser.newPage()` như P05), trừ khi ghi khác.

**AC1 — Validator nhận:** `blur:64` trên rectangle, text, frame, group, image; `backdropBlur:4` trên rectangle, frame, group, image; `blur:{tokenId:<dimension>}`; `blur:0`.

**AC2 — Validator từ chối:** `blur:-1`, `blur:251` → `field:"blur"`, message chứa `R/2`; `backdropBlur` trên text → `field:"props.backdropBlur"`, `nodeKind:"text"`; `blur` trên page → `props.blur`; `filter:"blur(4px)"` → lỗi key `suggestion:"blur"`; `backdropFilter:4` → `suggestion:"backdropBlur"`.

**AC3 — Catalog.** Test B02 `validator_accepts_exactly_the_catalog_keys_per_kind` pass không sửa logic; props.md có `blur` ở 5 kind và `backdropBlur` ở 4 kind.

**AC4 — Scene và design context.** Scene fixture `spikes/pipeline/fixtures.mjs` trước/sau bằng nhau (bỏ `provenance`, `sourceSceneKey`), không node nào có `blur`/`backdropBlur`. `get_design_context` của node `backdropBlur:{tokenId:"blur-sm"}` (token = 4): `props` giữ ref, `resolvedProps.backdropBlur == 4`, `compact:true` có `blur-sm` trong `tokens`.

**AC5 — Blur lớp.** Root 400×300 fill trắng; rect 120×120 radius 60 `#6366f1` tại (140,90):
- `blur:20`: (200,150) = `99,102,241`; (200,90) = `185,187,249`; (200,80) = `213,214,251` (mờ lan ra ngoài bounds);
- `blur:6`: (200,90) = `175,176,248`; (200,80) = `249,249,254`;
- toàn vùng so CSS `filter: blur(Npx)` ở scale 1 và 2: max lệch ≤ 2 (đo 0 và 1).

**AC6 — Blur lớp có con và opacity.** Group `blur:8, opacity:0.5` chứa 2 rect chồng nhau: toàn vùng so CSS (`filter: blur(8px); opacity: .5`) ≤ 2; số layer `drawScene` báo (`opacityLayers` của P05, đếm cả layer do blur) = 1, tức một layer cho cả blur và opacity.

**AC7 — Backdrop.** Root 400×260, nền sọc: rect đỏ `#ef4444` 20×260 tại x = 0, 40, 80…, thanh xanh `#1d4ed8` (60,100) 280×30; overlay rect (100,60) 200×120 radius 12 `fill:"#0f172a66"`, `backdropBlur:8`.
- Vùng trong overlay cách mép ≥ 24 px: max lệch so CSS ≤ 2 ở scale 1 và 2 (đo 1).
- Dải mép 24 px: ≤ 4 ở scale 1 (đo 2); ở scale 2 chấp nhận ≤ 32 tại ≤ 20 kênh (đo: 30 tại 1 điểm).
- Ghi thêm trong test: lấy mẫu cả nội dung ngoài hộp cho dải mép lệch ≥ 40 (đo 54), tức gương là cần thiết.
- Ngoài overlay: byte-identical với cùng cảnh không có `backdropBlur`.

**AC8 — Thứ tự.** Overlay AC7 có thêm con text trắng: text sắc (pixel thân chữ = `255,255,255` ±2). Nếu P09b đã merge: overlay có `shadow:[{y:8,blur:16,color:"#000000"}]` → vùng trong overlay vẫn khớp CSS ±2 (backdrop không chứa bóng của chính nó).

**AC9 — Backdrop Root.** Group `opacity:0.5` (một con: overlay AC7) trên nền sọc: khớp CSS ±2 trong vùng overlay, tức nền **không** bị blur (CSS: group mờ là backdrop root); layer được tạo dù `units = 1`.

**AC10 — Transform và viewer.** `paintScene` (P01) trên canvas `setTransform(2,0,0,2,0,0)` cho RGBA giống từng byte `drawScene` scale 2 với cảnh AC5 + AC7. Viewer (`web/tests/zoom.mjs`): cảnh AC7 phóng 1440×900 không còn lỗi console và PNG worker = canvas parity của viewer byte-identical (`pipeline.test.mjs`).

**AC11 — Ngân sách.** `paintTree(..., { maxLayerPixels: 5000 })` với cảnh AC5 và AC7: không ném lỗi, `blurFallbacks ≥ 1`, ảnh bằng cảnh không blur.

**AC12 — Hiệu năng** (đo tay, ghi `.night/dev/P09c.md`). Overlay phủ 1440×900 `backdropBlur:8` trên cảnh dense của P01: `drawScene` trung vị ≤ 15 ms @1, ≤ 60 ms @2 (đo prototype ~8 / ~40). Viewer DPR 2, cử chỉ zoom của P01: p95 ≤ 50 ms, 0 long task ≥ 100 ms. Cảnh không blur: AC5 của P01 vẫn pass.

**AC13 — Không regression.** `fixture(420)` scale 1 và 2, `geometry.test.mjs`, `opacity.test.mjs` (P05): RGBA giống từng byte bản trước P09c.

**AC14 — Live.** Server tạm: frame modal (nền nhiều màu + overlay `backdropBlur:4` + dialog) → `render_frame` `ready`, `diagnostics:[]`; PNG có vùng nền dưới overlay mờ (độ lệch giữa hai cột sọc kề nhau trong overlay < 30% so với ngoài overlay).

**AC15 — Modular.** `renderer/backdrop.ts` ≤ 150 dòng, `layers.ts` ≤ 200, `scene-tree.ts` ≤ 150; `cargo fmt --check`, clippy, `npm run check:renderer`, `npm --prefix web run build` pass.

## Cách kiểm

```bash
W=<worktree>; cd $W && export CARGO_BUILD_JOBS=4
cargo test --offline -p opencanvas-layout -p opencanvas-storage -p opencanvas-server
cargo clippy --offline -p opencanvas-layout -p opencanvas-storage -p opencanvas-server --lib --bin opencanvas-server -- -D warnings
cargo fmt --all --check && npm run check:renderer
PLAYWRIGHT_BROWSERS_PATH=$W/.runtime/browsers npm run test:pipeline       # gồm blur.test.mjs, opacity.test.mjs
npm --prefix web test && npm --prefix web run build
PLAYWRIGHT_BROWSERS_PATH=$W/.runtime/browsers npm --prefix web run test:zoom
```

**Prototype tham chiếu** (scratch, có thể bị dọn): `node lblur.mjs` (AC5), `node blur.mjs` (AC7, ba cách lấy mẫu mép), `node perf4.mjs` (AC12), với `PLAYWRIGHT_BROWSERS_PATH=/Volumes/MySpace/Workspace/OpenCanvas/.runtime/browsers`.

**Live (AC14):** server tạm `--port 0 --data-root $(mktemp -d)`, script mẫu `.night/qa/mcp-tools.mjs`, dọn sau. Không đụng 3847.

## Rủi ro

- **Phụ thuộc P05:** chưa có layer thì không làm được `blur` đúng cho subtree. Nếu P05 trượt, chỉ làm `backdropBlur` (không cần layer, trừ AC6/AC9) và để `blur` lại.
- **Chi phí viewer:** backdrop phủ màn hình tốn ~40 ms/frame @2 trên software canvas, nên khi zoom màn có modal sẽ giật (~25 fps) trong headless. GPU thật có thể nhanh hơn nhiều nhưng chưa đo được. Giảm thiểu: P01b (snapshot bitmap trong lúc cử chỉ), hoặc cache ảnh backdrop theo (node, `sceneKey`, `k`) cho lúc pan.
- **Đọc lại canvas viewer:** nếu canvas Konva bị "tainted" (ảnh khác origin) thì `drawImage` canvas vào canvas vẫn chạy (không cần `getImageData`), nên không lỗi bảo mật. Ảnh asset đi qua cùng origin server.
- **Mép ở scale 2** lệch tới 30 tại một điểm (Chrome có thể mở rộng vùng đọc 1 px thiết bị). AC chỉ đặt chặt cho vùng trong.
- **Export (P03a/P03b):** PNG crop theo `bounds` sẽ cắt phần blur lan ra; SVG cần `<filter><feGaussianBlur>`, còn backdrop không có tương đương SVG chuẩn: phát diagnostic.

## Ngoài phạm vi / phase sau

- Các filter khác (`brightness`, `saturate`, `drop-shadow`), backdrop filter khác blur.
- Blur theo hướng/progressive blur, mask.
- Cache ảnh backdrop và layer blur giữa các frame trong viewer.
- Bounds đọc được của phần blur lan ra (`paintBounds`).
