# P14b — Op `import_svg` trong `apply_batch`: SVG text → node `vector`/`group`, an toàn, không lưu XML
Ưu tiên: P2 · Kích cỡ: ~90 phút dev · Phụ thuộc: **P14a** (bắt buộc: kind `vector`, `svgtypes` trong workspace) · Nguồn: note "Ý tưởng → Importer HTML/DOM → OpenCanvas (hoặc một tool `import_svg`)", note "Vector/SVG node"

Phase b của P14. Sau P14a, agent đã tự viết được icon một path. P14b nhận SVG như designer/agent có sẵn: file tải từ Google Fonts, Figma export, logo nhiều màu, SVG có `<g transform>`, `<rect rx>`, `<circle>`. Server tách SVG thành node vector, không bắt agent tự chuyển hình học và không lưu XML.

## Vấn đề

- P14a chỉ nhận `path` + `viewBox`. Phần lớn SVG thực tế còn có:
  - nhiều phần tử, mỗi phần tử một màu;
  - `transform` (rotate, scale, matrix) ở `<g>` hoặc ở phần tử;
  - hình cơ bản (`rect` có `rx`, `circle`, `ellipse`, `polygon`) thay vì `path`;
  - `fill-opacity`, `opacity`, `fill-rule`, `style="fill:…"`;
  - boilerplate của Figma: `<g clip-path="url(#clip0)">` với `<clipPath><rect width=24 height=24/></clipPath>`.
- Agent tự đổi những thứ này sang path đã nướng transform (nhân ma trận, đổi `A` dưới phép biến đổi không đều) thì tốn token và dễ sai. Hiện nay agent rasterize sang PNG.
- Ghi chú người dùng đề xuất `import_svg` như bước đầu của importer HTML/DOM (`/Users/luis/Documents/opencanvas.md:82-83`). Pipeline capture của họ đã phải "rasterize SVG icon" (`:6`).

## Bằng chứng

Code ở `night/integration@565b12b`.

- `crates/server/src/domain.rs:22-31`: `apply_batch` deserialize thẳng `BatchRequest` (`crates/storage/src/model/requests.rs:68-73`) rồi gọi storage. Đây là điểm duy nhất cần mở rộng op trước storage. HTTP không có route ghi batch (`crates/server/src/http.rs:38-46`).
- `crates/server/src/schemas/operations.rs:3-45`: schema op là `oneOf` các biến thể `{op: const}`, nên thêm một biến thể là thay đổi cộng thêm. `create_node.props` là `maxProperties 50` (`:11`).
- `crates/storage/src/model.rs:6-8`: `MAX_BATCH_OPS = 500`, `MAX_BATCH_BYTES = 2 MiB`, kiểm ở `crates/storage/src/project.rs:64-72`. History và bundle lưu ops đã commit, kiểm lại ≤ 500 khi import bundle (`bundle_history.rs:53`). Mở rộng op **trước** storage thì history chỉ chứa `create_node`, bundle không cần parser SVG.
- `crates/storage/src/error.rs:12,62`: lỗi có `op_index`. Sau khi mở rộng phải ánh xạ lại về chỉ số op gốc.
- `crates/storage/src/assets.rs:200-211` và `crates/storage/tests/assets.rs:44`: `image/svg+xml` bị từ chối trong `import_asset` và giữ nguyên như vậy (P14b không lưu SVG).
- Crate offline có sẵn trong `~/.cargo/registry`: `roxmltree 0.20.0` (parser XML thuần Rust, không I/O, `ParsingOptions.allow_dtd = false` mặc định, chống billion-laughs) và `svgtypes 0.15.3` (path, transform, color, viewBox). `usvg 0.45.1` cũng có, nhưng loại vì kéo `fontdb`/`rustybuzz`, và resolver ảnh mặc định đọc file từ đĩa theo `href`: bề mặt tấn công và phụ thuộc lớn hơn nhu cầu.

**Prototype** (Rust 230 dòng, `roxmltree` + `svgtypes`, build offline 4,9 s; so pixel bằng Chromium 156.0.8078.4)

Script ở scratchpad, không commit: `/private/tmp/claude-501/-Volumes-MySpace-Workspace-OpenCanvas/0c1758af-4597-4d58-b91f-4496d5b95b83/scratchpad/p14proto/src/main.rs` (`import`, `bake_arcs`), `.../scratchpad/p14/{svg/,mkimports*.mjs,compare3.mjs}`.

Kết quả chuyển đổi:

| SVG vào | Byte | Ra | So SVG image (max kênh, px > 16 / px mực), DPR 1 · 2 |
| --- | --- | --- | --- |
| Material `search` dạng Google Fonts (không `fill`) | 606 | 1 vector, `path` **nguyên văn** | 0 · 0 |
| Material `settings` có `fill="currentColor"`, `<title>` | 1 822 | 1 vector, `fill` = token truyền vào | 0 · 0 |
| Figma export: `clip-path` no-op + path + `rect rx` | 456 | group + 2 vector | 0 · 0 (DOM ≤ 1) |
| `transforms.svg`: rect rx dưới `rotate·scale(1.25,0.8)`, ellipse `rotate(-20 38 38)`, path `A` dưới `matrix`, circle `fill-opacity`, evenodd, polygon `opacity` | 656 | group + 6 vector | **18 · 22**, 1/889 · 1/3 393 px > 16 |
| cùng file, nướng cung thành cubic (`svgtypes::SimplifyingPathParser`, kurbo tolerance 0,1) | | | 45 · 62, 21/889 · 48/3 393 |

- Path đã nướng, vẽ lại bằng chính `<path>` trong SVG, cũng cho 0/255. Vậy lệch còn lại không nằm ở renderer.
- Nguồn lệch: Skia vẽ `<rect rx>`/`<ellipse>` bằng RRect/oval phân tích, còn ta vẽ bằng path.
- Đổi cung sang cubic (sai số hình học chỉ 0,0015 đơn vị) vẫn làm AA lệch nhiều hơn. Vì vậy **cung được giữ là cung**: ảnh affine của một ellipse là một ellipse. `rx', ry', φ'` lấy từ trị riêng của `E·Eᵀ` với `E = M·R(φ)·diag(rx, ry)`; đảo cờ `sweep` khi `det M < 0`.
- Chặn đúng ở bước parse:
  - `<script>` → `Unsupported element <script> at line 1`;
  - `onload` → `Event handler attribute 'onload' on <svg>`;
  - `<image xlink:href="http://127.0.0.1:3847/x.png">`, `<use href>`, `<foreignObject>` → phần tử không hỗ trợ;
  - `<!DOCTYPE … <!ENTITY …>` → `Invalid SVG XML: XML with DTD detected`;
  - path có `stroke` → lỗi rõ (chờ P14c);
  - `d="M0 0 L5"` → lỗi path có vị trí (dùng chung validator P14a).

**Tham khảo**

- Figma Plugin API `figma.createNodeFromSvg(svg)`: tạo một Frame chứa vector con (https://developers.figma.com/docs/plugins/api/figma/#createnodefromsvg).
- Penpot Plugin API `penpot.createShapeFromSvg(svg)` (https://doc.plugins.penpot.app/interfaces/Penpot). Phần tử không chuyển được thành shape thì Penpot giữ thành kiểu `svg-raw` (https://doc.plugins.penpot.app/interfaces/SvgRaw). Ta **không** giữ raw, mà từ chối rõ ràng.
- SVG 2 `transform`, basic shapes → path tương đương (https://www.w3.org/TR/SVG2/shapes.html), elliptical arc implementation notes (https://www.w3.org/TR/SVG2/implnote.html#ArcImplementationNotes).

## Giải pháp

### 1. Op mới trong `apply_batch`

```json
{"op":"import_svg","id":"ic-search","parentId":"toolbar","index":2,
 "svg":"<svg xmlns=\"http://www.w3.org/2000/svg\" height=\"24\" viewBox=\"0 -960 960 960\" width=\"24\"><path d=\"M376.57-324.93q…Z\"/></svg>",
 "color":{"tokenId":"icon-muted"},
 "props":{"width":18,"height":18,"name":"Material Symbols Rounded 500/search"}}
```

| Field | Bắt buộc | Nghĩa |
| --- | --- | --- |
| `id`, `parentId`, `index` | id, parentId | như `create_node`; `id` là node gốc được tạo |
| `svg` | có | SVG text UTF-8, ≤ 262 144 byte |
| `color` | không | hex hoặc `{tokenId}` màu. Thay cho `currentColor` **và** các fill để giá trị khởi đầu (không khai báo), vì icon đơn sắc thường dựa vào màu ngữ cảnh. Fill khai báo màu cụ thể giữ nguyên. Mặc định `#000000`. |
| `props` | không | props của node gốc, chỉ nhận `name, x, y, width, height, minWidth, maxWidth, minHeight, maxHeight, opacity`. Mặc định `width/height` = thuộc tính `width/height` (số hoặc `Npx`) của `<svg>`, nếu không có thì kích thước viewBox. |

**Kết quả**, theo thứ tự tài liệu:

- **Một** phần tử vẽ được và không có group opacity → **một** `vector` mang `id`:
  - `path`: `d` nguyên văn khi không có transform; đã nướng khi có;
  - `viewBox` của `<svg>`;
  - `fill`, `fillRule`, `opacity` (= `opacity × fill-opacity`, 3 chữ số).
- Nhiều phần tử → `group` mang `id` (`width/height` như trên) chứa các `vector` `id-1`, `id-2`, …:
  - mỗi vector `width:"fill"`, `height:"fill"`, cùng `viewBox`, nên đổi kích thước group thì mọi lớp co giãn cùng nhau;
  - `<g opacity<1>` chứa ≥ 2 phần tử vẽ được thành một `group` con có `opacity`, để P05 composite như CSS. Chứa 1 phần tử thì nhân vào phần tử đó.
- `preserveAspectRatio` của `<svg>`: mặc định → `fit` mặc định (contain); `none` → `fit:"fit"`; `xMidYMid slice` → `fit:"cover"`; căn lệch (`xMinYMin` …) → lỗi.
- Response giữ y như `apply_batch` hiện nay. `changedIds` liệt kê mọi node sinh ra; không thêm field.

**Vì sao là op trong `apply_batch`** mà không phải tool riêng:

- 40 icon cùng frame, nút và text nằm trong **một** batch, một revision, cùng `expectedRevision`/idempotency/conflict.
- Agent không phải học tool thứ 19.
- SVG đi vào context agent đúng một lần, response chỉ có id.
- Phương án `convert_svg` trả ops cho agent gửi lại đã bị loại, vì SVG đi qua context ba lần.

**Vì sao tách thành node** mà không giữ asset SVG vẽ bằng `drawImage`:

- Token màu dùng được; AI coder đọc được path; node chọn và sửa được.
- Không lưu và không phục vụ XML, nên không có đường cho script hay external ref chạy trong origin.

### 2. Mở rộng op (server, trước storage)

- `crates/server/src/svg_import/mod.rs`: `pub fn expand(args: &mut Value) -> Result<IndexMap>`.
  - Duyệt `operations`. Mỗi op `import_svg` được thay bằng các `create_node` sinh ra. `IndexMap` ghi chỉ số op gốc của từng op sau mở rộng.
  - `domain.rs:26-31` gọi `expand` trước `serde_json::from_value::<BatchRequest>`. Lỗi storage có `op_index` được đổi lại về op gốc.
  - Lỗi của chính importer mang `opIndex` gốc và field `svg` (hoặc `color`/`props.<key>`), theo format B02.
- **Idempotency:** mở rộng là hàm thuần, xác định. Storage băm request **sau** mở rộng, nên replay đúng body cho cùng ops và cùng kết quả. History và bundle chỉ có `create_node`.
- **Giới hạn:**
  - `svg` ≤ 262 144 byte (schema `maxLength` + kiểm lại);
  - roxmltree `nodes_limit` 10 000;
  - ≤ 200 node sinh ra mỗi op (`SVG produces 412 nodes; the limit is 200 per import_svg`);
  - path mỗi node ≤ 32 KiB (validator P14a);
  - tổng op sau mở rộng ≤ 500. Storage đã kiểm; message thêm "after import_svg expansion".

### 3. Chuyển đổi (`svg_import/parse.rs`, `svg_import/geometry.rs`)

**Phần tử**

| Nhóm | Xử lý |
| --- | --- |
| `svg` (gốc, namespace SVG hoặc không namespace), `g` | Duyệt vào. `transform` nhân dồn; `fill`, `fill-rule`, `fill-opacity` kế thừa; `opacity` theo luật group ở trên. |
| `path` | Dùng `d` (validator P14a). |
| `rect` (có/không `rx`/`ry`) | Đổi sang path như prototype `shape_d`, theo luật SVG 2: `rx`/`ry` tự suy và kẹp nửa cạnh. |
| `circle`, `ellipse` | 2 cung `A` |
| `polygon`, `polyline` | `M…L…(Z)` |
| `title`, `desc`, `metadata` | Bỏ qua. |
| `defs` | Bỏ qua nếu không được tham chiếu. |
| `line`; phần tử có `fill="none"` không stroke; `display="none"`; `visibility="hidden"` | Bỏ qua, vì không vẽ gì. |

Mọi phần tử khác bị **từ chối**, mỗi loại một lời khuyên:
- `<text>`: "convert text to outlines";
- `<style>`: "inline fills as attributes; CSS is not read";
- `<linearGradient>`/`<radialGradient>`/`<pattern>` khi được `fill="url(#…)"` tham chiếu: "gradient fills are not imported yet";
- `use`, `symbol`, `image`, `foreignObject`, `script`, `a`, `switch`, `mask`, `filter`, `marker`, `<svg>` lồng.

**Thuộc tính và CSS trong `style`**

- Đọc: `fill`, `fill-rule`, `fill-opacity`, `opacity`, `stroke`, `display`, `visibility`, `transform` và hình học của từng phần tử. Trong `style=""`, khai báo thắng thuộc tính, như CSS.
- Từ chối:
  - `on*` (mọi event handler);
  - `href`/`xlink:href` ở bất kỳ đâu;
  - `mask`, `filter`, `marker-*`;
  - `clip-path` khác "một `<rect>` phủ toàn viewBox". Dạng phủ toàn viewBox là no-op vì vector đã clip theo bounds, đúng boilerplate Figma;
  - `stroke` khác `none`: "stroke is not imported yet; remove it or wait for vector stroke (P14c)".
- Bỏ qua thuộc tính lạ còn lại (`class`, `id`, `data-*`, `aria-*`, `xml:space`, `version` …), vì chúng không đổi pixel khi không có CSS.

**Màu**

- `svgtypes::Color` nhận tên màu, `rgb()`, `hsl()`, `#rgb`.
- Ghi ra `#rrggbb`, hoặc `#rrggbbaa` khi alpha < 255.
- `currentColor` và fill chưa khai báo → `color` (mặc định `#000000`).

**Transform**

- Parse bằng `svgtypes::Transform`.
- Phần tử có transform tích luỹ là đơn vị: giữ `d` nguyên văn (round-trip, ít token).
- Ngược lại: chuẩn hoá sang lệnh tuyệt đối `M L C Q A Z` (H/V → L, S/T → C/Q phản chiếu, tương đối → tuyệt đối), nhân điểm qua ma trận. **Cung giữ là cung** (công thức ở Bằng chứng; prototype `bake_arcs` ~45 dòng).
- Số in tối đa 3 chữ số thập phân, bỏ 0 thừa.

**Lỗi** (VALIDATION_FAILED, `opIndex` gốc, `field:"svg"`, message có thẻ và dòng):
- `Unsupported element <text> at line 4: convert text to outlines; import_svg accepts svg, g, path, rect, circle, ellipse, polygon, polyline`;
- `Invalid SVG XML at 3:17: …`;
- `DOCTYPE and entities are not accepted`;
- `Root element must be <svg>`;
- `Give the <svg> a viewBox or numeric width and height`.

### 4. Tài liệu và skill

- `author-frame.md`:
  > Icons/logos from SVG: `apply_batch` op `import_svg` with the SVG text and `color:{tokenId}`. Single-path icons can also be written directly as `vector`. Never rasterize SVG to PNG.
- `docs/SERVER.md`: mục op `import_svg` (bảng phần tử, giới hạn, quy tắc màu).
- Mô tả tool `apply_batch` (`schemas.rs:131`) thêm một câu:
  > import_svg expands SVG text into vector nodes (fills only; no script, CSS, text, images or external references).

## Phạm vi file

| File | Việc | Dòng |
| --- | --- | --- |
| `crates/server/src/svg_import/mod.rs` (mới) | `expand`, bảng index, đọc field op, kiểm giới hạn, dựng `create_node` | ~120 |
| `crates/server/src/svg_import/parse.rs` (mới) | parse roxmltree, duyệt phần tử, cascade fill/opacity/style, danh sách cấm, `clip-path` no-op, màu | ~190 |
| `crates/server/src/svg_import/geometry.rs` (mới) | shape → `d`, ma trận, chuẩn hoá tuyệt đối, nướng cung, in số | ~170 |
| `crates/server/src/svg_import/tests.rs` (mới, `#[cfg(test)]`) | unit: bảng phần tử, transform, lỗi | ~180 |
| `crates/server/src/domain.rs` | gọi `expand`, ánh xạ `op_index` | +12 |
| `crates/server/src/schemas/operations.rs` | biến thể `import_svg` | 49 → ~56 |
| `crates/server/src/schemas.rs` | câu mô tả `apply_batch` | ±1 |
| `crates/server/Cargo.toml`, `Cargo.lock` | `roxmltree = "0.20"`, `svgtypes = "0.15"` | +2 |
| `crates/server/tests/import_svg.rs` (mới) | tích hợp MCP AC1–AC8 | ~190 |
| `crates/server/tests/fixtures/svg/*.svg` (mới) | 6 file nhỏ: material, figma, transforms, multicolor, 2 file độc hại | data |
| `spikes/pipeline/import-svg.test.mjs` (mới) | AC9 pixel (oracle SVG image) | ~120 |
| `crates/server/skills/author-frame.md`, `docs/SERVER.md` | tài liệu | ~25 |

`parse.rs` gần 200 dòng: nếu vượt thì tách `svg_import/style.rs` (cascade + màu).

## Tương thích MCP

- `tools/list`: `apply_batch.operations.items.oneOf` có thêm biến thể `import_svg`; mô tả `apply_batch` thêm một câu. Không tool mới, không field cũ đổi nghĩa.
- Response `apply_batch` không đổi shape. `changedIds` có thêm id sinh ra.
- Client cũ không gửi `import_svg` thì không thấy gì khác.
- History, `get_changes` và bundle chỉ chứa `create_node`, nên binary không có P14b vẫn đọc được project (chỉ cần P14a cho kind `vector`).
- Không cho phép `image/svg+xml` trong `import_asset` (giữ test `crates/storage/tests/assets.rs:44`).

## AC

1. **Icon Material, một batch:** 33 icon QueryNest (`icons.tsx:2-34`, path weight 500), mỗi icon gói thành `<svg xmlns viewBox="0 -960 960 960" width="24" height="24"><path d="…" fill="currentColor"/></svg>` và gửi `color:{tokenId:"icon-fg"}`, `props:{width:18,height:18}`, trong **một** `apply_batch` 33 op:
   - một commit (revision +1), `changedIds` có đúng 33 id;
   - mỗi node `kind:"vector"`, `props.path` **bằng byte** với `d` nguồn, `viewBox:[0,-960,960,960]`, `fill:{tokenId:"icon-fg"}`.
2. **Mặc định:** file Material dạng Google Fonts (không `fill`, `width/height="24"`), không gửi `color`/`props` → 1 vector `width:24, height:24, fill:"#000000"`.
3. **Nhiều phần tử, Figma:** `figma-export.svg` (`clip-path` no-op, path `#344054`, `rect rx=1.5` `#F04438`) → `group` `ic` 24×24 + `ic-1`, `ic-2` (`width/height:"fill"`, `viewBox:[0,0,24,24]`, đúng fill hex). `ic-2.path` chứa lệnh `A`.
4. **Transform:** fixture = `transforms.svg` của prototype cộng một `<g opacity=".5">` bọc 2 `<rect>`:
   - phần prototype ra 6 vector như đã đo;
   - circle `fill-opacity .5` → `opacity:0.5`; polygon `opacity .8` → `0.8`; path evenodd → `fillRule:"evenodd"`;
   - path của ellipse xoay chỉ có lệnh `M`, `A`, `Z` (cung được giữ);
   - `<g opacity=".5">` bọc 2 rect → group con `opacity:0.5` với 2 vector.
5. **Bảo mật**, mỗi file một batch, đều VALIDATION_FAILED, `opIndex` đúng, field `svg`, message nêu thẻ/thuộc tính và dòng, revision không đổi. Các ca: `<script>`; `onload` trên `<svg>`; `onclick` trên `<g>`; `<image xlink:href="http://…">`; `<image href="data:…">`; `<use href="#a">`; `<a href="javascript:…">`; DOCTYPE có ENTITY; `<foreignObject>`; `<style>`; `fill="url(#g)"`; `mask`; `filter`; `clip-path` hình tròn; SVG 262 145 byte; SVG sinh 201 node.
6. **Ánh xạ lỗi:**
   - batch `[create_node frame OK, import_svg <script>, create_node]` → `opIndex: 1`;
   - batch `[import_svg (3 node sinh) với parentId không tồn tại, create_node]` → `opIndex: 0` (không phải 0..2), field `parentId`;
   - `import_svg` có `d="M0 0 L5"` → field `path`, message có `character`.
7. **Idempotency:** replay y nguyên body AC1 → `replayed:true`, cùng `committedRevision`, không commit mới. Cùng key mà `svg` khác → lỗi idempotency hiện có.
8. **Stroke:** icon Lucide `search` (`stroke="currentColor" stroke-width="2" fill="none"`) → lỗi `stroke is not imported yet`.
9. **Pixel** (`spikes/pipeline/import-svg.test.mjs`): import qua MCP trên server tạm, `render_frame` scale 1 và 2, so với oracle là chính SVG gốc rasterize như ảnh ở cùng vị trí/kích thước:
   - Material, Figma: **max kênh ≤ 1** (prototype 0);
   - `transforms.svg`: max ≤ 32 **và** ≤ 0,5 % pixel mực lệch > 16 (prototype 18–22 và 0,03–0,11 %).
10. **Kích thước:** response `apply_batch` của AC1 ≤ 2 500 byte. Chuyển đổi 33 icon không transform tạo 0 byte path mới so với nguồn (path giữ nguyên).
11. **Modular:** mỗi file mới ≤ 200 dòng (trừ fixture); báo `wc -l`.

## Cách kiểm

- `CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-server svg_import` (unit: bảng phần tử, nướng cung so với ellipse giải tích ≤ 1e-6, lỗi).
- `cargo test --offline -p opencanvas-server --test import_svg` (AC1–8, AC10; fixture 33 path lấy từ `spikes/pipeline/vector-fixture.mjs` của P14a, hoặc bản JSON chung).
- `npm run build:renderer && cargo build --offline -p opencanvas-layout -p opencanvas-server && node --test spikes/pipeline/import-svg.test.mjs` (AC9).
- Kiểm live trên server tạm `--port 0 --data-root $(mktemp -d)`:
  - gửi `transforms.svg` và một SVG logo nhiều màu qua MCP, mở viewer zoom 400 %;
  - gửi file `<script>` và xác nhận lỗi;
  - dọn process và data.

## Rủi ro

- **Lệch AA với primitive bị biến đổi** (≤ 22/255, ~0,1 % pixel mép): Skia vẽ RRect/oval khác path. Đã đo, AC dùng ngưỡng. Không biến đổi thì 0.
- **SVG thực tế đa dạng.** Illustrator dùng `<style>` + class; nhiều icon set dùng stroke; logo có gradient. P14b từ chối rõ ràng chứ không vẽ sai. Tỉ lệ nhận phụ thuộc nguồn: Material/Figma fill-only nhận hết; Lucide/Tabler chờ P14c.
- **Ánh xạ `op_index`:** sai ánh xạ thì agent sửa nhầm op. AC6 khoá hành vi.
- **Bề mặt parser XML:** roxmltree không I/O, DTD tắt, giới hạn node. Server không fetch và không lưu XML; output chỉ là chuỗi path đã validate.
- **File nóng:** `domain.rs`, `schemas/operations.rs` (P02a đã thêm op note; P06a/P07a có thể thêm). Rebase nhẹ, vì chỉ thêm một biến thể và một lời gọi.

## Ngoài phạm vi / phase sau

- Stroke (P14c, sau P04a/P14a).
- `<style>` với selector class/type đơn giản.
- Gradient (cần P09a + gradient trên vector).
- `<use>`/`<symbol>` nội bộ (mở rộng thành bản sao).
- `<text>` → outline.
- Importer HTML/DOM đầy đủ: dùng `import_svg` cho phần icon; phần DOM là đề xuất riêng.
- Export ngược vector → SVG thuộc P03b (xem P14a).
