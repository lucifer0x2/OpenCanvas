# P04b — `radius` từng góc `[topLeft, topRight, bottomRight, bottomLeft]`
Ưu tiên: P1 · Kích cỡ: ~45 phút dev · Phụ thuộc: **P04a** (dùng `Sides<T>`, `renderer/stroke.ts`, `validate/stroke.rs`); cùng vùng vẽ với P01/P05 nên xếp tuần tự · Nguồn: ghi chú người dùng mục "Thiếu tính năng → Corner radius từng góc (tab chỉ bo góc trên…)"

Phase b của P04. Sau P04a, viền và nền đã đúng với radius **đều**. Phase này cho từng góc một radius riêng: tab chỉ bo góc trên, bubble chat, nửa trái của segmented control, card ghép.

## Vấn đề

- `radius` chỉ nhận một số cho cả bốn góc.
- Muốn tab chỉ bo góc trên, agent phải xếp rect bo góc chồng lên rect vuông (thừa 1 node), hoặc dùng frame `clip` bo đều rồi che hai góc dưới. Cách nào cũng làm AI coder đọc sai cấu trúc.
- Khi có viền (P04a) thì không giả được nữa: viền đi theo đúng hình node, nên hình phải đúng ngay từ prop.

## Bằng chứng

**Validator, catalog, resolver**

- `crates/storage/src/validate/properties.rs:25`: `"radius" => number(v, 0., 100_000., k)?`, mảng bị từ chối.
- `crates/storage/src/props_catalog.rs:92`: mô tả `"one radius for all corners"`; alias `cornerRadius`, `borderRadius`, `corner_radius`, `rx`.
- `crates/storage/src/validate/refs.rs:98-99`: `radius` resolve token `dimension` ở cấp đầu, không resolve phần tử mảng.

**Layout**

- `crates/layout/src/model.rs:75` có `radius: Option<f32>` và `:161` có `radius: f32`.
- `crates/layout/src/validation.rs:38-50` kiểm `p.radius` hữu hạn.
- `crates/layout/src/engine.rs:150` chiếu ra scene.

**Renderer**

- Có 3 chỗ dùng `roundRect(..., node.radius ?? 0)`:
  - fill: `renderer/drawing.ts:23`;
  - clip ảnh: `:54`;
  - clip tổ tiên: `:153`.
- `CanvasRenderingContext2D.roundRect` nhận mảng 4 phần tử theo thứ tự [tl, tr, br, bl] và tự thu nhỏ radius quá lớn như CSS (https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D/roundRect). Nên chỉ cần truyền nguyên giá trị.

**Prototype trên Chromium 156.0.8078.4**

Script `tab.mjs` trong `/private/tmp/claude-501/-Volumes-MySpace-Workspace-OpenCanvas/20b07377-6a8c-4f52-bd5d-b7f3bcd5074a/scratchpad/p04/` cho ra đúng các pixel ở AC4–AC6:

- rect `[8,8,0,0]`;
- rect `[8,8,0,0]` có stroke inside 2;
- clip `[12,0,0,0]`.

**Tham khảo**

- CSS `border-radius: tl tr br bl`.
- Figma REST `rectangleCornerRadii`: "Array of length 4 … starting in the top left and proceeding clockwise" (https://developers.figma.com/docs/rest-api/file-node-types/).
- Penpot `r1..r4`.

## Giải pháp

**Prop authored, mở rộng không phá vỡ**

```json
{"radius": 8}
{"radius": [8, 8, 0, 0]}
{"radius": [{"tokenId": "radius-md"}, {"tokenId": "radius-md"}, 0, 0]}
```

- Dạng đầu là dạng cũ, giữ nguyên.
- Dạng thứ hai là tab. Dạng thứ ba lấy radius từ token.
- Mỗi phần tử là số 0..100000 hoặc `{tokenId}` loại dimension.
- Mảng phải **đúng 4**, thứ tự **[topLeft, topRight, bottomRight, bottomLeft]**: CSS, Figma và canvas cùng thứ tự.
- Không nhận 2–3 giá trị, không nhận object `{topLeft,…}` (tốn token, thêm một dạng).
- Authored giữ nguyên. `resolvedProps.radius` cùng shape, chỉ thay ref.
- Kind giữ như hiện tại (`Any`, theo B02). Siết kind cho radius là việc khác.

**Lỗi**

- `field:"radius"`, message: `Radius must be a number 0..100000, a dimension token, or exactly 4 of them [topLeft, topRight, bottomRight, bottomLeft]`.
- Catalog thêm alias để B02 gợi ý `radius`: `cornerRadii`, `rectangleCornerRadii`, `topLeftRadius`, `topRightRadius`, `bottomLeftRadius`, `bottomRightRadius`, `borderTopLeftRadius`.

**Scene**

- `SceneNode.radius` thành `Sides<f32>` (untagged `All(f32)` | `Each([f32;4])`), serialize thành **số khi authored là số**. Nhờ vậy scene cũ byte-identical.
- TS: `radius: number | [number, number, number, number]`.
- Bump version lên số kế tiếp sau P04a (ví dụ `scene-3`/`web-3`), vì shape scene đổi và renderer cũ vẽ sai viền với mảng.

**Renderer**

- Ba chỗ `roundRect` truyền `node.radius` nguyên văn; bỏ `?? 0` vì Rust luôn chiếu giá trị.
- `strokeEdges` của P04a chuẩn hoá `number | Quad` về 4 góc. Mỗi góc tự áp luật "r = 0 thì giữ vuông" và luật radius trong `max(0, r − w)`, nên viền theo đúng hình.

**Layout:** không đổi. Radius không ảnh hưởng layout, như CSS.

## Phạm vi file

| File | Việc | Dòng (sau P04a → ước) |
| --- | --- | --- |
| `crates/storage/src/validate/corners.rs` (mới) | `validate_radius(v)` (số hoặc đúng 4), `resolve_corners` cho phần tử mảng | ~50 |
| `crates/storage/src/validate/properties.rs:25`, `refs.rs:95-118`, `validate.rs` | gọi 2 hàm trên; `mod corners` | +4 |
| `crates/storage/src/props_catalog.rs:92` | mô tả và alias `radius` | ±1 |
| `crates/layout/src/model.rs`, `model/paint.rs`, `engine.rs:150`, `validation.rs:45`, `lib.rs` | `radius: Option<Sides<f32>>`, scene `Sides<f32>`, kiểm từng góc hữu hạn ≥ 0, bump version | +8 |
| `renderer/types.ts`, `renderer/drawing.ts` (hoặc `paint.ts`), `renderer/stroke.ts` | type, 3 chỗ `roundRect`, chuẩn hoá trong `strokeEdges` | +6 |
| `web/tests/state.test.ts`, `web/tests/fixture-data.mjs` | chuỗi version | 2 dòng mỗi file |
| `crates/storage/tests/stroke_props.rs` → thêm ca, hoặc `corner_radii.rs` mới | AC1, AC2 | +60 |
| `spikes/pipeline/stroke.test.mjs` (của P04a) | AC4–AC7, AC9 | +60, giữ ≤ 200; vượt thì tách `corners.test.mjs` |
| `crates/server/tests/stroke_context.rs` (của P04a) | AC3 | +25 |
| `crates/server/skills/author-frame.md`, `implement-frame.md`, `docs/CONTRACT.md` | 1–2 dòng mỗi file (map `border-radius: a b c d`) | nhỏ |

## Tương thích MCP

- `tools/list` không đổi.
- Dữ liệu `radius` số cũ validate và render như trước; scene giữ số.
- props.md tự cập nhật mô tả `radius`.
- Thay đổi chỉ là nới rộng: trước đây mảng bị từ chối.

## AC

Toạ độ frame-local, scale 1; root frame không fill. "xanh" = `[37,99,235,255]`, "đỏ" = `[255,0,0,255]`, "lục" = `[22,163,74,255]`, "trong" = `[0,0,0,0]`.

**AC1 — Validator nhận** (`apply_batch` commit):
- `radius:[8,8,0,0]`;
- `radius:[{tokenId:"radius-md"},{tokenId:"radius-md"},0,0]`;
- `radius:8` cũ; scene của node này có `"radius":8`, là số chứ không phải mảng.

**AC2 — Validator từ chối, `field:"radius"`:**
- `[8,8]`, message chứa `[topLeft, topRight, bottomRight, bottomLeft]`;
- `[8,8,-1,0]`;
- `[8,"8",0,0]`;
- `[8,8,0,0,0]`.

Gợi ý alias:
- `topLeftRadius:8` → `field:"props.topLeftRadius"`, `suggestion:"radius"`;
- `cornerRadii` → `suggestion:"radius"`.

**AC3 — `get_design_context`:**
- `props.radius` giữ nguyên mảng có `{tokenId}`.
- `resolvedProps.radius` = `[8,8,0,0]` khi `radius-md=8`; theme có nhánh khác thì ra giá trị khác.
- `compact:true` liệt kê `radius-md` trong `tokens`.

**AC4 — Pixel tab.** Rect `x10 y10 100×40`, `radius:[8,8,0,0]`, fill `#2563eb`:
- trong: (10,10), (11,11), (109,10);
- xanh: (10,49), (109,49), (60,30).

**AC5 — Tab có viền P04a.** Thêm `stroke:{color:"#ff0000",width:2}`:
- đỏ: (10,49), (109,49), (60,48), (60,10), (10,30); góc dưới vuông tô đủ màu viền;
- xanh: (60,47), (14,14), (60,12), (12,30);
- trong: (10,10), (11,11), (109,10).

**AC6 — Clip theo radius từng góc.** Frame `x10 y10 100×60`, `radius:[12,0,0,0]`, `clip:true`, con free rect lục `0,0,100,60`:
- trong: (10,10), (12,12);
- lục: (109,10), (10,69), (109,69).

**AC7 — Ảnh.** Image fixture (SVG nửa trái `#2563eb`, nửa phải `#f59e0b`) `x10 y10 80×40`, `fit:"fill"`, `radius:[0,0,16,16]`:
- (10,10) xanh; (89,10) = `[245,158,11,255]`;
- (89,49) và (10,49) trong.

**AC8 — Tương thích:**
- Toàn bộ test cũ pass sau khi chỉ đổi chuỗi version.
- Scene fixture `spikes/pipeline/fixtures.mjs` không có mảng radius nào và bằng bản trước, trừ `provenance`/`sourceSceneKey`.
- `tools/list` byte-identical.

**AC9 — Viewer và PNG:** fixture có node radius mảng; canvas viewer và PNG worker byte-identical (zero tolerance như `pipeline.test.mjs:80-128`).

**AC10 — Modular:**
- File mới ≤ 200 dòng; không file mã nào vượt 250.
- fmt, clippy (lệnh ACCEPTANCE) và `npm run check:renderer` pass.

## Cách kiểm

```bash
cd <worktree> && export CARGO_BUILD_JOBS=4
cargo test --offline -p opencanvas-layout -p opencanvas-storage -p opencanvas-server
cargo clippy --offline -p opencanvas-layout -p opencanvas-storage -p opencanvas-server --lib --bin opencanvas-server -- -D warnings
cargo fmt --all --check && npm run check:renderer
PLAYWRIGHT_BROWSERS_PATH=<worktree>/.runtime/browsers npm run test:pipeline
npm --prefix web test && npm --prefix web run build
```

**Live:**
- Server tạm `--port 0 --data-root $(mktemp -d)`.
- Tạo tab `radius:[8,8,0,0]` + stroke, `render_frame` → `ready`, `diagnostics:[]`.
- PNG có pixel (10,10) trong suốt và (10,49) màu viền.
- Dọn process và data.

## Rủi ro

- **P03b (SVG export)** hiện map `radius` thành `rx` đều. Gặp mảng thì phải xuất `<path>`, hoặc phát diagnostic. Ghi cho P03b.
- **Bump version lần hai trong đêm:** `sceneKey` đổi thêm một lần. Nếu P04a và P04b deploy cùng lúc thì gộp thành một bump.
- **Radius quá lớn:** canvas tự thu nhỏ đồng loạt. CSS cũng thu nhỏ theo cùng hệ số nhỏ nhất, nên khớp; không validate tổng radius theo kích thước.

## Ngoài phạm vi / phase sau

- Radius elip authored (`{x,y}` mỗi góc, CSS `border-radius: a / b`).
- `cornerSmoothing` (squircle, Figma).
- Siết kind cho `radius`: hiện nhận cả trên text/page nhưng bị bỏ qua.
- Radius theo phần trăm.
