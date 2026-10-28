# P10b — Text: `letterSpacing`, `textAlign`, `textDecoration`, `textTransform` (khớp pixel Chrome)
Ưu tiên: P1 (yêu cầu trong note "Thiếu tính năng → Text") · Kích cỡ: ~90 phút dev · Phụ thuộc: **P10a** (bắt buộc: `measurement.ts` mới, baseline, `textWrap`, version). Xếp tuần tự với P01/P04a/P05 vì chạm `renderer/drawing.ts` (chỉ nhánh text, ~10 dòng) · Nguồn: note "Thiếu tính năng → Text" (letterSpacing "phải tách từng ký tự thành node!", textAlign, textDecoration "strikethrough của row bị xoá phải vẽ bằng rect", textTransform)

Phase b của P10. Không gồm italic/weight theo family (P10c).

## Vấn đề

- Text chỉ có `text`, `fontId`, `fontSize`, `lineHeight`, `fill`.
- Agent port UI thật phải giả lập:
  - **letterSpacing:** tách mỗi ký tự thành một text node. Một nhãn 10 ký tự thành 10 node, mất khả năng sửa chữ, còn AI coder thấy 10 `<span>`.
  - **textAlign:** không căn được dòng trong box. Text nhiều dòng căn giữa (empty state, modal) chỉ làm được bằng mỗi dòng một node.
  - **textDecoration:** gạch ngang dòng bị xoá trong data grid phải vẽ bằng rect; gạch chân link cũng vậy.
  - **textTransform:** nhãn UPPERCASE phải gõ sẵn chữ hoa. Coder không biết nguồn là chữ thường + `text-transform`.
- Mỗi cách né tốn node, token và làm sai cấu trúc cho coder.

## Bằng chứng

Code ở `night/integration@52a8715`, sau P10a.

| Vị trí | Hiện trạng |
| --- | --- |
| `crates/storage/src/props_catalog.rs:85-89` | text props: `text`, `fontId`, `fontSize`, `lineHeight`, `fill`. |
| `crates/server/src/props_doc.rs:55-56` | "Not supported: … letter spacing, text alignment…". |
| `crates/storage/src/validate/node.rs:20-53` | text style chỉ nhận `fontId`, `fill`, `fontSize`, `lineHeight`; key khác → `Unsupported text style property`. |
| `crates/storage/src/validate/refs.rs:95-110` | `resolve_props` resolve token theo bảng kiểu (`x`/`y`/`opacity` → `number`). |
| `crates/layout/src/model.rs:108-121` | `MeasureRequest` không có field kiểu chữ. |
| `crates/layout/src/model.rs:123-129` | `TextLine.x` có sẵn nhưng luôn 0 (`renderer/measurement.ts:48`). |
| `crates/layout/src/engine.rs:104-111` | lấy `lines` từ cache, không chỉnh. |
| `crates/layout/src/engine.rs:115-154` | `SceneNode` được chiếu tại đây. |
| `renderer/drawing.ts:35-41` | vẽ text: `font`, `fontKerning`, `textBaseline`, `fillStyle`, `fillText` từng dòng. |
| `renderer/measurement.ts:23-99` | đo theo font/size, không có letterSpacing/transform. |

**Prototype trong Chromium pinned 156.0.8078.4 (mac).**
Script ở `/private/tmp/claude-501/-Volumes-MySpace-Workspace-OpenCanvas/c338aab6-301e-4c1f-a358-35b465f72775/scratchpad/p10/` (`props.mjs`, `deco.mjs`, `deco2.mjs`, kết quả `props.out`, `deco.out`). Font: NotoSans bundled, SF Pro (`SFNS.ttf`), Georgia. Size 12/18, 14/20, 24/32. Mọi ca so screenshot DOM với screenshot canvas cùng vị trí, baseline theo P10a.

- **letterSpacing**, `ctx.letterSpacing = "<n>px"` với n ∈ {1, −0.5, 2.4}:
  - 27/27 ca **0 pixel khác** DOM `letter-spacing:<n>px`.
  - Width DOM = `ceil64(measureText)` (có tính spacing sau ký tự cuối) ở 27/27 ca, ví dụ Noto 14 "Hello Sao chép" +1px → 111,140625.
- **textAlign** center/right trong box 300, `x = floor64((300 − w)/2)` hoặc `300 − w` với `w = ceil64(width)`: 18/18 ca **0 pixel khác** DOM `text-align`.
- **textDecoration:**
  - Công thức kiểu Blink:
    - `t = max(1, fs/10)`, độ dày `h = max(1, trunc(t))`;
    - underline top = `round(baseline + max(1, ceil(t/2)))`;
    - line-through top = `round(baseline − A/3 − t/2)`, với `A` = `fontBoundingBoxAscent`.
  - 144/144 ca (3 font × 12 size 10–40 × 2 lineHeight × 2 kiểu) **0 pixel khác** DOM.
  - Thử bảng `post`/`OS/2` của font thì lệch 1 hàng ở SF Pro: Chrome không dùng metrics gạch chân của font ở `text-decoration-thickness:auto`.
  - Ngoại lệ đã giải thích: chuỗi có nét thò xuống (Georgia "123" old-style, "p", "g") lệch 5–43 px vì Chrome mặc định `text-decoration-skip-ink:auto` (ngắt gạch quanh nét). Bỏ chữ số thì 144/144 khớp.
- **textTransform:**
  - uppercase bằng JS `toUpperCase()` khớp Chrome ở 3/3 chuỗi (tiếng Việt, ß → SS, ﬁ → FI).
  - **capitalize bằng regex JS sai** ở 2/3 chuỗi. Chrome cho `"Hello-World O'neil ﬁx ß Straße"` và `"X.Y Z_w (Ab) 3rd «Élan»"`; regex cho `"Hello-world …"` và `"X.y …"`.
  - Đọc `innerText` của một phần tử có `text-transform` trong trang worker cho đúng kết quả Chrome ở mọi ca.

**Tham khảo**
- CSS: `letter-spacing` (length, cộng sau mỗi ký tự), `text-align`, `text-decoration-line`, `text-transform` (https://developer.mozilla.org/en-US/docs/Web/CSS/text-transform), `text-decoration-skip-ink`.
- Canvas `letterSpacing` (https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D/letterSpacing; Chrome 99+; Firefox/Safari chỉ có ở bản gần đây, dev kiểm bảng tương thích MDN khi ghi docs).
- Figma TextNode: `letterSpacing {value, unit: PIXELS|PERCENT}`, `textAlignHorizontal: LEFT|CENTER|RIGHT|JUSTIFIED`, `textDecoration: NONE|UNDERLINE|STRIKETHROUGH`, `textCase: ORIGINAL|UPPER|LOWER|TITLE` (https://developers.figma.com/docs/plugins/api/TextNode/).
- Penpot text: `letterSpacing`, `textAlign`, `textDecoration`, `textTransform` (https://doc.plugins.penpot.app/interfaces/Text).

## Giải pháp

### 1. Prop authored (tên CSS camelCase, giá trị phẳng)

```json
{"kind":"text","props":{"text":"Deleted row","fontId":"font_bundled","fontSize":13,"lineHeight":20,
  "letterSpacing":0.5,"textAlign":"center","textDecoration":"line-through","textTransform":"uppercase"}}
```

| Key | Giá trị | Mặc định | Aliases (gợi ý B02) |
| --- | --- | --- | --- |
| `letterSpacing` | số px −1000..1000, hoặc `{tokenId}` loại number | 0 | `tracking`, `letter-spacing`, `charSpacing`, `characterSpacing`, `kerning` |
| `textAlign` | `"left"` \| `"center"` \| `"right"` | `"left"` | `align`, `textAlignHorizontal`, `horizontalAlign`, `text-align`, `alignText` |
| `textDecoration` | `"none"` \| `"underline"` \| `"line-through"` | `"none"` | `decoration`, `textDecorationLine`, `underline`, `strikethrough`, `strikeThrough`, `lineThrough` |
| `textTransform` | `"none"` \| `"uppercase"` \| `"lowercase"` \| `"capitalize"` | `"none"` | `textCase`, `case`, `text-transform`, `uppercase`, `capitalize` |

- Scope text.
- Text style được mang cả bốn key; quy tắc chung: "text style giữ mọi key kiểu chữ trừ `text`". Node ghi đè style như hiện nay (`refs.rs` merge style rồi authored).
- Lỗi chỉ cách sửa, ví dụ:
  - `Letter spacing is in pixels (-1000..1000); convert em by multiplying by fontSize`;
  - `Text align must be left, center or right`;
  - `Text decoration must be none, underline or line-through`.

**Vì sao chọn như vậy**

- Tên và giá trị CSS: LLM biết sẵn, coder map 1:1. Figma `textCase`/`STRIKETHROUGH` vào alias.
- `letterSpacing` chỉ nhận px, giống `fontSize`/`lineHeight` đều là px. Không nhận chuỗi `"0.02em"`: phải parse, và hai cách viết cho một giá trị.
- Không có `justify`: canvas phải phân bổ khoảng trắng theo từng dòng (`wordSpacing`), hiếm trong UI. Validator trả `Text align "justify" is not supported; use left` để agent không đoán.
- Không có `start`/`end`: chưa hỗ trợ RTL.
- Không cho `textDecoration` nhiều giá trị: CSS cho `underline line-through` nhưng UI hầu như không dùng.

### 2. Đo và dựng dòng

**MeasureRequest** (`model.rs`) thêm `letter_spacing: Option<f32>` và `text_transform: Option<String>`, cả hai `skip_serializing_if = "Option::is_none"`. Text không dùng thì khoá đo y hệt sau P10a.

**`renderer/measurement.ts`**
- Set `context.letterSpacing = \`${r.letterSpacing ?? 0}px\`` mỗi request: context dùng lại giữa các request nên phải reset.
- Transform trước khi tách từ: một `<span>` ẩn nhưng được render (`position:fixed; opacity:0; white-space:pre`, không `display:none`), set `style.textTransform`, `textContent = text`, đọc `innerText`. Chỉ một đường cho cả 3 giá trị, khớp Chrome tuyệt đối.
- Dòng trong scene chứa text **đã transform**. Authored `text` không đổi.
- `Measurement` thêm `ascent` (= `fontBoundingBoxAscent`, đã có từ P10a) để Rust tính decoration.

**Căn dòng (Rust, `engine.rs` → module mới `text_lines.rs`)**
- Sau khi lấy `lines` từ cache, với box width `W = l.size.width`:
  - `x = 0` khi left;
  - `x = floor64((W − w)/2)` khi center;
  - `x = W − w` khi right;
  - **kẹp ≥ 0**: dòng tràn thì start-aligned như CSS Text 3 §7.1.
- Tính ở Rust vì chỉ Rust biết box cuối, kể cả text `textWrap:"nowrap"` (P10a đo không width) có width cố định. `textAlign` không vào khoá đo.

**Decoration (Rust, `text_lines.rs`)**
- `SceneNode.decoration = {line, offset, thickness}` với `offset` tính từ baseline từng dòng:
  - underline: `max(1, ceil(t/2))`;
  - line-through: `−ascent/3 − t/2`;
  - `t = max(1, fs/10)`, `thickness = max(1, trunc(t))`.
- Màu = `fill` của text, như `currentColor`.

### 3. Scene và vẽ

**SceneNode** thêm `letter_spacing: Option<f32>` (bỏ khi 0) và `decoration: Option<SceneDecoration>`, đều `skip_serializing_if`. Node không dùng thì serialize y hệt.

**`renderer/drawing.ts`**, nhánh text:

```ts
context.letterSpacing = `${node.letterSpacing ?? 0}px`;
for (const line of node.lines) {
  context.fillText(line.text, b.x + line.x, b.y + line.y);
  const d = node.decoration;
  if (d) context.fillRect(b.x + line.x, Math.round(b.y + line.y + d.offset), line.width, d.thickness);
}
```

- `letterSpacing` thuộc drawing state, nên `save/restore` mỗi node đã có sẵn tự reset.
- Không đo, không dựng lại dòng ở bước vẽ: comment `drawing.ts:39` giữ nguyên.

**Phiên bản:** bump `SCENE_VERSION` (field mới) và `RENDERER_VERSION` (request/measurement mới) lên số kế tiếp sau P10a.

### 4. Đọc qua MCP

- `get_design_context` trả authored props nguyên văn, `resolvedProps.letterSpacing` là số khi authored là token. `compact` tự gom token (`compact.rs:75-97` đi qua mọi object).
- `inspect_layout` lines có `x` khác 0 khi căn, và text đã transform.
- `implement-frame.md` thêm bảng map:
  - `letterSpacing` → `letter-spacing: <n>px`;
  - `textAlign` → `text-align`;
  - `textDecoration` → `text-decoration-line`. Muốn khớp pixel tuyệt đối thì `text-decoration-skip-ink: none`, vì renderer không ngắt gạch quanh nét thò xuống;
  - `textTransform` → `text-transform`. Dùng authored `text`, không chép text đã transform trong lines.

## Phạm vi file

| File | Việc | Dòng (hiện → ước) |
| --- | --- | --- |
| `crates/storage/src/props_catalog.rs` | 4 entry (value, aliases) | ~127 → ~131 |
| `crates/storage/src/validate/text_props.rs` (mới) | `text_prop(key, v)`: enum 3 key + số letterSpacing; dùng cho node (`properties.rs`) và text style (`node.rs`) | ~70 |
| `crates/storage/src/validate/properties.rs`, `node.rs`, `refs.rs:97-100`, `validate.rs` | gọi `text_prop`; style nhận 4 key; `letterSpacing` resolve theo `number`; `mod text_props` | +4, +2, +1, +1 |
| `crates/layout/src/model.rs` | Props 4 field; MeasureRequest 2; `Measurement.ascent` (`serde(default)`); SceneNode 2 + `SceneDecoration` | 211 → ~228 (nếu > 230 thì tách `model/text.rs`) |
| `crates/layout/src/text_lines.rs` (mới) | `align_lines`, `decoration`, floor64; unit test cùng file hoặc `text_lines_tests.rs` | ~90 |
| `crates/layout/src/engine.rs`, `util.rs`, `validation.rs`, `lib.rs` | gọi `text_lines`; request thêm 2 field; kiểm enum/range cho input trực tiếp; bump version | 185→~192, 62→~66, ~136→~146 |
| `renderer/measurement.ts` | letterSpacing, transform qua `innerText`, `ascent` | ~106 → ~130 (transform tách `renderer/text-transform.ts` ~25 nếu vượt) |
| `renderer/drawing.ts` (hoặc `paint.ts` nếu P01 đã merge) | letterSpacing + rect decoration | 161 → ~167 |
| `renderer/types.ts` | field mới + version | ~115 → ~125 |
| `crates/storage/tests/text_props.rs` (mới) | AC1–AC2 | ~110 |
| `spikes/pipeline/text-props.test.mjs` (mới) | AC3–AC7, AC9 pixel so DOM (theo mẫu P10a `text-measure.test.mjs`) | ~180 |
| `crates/server/tests/text_props_context.rs` (mới) | AC8 | ~80 |
| `crates/server/src/props_doc.rs`, 2 skill, `docs/CONTRACT.md`, `docs/VIEWER.md`, `spikes/pipeline/README.md` | bỏ "letter spacing, text alignment" khỏi Not supported; bảng map; field scene | vài dòng mỗi file |

- Không đụng `mcp.rs`, `schemas.rs`, `tools/list`.
- File mã mới ≤ 180 dòng. `model.rs` là file nóng: dev tách `model/text.rs` nếu vượt 230.

## Tương thích MCP

- `tools/list` byte-identical.
- `allowedKeys` của text và props.md thêm 4 key. Text style nhận thêm 4 key.
- Dữ liệu cũ validate như trước. Scene của node không dùng key mới y hệt, trừ `provenance` version. Dòng có `x` = 0 như cũ.
- Node có `textTransform` thì `lines[].text` khác `props.text`. Agent dùng `textContains` của `find_nodes` vẫn tìm theo **authored** text (ghi trong props.md).

## AC

Pixel: Chromium pinned của worker, so screenshot DOM cùng bytes font, vị trí nguyên. "0 px khác" = mọi kênh R của mọi pixel bằng nhau. Font NotoSans bắt buộc; SF Pro/Georgia chạy nếu có trên máy, ngược lại skip có log.

**AC1 — Validator nhận** (`apply_batch` +1 revision):
- text có `letterSpacing:0.5`, `letterSpacing:-1`, `letterSpacing:{tokenId:<number token>}`, `textAlign:"center"`, `textDecoration:"line-through"`, `textTransform:"capitalize"`;
- text style chứa cả bốn key, áp lên node: `resolvedProps` có đủ, node ghi đè style.

**AC2 — Validator từ chối** (đúng `field`/message, batch nguyên tử):
- `letterSpacing:"1px"` → `field:"letterSpacing"`, message có `pixels`.
- `letterSpacing:1001` → `field:"letterSpacing"`.
- `textAlign:"justify"` → message `not supported; use left`.
- `textAlign:"middle"` → `field:"textAlign"`.
- `textDecoration:"strike"` → `field:"textDecoration"`.
- `textTransform:"upper"` → `field:"textTransform"`.
- `textAlign` trên rectangle → `field:"props.textAlign"`, `nodeKind:"rectangle"`.
- `tracking:1` → `suggestion:"letterSpacing"`.
- `textCase:"UPPER"` → `suggestion:"textTransform"`.
- Test B02 catalog pass không sửa logic; props.md dòng `text` chứa 4 key.

**AC3 — letterSpacing.**
- Noto 14/20 "Hello Sao chép", hug, `letterSpacing` ∈ {1, −0.5, 2.4}: `bounds.width` == width DOM `letter-spacing` (mac: 111,140625 / 90,140625 / 130,734375).
- PNG so DOM: 0 px khác.
- Viewer `drawScene` và PNG worker byte-identical.

**AC4 — textAlign.**
- Text width 300 Noto 14/20 "Hello Sao chép":
  - center: `lines[0].x == floor64((300 − w)/2)` (mac: w 97,140625 → x 101,421875);
  - right: `x == 300 − w` (202,859375);
  - 0 px khác DOM `text-align`.
- 3 dòng (width 120, center): mỗi dòng có `x` riêng theo công thức.
- `textWrap:"nowrap"` width 40, center, "customers": `x == 0` (tràn thì start).
- Hug 1 dòng center: `x == 0`.

**AC5 — textDecoration.**
- Noto, size {10, 12, 14, 16, 20, 24, 32, 40} × lineHeight {round(1,25·fs), round(1,5·fs)} × {underline, line-through}, chuỗi "Hello world HEAD Mixed" tại (10,10): 0 px khác DOM (32 ca; prototype 144/144 trên 3 font).
- Số tham chiếu Noto 12/18 tại y 10: underline hàng 24; line-through hàng 18; độ dày 1. Noto 24/32: underline hàng 37–38, độ dày 2.
- Decoration nằm trong `scene.nodes[i].decoration` với `offset`, `thickness` như công thức.

**AC6 — textTransform.**
- `"đường phố hà nội"` uppercase → `lines[0].text == "ĐƯỜNG PHỐ HÀ NỘI"`.
- `"hello-world o'neil ß straße"` capitalize → `"Hello-World O'neil ß Straße"` (bằng `innerText` của DOM trong cùng trình duyệt; test so với DOM, không hard-code).
- `"Sao Chép"` lowercase → `"sao chép"`.
- Width = width DOM của text đã transform.
- `get_design_context props.text` giữ nguyên văn.

**AC7 — Kết hợp + wrap.** Text width 160, center, uppercase, `letterSpacing:1`, line-through, 3 dòng: số dòng và text từng dòng bằng DOM cùng style (`width:160px`); PNG so DOM ≤ 0,5 % pixel khác. Ngưỡng này dành cho điểm ngắt dòng có spacing cuối dòng; nếu 0 thì ghi 0.

**AC8 — MCP** (`text_props_context.rs`):
- `get_design_context` với token `tracking-wide = 0.5`: `props.letterSpacing == {"tokenId":"tracking-wide"}`, `resolvedProps.letterSpacing == 0.5`.
- `compact:true, resources:"referenced"` có token đó.
- `inspect_layout` lines có `x` > 0 với node center.

**AC9 — Tương thích.**
- Test hiện có pass sau khi đổi chuỗi version.
- Scene fixture không dùng key mới, so với P10a (bỏ `provenance`): byte-identical.
- `MeasureRequest` của text không dùng key mới không có `letterSpacing`/`textTransform`.
- `tools/list` byte-identical.
- `drawScene` với scene version cũ → `PIPELINE_MISMATCH`.

**AC10 — Live** trên server tạm:
- `apply_batch` row "Deleted" (line-through, uppercase, letterSpacing 1) → `render_frame` `ready`, `diagnostics:[]`.
- PNG có hàng gạch đúng vị trí AC5 khi đặt text ở toạ độ nguyên.

**AC11 — Modular.**
- File mã mới ≤ 200; `drawing.ts`/`paint.ts` ≤ 200; `measurement.ts` ≤ 150; không file sửa nào vượt 250.
- fmt, clippy, `check:renderer` pass.

## Cách kiểm

```bash
W=<worktree>; cd $W && export CARGO_BUILD_JOBS=4 PLAYWRIGHT_BROWSERS_PATH=$W/.runtime/browsers
cargo test --offline -p opencanvas-layout -p opencanvas-storage -p opencanvas-server
cargo clippy --offline -p opencanvas-layout -p opencanvas-storage -p opencanvas-server --lib --bin opencanvas-server -- -D warnings
cargo fmt --all --check && npm run check:renderer
npm run test:pipeline            # text-props.test.mjs mới
npm --prefix web test && npm --prefix web run build
```

**Live:** `target/debug/opencanvas-server --port 0 --data-root $(mktemp -d)`; MCP qua `.night/qa/mcp-tools.mjs` (tools/list) và script node `create_project` → `apply_batch` → `render_frame` → đọc PNG. Dọn sau.

## Rủi ro

- **Skip-ink:** gạch chân của Chrome ngắt quanh g/p/y và chữ số old-style; renderer vẽ liền. Lệch vài chục pixel quanh nét thò xuống, đúng vị trí và độ dày. Ghi trong skill (coder đặt `text-decoration-skip-ink:none` khi cần khớp tuyệt đối). Làm skip-ink cần lấy outline glyph, để sau.
- **Viewer ngoài Chromium:** canvas `letterSpacing` chỉ có ở Firefox/Safari bản gần đây. Trình duyệt cũ vẽ không spacing (dòng đã dựng sẵn nên không vỡ layout). PNG worker luôn đúng.
- **Decoration ở scale ≠ 1:** làm tròn theo CSS px, nên khi zoom (P01 vẽ vector) đường có thể lệch ≤ 0,5 px CSS so với Chrome ở DPR đó. AC pixel chỉ ở scale 1.
- **`innerText` cần layout:** mỗi request có transform tốn một lần reflow, cỡ chục µs. Gộp trong `measureBatch`, không đáng kể. Request không có transform không đụng DOM.
- **File nóng:** `drawing.ts` (P01/P04a/P05), `model.rs`, `props_catalog.rs`. Dev orchestrator xếp sau P10a và rebase. Nhánh text trong `paintNode` ít đụng phần P04a/P05 sửa (vòng vẽ/opacity).
- **letterSpacing + wrap:** spacing cuối dòng có thể làm điểm ngắt khác DOM ở ca sát ngưỡng. AC7 đo; nếu lệch thì ghi follow-up.

## Ngoài phạm vi / phase sau

- `textAlign:"justify"`, `textDecoration` nhiều giá trị, kiểu gạch (`wavy`, `dotted`), màu/độ dày gạch riêng, `text-underline-offset`.
- `letterSpacing` theo % hoặc em.
- Skip-ink.
- Rich text (nhiều style trong một node).
- Căn dọc trong box cố định (`textAlignVertical`), `maxLines`/ellipsis.
- **P10c:** italic và weight theo family.
