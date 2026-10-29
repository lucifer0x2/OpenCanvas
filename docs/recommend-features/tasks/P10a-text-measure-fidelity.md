# P10a — Text đo như Chrome: hết tự vỡ chữ khi hug ("⌘C", note #4), baseline theo metrics font (#5), width không bị hinting số nguyên (#13), prop `textWrap`
Ưu tiên: P1 (sửa lỗi độ trung thực, chạm mọi text; nên làm trước P10b, P11) · Kích cỡ: ~75 phút dev · Phụ thuộc: không bắt buộc (B02 đã merge: dùng `props_catalog`). Xếp version với P01/P04a/P05 (cùng bump `RENDERER_VERSION`); P11 bỏ #13 và "⌘C" khỏi phạm vi vì đã ở đây · Nguồn: note #4, #5, #13; phát hiện của phiên P08 (`P08a-stack-align-padding.md:386-390`)

Phase a của P10. Phase b (`P10b-text-style-props.md`) thêm `letterSpacing`, `textAlign`, `textDecoration`, `textTransform`. Phase c (`P10c-font-family-weight.md`) chọn mặt font theo `fontFamily`/`fontWeight`/`fontStyle`. P10a tự đủ giá trị: mọi text hiện có đặt đúng chỗ như trình duyệt và không còn tự vỡ chữ.

## Vấn đề

Bốn lỗi, cùng nằm ở đường đo text, tách được khỏi tính năng mới:

1. **Text hug tự vỡ chữ ở ký tự cuối** ("⌘C" → `["⌘","C"]`, "Prefer" → "Prefe", "customers" → "customer / s"). Width đo được làm tròn 1/64 *xuống* rồi đo lại ở chính width đó thì không vừa.
2. **Text hug sát mép parent `free` xuống dòng** (note #4). Đây là hành vi CSS (`position:absolute` co theo chỗ còn lại), nhưng agent port từ DOM (nhãn một dòng, breadcrumb) không có cách nói "không xuống dòng". Người dùng phải nới container +12px.
3. **Baseline sai 1–5 px** (note #5): `y = lineHeight/2 + fontSize/2` thay vì ascent/descent thật và half-leading như CSS. Mọi text của mọi project đều thấp hơn trình duyệt.
4. **Width là số nguyên trên Docker** (note #13): Chromium headless shell trên Linux mặc định hinting nên advance glyph bị làm tròn. "Filter objects" 14px đo 86 thay vì 83,93, nên text rộng hơn bản mac và DOM của coder tới +2 px. Đây là nguyên nhân thứ hai làm vỡ dòng ở mục 2, và giải thích vì sao người dùng cần nới +12px.

## Bằng chứng

Code ở `night/integration@52a8715` (đường dẫn tương đối repo).

**Đo text (renderer, chạy trong Chromium của worker)**

| Vị trí | Hiện trạng |
| --- | --- |
| `renderer/measurement.ts:18` | `round = Math.round(v*64)/64`: làm tròn *gần nhất*, có thể xuống. |
| `renderer/measurement.ts:55`, `:94` | width từng dòng và width node = `round(measureText)`. |
| `renderer/measurement.ts:65`, `:73`, `:83` | quyết định xuống dòng so `measureText(...)` **thô** với `maxWidth`. |
| `renderer/measurement.ts:44` | `maxWidth = r.width` (Rust đã lượng tử). |
| `renderer/measurement.ts:50-54` | `y = n·lh + fontSize + (lh − fontSize)/2` = `n·lh + lh/2 + fs/2`: không dùng metrics font. |
| `renderer/drawing.ts:35-41` | vẽ `fillText(line.text, b.x+line.x, b.y+line.y)` với `textBaseline="alphabetic"`; không tự đo (comment `:39`). Sửa `line.y` ở measurement là đủ, viewer và PNG cùng hưởng. |

**Rust (`crates/layout`)**

| Vị trí | Hiện trạng |
| --- | --- |
| `crates/layout/src/engine.rs:36-70` | closure đo Taffy: `width = known.width` hoặc available (`:45-49`); trả `measure.width` từ cache. |
| `crates/layout/src/engine.rs:104-111` | lượt dựng scene **đo lại** text ở `l.size.width` (= width vừa trả về, đã làm tròn xuống) → ra dòng khác lượt sizing. Chiều cao node lấy từ lượt sizing (1 dòng), nên dòng 2 nằm ngoài bounds và bị clip ("Prefe"). |
| `crates/layout/src/util.rs:33-60` | `request()`: `width.map(quantize)` (`:56`), khoá = `hash(RENDERER_VERSION, request)` (`:58`). |
| `crates/layout/src/tree.rs:69-73` | con của `free` là `Position::Absolute` với `inset.left = x`: Taffy cho available = width parent − x, giống CSS. |
| `crates/layout/src/util.rs:25-31` | `hug` và bỏ trống đều là `auto()`; P08a phân biệt hai cái khi `align:"stretch"` (hug = fit-content). |
| `crates/layout/src/lib.rs:13-14`, `renderer/types.ts:1-2` | `scene-1`/`web-1`; `web-1` còn hard-code ở `web/tests/state.test.ts:70`, `web/tests/fixture-data.mjs:80`, `web/tests/fixture-dense.mjs:46`, `spikes/pipeline/README.md:23`. |

**Worker và catalog**

- `worker/runtime.mjs:57-61`: `chromium.launch({headless, executablePath, timeout})`, không có `args`.
- `crates/server/src/fingerprint.rs:9-14`: `worker/runtime.mjs` nằm trong pipeline identity, nên đổi args tự làm mới cache.
- `crates/storage/src/props_catalog.rs:85-88`: text chỉ có `text`, `fontId`, `fontSize`, `lineHeight`.
- `crates/server/src/props_doc.rs:55-56`: mục "Not supported" liệt kê letter spacing, text alignment.
- `crates/storage/src/validate/node.rs:20-53`: text style chỉ nhận `fontId`, `fill`, `fontSize`, `lineHeight`.

**Đo thật** — Chromium pinned 156.0.8078.4, mac và image `opencanvas:local`.
Script ở scratchpad, không commit: `/private/tmp/claude-501/-Volumes-MySpace-Workspace-OpenCanvas/c338aab6-301e-4c1f-a358-35b465f72775/scratchpad/p10/` (`probe.mjs`, `pixels.mjs`, `e2e.mjs`, `linux.mjs`, `mac.mjs`, thư mục `proto/` = bản sao pipeline đã sửa `measurement.ts`).

*1. Làm tròn xuống, rồi tự vỡ* (`probe.mjs`, `mac.mjs`):

| Font 14px | Chuỗi | `measureText` thô | round 1/64 | ceil 1/64 | width DOM `max-content` |
| --- | --- | --- | --- | --- | --- |
| Noto (bundled) | ⌘C | 22,847992 | 22,84375 ↓ | 22,859375 | 22,859375 |
| Noto | Prefer | 40,361969 | 40,359375 ↓ | 40,375 | 40,375 |
| SF Pro | customers | 69,220703 | 69,21875 ↓ | 69,234375 | — |

- 11/30 cặp (5 font × 6 chuỗi, 14px) bị làm tròn xuống, gồm đúng "Prefer" và "customers" người dùng gặp.
- Width DOM của Chrome **bằng ceil 1/64** ở cả 12/12 chuỗi đo (LayoutUnit của Blink làm tròn lên cho shrink-to-fit).
- "⌘" không có trong NotoSans (rơi về font hệ thống, thuộc P11 `MISSING_GLYPH`), nhưng lỗi vỡ chữ là do làm tròn: "⇧⌘V" cũng fallback mà không vỡ, vì nó làm tròn lên.

*2. Worker thật trước/sau* (`e2e.mjs`; trước = `night/integration`, sau = `proto/` chỉ sửa `measurement.ts`):

| Ca | Trước | Sau |
| --- | --- | --- |
| row 300, hug "⌘C" 14/20 | `["⌘","C"]`, w 22,84375, **h 40** | `["⌘C"]`, w 22,859375, h 20 |
| free 400, hug "Prefer" tại x 10 | `["Prefe","r"]`, **h 20** (dòng 2 ngoài bounds) | `["Prefer"]`, w 40,375 |
| free 400, hug "customers" tại x 360 | `["custo","mers"]`, h 40 | không đổi (đúng CSS) → cần `textWrap` |
| PNG worker so DOM, 5 khối 2 dòng (12/18…32/40) | 10 392 px lệch | **0 px lệch** |

*3. Baseline* (`probe.mjs`, marker inline-block cao 0 trong DOM):
- Hiện tại thấp hơn DOM 1–5 px ở 35/35 ca (5 font × 7 cặp size/lineHeight): SF Pro 12/18 thấp 2 px (≈ 0,17em, khớp note #5); Georgia 32/40 thấp 5 px.
- Công thức `A + floor((lh − (A + D)) / 2)` với `A`, `D` = `fontBoundingBoxAscent/Descent` của canvas **bằng đúng** baseline DOM ở 35/35 ca, kể cả `lh < A + D` (Noto 15/15, 11/13).
- Pixel (`pixels.mjs`, 2 dòng gồm dấu tiếng Việt): 24/24 ca (Noto, SF Pro, Georgia, SF Mono × 6 cặp) **0 pixel lệch**. Bản hiện tại lệch 1 047–5 185 px mỗi ca.

*4. Linux hinting* (`linux.mjs` trong container tạm `docker run --rm --network none opencanvas:local`, không đụng container `opencanvas`):

| Noto 14px | Linux mặc định | Linux `--font-render-hinting=none` | mac (mặc định và có cờ) |
| --- | --- | --- | --- |
| Filter objects | 86 | 83,929688 | 83,929901 |
| Sao chép | 61 | 59,527771 | 59,527924 |
| Hello world | 76 | 75,081711 | 75,081940 |
| customers | 70 | 69,075745 | 69,075928 |

- `deviceScaleFactor: 2` không đổi kết quả.
- Trên mac, cờ không đổi bất kỳ số nào (12/12 chuỗi giống hệt).
- Có cờ, Linux lệch mac ≤ 6·10⁻⁴ px; sau ceil 1/64 thì bằng nhau ở cả 12 chuỗi.
- `fontBoundingBoxAscent/Descent` giống nhau trên hai nền (13/4 ở 12px, 15/4 ở 14px).

**Tham khảo**
- CSS 2.1 §10.8.1: half-leading `(line-height − (A + D)) / 2` đặt trên và dưới. Blink `NGLineHeightMetrics::AddLeading` floor phần trên, khớp số đo.
- CSS 2.1 §10.3.7: width `auto` của phần tử absolute là shrink-to-fit trong `containing block − left`.
- CSS Text 4 `text-wrap-mode: wrap | nowrap` (https://drafts.csswg.org/css-text-4/#text-wrap-mode) và shorthand `text-wrap` (https://developer.mozilla.org/en-US/docs/Web/CSS/text-wrap).
- Figma `textAutoResize: WIDTH_AND_HEIGHT` ("auto width") không bao giờ tự xuống dòng (https://developers.figma.com/docs/plugins/api/properties/TextNode-textautoresize/).
- `TextMetrics.fontBoundingBoxAscent` (https://developer.mozilla.org/en-US/docs/Web/API/TextMetrics/fontBoundingBoxAscent).

## Giải pháp

### 1. Đo như LayoutUnit của Blink (`renderer/measurement.ts`)

```ts
const ceil64 = (v: number) => Math.ceil(v * 64) / 64;   // width: luôn ≥ giá trị thô
const width = (t: string) => ceil64(context.measureText(t).width);
// fits: so hai số đều là bội 1/64 → so sánh chính xác, đo lại ở width của chính nó luôn vừa
if (width(next) <= maxWidth) ...
const m = context.measureText(""), A = m.fontBoundingBoxAscent, D = m.fontBoundingBoxDescent;
const baseline = A + Math.floor((r.lineHeight - (A + D)) / 2);
y = round(n * r.lineHeight + baseline);                  // round 1/64 giữ nguyên cho y
```

- Width dòng và width node dùng `ceil64`, giống width DOM.
- Mọi so sánh xuống dòng dùng `width()` đã ceil; `maxWidth` là bội 1/64 do Rust `quantize`. Hai bên cùng lưới nên phép so là chính xác, không cần epsilon.
- Bất biến mới: đo ở `width = W` với `W` = width đo không giới hạn thì ra cùng các dòng.
- Chiều cao node giữ `lines × lineHeight`. Glyph có thể tràn trên/dưới khi `lh < A + D`, như CSS.
- Đã chạy trong `proto/`: diff ~10 dòng, `measurement.ts` 100 → ~104 dòng.

**Vì sao không sửa ở Rust** (ví dụ cộng epsilon vào width khi đo lại):
- Bất biến phải đúng ở chỗ quyết định xuống dòng.
- Ceil cũng làm width hug bằng đúng width DOM, nên coder thấy cùng số.

### 2. Tắt hinting của Chromium Linux (`worker/runtime.mjs`)

- Thêm `args: ["--font-render-hinting=none"]` vào `chromium.launch`.
- Nhờ đó measure (và pixel PNG) trên Docker giống mac và giống trình duyệt của coder.
- Fingerprint đổi theo file, nên cache render cũ không bị dùng lại.
- Không tác dụng trên mac (đã đo).

### 3. Prop `textWrap` (note #4, trường hợp parent hẹp)

```json
{"kind":"text","props":{"text":"customers","fontId":"font_bundled","fontSize":14,"lineHeight":20,"width":"hug","textWrap":"nowrap"}}
```

- Giá trị: `"wrap"` (mặc định, như hiện nay) hoặc `"nowrap"`. Scope text. Text style cũng được mang key này: nhãn một dòng gom vào style, không tốn token mỗi node.
- `nowrap` = không tự xuống dòng. LF trong `text` vẫn xuống dòng, vì mặc định hiện tại đã giữ LF.
  - Width `hug`/bỏ trống = đoạn rộng nhất.
  - Width số/`fill`: box giữ width authored, dòng dài hơn thì tràn phải, như CSS `nowrap` với `overflow: visible`.
  - min-content = max-content.
- **Rust:** `Props.text_wrap: Option<String>`. Trong `request()`, `width = if nowrap { None } else { width }`. Text `nowrap` dùng chung khoá cache với lượt đo không giới hạn; `MeasureRequest` không thêm field. `validation.rs` từ chối giá trị lạ (`LAYOUT_INVALID`).
- **Không đổi nghĩa `hug`** (vẫn fit-content như CSS và như P08a đã dựa vào), vì:
  - đổi `hug` thành "không giới hạn" làm đổi scene của text hug nhiều dòng hiện có (vi phạm "không đổi nghĩa field cũ");
  - CSS cũng co text absolute theo containing block.
  - Bảng map ghi: Figma "auto width" = `width:"hug"` + `textWrap:"nowrap"`.
- **Tên:** `textWrap` theo CSS `text-wrap: nowrap` (Chrome 114+, Safari 17.4, Firefox 124). Không chọn `whiteSpace`:
  - CSS `white-space: nowrap` gộp LF thành dấu cách, còn mặc định của ta giữ LF (= `pre-line`). Dùng tên `whiteSpace` sẽ dạy coder map sai.
  - Vẫn để `whiteSpace` làm alias, nên agent gõ `whiteSpace` được gợi ý `textWrap` (cơ chế B02).
  - Aliases: `whiteSpace`, `wrap`, `noWrap`, `nowrap`, `wordWrap`, `textAutoResize`, `autoResize`, `singleLine`.
- **Map cho coder** (`implement-frame.md`):
  - text mặc định → `white-space: pre-line`;
  - `textWrap:"nowrap"` → `white-space: nowrap` khi text không có LF, ngược lại `white-space-collapse: preserve-breaks; text-wrap-mode: nowrap`.

### 4. Phiên bản

- Bump `RENDERER_VERSION` lên số kế tiếp chưa dùng (`web-2` nếu P10a merge trước P04a/P05/P01) ở `lib.rs:14` và `types.ts:2`, cùng 3 test và README.
- `SCENE_VERSION` giữ nguyên vì shape scene không đổi.
- Hệ quả:
  - Mọi khoá đo và `sceneKey` đổi một lần.
  - Tab viewer cũ báo `PIPELINE_MISMATCH` tới khi reload.
  - Mọi text dịch lên 1–5 px. Đây là bản sửa lỗi, xem Rủi ro: project QueryNest đã tự bù.

## Phạm vi file

| File | Việc | Dòng (hiện → ước) |
| --- | --- | --- |
| `renderer/measurement.ts` | `ceil64`, `width()` dùng chung cho đo và so; baseline theo `fontBoundingBox*` | 100 → ~106 |
| `worker/runtime.mjs` | `args: ["--font-render-hinting=none"]` + 1 dòng comment | 85 → ~87 |
| `crates/layout/src/model.rs`, `util.rs`, `validation.rs`, `lib.rs` | field `text_wrap`; `request()` bỏ width khi nowrap; kiểm giá trị; bump version | 210→211, 60→62, 130→~136 |
| `crates/storage/src/props_catalog.rs` | entry `textWrap` (Text, value, aliases) | 126 → ~127 |
| `crates/storage/src/validate/properties.rs`, `validate/node.rs` | kiểm `"wrap"\|"nowrap"` cho node và text style (helper chung trong `values.rs`) | 124→~126, 110→~112, 75→~83 |
| `renderer/types.ts` + 3 test + `spikes/pipeline/README.md` | chuỗi version | 1 dòng mỗi file |
| `spikes/pipeline/text-measure.test.mjs` (mới) | AC1–AC4, AC6 (Chromium, NotoSans; font hệ thống nếu có) | ~180 |
| `crates/layout/src/text_wrap_tests.rs` (mới, `#[cfg(test)]`) | AC6 phần layout với máy đo giả (theo `wrap_tests.rs`) | ~80 |
| `crates/storage/tests/text_wrap.rs` (mới) | AC7 | ~70 |
| `crates/server/src/props_doc.rs`, `skills/author-frame.md`, `skills/implement-frame.md` | ghi chú hug = fit-content, `textWrap`; map CSS; sửa `implement-frame.md:53-55` (line y = baseline theo metrics font như trình duyệt) | +2, +3, +5 |
| `docs/CONTRACT.md`, `docs/VIEWER.md` | quy tắc đo: width ceil 1/64, baseline half-leading, cờ hinting | vài dòng |

- Không file mã nào vượt 200 dòng.
- Không đụng `mcp.rs`, `schemas.rs`, `drawing.ts`.
- `props_catalog.rs` và `props_doc.rs` cũng bị P04a/P08a sửa (mỗi bên 1–2 dòng): rebase không xung đột logic.

## Tương thích MCP

- `tools/list` byte-identical vì `props` là object tự do.
- `allowedKeys` của text và dòng `text` trong `skill://opencanvas/props.md` thêm `textWrap`.
- Dữ liệu cũ hợp lệ như trước. Text không có `textWrap` sinh `MeasureRequest` cùng shape.
- **Thay đổi giá trị đọc ra (cố ý, sửa lỗi):**
  - `lines[].y` trong `get_design_context`/`inspect_layout`/scene nhỏ hơn 1–5 px;
  - width text tăng ≤ 1/64 px;
  - text từng bị vỡ ở ký tự cuối nay còn một dòng và height nhỏ lại.
- Tài liệu cập nhật: props.md (sinh tự động), 2 skill, CONTRACT, VIEWER, README pipeline.

## AC

Mọi AC pixel chạy trong Chromium pinned của worker. "DOM" = cùng trang, cùng bytes font, `white-space:pre`, `width:max-content`.

**AC1 — Đo lại ở width của chính nó không vỡ** (`text-measure.test.mjs`, gọi `renderer.measureBatch` trong trang worker).
- Với mỗi chuỗi trong danh sách ≥ 40 chuỗi, gồm "⌘C", "⇧⌘V", "Prefer", "customers", "Sao chép", "Filter objects", chữ Việt có dấu, số và ký hiệu, ở size 11, 12, 13, 14, 16, 24:
  - đo với `width:null` ra `w`, 1 dòng;
  - đo lại với `width:w` ra đúng 1 dòng cùng text, cùng `w`.
- Trên `night/integration` hiện tại, test này fail ít nhất ở "⌘C" 14 và "Prefer" 14.

**AC2 — Worker thật** (fixture như `glyph.mjs` của P08), font bundled 14/20:
- Row width 300, `padding:4`, `gap:100`, các text hug "⌘C", "⇧⌘V", "AB": mỗi node `lines.length == 1` và `bounds.height == 20`.
- Frame free 400, hug "Prefer" tại x 10: `lines == ["Prefer"]` và `bounds.width == max(lines.width)`.

**AC3 — Width bằng DOM.**
- Với các chuỗi AC1 ở 12 và 14px, `measureBatch(width:null).width` **bằng đúng** `getBoundingClientRect().width` của span DOM `max-content`.
- Ví dụ mac: "⌘C" 14 = 22,859375; "Prefer" 14 = 40,375.

**AC4 — Baseline bằng DOM.**

Font: NotoSans bắt buộc. SF Pro (`/System/Library/Fonts/SFNS.ttf`), Georgia, SF Mono thì chạy nếu file tồn tại, ngược lại `skip` có log.

Cặp size/lineHeight: 11/13, 12/18, 13/16, 14/20, 15/15, 16/24, 32/40.

- **Số:** `lines[0].y` == top của marker inline-block cao 0 sau dòng 1 trong DOM, sai số 0. `lines[1].y − lines[0].y == lineHeight`.
- **Pixel:**
  - worker `compute`+`render` một frame trắng chứa 5 khối text 2 dòng ("Hxg Sao chép Ấp" / "second line ỹ", 12/18, 13/16, 14/20, 16/24, 32/40);
  - so với screenshot DOM cùng vị trí;
  - kết quả: **0 pixel khác** với NotoSans. Font tuỳ chọn: 0 pixel khác hoặc |Δ trọng tâm mực theo y| ≤ 0,5 px.
- Số tham chiếu (NotoSans, mac): 12/18 → `y = 13`, 14/20 → 15, 16/24 → 18, 15/15 → 13, 32/40 → 32. Bản cũ: 15, 17, 20, 15, 36. Linux đã đo cùng `A/D` ở 12px (13/4) và 14px (15/4), nên cùng `y`.

**AC5 — Không còn width nguyên trên Linux.**
- `rg -- "--font-render-hinting=none" worker/runtime.mjs` có kết quả.
- Live (QA có Docker):
  - `docker build -t opencanvas:p10a .` trên nhánh;
  - chạy probe kiểu `linux.mjs` trong `docker run --rm --network none opencanvas:p10a` qua `startRuntime()` của worker;
  - NotoSans 14: "Filter objects", "Sao chép", "Hello world", "customers" có width ceil 1/64 lần lượt 83,9375; 59,53125; 75,09375; 69,078125, bằng mac.
- Không dùng tag `opencanvas:local`/`candidate` của gốc, không đụng container `opencanvas`. Không có Docker thì ghi skip, không ghi pass.

**AC6 — `textWrap`.**
- Frame free 400, text hug "customers" 14/20 tại x 360:
  - không có `textWrap`: `["custo","mers"]`, height 40 (giữ hành vi CSS);
  - `textWrap:"nowrap"`: `["customers"]`, width 69,078125, height 20.
- Width 40 + `nowrap`: `bounds.width == 40`, 1 dòng, `lines[0].width == 69,078125`.
- `"a b\nc d"` + `nowrap`: 2 dòng `["a b","c d"]`.
- Column `align` mặc định, width 100, text dài 30 ký tự + `nowrap`: 1 dòng (layout unit, máy đo giả 8 px/ký tự → width 240).
- `textWrap:"wrap"` cho scene giống hệt khi không có key.

**AC7 — Validator** (`crates/storage/tests/text_wrap.rs`, theo format B02):
- Nhận `"wrap"`/`"nowrap"` trên text và trong text style (style áp lên node, `resolvedProps.textWrap` có giá trị).
- `textWrap` trên rectangle → `field:"props.textWrap"`, `nodeKind:"rectangle"`.
- `"no-wrap"` → `field:"textWrap"`, message `Text wrap must be "wrap" or "nowrap"`.
- `whiteSpace:"nowrap"` → `field:"props.whiteSpace"`, `suggestion:"textWrap"`.
- Test B02 `validator_accepts_exactly_the_catalog_keys_per_kind` pass không sửa logic.

**AC8 — Tương thích.**
- Toàn bộ test hiện có pass sau khi chỉ đổi chuỗi version: `cargo test` layout/storage/server, `npm run test:pipeline`, `npm --prefix web test`.
- Scene fixture `spikes/pipeline/fixtures.mjs` trước/sau, bỏ `provenance`:
  - cùng tập node và bounds non-text;
  - width text chênh trong [0, 1/64];
  - `lines[].y` khớp công thức AC4;
  - không có key mới.
- `tools/list` byte-identical (`crates/server/tests/tool_schemas.rs`).

**AC9 — Phiên bản.**
- `drawScene` với scene `rendererVersion` cũ ném `PIPELINE_MISMATCH`.
- Khoá đo của cùng một request khác khoá trước khi bump.

**AC10 — Modular.**
- File mới ≤ 200 dòng; `measurement.ts` ≤ 120.
- `cargo fmt --all --check`, clippy (lệnh ACCEPTANCE), `npm run check:renderer` pass.

## Cách kiểm

```bash
W=<worktree>; cd $W && export CARGO_BUILD_JOBS=4 PLAYWRIGHT_BROWSERS_PATH=$W/.runtime/browsers
cargo test --offline -p opencanvas-layout -p opencanvas-storage -p opencanvas-server
cargo clippy --offline -p opencanvas-layout -p opencanvas-storage -p opencanvas-server --lib --bin opencanvas-server -- -D warnings
cargo fmt --all --check
npm run check:renderer
npm run test:pipeline            # gồm text-measure.test.mjs mới
npm --prefix web test && npm --prefix web run build
npm run test:e2e                 # known-overflow.json có thể đổi ≤ 1/64 px: xem Rủi ro
```

**Live**
- `target/debug/opencanvas-server --port 0 --data-root $(mktemp -d)`.
- Chạy `create_project` → `apply_batch` text hug "⌘C" trong row + "customers" `textWrap:"nowrap"` → `render_frame` → `inspect_layout fields:["lines"]`: mỗi node 1 dòng; `lines[0].y` = 15 với 14/20.
- Dọn process và data. Không đụng 3847.
- AC5 Docker theo lệnh ở trên.

Script đo tham chiếu đã chạy được (dev chép ý, không chép file): `scratchpad/p10/e2e.mjs <root>` in bảng trước/sau và so PNG–DOM.

## Rủi ro

- **Project thật đã tự bù baseline (note #5):** QueryNest đặt `y = baselineBrowser − lh/2 − fs/2`.
  - Sau P10a, text của project đó cao hơn ý đồ đúng bằng độ lệch cũ (≈ 2 px ở 12–14px SF Pro).
  - Gốc cần ghi vào `REPORT.md`: bỏ bù trong script capture rồi chạy lại, hoặc chấp nhận dịch.
  - Không có migration dữ liệu, vì phần bù nằm trong `y` authored nên không phân biệt được với ý đồ.
- **E2E/known-overflow:** width tăng ≤ 1/64 có thể lật một phép so overflow sát ngưỡng trong `spikes/e2e/known-overflow.json`. Chạy `test:e2e`; nếu lệch thì cập nhật baseline với giải thích.
- **Glyph fallback khác nhau giữa mac và Linux** ("⌘" không có trong NotoSans): width "⌘C" có thể khác giữa hai nền dù đã tắt hinting. AC1/AC3 vì vậy so trong cùng trình duyệt. Chẩn đoán `MISSING_GLYPH` thuộc P11.
- **Thứ tự version:** P01/P04a/P05/P10a đều bump. Task merge sau lấy số kế tiếp; cập nhật cùng 3 test.
- **`--font-render-hinting=none` đổi raster glyph trên Linux** (mịn hơn, không snap). PNG Docker sẽ khác PNG Docker cũ, đúng mục đích là gần mac. Không có golden PNG nào trong test (đã `find`).
- **Width ceil thay cho round:** hug width tăng tối đa 1/64; tổng width của row nhiều text tăng ≤ n/64. Khớp DOM nên coi là đúng hơn.

## Ngoài phạm vi / phase sau

- **P10b:** `letterSpacing`, `textAlign`, `textDecoration`, `textTransform`.
- **P10c:** `fontFamily`/`fontWeight`/`fontStyle`.
- **P11:** `MISSING_GLYPH` (#12), `sortKey` (#14). #13 đã xử lý ở đây.
- Cắt dòng có dấu "…" (`textOverflow`/`maxLines`, Figma `TRUNCATE`), `text-wrap: balance|pretty`, căn dọc text trong box cố định (Figma `textAlignVertical`), baseline alignment trong stack (P08b sau khi có `lines[0].y` đúng).
