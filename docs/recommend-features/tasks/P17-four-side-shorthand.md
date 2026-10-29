# P17 — Một quy ước cho mảng 4 cạnh/4 góc: shorthand CSS 1–4 cho `stroke.width`, `stroke.color`, `radius` (như `layout.padding`)
Ưu tiên: P2 · Kích cỡ: ~50 phút dev (≈35′ nếu P04b chưa làm và tự dùng helper này) · Phụ thuộc: **P04a** (bắt buộc), P08a (đã merge `ec1fc01`); P04b/P14c: AC điều kiện · Nguồn: hàng đợi nghiên cứu (P17), đối chiếu P04a/P04b/P08a/P14c; note #74 (padding)

## Vấn đề

Hiện có hai quy ước cho cùng một ý "4 cạnh / 4 góc":

| Prop | Nhận | Nguồn |
| --- | --- | --- |
| `layout.padding` | số, hoặc **1–4** phần tử theo shorthand CSS | P08a, đã merge |
| `stroke.width`, `stroke.color` | số/màu, hoặc **đúng 4** `[t,r,b,l]` | P04a, chờ merge |
| `radius` | số, hoặc **đúng 4** `[tl,tr,br,bl]`; "không nhận 2–3 giá trị" | P04b, chưa làm |

Agent viết theo thói quen CSS:

- `padding:[12,16]` qua.
- `stroke:{width:[1,0]}` (viền trên/dưới, `border-width:1px 0`) bị `VALIDATION_FAILED`.
- `radius:[8,0]` (`border-radius:8px 0`) cũng bị từ chối.

Cùng một cách viết mà lỗi khác nhau tuỳ prop, nên agent phải nhớ ngoại lệ hoặc tốn một lượt gửi lại cả batch. AI coder đọc `get_design_context` thì cũng phải biết mảng nào là shorthand, mảng nào luôn đủ 4.

## Bằng chứng

**Padding nhận 1–4, có hàm mở rộng riêng**

- `crates/layout/src/model/layout.rs:10-38` (integration `0847b4b`): `Padding::{All(f32), Sides(Vec<f32>)}`. `edges()` mở rộng `[a]`, `[v,h]`, `[t,h,b]`, `[t,r,b,l]`.
- `crates/storage/src/validate/layout_rule.rs:48-58`: `padding()` nhận `1..=4`.
- `layout_rule.rs:8`: `PADDING_HELP` liệt kê 4 dạng CSS.
- `layout_rule.rs:120-139`: `resolve_layout_refs` resolve token **từng phần tử** padding. Một object có key cạnh như `{top:8}` cũng bị coi là ref token.
- Nơi gọi: `crates/layout/src/stack.rs:28` và `crates/layout/src/validation.rs:83`.

**Stroke (nhánh `night/P04a` @ `6463a42`, đọc bằng `git show`) bắt đúng 4**

- `crates/layout/src/model/stroke.rs:5-20`: `Sides<T>::{All(T), Each([T; 4])}`, mảng cố định 4.
- `crates/storage/src/validate/stroke.rs:7`: `SIDES = "exactly 4 of them [top, right, bottom, left]"`.
- `stroke.rs:74-80`: hàm `sides()` riêng, `each.len() == 4`.
- `stroke.rs:12-30`: `resolve_stroke` tự lặp phần tử mảng, trùng ý với `resolve_layout_refs`.
- `crates/storage/tests/stroke_props.rs:133-157` **khẳng định** hai trường hợp bị từ chối:
  - `{"width":[1,2]}`;
  - `{"color":["#fff","#fff","#fff"]}`.
- `crates/storage/src/props_catalog.rs:103` mô tả: "or exactly 4 of them".
- `crates/server/skills/implement-frame.md:41`: "width/color arrays are [top, right, bottom, left]".

**Radius (P04b, chưa có nhánh)**

- `.night/proposals/P04b-corner-radii.md` mục Giải pháp: "Mảng phải **đúng 4** … Không nhận 2–3 giá trị".
- Hiện tại `crates/storage/src/validate/properties.rs:25` chỉ nhận số, còn `crates/storage/src/validate/refs.rs:99` resolve token ở cấp đầu.

**Vector stroke (P14c)**

- `.night/proposals/P14c-vector-stroke.md:64-65, 91-92`: vector chỉ nhận **một** width/màu. Mảng bị lỗi riêng, không đổi bởi P17.

**Chuẩn tham chiếu: một luật mở rộng chung cho 4 thuộc tính**

Cả `border-width`, `border-color`, `padding` (thứ tự t,r,b,l) và `border-radius` (thứ tự tl,tr,br,bl) dùng cùng luật theo chỉ số:

| Viết | Mở rộng |
| --- | --- |
| `[a]` | `[a,a,a,a]` |
| `[a,b]` | `[a,b,a,b]` |
| `[a,b,c]` | `[a,b,c,b]` |
| `[a,b,c,d]` | `[a,b,c,d]` |

- https://developer.mozilla.org/en-US/docs/Web/CSS/border-width
- https://developer.mozilla.org/en-US/docs/Web/CSS/border-radius (2 giá trị = tl+br, tr+bl)
- `CanvasRenderingContext2D.roundRect` nhận danh sách 1–4 radius đúng luật này: https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D/roundRect
- Figma REST dùng object `individualStrokeWeights {top,right,bottom,left}` và `rectangleCornerRadii` (4 phần tử): https://developers.figma.com/docs/rest-api/file-node-types/. Agent đến từ Figma sẽ thử object, nên lỗi cần gợi ý dạng mảng.

**`tools/list` không chứa catalog props**

- `crates/server/src/schemas.rs` không nhắc `stroke`/`radius`/`padding` (`rg -n "stroke|radius" crates/server/src/schemas.rs crates/server/src/schemas/ crates/server/src/instructions.rs` → 0 dòng).
- Mô tả prop chỉ nằm trong resource `skill://opencanvas/props.md`.

## Giải pháp

**Một luật: "quad"**

Mọi prop 4 cạnh/4 góc nhận `v` **hoặc danh sách 1–4 `v`**, mở rộng theo luật CSS ở trên. Mỗi `v` có thể là ref.

| Prop | Thứ tự | Phần tử | CSS tương ứng |
| --- | --- | --- | --- |
| `layout.padding` | t,r,b,l | số / `{tokenId}` dimension | `padding` (giữ nguyên) |
| `stroke.width` | t,r,b,l | số / `{tokenId}` dimension | `border-width` |
| `stroke.color` | t,r,b,l | hex / `{tokenId}` color / `{styleId}` fill | `border-color` |
| `radius` (P04b) | tl,tr,br,bl | số / `{tokenId}` dimension | `border-radius` |

```json
{"stroke": {"color": "#D0D5DD", "width": [1, 0]}}
{"stroke": {"color": [{"tokenId": "border"}, "#0000"], "width": 1}}
{"radius": [8, 8, 0, 0]}
{"radius": [12, 0]}
```

- Dòng 1: viền trên/dưới.
- Dòng 2: viền trên/dưới có màu từ token, viền trái/phải trong suốt.
- Dòng 3: tab, bo hai góc trên.
- Dòng 4: bo chéo, `border-radius:12px 0`.

**Ở đâu chuẩn hoá — chọn "authored giữ nguyên, scene đủ 4"**

- `props` và `resolvedProps`:
  - giữ **đúng shape authored**, chỉ thay ref (`[1,0]` vẫn là `[1,0]`), giống `resolvedProps.layout.padding` của P08a;
  - AI coder chép thẳng sang CSS shorthand: `width:[1,0]` → `border-width:1px 0`;
  - compact `resolvedPropsMode:"differing"` không phình vì khác shape.
- Scene:
  - `stroke.width`/`color` luôn `[4]`, như P04a đã làm;
  - `radius` là số khi authored là số, còn danh sách thì thành `[4]`, đúng shape P04b đã định.
  - Renderer **không đổi**, không bump `scene-*`/`web-*`.

**Phương án đã loại**

- (B) Mọi prop đúng 4, kể cả padding: phá API P08a đã merge và ngược thói quen CSS.
- (C) Storage chuẩn hoá thành 4 lúc ghi: đọc lại khác cái vừa ghi (agent tưởng update hỏng), tốn byte, lệch P08a.
- (D) Chuẩn hoá trong `resolvedProps`: compact đánh dấu mọi danh sách là "differing" và mất mapping shorthand cho coder.
- (E) Thêm dạng object `{top,…}`: thêm một shape và tốn token; chỉ dùng làm **gợi ý trong lỗi**.

**Một hàm dùng chung mỗi tầng**

Layout: `crates/layout/src/model/sides.rs` (mới)

- Chuyển `Sides<T>` từ `model/stroke.rs` sang đây.
- Đổi thành `#[serde(untagged)] enum Sides<T> { All(T), List(Vec<T>) }` và `fn quad(&self) -> Option<[T; 4]>` (luật mở rộng, `None` khi rỗng hoặc > 4).
- `pub type Padding = Sides<f32>`, `impl<T: Default> Default`.
  - JSON vẫn là số hoặc mảng như cũ, nên hash input layout cũ không đổi.
  - `edges()` → `quad()` ở 2 chỗ gọi.
- `StrokeInput::widths()`/`scene()` dùng `quad()`, còn `check()` báo lỗi khi `None`.
- P04b dùng `Sides<f32>` cho `Props.radius`. Engine chiếu `All(r)` → số và `List` → 4 phần tử.

Storage: `crates/storage/src/validate/quad.rs` (mới)

- `enum Quad { Sides, Corners }` với `help()`:
  - Sides: `"or a list of 1–4 like CSS: [all] | [vertical, horizontal] | [top, horizontal, bottom] | [top, right, bottom, left]"`;
  - Corners: `"… [all] | [topLeft+bottomRight, topRight+bottomLeft] | [topLeft, topRight+bottomLeft, bottomRight] | [topLeft, topRight, bottomRight, bottomLeft]"`.
- `fn quad(v, valid) -> bool`: một giá trị hoặc 1..=4 phần tử hợp lệ. Thay `sides()` của stroke và `padding()` của layout_rule, P04b dùng luôn.
- `Candidate::resolve_quad(&self, v, expected)` resolve ref ở scalar hoặc từng phần tử. Chỉ coi object có `tokenId`/`styleId` là ref, còn object khác để validator báo lỗi. Thay hai vòng lặp trong `resolve_stroke` và `resolve_layout_refs`, và dùng cho `radius` ở `refs.rs`.
- `fn quad_error(field, lead, quad, v) -> StorageError`: thông điệp + `suggestion`.
  - Object có key cạnh hoặc góc (`top/right/bottom/left`, `topLeft/topRight/bottomRight/bottomLeft`):
    - với số, cạnh thiếu = 0;
    - với màu, phải đủ 4 cạnh;
    - gợi ý là **mảng shorthand ngắn nhất** dạng JSON, ví dụ `{top:1,bottom:1}` → `"[1,0]"`.
  - Danh sách 0 hoặc > 4 phần tử: không có `suggestion`, thông điệp có `help()`.

**Lỗi ví dụ**

```json
{"code":"VALIDATION_FAILED","field":"stroke.width","message":"Stroke width must be a number 0..100000 or a dimension token, or a list of 1–4 like CSS: [all] | [vertical, horizontal] | [top, horizontal, bottom] | [top, right, bottom, left]; write sides as a list: [1,0]","node":{"nodeId":"card","suggestion":"[1,0]"}}
```

- Key lạ trong `stroke` có thêm `suggestion`, cạnh thông điệp P04a hiện có:
  - `weight`, `strokeWeight`, `individualStrokeWeights` → `width`;
  - `strokeAlign`, `position` → `align`;
  - `paint`, `strokes` → `color`.
- Mỗi gợi ý là một dòng trong `match`.

## Phạm vi file

Dòng ước lượng tính sau khi P04a merge. Không file nào vượt 200 dòng.

| File | Thay đổi |
| --- | --- |
| `crates/layout/src/model/sides.rs` (mới) | ~55 dòng: `Sides<T>`, `quad()`, `Default`, unit test luật mở rộng (`#[cfg(test)]` ~20) |
| `crates/layout/src/model/stroke.rs` | 74 → ~55: bỏ `Sides`, dùng `quad()` |
| `crates/layout/src/model/layout.rs` | −28: `Padding` = alias |
| `crates/layout/src/model.rs` | +1 `mod sides`, re-export |
| `crates/layout/src/stack.rs:28`, `validation.rs:83` | `edges()` → `quad()` |
| `crates/storage/src/validate/quad.rs` (mới) | ~110 dòng |
| `crates/storage/src/validate/stroke.rs` | 80 → ~70: bỏ `sides()`, `SIDES`; dùng `quad`/`quad_error`; +suggestion key lạ |
| `crates/storage/src/validate/layout_rule.rs` | 140 → ~125: `padding()` và resolve dùng `quad`; giữ `PADDING_HELP` = lead + `Quad::Sides.help()` để test P08a vẫn khớp `[top, right, bottom, left]` |
| `crates/storage/src/validate.rs` | +1 `mod quad` |
| `crates/storage/src/props_catalog.rs` | mô tả stroke (và radius nếu P04b đã có): "exactly 4 of them" → "or 1–4 like CSS border-width/border-color"; không tăng dòng |
| `crates/storage/tests/stroke_props.rs` | 2 ca `[1,2]` và 3 màu đổi thành ca hợp lệ; thêm ca `[]` và 5 phần tử |
| `crates/storage/tests/quad_shorthand.rs` (mới) | ~140 dòng, AC1–AC2, AC5–AC7, AC11 |
| `crates/server/tests/stroke_context.rs` | +~25: AC4 |
| `crates/server/skills/author-frame.md` | +2 dòng: "padding, stroke.width, stroke.color, radius đều nhận 1–4 như CSS" |
| `crates/server/skills/implement-frame.md` | sửa dòng 41: `stroke.width:[1,0]` → `border-width:1px 0`; thêm radius `[12,0]` → `border-radius:12px 0` |
| `docs/CONTRACT.md` hoặc `docs/SERVER.md` | 1 câu: authored/resolved giữ shorthand, scene đủ 4 |

**Nếu P04b chưa triển khai khi P17 vào**

- P04b dùng `Sides<f32>` + `quad`/`resolve_quad`/`Quad::Corners` ngay từ đầu.
- Bỏ câu "Không nhận 2–3 giá trị" trong P04b; dev P04b ghi lệch này ở `.night/dev/P04b.md`.
- P17 khi đó không đụng radius.

**Nếu P04b đã merge với "đúng 4"**

- P17 sửa thêm `properties.rs`/`refs.rs` (radius gọi `quad`/`resolve_quad`) và kiểu scene trong `engine.rs` (~+15 dòng).

## Tương thích MCP

- **`tools/list`: Δ 0 B** (ngân sách ≤ +100 B): schema tool không chứa catalog props. Không tool mới, không đổi `instructions`.
- `skill://opencanvas/props.md`: dòng `stroke` ngắn đi hoặc bằng ("exactly 4 of them [top, right, bottom, left]" → "or 1–4 like CSS border-width").
- Field cũ giữ nghĩa:
  - mọi input hợp lệ trước đây (số, mảng 4, padding 1–4) cho **cùng** authored, resolvedProps, scene và hash layout input;
  - chỉ thêm dạng hợp lệ (danh sách 1–3 cho stroke/radius).
- Lỗi:
  - `field` giữ nguyên (`stroke.width`, `stroke.color`, `layout.padding`, `radius`), thông điệp vẫn chứa `[top, right, bottom, left]` (test P04a/P08a bám chuỗi này);
  - thêm `suggestion` (theo format B02).
- Không bump `scene-*`/`web-*`: shape scene không đổi.
- Tài liệu: 2 skill, props catalog, CONTRACT/SERVER (1 câu).

## AC

1. `stroke.width` nhận `2`, `[2]`, `[2,0]`, `[2,0,1]`, `[2,0,1,3]`. Scene `stroke.width` lần lượt là `[2,2,2,2]`, `[2,2,2,2]`, `[2,0,2,0]`, `[2,0,1,0]`, `[2,0,1,3]`. Kiểm bằng `cargo test -p opencanvas-storage --test quad_shorthand` (commit rồi đọc scene của frame).
2. `stroke.color` nhận 1–4 phần tử, trộn hex, `{tokenId}` color và `{styleId}` fill. `["#000", {"tokenId":"line"}]` cho scene `["#000",<line>,"#000",<line>]`; đổi theme thì phần tử token đổi theo.
3. Padding không đổi hành vi:
   - test P08a (`crates/storage/tests/stack_layout.rs`, `crates/layout/src/stack_tests.rs`, `stretch_tests.rs`, `crates/server/tests/stack_context.rs`) pass **không sửa**;
   - `serde_json::to_vec(&LayoutRule)` cho padding `12`, `[12,16]`, `[1,2,3]` giống byte trước/sau (unit test trong `sides.rs`).
4. `get_design_context` một rect có `stroke:{color:"#000",width:[{"tokenId":"hair"},0]}` trả:
   - `props.stroke.width == [{"tokenId":"hair"},0]`;
   - `resolvedProps.stroke.width == [1,0]` (độ dài giữ 2);
   - với `compact:true` và stroke không ref, `stroke` không xuất hiện trong `resolvedProps`.
5. Danh sách sai kích thước bị từ chối, không tạo revision:
   - `[]` và `[1,2,3,4,5]` ở `stroke.width`, `stroke.color`, `layout.padding` → `VALIDATION_FAILED`;
   - `field` đúng tên;
   - message chứa `1–4` và `[top, right, bottom, left]`;
   - không có `suggestion`.
6. Gợi ý từ object cạnh:
   - `stroke.width:{"top":1,"bottom":1}` → `suggestion == "[1,0]"`;
   - `layout.padding:{"top":8,"left":16}` → `"[8,0,0,16]"`;
   - `stroke.color:{"top":"#000"}` → không `suggestion`, message nói dùng danh sách;
   - `layout.padding:{"tokenId":"space-4"}` vẫn resolve như token (không coi là object cạnh).
7. Key lạ trong stroke có `suggestion`: `weight` và `individualStrokeWeights` → `"width"`, `strokeAlign` → `"align"`; field vẫn `stroke.<key>`.
8. Một hàm mỗi tầng:
   - `rg -n "fn quad\b" crates/layout/src crates/storage/src` → đúng 1 hàm ở `model/sides.rs` và 1 ở `validate/quad.rs`;
   - `rg -n "exactly 4" crates/storage/src crates/layout/src crates/server/skills` → 0 dòng;
   - `rg -n "fn sides\(|fn padding\(" crates/storage/src/validate` → 0 dòng.
9. Scene không đổi:
   - `npm run test:pipeline` pass mà không đổi chuỗi `scene-*`/`web-*`;
   - test Rust: scene JSON của rect `stroke.width:[1,0]` giống byte rect `[1,0,1,0]`.
10. `tools/list`:
    - `cargo test -p opencanvas-server --test tools_budget` pass;
    - `serde_json::to_vec(tools/list result).len()` trước/sau chênh 0 B (≤ +100 B);
    - dòng `stroke` trong props.md không dài hơn trước.
11. *(điều kiện P04b)* `radius`:
    - `[8]`, `[8,0]`, `[8,0,4]`, `[8,8,0,0]` → scene `[8,8,8,8]`, `[8,0,8,0]`, `[8,0,4,0]`, `[8,8,0,0]`;
    - `radius:8` → scene số `8` (byte cũ);
    - `{"topLeft":8,"topRight":8}` → `suggestion "[8,8,0,0]"`;
    - message chứa `[topLeft, topRight, bottomRight, bottomLeft]`;
    - pixel PNG của `[8,0]` ≡ `[8,0,8,0]` (pipeline test so byte).
12. *(điều kiện P14c)* stroke trên `vector` vẫn từ chối mọi danh sách, kể cả `[2]`, với thông điệp P14c không đổi.
13. Kiểm chung:
    - `cargo fmt --all --check` pass;
    - `cargo clippy -p opencanvas-layout -p opencanvas-storage --all-targets -D warnings` pass;
    - `cargo test -p opencanvas-layout -p opencanvas-storage -p opencanvas-server` pass;
    - file mới ≤ 200 dòng; `wc -l` file sửa không tăng quá 250.

## Cách kiểm

```bash
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-layout sides
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-storage --test quad_shorthand --test stroke_props --test stack_layout
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server --test stroke_context --test stack_context --test tools_budget
npm run test:pipeline
```

- Test mới:
  - `quad_shorthand.rs`: AC1, AC2, AC5, AC6, AC7, AC11 trên store tạm;
  - unit test `sides.rs`: luật mở rộng + byte JSON (AC3, AC9);
  - `stroke_context.rs`: AC4.
- Đo AC10: chạy `tools_budget` có in `len()` trước/sau, hoặc gọi `tools/list` trên server tạm (`--port 0 --data-root $(mktemp -d)`) ở HEAD trước/sau và so `wc -c`.
- Live (tuỳ chọn) trên server tạm:
  - `apply_batch` rect `stroke:{color:"#000",width:[1,0]}`;
  - rồi `render_frame` → `diagnostics: []`;
  - PNG có viền trên/dưới 1 px, không có trái/phải.
  - Dọn process và thư mục tạm sau.

## Rủi ro

- **Hiểu nhầm `radius:[8,0]`.** CSS nghĩa là chéo (tl+br, tr+bl), không phải "trên/dưới". Agent có thể muốn tab mà nhận góc chéo. Giảm rủi ro:
  - help/catalog ghi rõ mapping từng dạng;
  - skill author-frame dùng ví dụ tab `[8,8,0,0]`.
  - Không chặn được bằng validate vì là input hợp lệ.
- **Cache.** `[1,0]` và `[1,0,1,0]` cho scene giống nhau nhưng hash layout input khác, nên tốn hai mục cache render. Vô hại; không chuẩn hoá hash để giữ byte input cũ.
- **Rollback.** Binary chỉ có P04a ("đúng 4") không deserialize được stroke `[1,0]` đã lưu, nên render frame đó lỗi. Nên deploy P17 cùng hoặc ngay sau P04a, trước khi agent ghi dữ liệu thật.
- **Xung đột file.** Xung đột với P04b nếu chạy song song (`properties.rs`, `refs.rs`, `engine.rs`). Dev orchestrator xếp tuần tự: P04a → P17 → P04b (P04b dùng helper), hoặc P04a → P04b → P17.
- **Test bám chuỗi.** Test P04a/P08a bám chuỗi `[top, right, bottom, left]`; `help()` phải giữ đúng chuỗi đó.

## Ngoài phạm vi / phase sau

- Chuỗi shorthand CSS (`"1px solid #ddd"`, `"8px 16px"`): vẫn lỗi `stroke` "object" như P04a.
- Dạng object `{top,…}` làm input hợp lệ: chỉ gợi ý.
- `field` theo chỉ số phần tử (`stroke.width[2]`).
- Style từng cạnh (dashed) và `null` để xoá stroke (follow-up P04a).
- Shorthand cho vector stroke (P14c).
- `inset`/`margin` (chưa có prop).
