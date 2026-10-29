# P12b — Variable font: import một file, chọn `wght` bằng `fontWeight`, `opsz` tự theo cỡ chữ
Ưu tiên: P2 · Kích cỡ: ~75 phút dev · Phụ thuộc: **P10c (bắt buộc**: prop `fontWeight` và `validate/font_face.rs`). P12a không bắt buộc, chỉ giúp dọn 18 mặt tĩnh cũ sau khi chuyển · Nguồn: note người dùng `/Users/luis/Documents/opencanvas.md:8,72` ("phải tự instance thành 18 file tĩnh bằng fontTools…")

Phạm vi hẹp có chủ đích: chỉ trục `wght` (qua `fontWeight` của P10c) và `opsz` tự động. Trục tuỳ ý (`wdth`, `GRAD`, `opsz` cố định) qua `fontVariationSettings` là P12c, xem cuối file. Không đổi renderer, scene version hay `tools/list` (ngoài ≤ 120 B mô tả).

## Vấn đề

- Mỗi mặt font là một record `(family, weight, style)` gắn một asset. Muốn dùng SF Pro 9 weight × 2 style, người dùng phải chạy fontTools sinh 18 file tĩnh rồi `import_asset` 18 lần.
- Có cách tắt nhưng không ai biết: import **cùng một file variable** nhiều lần với metadata weight khác nhau. Asset được khử trùng theo nội dung nên chỉ lưu một file, và Chrome tự đặt `wght` theo descriptor (spike bên dưới). Nhưng mỗi lần vẫn phải gửi lại toàn bộ base64: SFNS.ttf 8,33 MB tương đương 11,1 MB base64, nhân 18. `list_fonts` cũng vẫn có 18 record.
- Weight trung gian (550, 650) không có cách khai báo nếu chưa import riêng mặt đó.

## Bằng chứng

Code ở `night/integration@0847b4b`. P10c chưa merge (board dev: "sau P10b").

| Vị trí | Hiện trạng |
| --- | --- |
| `crates/storage/src/model.rs:80-92` | `Font {id, family, weight: u16, style, asset_id, …}`, mỗi record một weight. |
| `crates/storage/src/db.rs:10` | `fonts(id, data_json)`: record lưu dạng JSON. Thêm field optional thì **không cần migration**. |
| `crates/storage/src/assets.rs:57-58,93-121` | Asset id = sha256 nội dung. Font kiểm `weight` 100..900, không đọc gì trong bytes font. |
| `crates/server/src/schemas.rs:139` | Schema `import_asset.font.weight` cho phép 1..1000, storage lại chặn 100..900 (lệch sẵn, giữ nguyên). |
| `crates/server/src/jobs/input.rs:24-35` | Manifest font của scene: mỗi record một entry `{id, family, hash, url, weight, style}`, chọn theo `props.fontId`. |
| `crates/layout/src/util.rs:46-55` | `MeasureRequest.weight = font.weight` lấy từ manifest. Khoá đo đã gồm `font_id` + `weight`. |
| `renderer/resources.ts:79-92` | `FontFace(family, bytes, {weight: String(font.weight), style})`, xoá mặt cũ cùng family/weight/style. |
| `renderer/measurement.ts:16-42`, `renderer/paint.ts:6,31` | `ctx.font = "<style> <weight> <size>px \"<family>\""`. Đo và vẽ dùng cùng chuỗi, cỡ chữ là CSS px (scale nằm ở transform). |

**Fixture có sẵn.** `renderer/fonts/NotoSans.ttf` (font bundled, OFL) **là variable font**. Prototype parser `fvar` (JS, ~25 dòng, cùng thuật toán đề xuất cho Rust) đọc ra:

| File | Trục (min/default/max) |
| --- | --- |
| `renderer/fonts/NotoSans.ttf` | `wght` 100/400/900, `wdth` 62.5/100/100 |
| macOS `SFNS.ttf` | `wght` 1/400/1000, `opsz` 17/28/96, `wdth` 30/100/150, `GRAD` (ẩn) |
| macOS `SFNSItalic.ttf` | `wght` 1/400/1000, `opsz` 17/28/28 |
| macOS `Skia.ttf` | `wght` 0.48/1/3.2 (thang GX cũ, không theo CSS) |

**Spike Chrome** (HeadlessChrome 156, `/Applications/Google Chrome.app`, Playwright của repo; script ở scratchpad phiên, không ghi vào repo). Đo `measureText("Hamburgefonstiv 0123")` 40px với SFNS.ttf:

- FontFace `weight:"1 1000"`, `ctx.font` 100/400/650/900 → rộng 360,37 / 383,44 / 412,70 / 441,96. Ink alpha khi vẽ 100 vs 900: 528 056 vs 1 793 363. → **`wght` áp cả lúc đo lẫn lúc vẽ.**
- FontFace descriptor tĩnh `weight:"600"` + `ctx.font 600` → 406,85, **bằng** face dải + 600 (406,85). → Một FontFace mỗi weight với descriptor tĩnh là đủ, renderer không phải sửa.
- `opsz` tự động: rộng/px ở 10px = 10,699, ở 80px = 9,586. Face có descriptor `variationSettings:"'opsz' 28"` ở 10px = 9,586/px. → **Canvas áp `font-optical-sizing:auto`** theo cỡ CSS px, giống CSS mặc định của coder.
- `FontFace.variationSettings` được Chrome áp lên canvas (`'wght' 900` ở `ctx 400` → 120,36 so với 106,99). Đây là đường cho P12c.
- Skia không đổi theo weight vì dải `wght` 0.48–3.2 không thuộc thang CSS. Parser phải báo đúng dải để lỗi giải thích được.

**Tham khảo:** CSS Fonts 4 `@font-face font-weight: <min> <max>` và `font-optical-sizing` (https://www.w3.org/TR/css-fonts-4/#font-optical-sizing-def); OpenType `fvar` (https://learn.microsoft.com/typography/opentype/spec/fvar); Figma variable fonts (https://help.figma.com/hc/en-us/articles/5579502031511-Use-variable-fonts).

## Giải pháp

### 1. Import: server tự đọc `fvar`, agent không khai thêm gì

```json
{"mediaType":"font/ttf","base64":"…","font":{"id":"sf_text","family":"SF Pro Text","weight":400,"style":"normal"}}
```

- Module mới `crates/storage/src/font_axes.rs`: `parse_axes(&[u8]) -> Result<Option<BTreeMap<String, FontAxis>>, &'static str>`.
  - Chỉ nhận sfnt `0x00010000`, `OTTO`, `true`. `ttcf`/`wOFF`/`wOF2` → `None` (WOFF2 nén Brotli, xem Rủi ro). Không có `fvar` → `None`.
  - Đọc table directory, rồi header `fvar`: `axesArrayOffset`, `axisCount`, `axisSize ≥ 20`. Mỗi record: tag, min/default/max (Fixed 16.16, làm tròn 1/1000).
  - **Bỏ trục ẩn** (`flags & 1`). Kiểm biên mọi offset; sai → `Err("fvar table is malformed")`.
- `Font` thêm `axes: Option<BTreeMap<String, FontAxis {min, default, max}>>`, `#[serde(default, skip_serializing_if = "Option::is_none")]`. Font tĩnh và dữ liệu cũ giữ nguyên byte.
- `assets.rs`: khi có `font`, gọi `parse_axes`.
  - Có `wght` thì `font.weight` là **weight mặc định** của record và phải nằm trong dải. Ngoài dải → `field:"font.weight"`, `Weight 950 is outside this variable font's wght axis 100..900`.
  - `fvar` hỏng → `field:"base64"`.
- `list_fonts` / `fonts` của `get_design_context` tự có `axes`, ví dụ `"axes":{"wght":{"min":100,"default":400,"max":900},"wdth":{"min":62.5,"default":100,"max":100}}`. Font tĩnh không có key này nên không tốn token.

### 2. Chọn mặt: mở rộng `select_face` của P10c

Đích `(family, weight, style)` như P10c. Ứng viên gồm:
- (a) record tĩnh khớp chính xác;
- (b) record **có `axes.wght`** cùng family + style, có `min ≤ weight ≤ max`.

Thứ tự chọn:
1. Mặt gốc nằm trong ứng viên → giữ mặt gốc.
2. Đúng một (a) → chọn (a). Mặt tĩnh thắng variable vì là file người dùng chủ ý import.
3. Không có (a) và đúng một (b) → chọn (b).
4. Còn lại → lỗi trùng của P10c.

Message "không có mặt" liệt kê cả dải: `No "SF Pro Text" 1100 normal font; imported faces: 400 normal, 1..1000 normal (variable)`.

`fontWeight` vẫn là số nguyên trong miền hợp lệ của P10c (100..900). `resolvedProps.fontId` = record variable, `fontWeight` giữ như đã merge.

### 3. Scene: một "instance" mỗi `(record variable, weight)`, renderer không đổi

Module mới `crates/server/src/jobs/fonts.rs` (chuyển `input.rs:24-35` vào đây và mở rộng):
- Với text có record mang `axes.wght` và `fontWeight` đã resolve **khác** `record.weight`: đặt `props.fontId` **trong input scene** thành `"<id>@<weight>"`.
- Thêm entry manifest `{"id":"sf_text@650","family":…,"hash":…,"url":<cùng asset>,"weight":650,"style":…}`.
- Text không dùng `fontWeight`, hoặc dùng đúng weight mặc định, giữ `fontId` gốc. Scene hiện có byte-identical.
- Layout (`util.rs:52`) lấy `weight` 650 từ manifest. Renderer đăng ký `FontFace(family, bytes, {weight:"650"})`: spike cho thấy Chrome đặt `wght=650`. Khoá đo khác nhau theo weight, `opsz` theo cỡ chữ tự động ở cả đo lẫn vẽ.
- Snapshot và `resolvedProps` không thấy id `@`; chỉ scene/`inspect_layout` thấy. Ghi trong `docs/SERVER.md`.

### 4. Tài liệu cho agent

- Mô tả `import_asset` thêm một câu (≤ 120 B): `A TTF/OTF variable font is read for fvar axes: import it once; weight is its default; text picks weights with fontWeight.`
- `author-frame.md`: import một file variable cho mỗi style; đổi đậm bằng `fontWeight`; `opsz` tự theo `fontSize`.
- `implement-frame.md`: record có `axes.wght` → `@font-face { font-weight: <min> <max> }`. Để `font-optical-sizing` mặc định (`auto`) để khớp render.

### Vì sao chọn như vậy (phương án đã loại)

| Phương án | Lý do loại |
| --- | --- |
| Agent khai `axes` trong metadata import | Dễ sai dải, lại tốn token. Đọc `fvar` chỉ ~80 dòng Rust, không cần crate. |
| Renderer nhận dải weight (`FontFace weight:"100 900"`, scene node mang `fontWeight`) | Phải sửa `resources.ts`, `measurement.ts`, `paint.ts`, layout và bump renderer/scene version, làm mất cache render. Instance phía server cho cùng pixel (spike: 406,85 = 406,85) mà không đụng renderer. |
| Server tự instance ra file tĩnh (như fontTools) | Phải nội suy `gvar`/`CFF2`, quá lớn. |
| Giữ cách import N lần | N × upload 11 MB, N record trong `list_fonts`, không có weight trung gian. |

Không thêm tool: chỉ mở rộng `import_asset`, `list_fonts` và luật chọn mặt của P10c.

## Phạm vi file

| File | Việc | Dòng (hiện → ước) |
| --- | --- | --- |
| `crates/storage/src/font_axes.rs` (mới) | `parse_axes` + unit test sfnt tổng hợp (không `fvar`, `fvar` hỏng, trục ẩn) | ~130 |
| `crates/storage/src/model.rs` | `FontAxis`, `Font.axes` | 132 → ~142 |
| `crates/storage/src/assets.rs` | gọi `parse_axes`, kiểm `weight` trong dải | 213 → ~226 |
| `crates/storage/src/validate/font_face.rs` (của P10c) | ứng viên (b), message dải | ~120 → ~150 |
| `crates/server/src/jobs/fonts.rs` (mới) | manifest + instance `@weight` (tách từ `input.rs`) | ~70 |
| `crates/server/src/jobs/input.rs` | gọi `fonts::scene_fonts` | 87 → ~75 |
| `crates/server/src/schemas.rs` | 1 câu mô tả `import_asset` | +0 dòng |
| `crates/storage/tests/variable_font.rs` (mới) | AC1–AC4 | ~140 |
| `crates/server/tests/variable_font_scene.rs` (mới) | AC5–AC6 | ~120 |
| `docs/SERVER.md`, `docs/RESOLUTION.md`, 2 skill | vài dòng | — |

Không đụng `renderer/`, `worker/`, `crates/layout`, `mcp.rs`. Không file nào vượt 250 dòng.

## Tương thích MCP

- `tools/list`: chỉ mô tả `import_asset` dài thêm ≤ 120 B. Schema không đổi, test `tools_budget.rs` vẫn đạt.
- Font cũ (kể cả NotoSans đã import trước P12b) không có `axes` nên chạy y như cũ. Muốn bật variable thì import lại theo cùng id (P12a chính thức hoá) hoặc import record mới. Không backfill tự động. `font_bundled` giữ nguyên.
- `list_fonts` thêm key `axes` chỉ cho record variable. Text/scene không dùng P10c/P12b byte-identical.
- Bundle (`docs/BACKUP.md`): `axes` nằm trong `data_json`. Replay commit `import_asset` (`bundle_history.rs:84-95`) đọc lại bytes nên ra cùng `axes`.

## AC

Fixture: bytes `renderer/fonts/NotoSans.ttf` (variable, OFL, đã có trong repo).

1. **Parse** (`font_axes.rs`): NotoSans → `{"wght":{min:100,default:400,max:900},"wdth":{min:62.5,default:100,max:100}}`. Sfnt tổng hợp không có `fvar` → `None`. Sfnt có `fvar` mà `axisCount` vượt `length` → `Err`. Trục có `flags=1` bị bỏ. Bytes `wOF2…` → `None`.
2. **Import** (`variable_font.rs`):
   - `import_asset` NotoSans `{id:"nv", family:"Var Sans", weight:400}` → `list_fonts` có `nv.axes.wght` 100..900.
   - `weight:950` → `field:"font.weight"`, message chứa `100..900`.
   - Font có `axes` vẫn đi qua `persistence` → reload project → `axes` giữ nguyên.
3. **Chọn mặt:**
   - Chỉ `nv`: `{fontFamily:"Var Sans", fontWeight:650}` → `fontId "nv"`. `{fontId:"nv", fontWeight:300}` → `nv`.
   - Thêm `s600` tĩnh "Var Sans" 600: `{fontFamily:"Var Sans", fontWeight:600}` → `s600`. `{fontId:"nv", fontWeight:600}` → `nv` (mặt gốc thắng).
   - `{fontFamily:"Var Sans", fontStyle:"italic"}` → lỗi P10c, message chứa `100..900 normal (variable)`.
4. **Không đổi:** text chỉ có `fontId` (kể cả `nv`) → `resolved_nodes_for_theme` và `make_input` byte-identical so với trước P12b. Toàn bộ test storage/server hiện có pass.
5. **Scene** (`variable_font_scene.rs`):
   - Text `{fontId:"nv", fontWeight:650}` → input scene có `fonts` chứa `{id:"nv@650", weight:650, url:<asset nv>}`, node `fontId "nv@650"`.
   - Text `{fontId:"nv"}` cùng frame → entry `nv` weight 400.
   - Hai text cùng 650 chỉ sinh **một** entry `nv@650`.
6. **Pixel thật** (job render, worker Chrome như `tests/wrap.rs`):
   - Hai text hug "Hamburgefonstiv" 40px, `fontWeight` 400 và 800 → `render_frame` `ready`, `diagnostics:[]`.
   - `inspect_layout` width(800) ≥ 1,03 × width(400).
   - `get_design_context`: `resolvedProps.fontId == "nv"`, có `fontWeight:800`.
7. **Budget:** `tools_budget.rs` pass. `serde_json::to_vec(tools/list).len()` tăng ≤ 120 B so với trước (ghi số đo vào report).
8. **Live** trên server tạm `--port 0 --data-root $(mktemp -d)`:
   - `import_asset` NotoSans 1 lần → `apply_batch` 3 text 300/500/800 → `render_frame` ready. `list_fonts` có đúng 1 record + `axes`.
   - Nếu máy có `/System/Library/Fonts/SFNS.ttf` (không commit): import 1 lần, text 12px và 48px cùng chuỗi → width/fontSize lệch ≥ 3% (opsz auto áp nhất quán đo ↔ vẽ).
   - Dọn process và data.
9. **Modular:** file mới ≤ 150 dòng; `cargo fmt --check`, clippy `-D warnings` pass.

## Cách kiểm

```bash
W=<worktree P12b>; cd $W && export CARGO_BUILD_JOBS=4
cargo test --offline -p opencanvas-storage -p opencanvas-server
cargo test --offline -p opencanvas-server --test variable_font_scene
cargo clippy --offline -p opencanvas-storage -p opencanvas-server --lib --bin opencanvas-server -- -D warnings
cargo fmt --all --check
node .night/qa/mcp-tools.mjs   # so byte tools/list trước/sau
```

Live: `target/debug/opencanvas-server --port 0 --data-root $(mktemp -d)`, script MCP HTTP gọi `import_asset` → `apply_batch` → `render_frame` → `inspect_layout`. Không đụng 3847, container `opencanvas` hay `~/.opencanvas`.

## Rủi ro

- **WOFF2 variable** không được đọc trục: bảng nằm trong luồng Brotli và repo chưa có crate brotli. Import vẫn chạy như record tĩnh (như hôm nay). Ghi trong mô tả/skill: "variable cần TTF/OTF". Phase sau có thể thêm brotli.
- **Bộ nhớ renderer:** mỗi instance là một `FontFace` riêng từ cùng bytes (SFNS 8,3 MB × số weight dùng trong frame). Cũng nạp URL nhiều lần, vì `checkedBytes` không cache theo hash (`resources.ts:33,79`). Có thể memo theo hash trong một lần nạp (≈ 5 dòng renderer), nhưng ngoài phạm vi. Đo nếu frame có > 6 weight.
- **Va chạm mặt cùng family/weight/style:** `resources.ts:84-91` xoá mặt cũ. Node A chọn `s600` tĩnh và node B ép `{fontId:"nv", fontWeight:600}` trong cùng frame → instance `nv@600` và `s600` đè nhau. Hiếm; giống rủi ro trùng mặt P12a đã nêu. Diagnostic thuộc P11.
- **`opsz` auto** phụ thuộc Chrome áp theo CSS px. Spike khẳng định ở Chrome 156; renderer đổi cách scale font (nhân cỡ chữ thay vì transform) sẽ làm lệch. AC8 bắt lỗi khi chạy live.
- **Dải ngoài thang CSS** (Skia 0.48–3.2): mọi `fontWeight` 100..900 nằm ngoài dải → lỗi liệt kê dải thật. Đúng nhưng font như vậy không dùng được qua `fontWeight`.
- **Phụ thuộc P10c:** nếu P10c đổi tên `select_face`/luật, phần 2 phải rebase. Nên giao P12b sau khi P10c merge.

## Ngoài phạm vi / phase sau

- **P12c — `fontVariationSettings`** (trục tuỳ ý `wdth`/`GRAD`/`opsz` cố định):
  - Prop text/style `{"fontVariationSettings":{"wdth":90}}`, kiểm từng tag theo `axes`.
  - Instance scene `"<id>@<weight>~<hash>"` với family alias riêng + descriptor `variationSettings` (spike đã xác nhận Chrome áp lên canvas).
  - Coder nhận CSS `font-variation-settings`.
- Trục `ital`/`slnt` thay cho `fontStyle`; đọc WOFF2 (Brotli); backfill `axes` cho font đã import, kể cả `font_bundled`.
- Memo bytes theo hash trong `renderer/resources.ts`.
- Mở `fontWeight` 1..1000 cho record variable (P10c đang giới hạn 100..900).
