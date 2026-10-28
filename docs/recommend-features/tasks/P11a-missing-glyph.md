# P11a — `MISSING_GLYPH`: báo glyph rơi sang font fallback ngay lúc đo text (note #12)
Ưu tiên: P1 · Kích cỡ: ~70 phút dev · Phụ thuộc: không bắt buộc. Nên làm **sau P10a** (cùng sửa `renderer/measurement.ts`, cùng bump `RENDERER_VERSION`); làm trước thì P10a rebase vài dòng · Nguồn: note #12 (bối cảnh note #11)

Phase a của P11. Phase b (`P11b-sibling-order.md`, note #14) độc lập, khác vùng code. P11a tự đủ giá trị: mọi render và inspect chỉ ra node nào có ký tự font không vẽ được, trước khi agent mất công nới container.

## Vấn đề

Text chỉ khai báo một family. Ký tự không có trong cmap của font (người dùng gặp "−" U+2212, "↵", "⌄") được trình duyệt vẽ bằng font hệ thống, từng ký tự một, **không có tín hiệu nào** trong output:

- width đo bằng font fallback, nên dòng vỡ khác thiết kế (note #12: "đo width sai → wrap");
- font fallback phụ thuộc máy: PNG worker (Docker Linux) và viewer (Chrome trên mac của người dùng) chọn font khác nhau, nên cùng scene vẽ ra hai kiểu chữ;
- người dùng chỉ phát hiện bằng mắt, sau đó phải import lại cả bộ font (note #11: 18 font `font_sf_*` bị bỏ lại).

Phát hiện thêm khi đo: **font mặc định `font_bundled` (Noto Sans) thiếu `⌘ ↵ ⌄ → ← ↑ ↓ ✓ ✕`, emoji, CJK.** Text bỏ trống `fontId` đều dùng font này (`crates/layout/src/engine.rs:130-136`), nên "⌘C" trong P08/P10a cũng là ca fallback: ⌘ rộng 14,0 px ở cỡ 14 là do font hệ thống vẽ.

## Bằng chứng

Code ở `night/integration@900e0dc` (đường dẫn tương đối repo).

| Vị trí | Hiện trạng |
| --- | --- |
| `renderer/measurement.ts:16-17` | `fontCSS` = `style weight size "family"`, không có danh sách fallback; trình duyệt tự rơi sang font hệ thống. |
| `renderer/measurement.ts:42`, `:55`, `:65`, `:73`, `:83` | `measureText` của dòng và quyết định xuống dòng đã gồm cả width của glyph fallback. |
| `renderer/resources.ts:62`, `:80-84`, `:94-101` | Comment "no implicit system fallback" chỉ đúng ở mức nạp file. `FontFace` không có `unicodeRange`; `document.fonts.load/check` chỉ thử chuỗi tiếng Việt và không kiểm tra cmap. |
| `renderer/drawing.ts:35-41` | `fillText` cùng font nên vẽ glyph fallback. |
| `crates/layout/src/model.rs:131-135` | `Measurement {width,height,lines}`: không có chỗ chứa thông tin glyph. |
| `crates/layout/src/model.rs:188`, `engine.rs:182` | `Scene.diagnostics: Vec<String>` luôn rỗng ở phía layout. |
| `worker/pipeline.mjs:144` | Chỉ nối `resources.diagnostics` (font/asset không sẵn sàng). |
| `crates/server/src/jobs/record.rs:32`, `inspect.rs:74`, `render_response.rs:27` | `diagnostics` của scene đi thẳng vào `render_frame` (cả compact) và `inspect_layout`. Kênh đã có, chỉ cần dữ liệu. |
| `crates/storage/src/assets.rs:208-210` | Import font chỉ kiểm chữ ký TTF/OTF/WOFF2, không đọc cmap. |
| `~/.cargo/registry/cache` | Có `ttf-parser 0.25.1`, không có `brotli`/`woff2`, nên Rust không đọc được cmap của WOFF2 offline. Đây là lý do phát hiện ở trình duyệt. |

**Prototype** (`chrome-headless-shell` 1248 mac-arm64, Noto Sans bundled, 14px). Script ở `/private/tmp/claude-501/-Volumes-MySpace-Workspace-OpenCanvas/14034e76-758d-42c7-8c11-41ff9b05d4f4/scratchpad/p11/probe-font.mjs` và `probe-test.mjs`.

Cách đo: font "probe" 524 byte sinh trong JS, map mọi code point sang một glyph có advance 0. So width dưới `"Noto Sans"` với width dưới `"Noto Sans", "Probe"`.

| Chuỗi | width `"Noto Sans"` | width `+ Probe` | Thiếu |
| --- | --- | --- | --- |
| `A−` | 16,954 | 16,954 | — (Noto có U+2212) |
| `Enter ↵` | 52,808 | 38,808 | U+21B5 |
| `Chọn ⌄` | 46,691 | 38,262 | U+2304 |
| `⌘C` | 22,848 | 8,848 | U+2318 |
| `Tiếng Việt ă â đ ê ô ơ ư` | 149,408 | 149,408 | — |
| `e◌́` (NFD), `x‍y` (ZWJ) | bằng nhau | bằng nhau | — |
| `👩‍💻` | 18 | 0 | U+1F469, U+1F4BB (ZWJ không bị báo) |
| `→ ← ↑ ↓` / `✓ ✕` | 56,981 / 25,009 | 10,92 / 3,64 | 4 / 2 code point |
| `• · …`, `“” — –`, `€ £ ¥ ₫` | bằng nhau | bằng nhau | — |
| `日本` | 28,56 | 0 | U+65E5, U+672C |

- Không có dương tính giả trên 16 mẫu. Nếu family có glyph thì cả hai chuỗi font dùng cùng một glyph, nên width bằng nhau tuyệt đối (so `===`).
- Chi phí đường nhanh (không thiếu glyph): 2 000 chuỗi × 2 `measureText` mất 10,7–11,6 ms, tức thêm khoảng 3 µs cho mỗi request.
- Bẫy OTS: bản đầu có bảng `glyf` dài 0 nên bị từ chối ("OTS parsing error: glyf: zero-length table"). Cho glyph 1 header 12 byte (0 contour) thì nạp được. cmap (3,10) format 13 với hai dải U+0020–U+D7FF, U+E000–U+10FFFF được OTS chấp nhận.

## Giải pháp

### 1. Font probe (renderer, mới)

`renderer/glyph-probe.ts` export hai thứ:

- `probeFontBytes()`: TrueType tối thiểu (OS/2, cmap format 13, glyf 12 byte, head, hhea, hmtx advance 0, loca, maxp, name, post), sinh trong mã nên không có file nhị phân, request mạng hay hash;
- `PROBE_FAMILY = "OpenCanvas Glyph Probe"`.

`loadResources` (`renderer/resources.ts`) nạp `FontFace(PROBE_FAMILY, probeFontBytes())` một lần cho mỗi page (cờ module). Probe không vào manifest và không đổi `fontManifestHash`.

Nạp probe lỗi thì **không làm fail job**: đặt `resources.glyphProbe = false` và thêm vào `resources.diagnostics` chuỗi `GLYPH_PROBE_UNAVAILABLE`. Lúc đó không phát hiện được glyph thiếu, nhưng job vẫn render. Chuỗi này không bị chế độ strict coi là lỗi resource: tách khỏi danh sách được `throw` ở `resources.ts:146`.

### 2. Phát hiện lúc đo (`renderer/glyph-coverage.ts`, mới; `measurement.ts` +6 dòng)

```ts
// Trả code point thiếu, dạng "U+21B5", sắp tăng dần; [] khi đủ glyph.
missingCodePoints(context, fontCss, text, memo): string[]
```

Đường nhanh: `w = measureText(text)` dưới `fontCss` và dưới `fontCss + ', "OpenCanvas Glyph Probe"'`. Bằng nhau thì trả `[]`.

Khác nhau thì xét từng code point duy nhất, bỏ `\s`, `\p{Cc}`, `\p{Cf}`. Code point bị coi là thiếu khi `probeWidth(ch) === 0 && plainWidth(ch) > 0`. Kết quả nhớ theo `fontId + code point` trong một batch.

`measureBatch` gọi một lần cho mỗi request, trên `r.text` đã chuẩn hoá CRLF. Có code point thiếu thì gắn `missing: string[]` vào `Measurement`, không có thì **bỏ khoá** (không tốn byte).

Lines, width và vị trí **giữ nguyên**: glyph fallback vẫn được vẽ để người dùng thấy chữ. Chỉ thêm cảnh báo.

### 3. Rust layout

`Measurement` thêm field:

```rust
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub missing: Vec<String>
```

Cache cũ không có khoá này vẫn deserialize được.

Module mới `crates/layout/src/glyphs.rs`:

- `fn diagnostic(node, font, missing, text) -> String`;
- `fn cap(Vec<String>) -> Vec<String>`.

`engine.rs:104-111` đã lấy `m` của lượt dựng scene. Nếu `!m.missing.is_empty()` thì đẩy vào một `Vec`. Trước `:182`, gán `diagnostics: glyphs::cap(found)`.

Định dạng chuỗi (một dòng cho mỗi node, theo thứ tự vẽ):

```
MISSING_GLYPH t_kbd font_bundled (Noto Sans): U+2318 "⌘", U+21B5 "↵" are drawn with a fallback font; width and wrapping are unreliable
```

Giới hạn:

- tối đa 8 code point mỗi node, kèm ` +N more`;
- tối đa 20 node, sau đó một chuỗi `MISSING_GLYPH +N more nodes`;
- trần khoảng 3 KB trong response compact.

Ký tự trong ngoặc là chính glyph, để agent tìm/thay trong text mà không cần tra bảng mã. Ký tự điều khiển hoặc vô hình thì bỏ phần ngoặc.

**Vì sao chuỗi, không object:** `Scene.diagnostics` là `string[]` ở `renderer/types.ts:84`, `web/src/hooks/useScene.ts:16` và mọi job. Chuyển sang object sẽ vỡ viewer và kiểu của client cũ. Chuỗi mở đầu bằng mã cố định đọc được cho cả LLM lẫn `startsWith`. Dạng object cho `get_design_context` để phase sau (xem cuối file).

**Vì sao trình duyệt, không Rust cmap:**

- WOFF2 cần brotli, crate này không có offline;
- trình duyệt chính là thứ vẽ và đo, nên phát hiện đúng điều sẽ xảy ra, kể cả glyph mà font khai báo trong cmap nhưng OTS loại bỏ;
- so hai generic family (`monospace` với `serif`) thì âm tính giả khi cả hai rơi về cùng một font hệ thống (Docker ít font), nên probe advance-0 là cách duy nhất không phụ thuộc font máy.

### 4. Version và tài liệu

- Bump `RENDERER_VERSION` (`crates/layout/src/lib.rs:19`, `renderer/types.ts:2`), vì Measurement và scene đổi nội dung. Gộp chung lần bump với P10a nếu cùng đợt. Các chỗ hard-code `web-1` P10a đã liệt kê: `web/tests/state.test.ts:70`, `web/tests/fixture-data.mjs:80`, `web/tests/fixture-dense.mjs:46`, `spikes/pipeline/README.md:23`.
- `docs/SERVER.md` (mục diagnostics của render/inspect): định dạng `MISSING_GLYPH` và giới hạn.
- `crates/server/skills/author-frame.md`, một dòng: "`MISSING_GLYPH` in render_frame diagnostics: the font lacks those characters. Import a font (or subset) that covers them, or change the text; do not widen containers to absorb the fallback width."
- `implement-frame.md`, một dòng: ký tự đó trong code cũng sẽ dùng font fallback của trình duyệt người xem.
- Nếu P12a (thay font theo id) đã merge, câu skill đổi thành "re-import the font under the same font.id with those characters" (khớp `P12a-font-delete-replace.md:163`). P11a phát hiện, P12a sửa; không phụ thuộc code.

## Phạm vi file

| File | Thay đổi | Dòng ước lượng |
| --- | --- | --- |
| `renderer/glyph-probe.ts` | mới: sinh bytes font probe | ~70 |
| `renderer/glyph-coverage.ts` | mới: `missingCodePoints` + memo | ~45 |
| `renderer/measurement.ts` | gọi coverage, gắn `missing` | +6 (100 → ~106) |
| `renderer/resources.ts` | nạp probe một lần, `glyphProbe` | +14 (152 → ~166) |
| `renderer/types.ts` | `Measurement.missing?`, `Resources.glyphProbe` | +2 |
| `crates/layout/src/model.rs` | field `missing` | +2 (210 → 212) |
| `crates/layout/src/glyphs.rs` | mới: format + cap + unit test | ~90 |
| `crates/layout/src/engine.rs` | thu `missing`, gán diagnostics | +6 (185 → ~191) |
| `crates/layout/src/lib.rs` | `mod glyphs`, bump version | +2 |
| `spikes/pipeline/glyphs.test.mjs` | mới: test Chromium thật | ~110 |
| `docs/SERVER.md`, `crates/server/skills/author-frame.md`, `implement-frame.md` | tài liệu | +6 |
| Fixture/hard-code version | đổi chuỗi | 4 file × 1 dòng |

Không file nào vượt 250 dòng. `renderer/dist/index.js` không được track, sinh lại bằng `npm run build:renderer`.

## Tương thích MCP

- `tools/list` **không đổi một byte**: không thêm tool, schema hay description. Kênh `diagnostics` đã được mô tả trong `schemas/render.rs:8`.
- `diagnostics` vẫn là `string[]`. Chuỗi cũ (`Font … failed readiness`) giữ nguyên.
- Width, lines, bounds không đổi, nên implementation agent đã viết theo scene cũ không lệch.
- `sceneKey` và `rendererVersion` đổi (bump), job cũ không được dùng lại. Đây là hành vi đúng.
- Rollback binary cũ: Measurement có `missing` chỉ nằm trong cache bộ nhớ worker, mất khi restart; không có dữ liệu bền nào đổi.

## AC

1. `spikes/pipeline/glyphs.test.mjs`: frame có 6 text `font_bundled` 14/20 (`"Enter ↵"`, `"Chọn ⌄"`, `"⌘C"`, `"→ ← ↑ ↓"`, `"👩‍💻"`, `"日本"`). `scene.diagnostics` có đúng 6 chuỗi bắt đầu `MISSING_GLYPH <id> font_bundled`, theo thứ tự vẽ, với code point đúng bảng prototype (`U+21B5`; `U+2304`; `U+2318`; `U+2190,U+2191,U+2192,U+2193` sắp tăng; `U+1F469,U+1F4BB` không có `U+200D`; `U+65E5,U+672C`).
2. Cùng test, các text `"A−"`, `"Tiếng Việt ă â đ ê ô ơ ư"`, `"“quoted” — –"`, `"€ £ ¥ ₫"`, `"é"`, `"x‍y"`, `"• · …"` cho `scene.diagnostics` không có `MISSING_GLYPH`.
3. Với frame trong AC1, `lines` và `bounds` của mọi node giống hệt kết quả cùng input khi probe tắt (test gọi `measureBatch` hai lần, `glyphProbe` true/false, so `width/height/lines` bằng `deepStrictEqual`).
4. `measureBatch` cho text không thiếu glyph trả object có đúng các khoá `width,height,lines` (không có `missing`).
5. Sau `loadResources`, `document.fonts.check('16px "OpenCanvas Glyph Probe"')` là `true` và không có console message chứa `OTS parsing error`.
6. Khi probe bị ép lỗi (test truyền bytes hỏng qua hook), job vẫn `ready` ở chế độ strict, `diagnostics` chứa `GLYPH_PROBE_UNAVAILABLE`, không có `MISSING_GLYPH`.
7. Giới hạn: một text có 30 code point thiếu cho một chuỗi liệt kê 8 code point đầu kèm ` +22 more`. 25 node thiếu glyph cho 20 chuỗi node cộng `MISSING_GLYPH +5 more nodes` (unit test `glyphs.rs`).
8. Unit test `crates/layout`: cache chứa `Measurement` có `missing:["U+21B5"]` thì `LayoutStep::Ready` có `diagnostics == ["MISSING_GLYPH t font_bundled (Noto Sans): U+21B5 \"↵\" are drawn with a fallback font; width and wrapping are unreliable"]`. JSON Measurement không có khoá `missing` deserialize ra `missing == []` và diagnostics `[]`.
9. Live (server tạm `--port 0 --data-root $(mktemp -d)`): text `"Enter ↵"` → `render_frame` (compact mặc định, `waitMs`) và `inspect_layout` cùng `sceneKey` đều có chuỗi `MISSING_GLYPH` của node đó. Frame chỉ có `"Tiếng Việt"` → `diagnostics: []`.
10. `cargo test -p opencanvas-server --test tools_budget --test tool_schemas` pass. Byte `tools/list` (`serde_json::to_vec`) bằng đúng trước thay đổi; ghi hai số trong `.night/dev/P11a.md`.
11. Chi phí: microbench 2 000 request tiếng Việt không thiếu glyph, `measureBatch` chậm thêm ≤ 10 µs/request trên mac headless. Ghi số trước/sau trong report, không assert trong CI.
12. `RENDERER_VERSION` đã bump; `rg -n '"web-1"' --glob '!spikes/responsive-proposal/*'` không còn kết quả (bằng chứng cũ giữ nguyên). File mới ≤ 200 dòng; `wc -l` các file sửa nằm trong bảng phạm vi.

## Cách kiểm

```bash
npm run check:renderer && npm run test:renderer
npm run test:pipeline                    # gồm spikes/pipeline/glyphs.test.mjs mới
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-layout
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server --test tools_budget --test tool_schemas --test render_response --test inspect_paging
```

- Live (AC9): server tạm, project tạm, `apply_batch` page/frame/2 text; `render_frame` + `waitMs`; `inspect_layout {sceneKey}`. Dọn process và data root sau khi kiểm.
- Kiểm thủ công trong Docker (tuỳ chọn, chỉ build image tạm với tag khác, không đụng container `opencanvas`): cùng bảng AC1. OTS và Chromium giống hệt nên kỳ vọng giống. Nếu không chạy được, ghi rõ "chưa kiểm Linux".

## Rủi ro

- **Âm tính giả có chủ đích:** dấu kết hợp (combining mark) bị thiếu có width 0 ở cả hai chuỗi font nên không bị báo. Nếu Chrome chuyển cả cụm grapheme (chữ gốc + dấu) sang font fallback, chữ gốc cũng không bị báo. Tiếng Việt NFC không dính; NFD hiếm. Ghi trong `docs/SERVER.md`.
- Emoji, CJK, mũi tên dùng `font_bundled` sẽ bắt đầu bị báo. Đó là thông tin đúng, nhưng các project hiện có (QueryNest) sẽ thấy nhiều dòng; cap 20 node giữ response nhỏ.
- Tên family probe trùng font người dùng thì phát hiện sai. Tên dành riêng gần như không thể trùng; phase sau có thể cấm tên đó khi import.
- Chromium tương lai siết OTS với cmap format 13. Khi đó AC5 fail và probe tự tắt (AC6), job vẫn chạy.
- Xung đột `renderer/measurement.ts` và cùng bump version với P10a: làm tuần tự.

## Ngoài phạm vi / phase sau

- **P11c (đề xuất, chưa viết):** `get_design_context` khi có `viewports` ready thêm object `{"code":"MISSING_GLYPH","nodeId","fontId","codePoints":[…]}` vào `diagnostics`, cho coder. Cần `SceneNode.missingGlyphs` (bỏ khi rỗng) để đọc không phải parse chuỗi.
- Cảnh báo ngay ở `apply_batch` (trước render): cần đọc cmap trong Rust (`ttf-parser` cho TTF/OTF; WOFF2 cần brotli) và lưu coverage lúc import font.
- `list_fonts` trả coverage, hoặc `find_nodes` lọc text thiếu glyph.
- Font fallback do người dùng khai báo (`fontFallback:[fontId]`), xoá/thay font (note #11, P12).
- Badge trên viewer.

## Phụ lục — prototype font probe đã chạy được (Chromium 1248, OTS chấp nhận, 524 B)

Dev chép vào `renderer/glyph-probe.ts`, thêm kiểu TS và đổi tên `name` thành `PROBE_FAMILY` nếu muốn. Bảng `glyf` phải khác rỗng (12 byte, 0 contour) và `loca` short `[0,0,6]`.

```js
const u16 = (v) => [(v >> 8) & 255, v & 255];
const i16 = (v) => u16(v & 0xffff);
const u32 = (v) => [(v >>> 24) & 255, (v >>> 16) & 255, (v >>> 8) & 255, v & 255];
const groups = [[0x20, 0xd7ff], [0xe000, 0x10ffff]];          // cmap (3,10) format 13 → glyph 1
const cmapSub = [...u16(13), ...u16(0), ...u32(16 + 12 * groups.length), ...u32(0), ...u32(groups.length),
  ...groups.flatMap(([a, b]) => [...u32(a), ...u32(b), ...u32(1)])];
const name = [..."OCProbe"].flatMap((c) => u16(c.charCodeAt(0)));
const tables = {
  "OS/2": [...u16(4), ...i16(0), ...u16(400), ...u16(5), ...u16(0), ...Array(20).fill(0), ...i16(0), ...Array(10).fill(0),
    ...Array(16).fill(0), ..."NONE".split("").map((c) => c.charCodeAt(0)), ...u16(0x40), ...u16(0x20), ...u16(0xffff),
    ...i16(800), ...i16(-200), ...i16(0), ...u16(800), ...u16(200), ...Array(8).fill(0), ...i16(0), ...i16(0), ...u16(0), ...u16(0x20), ...u16(0)],
  cmap: [...u16(0), ...u16(1), ...u16(3), ...u16(10), ...u32(12), ...cmapSub],
  glyf: [...i16(0), ...Array(8).fill(0), ...u16(0)],
  head: [...u32(0x10000), ...u32(0x10000), ...u32(0), ...u32(0x5f0f3cf5), ...u16(0x000b), ...u16(1000), ...Array(16).fill(0),
    ...i16(0), ...i16(0), ...i16(0), ...i16(0), ...u16(0), ...u16(8), ...i16(2), ...i16(0), ...i16(0)],
  hhea: [...u32(0x10000), ...i16(800), ...i16(-200), ...i16(0), ...u16(0), ...i16(0), ...i16(0), ...i16(0), ...i16(1), ...i16(0), ...i16(0),
    ...Array(8).fill(0), ...i16(0), ...u16(2)],
  hmtx: [...u16(0), ...i16(0), ...u16(0), ...i16(0)],            // advance 0 cho cả hai glyph
  loca: [...u16(0), ...u16(0), ...u16(6)],
  maxp: [...u32(0x10000), ...u16(2), ...u16(0), ...u16(0), ...u16(0), ...u16(0), ...u16(2), ...Array(16).fill(0)],
  name: [...u16(0), ...u16(1), ...u16(18), ...u16(3), ...u16(1), ...u16(0x409), ...u16(1), ...u16(name.length), ...u16(0), ...name],
  post: [...u32(0x30000), ...u32(0), ...i16(-100), ...i16(50), ...Array(20).fill(0)],
};
export function probeFont() {
  const tags = Object.keys(tables).sort(), n = tags.length;
  const es = Math.floor(Math.log2(n)), sr = 2 ** es * 16;
  const head = [...u32(0x10000), ...u16(n), ...u16(sr), ...u16(es), ...u16(n * 16 - sr)];
  let offset = 12 + 16 * n; const dir = [], body = [];
  for (const t of tags) {
    const data = [...tables[t]]; while (data.length % 4) data.push(0);
    let sum = 0; for (let i = 0; i < data.length; i += 4) sum = (sum + ((data[i] << 24) | (data[i+1] << 16) | (data[i+2] << 8) | data[i+3])) >>> 0;
    dir.push(...[...t].map((c) => c.charCodeAt(0)), ...u32(sum), ...u32(offset), ...u32(tables[t].length));
    body.push(...data); offset += data.length;
  }
  return Uint8Array.from([...head, ...dir, ...body]);
}
```

Phép phát hiện trong prototype, mỗi code point:

```js
missing = w(`14px "Noto Sans", "OC Glyph Probe"`, ch) === 0 && w(`14px "Noto Sans"`, ch) > 0
```

Chỉ chạy sau đường nhanh so cả chuỗi.
