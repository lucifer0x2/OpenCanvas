# P18b — DOM importer: mapper border → `stroke`, radius 4 góc, `box-shadow` → `shadow`, gradient → `fill`, bật theo catalog thực tế của server
Ưu tiên: P2 · Kích cỡ: ~85′ dev · Phụ thuộc: **P18a** (bắt buộc: `tools/dom-import/`, `map-style.mjs`, `stack.mjs`, `diagnostics.mjs`). Về prop đích: **không bắt buộc** — mỗi mapper chỉ bật khi server có prop tương ứng (P04a `stroke` @ `night/P04a` 6463a42 đang QA; P04b, P09a, P09b, P17 chưa làm). Text/font/alignSelf (P10b/P10c/P08b) và `textWrap` (P10a) để **P18e** (mục cuối) · Nguồn: note "Ý tưởng → Importer HTML/DOM" (`/Users/luis/Documents/opencanvas.md:82-83`), P18a mục "Ngoài phạm vi"

## Vấn đề

- P18a sinh op chỉ từ prop **đã merge** (`fill`, `opacity`, `clip`, `radius` một số, `layout`, text, image). Viền, bóng, gradient và bo từng góc thành diagnostics `BORDER_UNSUPPORTED`, `SHADOW_UNSUPPORTED`, `GRADIENT_FLATTENED`, `RADIUS_PER_CORNER` kèm `wouldUse`. Trên màn QueryNest đó là phần lớn chỗ lệch: input, card, keycap, nút gradient.
- Các prop đích đang merge **từng cái một** trong đêm (P04a đang QA, P04b/P09a/P09b xếp sau). Importer hard-code theo một trạng thái sẽ sai ở trạng thái kia:
  - sinh `stroke` khi server chưa có → cả batch `VALIDATION_FAILED`;
  - không sinh khi server đã có → mất độ sát vô cớ.
- Ba prop đích là **mở rộng giá trị của key đã có**: `radius` mảng (P04b), `fill` gradient (P09a), shorthand 1–4 (P17). Có key trong danh sách kind **không** nói được server nhận shape mới hay chưa.

## Bằng chứng

- **Catalog là nguồn duy nhất**: `crates/storage/src/props_catalog.rs:78-102`; được render nguyên văn thành `skill://opencanvas/props.md` (`crates/server/src/props_doc.rs:1-2,20-40`): mục "Keys by kind" (một dòng `` - `kind`: `k1`, … `` mỗi kind, `props_doc.rs:15-18`) và bảng "Keys" có cột Value lấy từ `PropSpec.value`.
  - Hôm nay: `radius` "one radius for all corners" (`:97`); `fill` chỉ màu/token/style (`:94`). Chưa có `stroke`, `shadow`.
- **Không có dry-run**: `rg 'dryRun|validateOnly' crates/` rỗng. Không thể hỏi server "shape này hợp lệ không" mà không ghi. Vì vậy dò bằng tài liệu catalog, rồi test live trên server tạm xác nhận lời dò là thật.
- **Font**: record có `id`, `family`, `weight`, `style` (`crates/storage/src/model.rs:82-87`). CLI đọc `list_fonts` (chỉ đọc) là đủ dựng map font của P18a, agent khỏi tự viết `fonts.json`.
- **Ảnh để so pixel**: `export_nodes` (merged `0847b4b`) trả PNG 1x của root frame qua `GET /api/v1/jobs/{job}/files/{index}` (`docs/SERVER.md:88-94`). Không base64 vào context.
- **Shape đích** (đọc từ đề xuất):

  | ĐX | Prop | Shape | Nguồn |
  | --- | --- | --- | --- |
  | P04a | `stroke` | `{color, width?=1, align?="inside"}`; `color`/`width` scalar hoặc **đúng 4** `[t,r,b,l]`; kind frame/group/rectangle/image | `P04a-stroke.md:78-105` |
  | P04a | layout | inside stroke cộng vào padding hiệu lực của row/column (như CSS border-box) | `P04a-stroke.md:173-181` |
  | P17 | shorthand | `stroke.width`/`stroke.color`/`radius` nhận 1–4 phần tử theo luật CSS | `P17-four-side-shorthand.md:76-92` |
  | P04b | `radius` | số hoặc **đúng 4** `[tl,tr,br,bl]` | `P04b-corner-radii.md:52-63` |
  | P09a | `fill` | `{type:"linear", angle?=180, stops:[[p,c]…]}` / `{type:"radial", center?, radius?, stops}`; 2..16 stop; không trên text | `P09a-gradient-fill.md:74-97` |
  | P09b | `shadow` | mảng 0..8 `{x?,y?,blur?,spread?,color,inset?}`, blur = bán kính CSS; không trên text | `P09b-box-shadow.md:70-93` |

- **Độ khớp đã đo** ở prototype (`board/research.md` P04/P09): stroke khớp mọi pixel AC; gradient ≤ 1/255; shadow ≤ 1–2/255 so với Chromium. Ngưỡng AC live bên dưới lấy từ các số này cộng biên làm tròn 8-bit.

## Giải pháp

### 1. Capability từ catalog thực tế (`caps.mjs`, mới)

`detectCaps(propsMd)` → object cờ, thuần, không I/O:

| Cờ | Bật khi (trên `props.md`) | Mapper |
| --- | --- | --- |
| `stroke` | dòng kind `frame` và `rectangle` có `` `stroke` `` | border → `stroke` |
| `strokeShort` | `stroke` và ô Value của `stroke` khớp `/1-4|1–4/` | rút gọn mảng (P17) |
| `radiusQuad` | ô Value của `radius` khớp `/topLeft|4 of them|1-4/` | radius 4 góc |
| `gradient` | ô Value của `fill` khớp `/gradient/i` | gradient → `fill` |
| `shadow` | dòng kind `frame` có `` `shadow` `` | `box-shadow` → `shadow` |

- Nguồn `props.md`, theo thứ tự:
  1. `--props <file>`;
  2. `--server <url>`: CLI gọi **chỉ đọc** `initialize` + `resources/read skill://opencanvas/props.md` (và `list_fonts {projectId}` nếu có `--project`) qua MCP HTTP, ~40 dòng trong `server-read.mjs`. Không gọi tool ghi nào.
  3. Không có cả hai: mọi cờ tắt, hành vi **byte-identical P18a**.
- `--caps stroke,-gradient` ép bật/tắt từng cờ (dùng khi marker đổi chữ). Cờ thực dùng ghi vào `summary.caps` (vài chục byte).
- **Vì sao dò bằng chữ trong Value**: là cách duy nhất không ghi dữ liệu (không có dry-run). Rủi ro chữ đổi được chặn bởi AC 9: test live dò trên server thật rồi **thử áp** shape mới vào project tạm, nên lời dò sai là test đỏ, không phải lỗi im lặng ở người dùng.
- Phương án loại:
  - dò bằng cách áp thử vào project của người dùng: ghi rác, cần quyền ghi;
  - đọc version server: version không nói prop nào đã merge (nhánh merge theo thứ tự khác nhau);
  - thêm field `capabilities` vào MCP: đổi `tools/list`/resource, trái mục tiêu "không thêm tool".

### 2. Mapper (mỗi mapper một hàm thuần `(el, rect, caps) → {props, diag}`)

**Border → `stroke`** (`map-border.mjs`, cờ `stroke`)
- Cạnh có hiệu lực: `border-*-style` ∉ {`none`,`hidden`} và width > 0. Không cạnh nào: không key.
- `width`: 4 số `[t,r,b,l]` (cạnh không hiệu lực = 0). Bằng nhau thì scalar; bằng 1 thì bỏ key.
- `color`: màu các cạnh có hiệu lực bằng nhau → scalar; khác → mảng 4, cạnh width 0 lấy màu cạnh có hiệu lực đầu tiên (P04a bắt buộc màu mỗi phần tử).
- Cờ `strokeShort`: mảng rút về dạng CSS ngắn nhất (`[1,0,1,0]` → `[1,0]`), cùng helper `shortestQuad` với radius.
- `align` luôn bỏ (mặc định `inside` = CSS border).
- `dashed`/`dotted`/`double`/`groove`/`ridge`/`inset`/`outset`: vẽ solid, diag `BORDER_STYLE_APPROX {styles}`.
- **Stack**: P18a cộng viền vào `padding`. Khi cờ bật, `padding` = padding CSS, còn `stack.mjs` cộng `stroke.width` vào padding **khi dự đoán vị trí** (đúng luật P04a). Kiểm chứng ≤ 0,5 px giữ nguyên, nên nếu server tính khác, container rơi về `free` chứ không đặt sai.
- `BORDER_UNSUPPORTED` không còn sinh.

```json
{"stroke":{"color":"#D0D5DD","width":[0,0,1,0]}}
{"stroke":{"color":"#D0D5DD","width":[0,1]}}
```
(dòng 1: toolbar `border-bottom:1px`; dòng 2: cùng nhưng `border-inline:1px` với `strokeShort`)

**Bo góc** (`map-radius.mjs`; luôn chạy, cờ `radiusQuad` chỉ đổi đầu ra)
- Đọc 4 `border-*-radius` computed. Mỗi giá trị là `h` hoặc `h v`, đơn vị px hoặc `%` (theo width cho `h`, height cho `v`).
- **Áp luật chồng góc CSS** (CSS Backgrounds §5.5): `f = min(1, Lᵢ / Σrᵢ)` trên 4 cạnh, nhân mọi bán kính với `f`. Nhờ đó pill `border-radius:9999px` ra đúng `height/2` ở cả hai trạng thái cờ. Làm tròn 0,01.
- Góc elip (`h ≠ v` sau co): dùng `min(h, v)`, diag `RADIUS_ELLIPTICAL`.
- Cờ tắt: như P18a (bằng nhau → số; khác → max + `RADIUS_PER_CORNER`).
- Cờ bật: bằng nhau → số; khác → `[tl,tr,br,bl]` (rút gọn nếu `strokeShort`, P17 dùng chung luật). `RADIUS_PER_CORNER` không còn sinh.

**`box-shadow` → `shadow`** (`map-shadow.mjs`, cờ `shadow`)
- Computed Chrome luôn có dạng `<color> <x>px <y>px <blur>px <spread>px [inset]`, các lớp cách nhau bằng dấu phẩy ở **độ sâu ngoặc 0** (phẩy trong `rgba(…)` không tách).
- Mỗi lớp → `{x,y,blur,spread,color,inset}`, bỏ field bằng 0/false. Màu qua parser màu của P18a (lỗi → `COLOR_UNPARSED`, bỏ lớp đó).
- Quá 8 lớp: giữ 8 lớp đầu (lớp trên cùng), diag `SHADOW_TRUNCATED`.
- **Gộp text**: P18a gộp "phần tử chỉ chứa một text, không nền/viền/padding" thành node text. Khi cờ bật, thêm điều kiện "không `box-shadow`" (shadow không hợp lệ trên text): phần tử đó thành frame + text.
- `SHADOW_UNSUPPORTED` không còn sinh.

**Gradient → `fill`** (`map-gradient.mjs`, cờ `gradient`, ~130 dòng: phần lớn nhất)
- Nhận đúng **một** lớp `linear-gradient(…)` hoặc `radial-gradient(…)` trong `background-image`. `repeating-*`, `conic-gradient`, nhiều lớp, gradient + `url()`, hoặc trên text (`background-clip:text`) → giữ fallback P18a và diag `GRADIENT_FLATTENED {reason}`.
- **Linear**:
  - không hướng → bỏ `angle` (mặc định 180);
  - `Ndeg|grad|rad|turn` → độ;
  - `to top|right|bottom|left` → 0/90/180/270;
  - `to <góc>`: CSS phụ thuộc tỉ lệ hộp. Với `θ = atan2(h, w)` theo độ: top right = θ, bottom right = 180 − θ, bottom left = 180 + θ, top left = 360 − θ. Ghi số đã tính, làm tròn 0,01.
- **Radial**:
  - `center` = `at X Y` đổi ra phân số (`%`/100, `px`/w hoặc /h, từ khoá left/center/right/top/bottom); `[0.5,0.5]` thì bỏ;
  - `radius`: ellipse farthest-corner → bỏ key; kích thước tường minh `R` (circle) / `Rx Ry` (`%` theo w/h) → số / `[rx, ry]`;
  - từ khoá closest-side / farthest-side / closest-corner / circle farthest-corner: tính số theo CSS Images §3.2.2 (corner của ellipse = side × √2), ghi số.
- **Stop**:
  - vị trí `%` → /100, `px` → / độ dài đường gradient (linear: `|w·sinθ| + |h·cosθ|`; radial: `rx`);
  - stop hai vị trí → hai stop; vị trí thiếu → điền theo luật CSS (đầu 0, cuối 1, kẹp không giảm, chia đều đoạn thiếu);
  - color hint (một `%` đứng riêng) → bỏ, diag `GRADIENT_HINT_APPROX`;
  - quá 16 stop → lấy mẫu lại 16 điểm đều, diag `GRADIENT_RESAMPLED`.
- **Có cả `background-color`**: CSS vẽ màu nền dưới gradient. Converter **ghép từng stop source-over lên màu nền** rồi bỏ màu nền. Phép này chính xác vì P09a nội suy premultiplied như CSS và source-over với một màu hằng là tuyến tính theo t. Sai số chỉ từ làm tròn 8-bit (≤ 1/255).
- `GRADIENT_FLATTENED` chỉ còn sinh cho các trường hợp không hỗ trợ ở trên.

```json
{"fill":{"type":"linear","angle":90,"stops":[[0,"#6366F1"],[1,"#8B5CF6"]]}}
{"fill":{"type":"radial","center":[0.5,0],"stops":[[0,"#6366F159"],[0.7,"#6366F100"]]}}
```

### 3. Diagnostics

- Bảng mã trong `diagnostics.mjs` thêm cột `requires` (tên cờ). `wouldUse` chỉ in khi cờ tương ứng **tắt**. Sau khi đề xuất đích merge, mã đó biến mất khỏi `diagnostics.json` mà không sửa bảng.
- Mã mới (đều info hoặc warn, không chặn): `BORDER_STYLE_APPROX`, `RADIUS_ELLIPTICAL`, `SHADOW_TRUNCATED`, `GRADIENT_HINT_APPROX`, `GRADIENT_RESAMPLED`.
- **Font qua `list_fonts`** (phụ, ~15 dòng trong `server-read.mjs`): khi có `--server --project` và không có `--fonts`, map `"family|weight|style" → id` dựng từ `list_fonts`, `"*"` = `font_bundled` nếu có. Bỏ một bước agent phải đọc `list_fonts` vào context và viết `fonts.json`.

## Phạm vi file

| File | Việc | Dòng |
| --- | --- | --- |
| `tools/dom-import/caps.mjs` (mới) | `detectCaps(md)`, parse dòng kind + ô Value, `--caps` override | ~70 |
| `tools/dom-import/server-read.mjs` (mới) | MCP HTTP chỉ đọc: `initialize`, `resources/read`, `list_fonts` | ~60 |
| `tools/dom-import/map-border.mjs` (mới) | border → stroke, `shortestQuad` | ~60 |
| `tools/dom-import/map-radius.mjs` (mới) | parse 4 góc, %, luật chồng góc, đầu ra theo cờ (chuyển phần radius khỏi `map-style.mjs`) | ~60 |
| `tools/dom-import/map-shadow.mjs` (mới) | tách lớp theo phẩy độ sâu 0, parse lớp | ~50 |
| `tools/dom-import/map-gradient.mjs` (mới) | tokenizer hàm CSS, linear/radial, stop fixup, ghép nền | ~140 |
| `tools/dom-import/map-style.mjs` | gọi 4 mapper theo cờ; điều kiện gộp text | ~130 → ~120 |
| `tools/dom-import/stack.mjs` | padding = CSS khi `stroke`, cộng stroke khi dự đoán | ~120 → ~130 |
| `tools/dom-import/diagnostics.mjs` | cột `requires`, 5 mã mới | ~60 → ~75 |
| `tools/dom-import/convert.mjs`, `cli.mjs` | truyền `caps`; cờ `--props/--server/--project/--caps`; `summary.caps` | +~25 |
| `tools/dom-import/fixtures/props-baseline.md`, `props-all.md` (mới) | `props.md` của integration hôm nay; bản giả lập đủ 5 cờ (thêm dòng/ô theo shape các đề xuất) | sinh |
| `tools/dom-import/fixtures/box.html` (mới) | 8 hộp: viền đều, viền dưới, viền 4 màu, pill 9999px, bo `8px 8px 0 0`, shadow 2 lớp + inset, linear `to top right` có nền, radial `circle at 50% 0` | ~60 |
| `tools/dom-import/mappers.test.mjs` (mới) | AC 1–7 trên snapshot tay | ~170 |
| `tools/dom-import/live-pixels.test.mjs` (mới, `test:dom-import:live`) | AC 9–10 | ~140 |

Không file nào quá 200 dòng. Không sửa `crates/`, không thêm dependency (giải mã PNG bằng `<canvas>` trong chính Playwright).

## Tương thích MCP

- `tools/list` **±0 byte**: không tool, không schema, không mô tả mới. Test budget F-TOOLS (`900e0dc`) giữ nguyên.
- CLI chỉ gọi `resources/read` và `list_fonts` (đọc); không ghi vào server. Áp op vẫn qua P15/`apply_batch` như P18a.
- Không cờ nào bật trên server chưa có prop: op sinh ra luôn hợp lệ với catalog đang chạy. Cờ tắt → output byte-identical P18a.
- Skill `author-frame.md` (mục "Redraw an existing web page" của P18a): thêm 2 dòng — chạy CLI với `--server --project` để tự bật mapper và tự map font; đọc `summary.caps`.

## AC

1. `npm run test:dom-import` pass, thêm ≥ 14 test mới; test P18a cũ pass không đổi.
2. **Cờ tắt = P18a**: convert snapshot fixture P18a với `props-baseline.md` cho NDJSON có sha256 bằng bản sinh không có `--props`.
3. **Dò cờ**: `detectCaps(props-baseline.md)` = mọi cờ false; `detectCaps(props-all.md)` = 5 cờ true; `--caps -gradient` trên `props-all.md` tắt đúng `gradient`.
4. **Stroke** (cờ `stroke`, không `strokeShort`): toolbar P18a sinh `stroke:{"color":"#D0D5DD","width":[0,0,1,0]}` và `layout.padding:[12,16]` (không cộng viền); viền 4 màu sinh `color` mảng 4; viền đều 1 px sinh `{"color":…}` không `width`; `dashed` sinh thêm `BORDER_STYLE_APPROX`. Không còn `BORDER_UNSUPPORTED`. Với `strokeShort`: `width:[1,0]` cho `border-block:1px`.
5. **Radius**: pill 40 px cao `9999px` → `radius:20` ở cả hai trạng thái `radiusQuad`; `8px 8px 0 0` → `[8,8,0,0]` khi bật, `8` + `RADIUS_PER_CORNER` khi tắt; `50%` trên hộp 100×60 → `30` kèm `RADIUS_ELLIPTICAL`.
6. **Shadow**: `0 4px 12px rgba(0,0,0,.25), inset 0 1px 0 rgba(255,255,255,.4)` → `[{"y":4,"blur":12,"color":"#00000040"},{"y":1,"color":"#FFFFFF66","inset":true}]`; phần tử một text có shadow sinh frame + text (không text mang `shadow`).
7. **Gradient**: `linear-gradient(to top right, #6366f1, #8b5cf6)` trên hộp 200×100 → `angle` 26,57 (±0,01); `linear-gradient(red 10%, 40%, blue)` → stop `[[0.1,…],[1,…]]` + `GRADIENT_HINT_APPROX`; `radial-gradient(circle at 50% 0, …)` trên 200×100 → `center:[0.5,0]`, `radius` = 141,42 (±0,01); gradient có stop `#0000` trên nền `#ffffff` → mọi stop là `#RRGGBB` đục, không còn key nền riêng; `repeating-linear-gradient` → fallback + `GRADIENT_FLATTENED`.
8. Mọi key sinh ra nằm trong danh sách kind của `props.md` đang dùng (kiểm trên cả hai fixture); `stroke`/`shadow`/gradient **không** xuất hiện trên op kind `text`.
9. **Dò cờ khớp server thật** (`live-pixels.test.mjs`, server tạm `--port 0 --data-root $(mktemp -d)`): đọc `props.md` qua `resources/read`; với mỗi cờ true, áp một op probe đúng shape vào project tạm → 0 lỗi; với mỗi cờ false, op probe → `VALIDATION_FAILED` với `field` đúng key. Chạy được trên `night/integration` hôm nay (mọi cờ false) và trên nhánh có P04a (cờ `stroke` true) mà không sửa test.
10. **Pixel** (cùng test): Playwright Chromium `deviceScaleFactor:1` chụp `fixtures/box.html` 640×480; convert với cờ dò được; áp ops; `export_nodes` root frame 1x; giải mã cả hai ảnh bằng `<canvas>`; trong khung mỗi hộp nới 24 px (gồm vùng bóng), chỉ so hộp có cờ tương ứng bật:
    - stroke, radius: ≥ 99,5 % pixel lệch ≤ 2/255 mỗi kênh, không pixel nào lệch > 64/255;
    - gradient: ≥ 99,5 % pixel lệch ≤ 2/255;
    - shadow: ≥ 99 % pixel lệch ≤ 3/255.
    Hộp có cờ tắt: chỉ assert bounds `inspect_layout` lệch ≤ 1 px như P18a AC 10. Test in tỉ lệ đo được từng hộp; dọn process và thư mục tạm.
11. `stdout` CLI vẫn ≤ 600 B (gồm `summary.caps`), một JSON hợp lệ.
12. `wc -l tools/dom-import/*.mjs` mọi file ≤ 200; `git diff --stat -- crates/` rỗng; `cargo test -p opencanvas-server` pass (budget tools/list).

## Cách kiểm

```bash
npm run test:dom-import
CARGO_BUILD_JOBS=4 cargo build -p opencanvas-server && npm run build:renderer && npm run test:dom-import:live
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server
wc -l tools/dom-import/*.mjs && git diff --stat -- crates/
```

- Chạy live hai lần: trên `night/integration` (mọi cờ tắt: AC 9 phần "false", AC 10 chỉ bounds) và trên nhánh có P04a (cờ `stroke` bật: AC 10 phần stroke). Ghi tỉ lệ pixel từng hộp vào report.
- `props-all.md`: dev chép `props.md` hôm nay, thêm `stroke`/`shadow` vào dòng kind frame/group/rectangle/image và chép Value đúng chữ dự kiến của từng đề xuất. Khi đề xuất merge thật, thay bằng output `resources/read` thật (AC 9 bắt lệch chữ).

## Rủi ro

- **Marker trong ô Value đổi chữ** khi dev triển khai P04b/P09a/P17 (ví dụ không viết "gradient"). Hậu quả: cờ tắt nhầm, mất độ sát nhưng op vẫn hợp lệ (an toàn). AC 9 chạy trên nhánh đó báo ngay; `--caps` ép bật tạm. Khuyến nghị gửi dev các đề xuất đó: ô Value giữ các chữ `gradient`, `topLeft`, `1-4`.
- **Stack + stroke**: nếu P04a merge khác luật inset (ví dụ không cộng ở `free`), kiểm chứng ≤ 0,5 px của `stack.mjs` đưa container về `free` với `STACK_FALLBACK`, không đặt sai.
- **Shadow bị cắt khi export**: `export_nodes` cắt theo bounds node, nên fixture đặt hộp cách mép root ≥ 32 px; bóng tràn root frame thật sẽ bị cắt như CSS `overflow:hidden` của root (`clip:true` P18a).
- **Gradient trên nền trong suốt một phần**: ghép stop lên nền bán trong suốt vẫn đúng toán học, nhưng làm tròn 8-bit hai lần có thể lệch 2/255; nằm trong ngưỡng AC 10.
- **`list_fonts` của project khác font máy chụp**: map tự động chỉ khớp face đã import; còn lại vẫn `FONT_FALLBACK` như P18a.

## Ngoài phạm vi / phase sau

- **P18e — mapper text/font/stack** (chưa viết file; cùng cơ chế `caps.mjs`, mỗi cờ = key có trong dòng kind tương ứng):
  - P10b `letterSpacing` (computed px, `normal` → bỏ), `textAlign` (`start`/`left` → bỏ, `end` → `right`, `justify` → bỏ + diag; text gộp phần tử hoặc nhiều dòng dùng **content box của phần tử** làm x/width để từng dòng căn đúng), `textDecoration` (`underline`/`line-through`; `overline`, nhiều giá trị, màu/độ dày riêng → diag), `textTransform` (thôi nướng vào chuỗi, gửi chuỗi gốc + key);
  - P10c `fontFamily` (họ đầu, bỏ nháy) + `fontWeight` (bỏ khi 400) + `fontStyle` (`italic`; `oblique` → `italic` + diag) khi face có trong `list_fonts`, thay `fontId`; còn lại giữ map `fonts`;
  - P08b: `alignSelf` khi con lệch vì `align-self` (stretch với kích thước cố định → start), kiểm chứng lại rồi mới giữ stack; `justify` `space-around`/`space-evenly` (dò qua ô Value của `layout`);
  - P10a `textWrap:"nowrap"` cho `white-space:nowrap|pre`, bỏ `textSlack`.
- Suy `fill`/`hug` từ `flex-grow`/`width:auto` và block flow → `column` (P18a đã ghi).
- `outline` → `stroke` `align:"outside"`, `text-shadow`, `backdrop-filter` → `backdropBlur` (P09c), gradient trên text, `conic-gradient`, nhiều lớp nền (cần node phụ).
- P18c (svg/img), P18d (tự so `render_frame.compare`) như P18a.
