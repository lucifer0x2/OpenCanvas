# P10c — Chọn mặt font theo `fontFamily` + `fontWeight` + `fontStyle` (italic, weight theo family)
Ưu tiên: P2 (cải tiến cho agent; người dùng nêu trong note "Thiếu tính năng → Text") · Kích cỡ: ~60 phút dev · Phụ thuộc: không về code. Nên sau P10b để text style nhận chung bộ key kiểu chữ: cùng sửa `validate/node.rs` và `props_catalog.rs`, rebase đơn giản. Liên quan P12 (xoá/thay font) cho trường hợp mặt font trùng · Nguồn: note "Thiếu tính năng → Text" (italic; "fontWeight theo family (hiện mỗi weight là một font record riêng)"), mục Kết quả ("comment SQL không nghiêng")

Phase c của P10. Độc lập với renderer: không đổi scene, không bump version.

## Vấn đề

- Text chọn font bằng `fontId` mờ đục, mỗi `(family, weight, style)` là một record.
- Muốn chữ đậm, agent phải tra `list_fonts` tìm ID của mặt 600 rồi đổi `fontId`. Không thể nói "giữ style Body, chỉ đậm hơn": text style mang `fontId` cố định, nên mỗi weight cần một text style riêng.
- Italic không có cách khai báo ngoài việc biết ID của mặt italic. Phiên QueryNest bỏ luôn italic cho comment SQL.
- Phía coder: `fontId:"font_sf2_600"` phải tra ngược ra `font-family`/`font-weight`. CSS và Figma (`fontName {family, style}`) đều nói theo family + weight + style.

## Bằng chứng

Code ở `night/integration@52a8715`.

| Vị trí | Hiện trạng |
| --- | --- |
| `crates/storage/src/model.rs:80-92` | `Font {id, family, weight, style, asset_id, …}`: mỗi record một mặt. |
| `crates/storage/src/assets.rs:93-121` | `import_asset` đăng ký font với `family` ≤ 128 byte, `weight` 100..900, `style` `normal`\|`italic` (`:99-100`). Không chặn hai record trùng `(family, weight, style)`: `rg family crates/storage/src/validate*` không ra kết quả. |
| `crates/storage/src/props_catalog.rs:86` | `fontId` bắt buộc với text, alias `font`, `fontFamily`, `family`, `fontName`. Agent gõ `fontFamily` hiện bị bảo dùng `fontId`. |
| `crates/storage/src/validate/prop_keys.rs:42-54` | thiếu `fontId` → `Text requires explicit text, fontId, fontSize and lineHeight (or a text style)`. |
| `crates/storage/src/validate/node.rs:20-53` | text style nhận `fontId`, `fill`, `fontSize`, `lineHeight`. |
| `crates/storage/src/validate/refs.rs:59-112` | `resolve_props` merge style rồi authored, resolve token. Đây là chỗ duy nhất cần chọn mặt font: kết quả đi vào layout input (`crates/server/src/jobs/input.rs:10`) và `resolvedProps` (`crates/server/src/reads/tree.rs:40-45,75`). |
| `crates/server/src/reads/compact.rs:33-37,82` | `resources:"referenced"` gom font qua `fontId` trong **authored** props. Mặt được chọn theo weight sẽ thiếu nếu không sửa. |
| `renderer/resources.ts:80-83` | mỗi record đăng ký `FontFace(family, bytes, {weight, style})`, nên Chrome đã hiểu nhiều mặt cùng family. |
| `renderer/measurement.ts:16-17`, `renderer/drawing.ts:13-14` | `font = "<style> <weight> <size>px \"<family>\""` lấy từ record. |

Hệ quả: chỉ cần storage đổi `fontId` đã resolve; layout, đo và vẽ chạy nguyên trạng.

**Dữ liệu thật:** project QueryNest có 18 `font_sf_*` cũ và 18 `font_sf2_*` mới cùng family/weight (note #11). Phải có luật khi trùng.

**Tham khảo**
- CSS `font-family` / `font-weight` (1..1000) / `font-style` (`normal|italic|oblique`).
- Figma `fontName: {family, style}` (style là tên như "Semibold Italic"), `fontWeight` chỉ đọc (https://developers.figma.com/docs/plugins/api/TextNode/#fontname).
- Penpot text: `fontFamily`, `fontWeight`, `fontStyle` (https://doc.plugins.penpot.app/interfaces/Text).

## Giải pháp

### 1. Prop authored

```json
{"text":"Delete","fontFamily":"SF Pro Text","fontWeight":600,"fontSize":13,"lineHeight":18}
{"styleId":"body","fontWeight":600}
{"text":"-- comment","fontId":"font_sf_mono_400","fontStyle":"italic"}
```

| Key | Giá trị | Ghi chú |
| --- | --- | --- |
| `fontFamily` | chuỗi = `family` của một font đã import (`list_fonts`) | thay cho `fontId`, không dùng cùng `fontId` trên **cùng node** |
| `fontWeight` | số nguyên 100..900 | mặc định: weight của mặt gốc, hoặc 400 |
| `fontStyle` | `"normal"` \| `"italic"` | mặc định: style của mặt gốc, hoặc `"normal"` |

- Scope text. Text style cũng được mang cả ba key (style không được có đồng thời `fontId` và `fontFamily`).
- Catalog: bỏ `fontFamily` khỏi alias của `fontId`. `fontId` hết bắt buộc trên catalog; luật mới là "text cần `fontId` **hoặc** `fontFamily`, trực tiếp hoặc qua style".
- Alias:
  - `fontWeight`: `weight`, `font-weight`, `bold`;
  - `fontStyle`: `italic`, `font-style`, `slant`;
  - `fontFamily`: `family`, `font-family`, `typeface`.

### 2. Chọn mặt (storage, trong `resolve_props`, module mới `validate/font_face.rs`)

Sau khi merge style + authored:

1. Nếu không có `fontFamily`, `fontWeight`, `fontStyle`: không làm gì. Dữ liệu cũ giữ nguyên byte.
2. **Mặt gốc:**
   - node authored `fontFamily` → family đó;
   - node authored `fontId` → record đó;
   - ngược lại lấy từ style (`fontFamily` hoặc `fontId`).
   - Node authored cả hai → lỗi `field:"props.fontFamily"`, `Use fontId or fontFamily on a text node, not both`.
3. **Đích** = `(family, fontWeight ?? gốc.weight ?? 400, fontStyle ?? gốc.style ?? "normal")`.
4. Tìm record khớp **chính xác**:
   - đúng một → `fontId` đã resolve = record đó;
   - nhiều và mặt gốc nằm trong số đó → giữ mặt gốc;
   - nhiều, không có mặt gốc → lỗi trùng: `Fonts font_sf_600 and font_sf2_600 are both "SF Pro Text" 600 normal; author fontId of the one you want`. P12 sẽ dọn trùng;
   - không có → lỗi kèm danh sách mặt có thật: `No "SF Pro Text" 600 italic font; imported faces: 400 normal, 600 normal. Import the face with import_asset font {family, weight, style}` (field `fontWeight` hoặc `fontStyle`, tuỳ key lệch);
   - family không tồn tại → `field:"fontFamily"`, liệt kê tối đa 10 family, `suggestion` = family trùng không phân biệt hoa thường nếu có.
5. `resolvedProps` có `fontId` của mặt đã chọn. `fontFamily`/`fontWeight`/`fontStyle` giữ như đã merge. Không chèn thêm key cho text không dùng P10c, để compact `differing` không phình.

**Không tự tổng hợp** đậm/nghiêng (CSS `font-synthesis`), không tìm weight gần nhất theo thuật toán CSS:
- PNG phải bằng thứ coder có khi nạp đúng các mặt đã khai báo.
- Lỗi chỉ đúng file cần import. Note người dùng cho thấy fallback âm thầm (#12) là thứ tốn thời gian nhất.

### 3. Đọc

- `Referenced::collect` (`compact.rs`) thêm `fontId` của resolved props (tree.rs đã có `resolved` ở `:40-45`). `fonts` trong compact `referenced` có đúng mặt được chọn.
- `skill://opencanvas/props.md` (sinh từ catalog) có 3 dòng mới và ghi chú luật chọn.
- `author-frame.md`: "giữ một text style cho mỗi vai trò, đổi đậm bằng `fontWeight`; italic cần import mặt italic".
- `implement-frame.md`: map `fontFamily`/`fontWeight`/`fontStyle` → CSS; với `fontId` lấy family/weight/style từ `fonts`.

### Vì sao chọn như vậy

- Ba key tên CSS, giá trị CSS. Agent gõ đúng ngay, coder map 1:1. `fontId` vẫn chạy, không đổi nghĩa.
- Chọn mặt ở storage (một chỗ) thay vì layout/renderer:
  - không đổi scene, khoá đo hay version;
  - viewer cũ không cần reload;
  - mọi đường đọc (layout input, `resolvedProps`) đã đi qua `resolve_props`.
- Không đổi tên field `fontId` → không cần migration.

**Phương án đã loại**

| Phương án | Lý do loại |
| --- | --- |
| Figma `fontName {family, style:"Semibold Italic"}` | Chuỗi style tự do khác nhau theo font; CSS không dùng. |
| Gộp record thành "family có nhiều mặt" (đổi schema `fonts`) | Cần migration và đổi `list_fonts`/`import_asset`. Lợi ích như trên nhưng tốn hơn nhiều. |
| Tổng hợp italic/bold khi thiếu mặt | Lệch pixel so với code thật nạp đúng font; che lỗi thiếu file. |

## Phạm vi file

| File | Việc | Dòng (hiện → ước) |
| --- | --- | --- |
| `crates/storage/src/validate/font_face.rs` (mới) | `select_face(candidate, merged, authored) -> Result<Option<String>>` + message lỗi | ~120 |
| `crates/storage/src/validate/refs.rs` | gọi `select_face` sau merge, ghi `fontId` | 156 → ~162 |
| `crates/storage/src/validate/node.rs` | text style nhận 3 key; cấm `fontId` + `fontFamily` cùng style | ~112 → ~122 |
| `crates/storage/src/validate/properties.rs` | kiểm kiểu `fontWeight` (nguyên 100..900), `fontStyle`, `fontFamily` (chuỗi ≤ 128) | ~126 → ~132 |
| `crates/storage/src/props_catalog.rs`, `validate/prop_keys.rs` | 3 entry; `fontId` không còn `required`, luật "fontId hoặc fontFamily" trong `check_required` (message mới) | +3, +6 |
| `crates/server/src/reads/compact.rs`, `reads/tree.rs` | gom `fontId` đã resolve | 134 → ~140, +2 |
| `crates/storage/tests/font_face.rs` (mới) | AC1–AC4 | ~150 |
| `crates/server/tests/font_face_context.rs` (mới) | AC5–AC6 qua MCP + job | ~100 |
| `crates/server/src/props_doc.rs`, 2 skill, `docs/RESOLUTION.md` | ghi chú luật chọn mặt | vài dòng |

- Không đụng `crates/layout`, `renderer/`, `worker/`, `mcp.rs`, `schemas.rs`.
- File mới ≤ 150 dòng; không file sửa nào vượt 200.

## Tương thích MCP

- `tools/list` byte-identical; mô tả `list_fonts` giữ nguyên, hướng dẫn nằm ở props.md và skill.
- Dữ liệu cũ (chỉ `fontId`) validate, resolve và render y hệt: bước 1 không làm gì.
- `allowedKeys` text thêm 3 key.
- `fontId` chuyển từ "required" sang "fontId or fontFamily" trong props.md. Text thiếu cả hai vẫn bị từ chối, cùng `field:"fontId"`. Message mới nhắc `fontFamily`.
- Alias đổi: agent gõ `fontFamily` trước đây được gợi ý `fontId`, nay là key hợp lệ. Thêm khả năng, không đổi nghĩa.

## AC

Fixture: import NotoSans bundled bytes 4 lần bằng `import_asset` với `font` (cùng bytes, khác metadata):
- `f400` "Test Sans" 400 normal;
- `f600` "Test Sans" 600 normal;
- `f700i` "Test Sans" 700 italic;
- `dup600` "Test Sans" 600 normal (trùng `f600`) — chỉ ở AC3.

**AC1 — Chọn đúng mặt** (`crates/storage/tests/font_face.rs`, `resolved_nodes_for_theme`):
- `{fontId:"f400", fontWeight:600}` → `fontId == "f600"`.
- `{fontFamily:"Test Sans", fontWeight:700, fontStyle:"italic"}` → `f700i`.
- `{fontFamily:"Test Sans"}` → `f400` (mặc định 400 normal).
- Style `body {fontId:"f400", fontSize:14, lineHeight:20}` + node `{styleId:"body", fontWeight:600}` → `f600`.
- Style `{fontFamily:"Test Sans", fontStyle:"italic", fontWeight:700, …}` + node không có font key → `f700i`.
- Node `{styleId:"body", fontFamily:"Test Sans", fontWeight:700, fontStyle:"italic"}` → `f700i` (node thắng style).
- `{fontId:"f600", fontWeight:600}` → `f600`.

**AC2 — Từ chối** (batch nguyên tử, đúng field/message):
- `{fontFamily:"Test Sans", fontWeight:800}` → `field:"fontWeight"`, message chứa `imported faces: 400 normal, 600 normal, 700 italic`.
- `{fontId:"f400", fontStyle:"italic"}` → `field:"fontStyle"`, message chứa `Import the face`.
- `{fontFamily:"test sans"}` → `field:"fontFamily"`, `suggestion:"Test Sans"`.
- `{fontId:"f400", fontFamily:"Test Sans"}` → `field:"props.fontFamily"`, message chứa `not both`.
- `fontWeight:950`, `fontWeight:600.5` → `field:"fontWeight"`.
- `fontStyle:"oblique"` → `field:"fontStyle"`.
- Text không có `fontId` lẫn `fontFamily` (và style không có) → `field:"fontId"`, message chứa `fontFamily`.
- Text style có cả `fontId` và `fontFamily` → `field:"style.value"`.

**AC3 — Trùng mặt.** Có `dup600`:
- `{fontFamily:"Test Sans", fontWeight:600}` → lỗi chứa cả `f600` và `dup600`.
- `{fontId:"dup600", fontWeight:600}` → `dup600` (mặt gốc thắng).
- Node cũ chỉ có `fontId` vẫn commit được.

**AC4 — Không đổi khi không dùng.**
- Snapshot fixture hiện có (`crates/storage/tests`, `crates/server/tests`): `resolved_nodes_for_theme` byte-identical trước/sau.
- Toàn bộ test hiện có pass không sửa, trừ test khẳng định message thiếu `fontId` cũ (nếu có thì cập nhật chuỗi).

**AC5 — Job và scene** (`font_face_context.rs`, server tạm trong test như `tests/wrap.rs`):
- Node `{fontFamily:"Test Sans", fontWeight:700, fontStyle:"italic", …}` → `render_frame` `ready`.
- Scene node `fontId == "f700i"`; `scene.fonts` chứa `f700i` với `weight 700`, `style "italic"`.
- Đổi style `body.fontWeight` 600 → 400 bằng op `upsert_style`: node theo style đổi `fontId` ở revision mới.

**AC6 — Đọc qua MCP.**
- `get_design_context` cho node AC5: `props` nguyên văn (không có `fontId`), `resolvedProps.fontId == "f700i"`.
- `compact:true, resources:"referenced"`: `fonts` có đúng `f700i` (và mặt gốc nếu authored), không có `f400`/`f600` khi không tham chiếu.

**AC7 — Catalog/doc.**
- Test B02 `validator_accepts_exactly_the_catalog_keys_per_kind` pass.
- props.md dòng `text` có `fontFamily`, `fontWeight`, `fontStyle`; alias của `fontId` không còn `fontFamily`.
- `tools/list` byte-identical.

**AC8 — Live** trên server tạm:
- `import_asset` 2 lần NotoSans với `font` (`Live Sans` 400 normal, 700 italic) → `apply_batch` text `{fontFamily:"Live Sans", fontWeight:700, fontStyle:"italic"}` → `render_frame` `ready`, `diagnostics:[]`.
- `inspect_layout` có node đó.
- Dọn process và data.

**AC9 — Modular.** File mới ≤ 150 dòng; `cargo fmt --check`, clippy (lệnh ACCEPTANCE) pass.

## Cách kiểm

```bash
W=<worktree>; cd $W && export CARGO_BUILD_JOBS=4
cargo test --offline -p opencanvas-storage -p opencanvas-server
cargo clippy --offline -p opencanvas-storage -p opencanvas-server --lib --bin opencanvas-server -- -D warnings
cargo fmt --all --check
```

**Live:** `target/debug/opencanvas-server --port 0 --data-root $(mktemp -d)`, script node gọi MCP HTTP (`.night/qa/mcp-tools.mjs` cho `tools/list`). Không đụng 3847.

## Rủi ro

- **Font trùng trong project thật** (QueryNest có `font_sf_*` và `font_sf2_*`): agent dùng `fontFamily` + `fontWeight` gặp lỗi trùng cho tới khi P12 cho xoá/thay font. Lỗi chỉ đúng `fontId` cần dùng, nên agent vẫn tiến được.
- **Family đặt tên kiểu "SF Pro Text Semibold"** (một family mỗi weight): `fontWeight` không giúp gì; lỗi liệt kê mặt của family đó cho agent thấy. Ghi trong `author-frame.md`: import cùng `family` cho mọi weight.
- **Bỏ `fontFamily` khỏi alias của `fontId`:** client nào dựa vào gợi ý đó chỉ mất một gợi ý; key nay hợp lệ.
- **Đổi luật required:** phải giữ message và `field:"fontId"` khi thiếu cả hai để không phá test/agent hiện có. AC2 khoá điều này.

## Ngoài phạm vi / phase sau

- Variable font (trục `wght`/`opsz`/`wdth` trên một file): P12. Khi đó `fontWeight` chọn giá trị trục thay vì record.
- `oblique <angle>`, `font-stretch`.
- Thuật toán khớp weight gần nhất của CSS; `font-synthesis`.
- Xoá/thay/upsert font và dọn trùng (P12).
