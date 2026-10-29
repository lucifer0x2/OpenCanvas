# P18a — Importer DOM → NDJSON ops: script capture trong trang + converter Node, suy stack có kiểm chứng, phần chưa hỗ trợ ra diagnostics (không thêm tool)
Ưu tiên: P2 · Kích cỡ: ~85′ dev · Phụ thuộc: không bắt buộc về code (dùng props đã merge trên `night/integration` `0847b4b`: fill/opacity/clip/radius/layout P08a/image). **P15** để áp file rẻ token (đường khuyến nghị trong skill). Mỗi mapper cho P04a/P04b/P09a/P09b/P10a/P10b/P10c/P14a/P14b là phase sau (P18b) · Nguồn: note "Ý tưởng → Importer HTML/DOM → OpenCanvas" (`/Users/luis/Documents/opencanvas.md:82-83`), cách làm ở dòng 6–7, note "Vector/SVG node" (dòng 79)

## Vấn đề

"Vẽ lại UI có sẵn" là việc người dùng đã làm thật: 18 frame QueryNest, ~6 900 node. Họ phải tự viết pipeline ~300 dòng: `getBoundingClientRect`, computed style, `Range` cho text, rasterize icon, rồi `create_node` gửi thẳng MCP HTTP theo batch 450 op. Kết quả "rất sát", nhưng:

- Không có trong repo. Agent sau phải viết lại, và lại vấp đúng những chỗ người dùng đã vấp: key props sai (note #1/#2), text cần nới width, viền phải giả bằng rect.
- Pipeline đó sinh toàn `free` với x/y tuyệt đối, nên coder nhận `NO_REFLOW_INFORMATION`. Mất luôn cấu trúc flex mà DOM vốn khai báo rõ.
- Style chưa hỗ trợ (border, shadow, gradient, letter-spacing, SVG) bị bỏ **im lặng** hoặc vẽ giả. Không ai biết frame lệch ở đâu.

## Bằng chứng

- **Props đang có** trên `night/integration`: bảng duy nhất ở `crates/storage/src/props_catalog.rs:78-102`. Converter chỉ được sinh các key này.
  - Hình và màu: `fill` hex/token (`:94`), `opacity` group composite (`:95`, P05 merged `565b12b`), `clip` (`:96`), `radius` một giá trị (`:97`).
  - Layout: `layout` row/column/free với padding 1–4 số, `gap`, `wrap`, `align` start|center|end|stretch, `justify` start|center|end|space-between (`:89`, P08a merged `ec1fc01`).
  - Text: `text`, `fontId`, `fontSize`, `lineHeight`, tất cả bắt buộc (`:90-93`).
  - Ảnh: `assetId`, `fit`, `crop` (`:98-100`).
- **Stack OpenCanvas là CSS flexbox qua Taffy.** `crates/layout/src/stack.rs:1` ("Row/column stack rules mapped onto Taffy flexbox, CSS style"), `:23-48` (padding, gap, `align_items`, `justify_content`).
  - Vì vậy một container `display:flex` có con kích thước cố định cho đúng cùng vị trí ở hai bên. Kiểm chứng bằng hình học là đủ, không cần mô phỏng engine khác.
  - Stack không có con `absolute`: `rg absolute crates/layout/src` rỗng. Con nào `position:absolute` thì cha phải là `free`.
- **Op và ID.**
  - `create_node` cần `id`, `kind`, `parentId`, `props` (`crates/server/src/schemas/operations.rs:13-17`); props tối đa 50 key.
  - ID chỉ gồm `[A-Za-z0-9_-]` và dài ≤ 128 byte (`crates/storage/src/validate/values.rs:4-10`).
- **Công cụ JS có sẵn.** `package.json:16` có `playwright 1.64.1`; test JS chạy bằng `node --test` (`package.json:8`). E2E đã có helper MCP HTTP `connectMcp` (`spikes/e2e/protocol.mjs:21`). Không cần dependency mới.
- **Đường áp file.** P15 (`.night/proposals/P15-bulk-apply-file.md`): `POST /api/v1/projects/{id}/batches` nhận NDJSON, mỗi dòng một op, tự chia chunk, key tất định theo dòng.
  - Đo ở P15: 450 op qua tool call tốn ~96 KB tham số; qua file chỉ ~170 B lệnh curl.
  - Chỉ đạt mức tiết kiệm đó khi **op do script sinh**. Đây chính là trường hợp importer.
- **Đề xuất liên quan** (định nghĩa prop đích của các mapper phase sau):

  | ĐX | Prop | Ví dụ |
  | --- | --- | --- |
  | P04a | `stroke` | `P04a-stroke.md:78-81` |
  | P04b | `radius` 4 góc | `P04b-corner-radii.md:52-55` |
  | P09a | gradient trong `fill` | `P09a-gradient-fill.md:74-77` |
  | P09b | `shadow` | `P09b-box-shadow.md:70-73` |
  | P10a | `textWrap` | `P10a-text-measure-fidelity.md:132` |
  | P10b | `letterSpacing`, `textAlign`, `textDecoration`, `textTransform` | `P10b-text-style-props.md:63-65` |
  | P10c | `fontFamily`, `fontWeight`, `fontStyle` | `P10c-font-family-weight.md:42-45` |
  | P14a | kind `vector` | `P14a-vector-path-node.md:93-96` |
  | P14b | op `import_svg` | `P14b-import-svg.md:63-66` |
  | P08b | `alignSelf` | `P08b-align-self-space.md:52` |

## Giải pháp

Một thư mục công cụ `tools/dom-import/`, không có code server. Gồm hai bước tách nhau, để phần chuyển đổi test được không cần trình duyệt.

### 1. Capture (trong trang): `capture.js`

- File tự chứa, không import. Định nghĩa `globalThis.__opencanvasCapture(opts)`.
- Chạy được qua Playwright `addScriptTag({content})` + `evaluate`, hoặc dán vào console: `copy(JSON.stringify(__opencanvasCapture({selector:'#root'})))`.
- **Duyệt DOM** từ `opts.selector` (mặc định `body`), giữ thứ tự DOM. Bỏ qua:
  - phần tử `display:none` hoặc `visibility:hidden`;
  - phần tử có diện tích 0 và không có con hiện;
  - phần tử nằm hoàn toàn ngoài rect của tổ tiên clip gần nhất (đếm vào `OFFSCREEN_SKIPPED`).
- **Phần tử**: ghi `rect` (`getBoundingClientRect`, làm tròn 0,01), `tag`, `sel` ngắn (`tag#id.class1`, ≤ 60 ký tự) và **chỉ các computed style khác giá trị mặc định** trong danh sách cố định:
  - box: `display`, `position`, `zIndex`, `opacity`, `overflowX`, `overflowY`, `transform`, `filter`, `backdropFilter`, `mixBlendMode`;
  - flex: `flexDirection`, `flexWrap`, `rowGap`, `columnGap`, `alignItems`, `justifyContent`, `alignSelf`, `order`, `flexGrow`;
  - khoảng: `margin*`, `padding*`;
  - nền và viền: `backgroundColor`, `backgroundImage`, `border{Top,Right,Bottom,Left}{Width,Style,Color}`, 4 `border*Radius`, `boxShadow`;
  - ảnh: `objectFit`;
  - pseudo: có `::before`/`::after` với `content` khác `none` hay không.
- **Text**: mỗi DOM Text node không rỗng thành `{text, rects, style}`.
  - `rects` lấy từ `Range.selectNodeContents(node).getClientRects()`.
  - `style` lấy từ phần tử cha: `color`, `fontFamily` (họ đầu tiên, bỏ nháy), `fontWeight`, `fontStyle`, `fontSize`, `lineHeight`, `letterSpacing`, `textAlign`, `textDecorationLine`, `textTransform`, `whiteSpace`.
  - Text được gộp khoảng trắng theo `white-space` như trình duyệt hiển thị.
- **Thẻ thay thế**:
  - `img`: thêm `src` (`currentSrc`), `naturalWidth`, `naturalHeight`;
  - `svg`: chỉ ghi `svgBytes` (độ dài `outerHTML`), không duyệt con;
  - `canvas`, `video`, `iframe`: chỉ ghi rect.
- **Nền canvas**: nếu root trong suốt, lấy nền của `body`, rồi đến `html`, giống trình duyệt.
- Kết quả là snapshot `{version:1, viewport:{width,height}, url, root:{…, children:[…]}}`, nằm ở file chứ không vào context agent.

### 2. Convert (Node): `convert.mjs` + `map-style.mjs` + `stack.mjs` + `diagnostics.mjs`

`convert(snapshot, {pageId, idPrefix, fonts, assets, at:[x,y], name})` trả `{ops, diagnostics, summary}`. Đây là hàm thuần, không I/O.

**ID và thứ tự**
- Root là `idPrefix`, con là `${idPrefix}-${n.toString(36)}` theo thứ tự preorder.
- `idPrefix` phải khớp `^[A-Za-z0-9_-]{1,100}$`; sai thì báo lỗi trước khi sinh op.
- Cùng snapshot thì cho NDJSON giống từng byte. Nhờ vậy chạy lại với cùng key P15 sẽ được replay.
- Op theo preorder (cha trước con), không có `index`, nên thứ tự vẽ là thứ tự DOM.
- Mỗi node có `name` = `sel` (phần tử) hoặc 40 ký tự đầu của text, để `find_nodes` tìm được.

**Chọn kind và gộp node** (giảm số node)
- Phần tử là **wrapper trơn** thì không sinh node, con của nó gắn vào tổ tiên gần nhất có sinh node. Wrapper trơn là phần tử thoả cả bốn điều kiện:
  - không có nền, viền, bo góc, shadow, opacity < 1 hay clip;
  - không được suy thành stack;
  - không phải flex item của một stack đã suy;
  - không có diagnostic riêng.
- Phần tử **chỉ chứa đúng một text** và không có nền, viền hay padding: sinh **một** node `text` dùng rect của chính phần tử. Rect phần tử là line box, nên khớp layout hơn rect glyph.
- Text khác: node `text` có:
  - `y = rect.top − (lineHeight − rect.height)/2` (half-leading);
  - `height = lineHeight × số dòng`, số dòng = số `top` khác nhau trong `rects`.
- Phần tử có con thành `frame`. Lá có hình thành `rectangle`. `img` đã có trong `assets` thành `image`.

**Ánh xạ style** (`map-style.mjs`, mỗi CSS prop là một hàm nhỏ trả `{props, diag}`)

| CSS | Prop OpenCanvas | Phase a |
| --- | --- | --- |
| `background-color` | `fill` `#RRGGBB`/`#RRGGBBAA` | có; `transparent` thì bỏ key |
| `color` (text) | `fill` | có |
| `opacity` < 1 | `opacity` | có (composite như CSS, P05) |
| `overflow` hidden/clip/auto/scroll | `clip:true` | có; auto/scroll thêm diag `SCROLL_FLATTENED` |
| `border-radius` đều | `radius` | có |
| 4 góc khác nhau | `radius` (lấy max) | diag `RADIUS_PER_CORNER` → P04b |
| `font-size`, `line-height` | `fontSize`, `lineHeight` | có; `normal` thì lấy `round(rect.height / dòng)` |
| `font-family` / `font-weight` / `font-style` | `fontId` qua map `fonts` | có; sau P10c dùng `fontFamily`/`fontWeight`/`fontStyle` |
| `display:flex` + direction/gap/padding/align/justify | `layout` row/column | có, **kiểm chứng** (bên dưới) |
| `border-*` | padding cộng thêm độ dày viền, không vẽ | diag `BORDER_UNSUPPORTED` → `stroke` P04a |
| `box-shadow` | không có | diag `SHADOW_UNSUPPORTED` → P09b |
| `linear-`/`radial-gradient` | `fill` = trung bình stop đầu và cuối | diag `GRADIENT_FLATTENED` → P09a |
| `letter-spacing`, `text-align` ≠ start, `text-decoration` | không có | diag `TEXT_STYLE_UNSUPPORTED` → P10b |
| `text-transform` | áp sẵn vào chuỗi `text` (giống hiển thị) | diag info `TEXT_TRANSFORM_BAKED` |
| `white-space:nowrap` | text một dòng: `width` = đo + `textSlack` (mặc định 1 px) khi cha free; đúng số đo khi trong stack | sau P10a dùng `textWrap:"nowrap"` |
| `img` chưa có trong `assets` | `rectangle` giữ chỗ `fill:"#0000000F"` | diag `IMAGE_UNMAPPED` (kèm `src` duy nhất) |
| `<svg>` | `rectangle` giữ chỗ | diag `SVG_UNSUPPORTED` → `import_svg` P14b / `vector` P14a |
| `background-image:url()`, `transform`, `filter`, `backdrop-filter`, `mix-blend-mode`, `::before`/`::after`, `canvas`/`video`/`iframe`, `z-index` làm đổi thứ tự vẽ | bỏ qua hoặc giữ chỗ | diag theo mã (`backdrop-filter` → P09c) |

Màu đọc từ chuỗi computed `rgb()`/`rgba()`/`color(srgb …)`. Không parse được thì diag `COLOR_UNPARSED` và bỏ key, **không đoán**.

**Suy stack có kiểm chứng** (`stack.mjs`)

Áp cho container có `display` là `flex`/`inline-flex` và đủ các điều kiện:
- `flex-wrap:nowrap`, hoặc mọi con nằm trên một dòng;
- direction là `row` hoặc `column`, không `-reverse`, không con nào có `order` ≠ 0;
- không con nào `position:absolute`/`fixed`;
- `align-items` không phải `baseline`.

Cách dựng layout:
1. `layout = {mode: "row"|"column", padding: [t,r,b,l], gap, align, justify}`.
   - `padding` = padding CSS + độ dày viền từng cạnh. Viền chưa vẽ (P04a), nhưng vẫn chiếm chỗ như trình duyệt.
   - `gap` = `column-gap` với row, `row-gap` với column; `normal` = 0.
   - `align`: flex-start/start/normal/stretch → `start`, center → `center`, flex-end/end → `end`. Con đã có kích thước đo nên `stretch` tương đương `start`.
   - `justify`: start/center/end/space-between như tên. Giá trị khác thì fallback.
   - Bỏ key nào bằng mặc định, để tiết kiệm byte.
2. Mọi con nhận `width`/`height` cố định bằng số đo và **không có `x`/`y`**.
3. **Kiểm chứng**: tính vị trí dự đoán của từng con theo quy tắc flex với kích thước cố định:
   - trục chính: bắt đầu từ padding, cộng offset của justify, rồi kích thước con + gap;
   - trục phụ: theo align.

   So với rect đo. Mọi con lệch ≤ `tolerance` (mặc định 0,5 px) thì giữ stack. Có con lệch hơn, ví dụ do `margin`, `margin-left:auto`, `align-self` hay `flex-wrap`: cả container chuyển sang `free` với x/y đo được và diag `STACK_FALLBACK {nodeId, reason, maxDelta}`.
   - `reason` thuộc `"margin"|"align-self"|"absolute-child"|"wrap"|"order"|"position"`.
   - Vì kiểm bằng chính hình học đã đo, stack sinh ra **không bao giờ** đặt con sai vị trí so với bản free.
4. Container không phải flex: `free`, con có `x`/`y` tương đối so với border box cha. Không ghi `layout`, vì mặc định đã là `free`.

**Root frame**
- `create_node {id: idPrefix, kind: "frame", parentId: pageId}`.
- `width`/`height` là số đo của phần tử root (root frame bắt buộc là số).
- `x`, `y` lấy từ `at` (mặc định 0,0); `clip:true`; `fill` là nền canvas.

**Font**
- `fonts` là map `"family|weight|style" → fontId`, ví dụ `{"SF Pro Text|600|normal": "font_sf_text_600", "*": "font_bundled"}`.
- Không khớp key nào thì dùng `"*"` và diag `FONT_FALLBACK {face, count}`.
- Thiếu cả `"*"` thì `convert` ném lỗi liệt kê mọi face chưa map và **không ghi file**. Agent sửa trong một lần, sau khi gọi `list_fonts`.

**Diagnostics**: gom theo mã, file nhỏ và đọc được.

```json
{"summary":{"ops":812,"bytes":143220,"nodes":{"frame":120,"rectangle":40,"text":630,"image":22},
            "stacks":{"row":31,"column":18,"fallbackFree":9}},
 "byCode":{"BORDER_UNSUPPORTED":{"count":44,"wouldUse":"stroke (P04a)","nodeIds":["qn-1f","qn-2a"],"sample":{"borderBottom":"1px solid rgb(208, 213, 221)"}},
           "IMAGE_UNMAPPED":{"count":3,"srcs":["http://localhost:5173/logo.png"],"nodeIds":["qn-9"]},
           "STACK_FALLBACK":{"count":9,"nodeIds":["qn-3c"],"sample":{"reason":"margin","maxDelta":12}}}}
```

Mỗi mã giữ tối đa 20 `nodeIds` và một `sample`. Bảng mã nằm trong `diagnostics.mjs`, kèm `wouldUse` trỏ tới đề xuất sẽ xử lý.

### 3. CLI: `cli.mjs` (npm script `import:dom`)

```bash
npm run import:dom -- --url http://localhost:5173/#/query --selector '#root' --viewport 1280x800 \
  --page page_screens --id-prefix qn-query --fonts fonts.json --assets assets.json --out /tmp/qn-query
```

- Mở Chromium headless với `deviceScaleFactor:1`, chờ `networkidle` cộng `document.fonts.ready`, rồi capture và convert.
- Ghi `<out>.ndjson` (mỗi dòng một op, dùng thẳng cho P15), `<out>.diagnostics.json` và `<out>.snapshot.json`.
- `--snapshot file.json` thay cho `--url`: convert snapshot capture tay từ console, không cần Playwright.
- **stdout chỉ một dòng JSON tóm tắt, ≤ 600 B**: `summary` cộng `count` theo mã cộng đường dẫn file. Đây là thứ duy nhất vào context agent.
- Áp vào project (khi đã có P15):

  ```bash
  curl -sS --data-binary @/tmp/qn-query.ndjson 'http://127.0.0.1:3847/api/v1/projects/p_…/batches?expectedRevision=R&idempotencyKey=qn-query'
  ```

### Vì sao chọn cách này

- **Không thêm tool MCP và không có code server.** `tools/list` không đổi; importer chạy ở nơi có trình duyệt, tức máy agent.
  - Tool `import_html` phía server phải nhúng trình duyệt vào server (Docker image phình, rủi ro tải URL tuỳ ý). Server cũng không thấy được app chạy trên localhost của agent.
  - Một op `import_dom` trong `apply_batch` vẫn bắt agent đưa snapshot (hàng trăm KB) qua tool call, tức đúng chi phí mà P15 muốn bỏ.
- **Tách capture / convert.** Phần khó (ánh xạ, kiểm chứng stack) là hàm thuần, test được bằng snapshot fixture. Capture chỉ đo, nên mỗi prop mới (P18b) chỉ sửa `map-style.mjs`.
- **Stack kiểm chứng thay vì đoán.** Bản `free` của người dùng đã "rất sát". Suy stack chỉ đáng làm khi không làm mất độ sát, và kiểm chứng hình học bảo đảm điều đó. Container không qua kiểm thì vẫn sát như trước, kèm lý do.
- **Không tự gọi MCP trong CLI.** P15 đã là đường áp chính thức: chunk, replay, lỗi theo dòng. CLI chỉ sinh file, nên không có hai đường áp song song.

## Phạm vi file

Tất cả là file mới trong `tools/dom-import/`, trừ ba file sửa ở cuối danh sách. Không file nào quá 200 dòng.

| File | Trách nhiệm | Dòng |
| --- | --- | --- |
| `capture.js` | DOM → snapshot, tự chứa, chạy trong trang | ~170 |
| `convert.mjs` | snapshot → ops: ID, kind, gộp wrapper, root, font, text | ~160 |
| `map-style.mjs` | parse màu/px/radius, các hàm CSS prop → props + diag | ~130 |
| `stack.mjs` | điều kiện, dựng `layout`, dự đoán vị trí, kiểm chứng | ~120 |
| `diagnostics.mjs` | bảng mã → `wouldUse`, gom theo mã, giới hạn 20 ID | ~60 |
| `cli.mjs` | parse args, Playwright, ghi 3 file, in tóm tắt | ~100 |
| `fixtures/basic.html` | toolbar flex, card có viền/shadow/gradient, list column, badge absolute, `margin-left:auto`, img, svg, text 2 dòng, wrapper trơn lồng | ~70 |
| `convert.test.mjs` | snapshot tay → ops/diag | ~150 |
| `stack.test.mjs` | kiểm chứng, fallback theo từng `reason` | ~110 |
| `capture.test.mjs` | Playwright `setContent(fixture)` → snapshot → ops; tất định | ~80 |
| `live.test.mjs` (chạy qua `test:dom-import:live`) | server tạm + áp ops + `inspect_layout` | ~110 |

File sửa:
- `package.json`: +3 script:
  - `import:dom`
  - `test:dom-import` (`node --test tools/dom-import/{convert,stack,capture}.test.mjs`)
  - `test:dom-import:live`
- `crates/server/skills/author-frame.md`: +~12 dòng mục "Redraw an existing web page". Gồm lệnh CLI, map font từ `list_fonts`, áp bằng P15, đọc `diagnostics.json`, và dòng "xoá bằng `delete_subtree <idPrefix>` rồi chạy lại".
- `README.md`: +3 dòng trỏ tới skill.

## Tương thích MCP

- `tools/list`: **±0 byte.** Không tool mới, không đổi schema hay mô tả tool. Đo bằng test budget hiện có (F-TOOLS, `900e0dc`).
- Op sinh ra chỉ dùng `create_node` và key trong `props_catalog.rs:78-102` tại thời điểm merge, nên mọi server cùng nhánh đều nhận.
- Mapper phase b chỉ được bật khi key tương ứng đã có trong catalog. Test `convert.test.mjs` đối chiếu mọi key sinh ra với danh sách key chép từ `skill://opencanvas/props.md`, ghi thành fixture `allowed-props.json`, cập nhật khi catalog đổi.
- Thứ thay đổi phía agent là skill `author-frame.md`, đọc qua `resources/read` khi cần; không nạp sẵn.
- Không ghi `~/.opencanvas`, không đụng cổng 3847 trong test.

## AC

1. `npm run test:dom-import` pass, gồm ≥ 12 test.
2. Fixture toolbar (`display:flex; padding:12px 16px; gap:8px; align-items:center; border-bottom:1px solid #D0D5DD`) sinh `layout:{"mode":"row","padding":[12,16,13,16],"gap":8,"align":"center"}`. Con không có `x`/`y`. Có diag `BORDER_UNSUPPORTED` với `wouldUse:"stroke (P04a)"` chứa ID của toolbar.
3. Fallback: container flex có con `margin-left:auto` sinh `free` với `STACK_FALLBACK reason:"margin"`. Container có con `position:absolute` sinh `reason:"absolute-child"`. Ở cả hai, x/y của mọi con lệch ≤ 0,01 px so với rect đo.
4. Diagnostics: card fixture sinh đúng các mã `SHADOW_UNSUPPORTED`, `GRADIENT_FLATTENED`, `RADIUS_PER_CORNER`, `TEXT_STYLE_UNSUPPORTED`, `IMAGE_UNMAPPED` (với `srcs` đúng 1 URL), `SVG_UNSUPPORTED`. `wouldUse` của từng mã đúng như bảng ánh xạ.
5. Mọi key trong `props` của mọi op nằm trong `allowed-props.json` theo `kind`. Mọi `id` khớp `^[A-Za-z0-9_-]{1,128}$`. Không op nào có hơn 50 key props.
6. Wrapper trơn lồng 3 cấp quanh một text không sinh node frame nào. Text gắn trực tiếp vào tổ tiên có sinh node, và x/y bằng rect đo trừ rect tổ tiên (±0,01).
7. **Tất định**: chạy CLI hai lần trên fixture cho 2 file `.ndjson` có sha256 bằng nhau (`capture.test.mjs`).
8. Font: snapshot có face chưa map và **không** có `"*"` thì CLI thoát mã 2, stderr liệt kê đủ các face, không tạo file `<out>.*`. Có `"*"` thì sinh `FONT_FALLBACK` với `count` đúng.
9. **stdout CLI ≤ 600 B** và là một JSON hợp lệ (`JSON.parse`). Kiểm bằng `Buffer.byteLength`.
10. **Live** (`npm run test:dom-import:live`), trên server tạm `--port 0 --data-root $(mktemp -d)`:
    - `create_project`, tạo page, áp ops fixture bằng `apply_batch` (chunk ≤ 500, qua `connectMcp`) với 0 lỗi;
    - `render_frame` rồi `inspect_layout` toàn bộ node: node không phải text lệch ≤ 1 px trên mỗi cạnh so với rect capture; text lệch ≤ 2 px ở x/y.
    - Nếu P15 đã merge thì áp thêm cùng file qua route bulk và assert `chunks[*].replayed` theo P15.
    - Sau test dọn process và thư mục tạm.
11. Không thay đổi `crates/` ngoài `skills/author-frame.md`: `git diff --stat` khớp. `cargo test -p opencanvas-server` (gồm test budget tools/list và test resource skill) pass.
12. `wc -l tools/dom-import/*.{js,mjs}`: mọi file ≤ 200 dòng.

## Cách kiểm

```bash
npm run test:dom-import
CARGO_BUILD_JOBS=4 cargo build -p opencanvas-server && npm run build:renderer && npm run test:dom-import:live
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server
wc -l tools/dom-import/*.js tools/dom-import/*.mjs
```

- Kiểm tay (không bắt buộc, ghi vào report): chạy CLI trên `web/` build preview của chính OpenCanvas (`npm --prefix web run preview`, cổng tạm). Ghi số op, byte, số stack/fallback và top 5 mã diag. Đây là số đo nền cho P18b.
- `live.test.mjs` khởi động server tạm bằng `startServer(dataRoot, 0)` (`spikes/e2e/server.mjs:8`) và kết nối bằng `connectMcp` (`spikes/e2e/protocol.mjs:21`); không bao giờ dùng 3847.

## Rủi ro

- **Đo text khác Chrome** (P10a chưa merge): text một dòng trong stack có width đúng số đo, có thể bị OpenCanvas ngắt dòng ("⌘C", note #4). Khi đó cao hơn và đẩy các con sau.
  - AC 10 dùng font bundled và chuỗi ASCII để tránh nhiễu.
  - P18b chuyển sang `textWrap:"nowrap"`.
  - Trong cha free thì có `textSlack` 1 px.
- **Kích thước cố định nên vẫn ít reflow**: stack có cấu trúc, nhưng con chưa có `fill`/`hug`. Coder thấy row/column, còn `reflow` giữa viewport chỉ đổi khi justify/align không phải start. Suy `fill` từ `flex-grow` (có kiểm chứng) để ở P18b.
- **Snapshot lớn**: app ~7 000 node cho snapshot vài MB. Chỉ nằm trên đĩa. `rects` text làm tròn 0,01 và style chỉ ghi khác mặc định.
- **Trang cần đăng nhập hoặc state**: CLI chỉ mở URL. Trang có state thì capture qua console (`--snapshot`), hoặc phase sau thêm `--storage-state` của Playwright.
- **Paint order**: DOM order khác thứ tự vẽ CSS khi có `z-index`/positioned. Đã ghi diag; chưa tự sắp lại.
- **Wrapper bị gộp** mất tên ngữ nghĩa (`nav`, `section`). Chấp nhận, vì `name` của con vẫn có `sel`. Phase sau có `--keep-landmarks`.

## Ngoài phạm vi / phase sau

- **P18b — bật mapper khi đề xuất đích merge**, mỗi mapper ~10–25 dòng trong `map-style.mjs`, một test, gỡ diag tương ứng:
  - `stroke` từ `border-*`, padding CSS không cộng viền nữa nếu P04a tính stroke vào insets (`stack.rs:22-34`): P04a;
  - `radius` 4 góc: P04b;
  - `fill` gradient: P09a;
  - `shadow` từ `box-shadow`: P09b;
  - `textWrap:"nowrap"` và bỏ `textSlack`: P10a;
  - `letterSpacing`/`textAlign`/`textDecoration`/`textTransform` (thôi áp sẵn vào chuỗi): P10b;
  - `fontFamily`/`fontWeight`/`fontStyle` thay map `fonts`: P10c;
  - `alignSelf`, gỡ `reason:"align-self"`: P08b;
  - `backdropBlur`: P09c.

  Cùng phase: suy `fill`/`hug` từ `flex-grow`/`width:auto` (vẫn kiểm chứng) và block flow → `column`.
- **P18c — icon và ảnh**: `<svg>` thành op `import_svg` (P14b, dùng `outerHTML` đã làm sạch) hoặc `vector` (P14a). `img`/`background-image` tự `import_asset` qua HTTP rồi map `assetId` (cần B05; xem P13b về upload raw).
- **P18d — tự kiểm**: sau khi áp, dùng `render_frame.compare` (P13a/P13b) với screenshot Playwright cùng viewport, ghi % lệch vào summary.
- Pseudo-element, `transform` xoay, `filter`, nhiều viewport (capture 2–4 kích thước và đối chiếu để sinh responsive), `--storage-state`, giữ landmark: chưa lên lịch.
