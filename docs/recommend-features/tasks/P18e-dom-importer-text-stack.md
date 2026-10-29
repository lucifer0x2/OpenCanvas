# P18e — DOM importer: mapper text (`textWrap`, `letterSpacing`, `textAlign`, `textDecoration`, `textTransform`, `fontFamily`/`fontWeight`/`fontStyle`) và stack (`alignSelf`, `space-around`/`space-evenly`), bật theo catalog
Ưu tiên: P2 · Kích cỡ: ~80′ dev (phần text ~55′ tự đủ; phần stack ~25′ làm sau nếu thiếu giờ) · Phụ thuộc: **P18a** + **P18b** (bắt buộc: `caps.mjs`, `server-read.mjs`, cột `requires` của `diagnostics.mjs`). Prop đích **không bắt buộc**: mỗi cờ chỉ bật khi server có key (P10a, P10b, P10c, P08b chưa merge trên `night/integration`) · Nguồn: P18b mục "Ngoài phạm vi" (`P18b-dom-importer-mappers.md:186-194`), note "Ý tưởng → Importer HTML/DOM" (`/Users/luis/Documents/opencanvas.md:82-83`)

## Vấn đề

- P18a xử lý text bằng đường vòng, mỗi đường có giá:
  - `letter-spacing`, `text-align` ≠ start, `text-decoration` bị bỏ, chỉ còn diag `TEXT_STYLE_UNSUPPORTED` (`P18a-dom-importer.md:113`). Text căn giữa lệch vị trí, link mất gạch chân.
  - `text-transform` được nướng sẵn vào chuỗi (`:114`), nên AI coder thấy "SAVE" thay vì "Save" + uppercase.
  - `white-space:nowrap` thành width đo + `textSlack` 1 px (`:115`): một số ma thuật, không nói được ý "một dòng".
  - Font qua map `fonts` → `fontId` mờ (`:108`), coder phải tra ngược ra family/weight.
- Stack: con có `align-self` khác cha, hoặc cha `justify-content: space-around|space-evenly`, làm cả container rơi về `free` (`P18a-dom-importer.md:135,142-143`). Tab bar và bottom nav là ca phổ biến, mất hết cấu trúc.
- Các prop đích (P10a/b/c, P08b) merge từng cái trong đêm. Importer phải bật theo **server đang chạy**, như P18b.

## Bằng chứng

- Catalog render thành `skill://opencanvas/props.md`. Dòng kind chỉ liệt kê **key**, không liệt kê alias (`crates/server/src/props_doc.rs:15-17,29-30`); alias nằm ở cột "Not this" của bảng (`:38`).
  - Hôm nay `fontFamily` là **alias** của `fontId` (`crates/storage/src/props_catalog.rs:96`). Nó không có trong dòng `text`, nên dò theo dòng kind không bắt nhầm.
  - Ô Value của `layout` hôm nay: justify chỉ `"start"|"center"|"end"|"space-between"` (`props_catalog.rs:94`). Có thể dò chữ `space-evenly`.
- Shape đích (đọc từ đề xuất):

  | ĐX | Key | Shape | Nguồn |
  | --- | --- | --- | --- |
  | P10a | `textWrap` | `"wrap"` (mặc định) \| `"nowrap"`; LF vẫn xuống dòng; `hug`+nowrap = đo không giới hạn | `P10a-text-measure-fidelity.md:130-141` |
  | P10b | `letterSpacing` | số px −1000..1000 (không nhận chuỗi `"0.02em"`) | `P10b-text-style-props.md:70,85` |
  | P10b | `textAlign` | `"left"`\|`"center"`\|`"right"`; `justify` bị từ chối | `P10b:71,184` |
  | P10b | `textDecoration` | `"none"`\|`"underline"`\|`"line-through"`, không đa giá trị; renderer không skip-ink | `P10b:72,88,142` |
  | P10b | `textTransform` | `"none"`\|`"uppercase"`\|`"lowercase"`\|`"capitalize"`; đo qua `innerText` của Chrome | `P10b:73,96` |
  | P10c | `fontFamily` / `fontWeight` / `fontStyle` | family của font đã import; int 100..900; `"normal"`\|`"italic"`; **không** cùng `fontId` trên một node; mặt phải khớp **chính xác**, trùng mặt → lỗi | `P10c-font-family-weight.md:50-54,70-77` |
  | P08b | `alignSelf` | `"start"`\|`"center"`\|`"end"`\|`"stretch"`, mọi kind, chỉ có tác dụng dưới row/column | `P08b-align-self-space.md:44-50` |
  | P08b | `layout.justify` | thêm `"space-around"`, `"space-evenly"`; gap là khoảng tối thiểu như CSS | `P08b:19-26,56` |

- Số đo P08b (Chrome = Taffy): space-around row 400, 3×50, gap 0 → x `41.671875, 175, 308.328125`; space-evenly → `62.5, 175, 287.5` (`P08b:25-26`). Dự đoán của `stack.mjs` lệch tối đa 1/64 px, trong `tolerance` 0,5 px.
- Độ khớp pixel đo ở P10b: textAlign center/right 18/18 ca 0 pixel khác DOM (`P10b:39`). Ngưỡng AC 9 dưới đây lấy từ đó cộng biên AA.

## Giải pháp

### 1. Cờ mới trong `caps.mjs` (cùng cơ chế P18b)

Bảng khai báo, thêm hàng chứ không thêm logic:

| Cờ | Bật khi (trên `props.md`) | Đề xuất |
| --- | --- | --- |
| `textWrap` | dòng kind `text` có `` `textWrap` `` | P10a |
| `letterSpacing`, `textAlign`, `textDecoration`, `textTransform` | dòng kind `text` có đúng key đó (mỗi key một cờ) | P10b |
| `fontFamily` | dòng kind `text` có `` `fontFamily` `` **và** có danh sách mặt font (`list_fonts` qua `--server --project`, hoặc `--font-list <file>`) | P10c |
| `alignSelf` | dòng kind `frame` có `` `alignSelf` `` | P08b |
| `spaceEvenly` | ô Value của `layout` khớp `/space-evenly/` | P08b |

- `--caps -textAlign,...` ép tắt/bật như P18b; `summary.caps` liệt kê cờ bật.
- Mọi cờ tắt: output **byte-identical P18a/P18b**.
- Dò theo key trong dòng kind là bền nhất: không phụ thuộc chữ mô tả (khác `radiusQuad`/`gradient` của P18b). Chỉ `spaceEvenly` dò chữ, vì đó là giá trị mới của key cũ.

### 2. Mapper text (`map-text.mjs`, mới; hàm thuần `(textRun, el, caps, faces) → {props, diag}`)

**`textWrap`** (cờ `textWrap`)
- `white-space` computed là `nowrap` hoặc `pre`, hoặc `text-wrap-mode:nowrap` → `textWrap:"nowrap"`. Bỏ `textSlack`.
- Width: `"hug"` khi text căn trái; số = content box khi có `textAlign` (để căn trong hộp).
- `pre`/`pre-wrap` có ≥ 2 khoảng trắng liên tiếp hoặc tab → diag `TEXT_APPROX {key:"textWrap", reason:"preserved-spaces"}`.

**`letterSpacing`** (cờ cùng tên)
- Computed `normal` hoặc `0px` → bỏ key. Khác → số px, làm tròn 0,01, kẹp −1000..1000.
- Chrome luôn trả px nên `em` không cần parse.

**`textAlign`** (cờ cùng tên)
- `start`/`left` → bỏ; `center`/`-webkit-center` → `"center"`; `right`/`end`/`-webkit-right` → `"right"`.
- `justify` → bỏ key, diag `TEXT_APPROX {key:"textAlign", reason:"justify"}` (P10b từ chối `justify`).
- `direction:rtl` → giữ như LTR, diag `reason:"rtl"`.
- **Hình học**: có `textAlign` thì node text lấy **content box của phần tử khối chứa** (rect − border − padding) làm `x`/`width`, `y` giữ của dòng đầu. Nhờ vậy mỗi dòng của text nhiều dòng căn đúng như trình duyệt. Phần tử là con stack: `width` = content box đó, vị trí do stack.

**`textDecoration`** (cờ cùng tên)
- CSS **lan** decoration xuống con inline mà computed của con vẫn là `none`. `capture.js` thêm (~12 dòng): OR `text-decoration-line` từ phần tử cha của text đi lên. Dừng sau phần tử là atomic inline (`inline-block`/`inline-flex`/`inline-grid`), float hoặc `position:absolute|fixed`, theo CSS Text Decoration 3 §2.
- Có `underline` → `"underline"`; ngược lại có `line-through` → `"line-through"`; ngược lại bỏ.
- Diag `TEXT_APPROX {key:"textDecoration", reason}`:
  - `overline` → `"overline"`;
  - hai dòng cùng lúc → `"multiple"`;
  - `text-decoration-style` ≠ `solid`, `-color` ≠ `color`, `-thickness` ≠ `auto|from-font` → `"style"`;
  - `underline` với `text-decoration-skip-ink` ≠ `none` → `"skip-ink"` (renderer không ngắt gạch quanh nét thò xuống, `P10b:142`).

**`textTransform`** (cờ cùng tên)
- `uppercase`/`lowercase`/`capitalize` → gửi chuỗi **gốc** (sau gộp khoảng trắng, chưa transform) + key. `TEXT_TRANSFORM_BAKED` không còn sinh.
- `capture.js` thêm `rawText` cạnh `text` khi `text-transform` ≠ `none` (~4 dòng).
- `full-width`/`full-size-kana` → vẫn nướng như P18a, diag `reason:"full-width"`.

**Font** (cờ `fontFamily`)
- `faces` = `list_fonts` của project (`server-read.mjs` P18b đã đọc) hoặc `--font-list <file>`.
- Family: duyệt danh sách `font-family` computed, bỏ nháy, bỏ generic (`sans-serif`, `system-ui`, …). Lấy family **đầu tiên có trong `faces`** (so không phân biệt hoa thường, ghi đúng chữ của `faces`).
- Weight: computed số → int. Style: `normal` | `italic`; `oblique` → `italic` + diag `TEXT_APPROX {key:"fontStyle", reason:"oblique"}`.
- Có **đúng một** record khớp `(family, weight, style)` → `fontFamily`, thêm `fontWeight` khi ≠ 400, `fontStyle:"italic"` khi nghiêng. **Không** gửi `fontId`.
- ≥ 2 record trùng mặt → gửi `fontId` của record id nhỏ nhất, diag `FONT_DUPLICATE {face, ids}`. Gửi `fontFamily` lúc này sẽ lỗi trùng của P10c (`P10c:75`).
- Không record khớp → đường map `fonts` của P18a/P18b, kể cả `"*"` + `FONT_FALLBACK`. Không bao giờ sinh mặt chưa import (P10c báo lỗi, `P10c:76`).

```json
{"kind":"text","props":{"text":"Save draft","fontFamily":"Inter","fontWeight":600,"fontSize":13,"lineHeight":18,"letterSpacing":0.4,"textTransform":"uppercase","textWrap":"nowrap","width":"hug"}}
{"kind":"text","props":{"text":"No results\nTry another query","fontFamily":"Inter","fontSize":14,"lineHeight":20,"textAlign":"center","x":24,"y":180,"width":312}}
```

### 3. Stack (`stack.mjs`, sửa)

**`alignSelf`** (cờ cùng tên)
- Con có computed `align-self` ≠ `auto`/`normal`: map bằng **cùng bảng** của `align-items` P18a (`flex-start|start|self-start` → start, `center`, `flex-end|end|self-end` → end, `stretch` → start vì con đã có kích thước đo).
- Ghi `alignSelf` chỉ khi khác `align` đã ghi của cha.
- Kiểm chứng ≤ 0,5 px dùng `alignSelf ?? align` cho trục phụ.
- `baseline`, `first baseline`, `last baseline` → vẫn fallback, `reason:"align-self"`.

**`space-around` / `space-evenly`** (cờ `spaceEvenly`)
- Map nguyên tên. Dự đoán trục chính:
  - `free = main − Σsize − gap·(n−1)`;
  - around: offset đầu `free/(2n)`, bước thêm `free/n`;
  - evenly: offset đầu `free/(n+1)`, bước thêm `free/(n+1)`.
- `free < 0` hoặc n = 1: vẫn dự đoán theo công thức, kiểm chứng quyết. Lệch thì fallback `reason:"justify"` (mã reason mới, không tính khi cờ tắt).
- Cờ tắt: như P18a (fallback).

### 4. Diagnostics

- `TEXT_STYLE_UNSUPPORTED` thêm payload `keys` = các key P10b có trong phần tử nhưng **cờ tắt**. Không còn key nào thì không sinh.
- `TEXT_TRANSFORM_BAKED` `requires:"textTransform"`. `STACK_FALLBACK` reason `align-self` (trừ baseline) `requires:"alignSelf"`.
- Mã mới, info/warn, không chặn, gộp theo `(key, reason)` kèm `count`: `TEXT_APPROX`, `FONT_DUPLICATE`.

## Phạm vi file

| File | Việc | Dòng |
| --- | --- | --- |
| `tools/dom-import/map-text.mjs` (mới) | textWrap, letterSpacing, textAlign + hình học content box, textDecoration, textTransform | ~140 |
| `tools/dom-import/map-font.mjs` (mới) | chọn family/weight/style theo `faces`, trùng mặt, fallback `fonts` (chuyển phần font khỏi `map-style.mjs`) | ~70 |
| `tools/dom-import/capture.js` | lan decoration, `rawText`, content box khối chứa | +~20 |
| `tools/dom-import/stack.mjs` | `alignSelf`, công thức around/evenly, reason `justify` | ~130 → ~160 |
| `tools/dom-import/caps.mjs` | 8 hàng cờ | ~70 → ~85 |
| `tools/dom-import/map-style.mjs`, `diagnostics.mjs`, `cli.mjs` | gọi 2 mapper; payload `keys`, 2 mã; cờ `--font-list` | ±0, +~10, +~5 |
| `tools/dom-import/fixtures/props-text.md` (mới) | `props.md` integration + dòng/ô theo shape P10a/b/c, P08b | sinh |
| `tools/dom-import/fixtures/text.html` (mới) | 10 mẫu text + 2 stack (mục AC) | ~70 |
| `tools/dom-import/text-mappers.test.mjs` (mới) | AC 2–7 trên snapshot tay | ~170 |
| `tools/dom-import/live-pixels.test.mjs` | thêm vùng text/stack và probe 8 cờ | +~40 (vượt 200 thì tách `live-text.test.mjs`) |

Không sửa `crates/`, không dependency mới. Mọi file ≤ 200 dòng.

## Tương thích MCP

- `tools/list` **±0 byte**: không tool, schema hay mô tả mới. Test budget giữ nguyên.
- CLI chỉ đọc server (`resources/read`, `list_fonts`), như P18b.
- Cờ chỉ bật khi key có trong catalog đang chạy, nên op luôn hợp lệ với server đó. Cờ tắt → byte-identical P18b.
- `author-frame.md` mục "Redraw an existing web page": thay 1 dòng: "với `--server --project`, font import sẵn được ghi bằng `fontFamily`, không cần `fonts.json`".

## AC

1. `npm run test:dom-import` pass, thêm ≥ 16 test; test P18a/P18b cũ pass không đổi.
2. **Cờ tắt = P18b**: convert snapshot `text.html` với `props-baseline.md` (P18b) cho NDJSON cùng sha256 với bản không có `--props`. `detectCaps(props-baseline.md)` có 8 cờ mới đều false; `detectCaps(props-text.md)` 8 cờ true; `--caps -textAlign` chỉ tắt `textAlign`.
3. **textWrap**: `<span style="white-space:nowrap">⌘C</span>` trong cha free → `textWrap:"nowrap"`, `width:"hug"`, không còn số `textSlack`; `white-space:pre` với 2 dấu cách liền → thêm `TEXT_APPROX {reason:"preserved-spaces"}`.
4. **P10b**:
   - `letter-spacing:.05em` trên 16px → `letterSpacing:0.8`; `normal` → không key.
   - `<p style="text-align:center;box-sizing:border-box;width:300px;padding:0 20px">` hai dòng → `textAlign:"center"`, `width:260`, `x` = content box trái; `justify` → không key + `TEXT_APPROX {reason:"justify"}`.
   - `<a style="text-decoration:underline"><span>Docs</span></a>` → span text có `textDecoration:"underline"`; `inline-block` ở giữa chặn lan (không key); `underline overline` → `"underline"` + `reason:"multiple"`.
   - `text-transform:uppercase` trên "Save draft" → `text:"Save draft"`, `textTransform:"uppercase"`, không còn `TEXT_TRANSFORM_BAKED`.
   - Cờ tắt từng key: key đó không sinh, `TEXT_STYLE_UNSUPPORTED.keys` liệt kê đúng key đó.
5. **Font** với `faces` = Inter 400/600 normal, Inter 400 italic, "Mono" 400 ×2 record:
   - `font-family:"Foo", Inter, sans-serif; font-weight:600` → `fontFamily:"Inter"`, `fontWeight:600`, không `fontId`;
   - Inter 400 italic → `fontStyle:"italic"`, không `fontWeight`; `oblique` → `italic` + `reason:"oblique"`;
   - Inter 700 → `fontId` từ `fonts`/`"*"` + `FONT_FALLBACK`, không `fontFamily`;
   - Mono 400 → `fontId` id nhỏ nhất + `FONT_DUPLICATE` đủ 2 id.
   - Không op nào có cả `fontId` và `fontFamily`.
6. **alignSelf**: row `align-items:center` cao 80, con giữa `align-self:flex-end` → container giữ `mode:"row"`, con có `alignSelf:"end"`, không `STACK_FALLBACK`; con `align-self:baseline` → vẫn `STACK_FALLBACK {reason:"align-self"}`.
7. **space-around/evenly**: row 400, 3 con 50, gap 0 → giữ row, `justify:"space-around"` (resp. `"space-evenly"`); cờ tắt → `STACK_FALLBACK`. Snapshot có x đo `41.671875/175/308.328125` → maxDelta ≤ 0,5.
8. Mọi key sinh ra nằm trong dòng kind của `props.md` dùng cho lần convert (kiểm trên 2 fixture props); `alignSelf` chỉ xuất hiện trên con của container row/column.
9. **Live** (`live-pixels.test.mjs`, server tạm `--port 0 --data-root $(mktemp -d)`):
   - **Probe**: mỗi cờ true → một op đúng shape áp vào project tạm 0 lỗi; cờ false → `VALIDATION_FAILED`, `field` đúng key (riêng `spaceEvenly`: `field:"layout.justify"`). Chạy được trên `night/integration` hôm nay (mọi cờ false) và trên nhánh có P10a/P10b/P08b mà không sửa test.
   - **Pixel**: Playwright Chromium `deviceScaleFactor:1` chụp `text.html` 640×480 (font fixture đã import vào project tạm qua `import_asset`; mọi underline đặt `text-decoration-skip-ink:none`), convert với cờ dò được, áp ops, `export_nodes` root 1x, giải mã bằng `<canvas>`. Với mỗi mẫu có cờ bật, trong khung mẫu nới 4 px: ≥ 99 % pixel lệch ≤ 8/255 mỗi kênh **và** bounds `inspect_layout` của node text lệch ≤ 1 px so với rect capture. Mẫu stack: bounds mọi con ≤ 1 px.
   - Mẫu có cờ tắt: chỉ assert bounds như P18a AC 10. Test in tỉ lệ từng mẫu; dọn process và thư mục tạm.
10. `stdout` CLI ≤ 600 B (gồm `summary.caps` 8 cờ mới), một JSON hợp lệ.
11. `wc -l tools/dom-import/*.{js,mjs}` mọi file ≤ 200; `git diff --stat -- crates/` chỉ có `skills/author-frame.md`; `cargo test -p opencanvas-server` pass.

## Cách kiểm

```bash
npm run test:dom-import
CARGO_BUILD_JOBS=4 cargo build -p opencanvas-server && npm run build:renderer && npm run test:dom-import:live
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server
wc -l tools/dom-import/*.mjs tools/dom-import/*.js && git diff --stat -- crates/
```

- Live chạy hai lần: trên `night/integration` (cờ false: probe phần "false", pixel chỉ bounds) và trên nhánh gộp P10a+P10b (+P08b nếu có). Ghi tỉ lệ pixel từng mẫu vào report.
- `props-text.md`: dev chép `props.md` hôm nay, thêm key vào dòng kind theo bảng Bằng chứng; khi đề xuất merge thật, thay bằng output `resources/read` (probe AC 9 bắt lệch).

## Rủi ro

- **Ngưỡng text 8/255** rộng hơn P18b vì anti-alias chữ giữa Chromium DOM và canvas renderer khác nhau ở rìa glyph, dù vị trí khớp. Bounds ≤ 1 px là chặn chính; ngưỡng pixel bắt lỗi thô (sai căn, mất gạch, sai chữ hoa). Dev ghi số đo thật; nếu thấp hơn nhiều thì siết.
- **Family đầu có trong `faces` ≠ family trình duyệt dùng** (máy chụp thiếu font đó): chữ capture và chữ vẽ khác metric. Bounds AC 9 bắt; người dùng thấy qua `FONT_FALLBACK` như P18a.
- **Lan decoration**: luật dừng (atomic inline, float, absolute) đủ cho UI thường; `display:contents`, table cell chưa thử. Sai thì chỉ mất/thừa gạch, không hỏng batch.
- **P10a/P10b/P08b đổi tên key khi dev triển khai**: cờ tắt, op vẫn hợp lệ (an toàn); probe AC 9 báo.
- **Phụ thuộc chuỗi**: P18e cần P18b merged. Nếu P18b chậm, có thể làm P18e trên P18a với `caps.mjs` tối thiểu (chỉ phần dòng kind), nhưng khi đó dev P18b phải rebase `caps.mjs`.

## Ngoài phạm vi / phase sau

- `alignSelf:"stretch"` ngữ nghĩa (bỏ kích thước trục phụ) và suy `fill`/`hug` từ `flex-grow`/`width:auto`: cần đổi luật "con có kích thước đo" của P18a, làm cùng phase suy responsive.
- Text style (`list_styles`): gom text trùng kiểu thành style để giảm token; cần ghi `styleId` và tạo style, ngoài vai trò chỉ sinh node.
- Rich text trong một node (span đậm giữa câu): OpenCanvas chưa có run; P18a tách node.
- `text-shadow`, `text-overflow:ellipsis`, `-webkit-line-clamp`, `writing-mode`, `font-variant`/`font-feature-settings`, decoration màu/độ dày/style riêng (P10b chưa có).
- `margin-*:auto` trong flex (fallback như P18a).
