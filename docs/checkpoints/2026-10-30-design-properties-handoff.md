# Bàn giao 5 nhóm thuộc tính — dừng theo yêu cầu người dùng

> **Đã tiếp tục và nghiệm thu ở phiên sau cùng ngày** — trạng thái cuối ở [implementation](../DESIGN-PROPERTIES-IMPLEMENTATION.md) và `docs/agents/LOG.md`.

Ngày 2026-10-30 (Asia/Ho_Chi_Minh). Người dùng báo hết limit và yêu cầu dừng,
lưu tiến độ để phiên khác tiếp tục. Root đã interrupt cả ba worker còn chạy;
không giao việc tiếp, không chạy thêm build/tests sau yêu cầu dừng.

## Checkout và quy tắc tiếp tục

- Workspace: `/Users/luis/Workspace/OpenCanvas`.
- **Trạng thái thực lúc dừng: `main` @ `6ccf1913704e7b9bec1172cadfd6cf0243bf6766`**,
  merge PR5 từ `codex/p24-components-instances`. Đầu phiên là nhánh P24 @
  `8931abd0ad5de2538d433079f7200f65eaf11d03`; root không thực hiện việc chuyển
  nhánh/merge này. Đọc lại status trước khi tiếp tục, không checkout/reset/stash.
- Rất nhiều thay đổi tracked/untracked còn trên đĩa. **Chưa commit/push phần này**.
  Không xóa các file untracked: có source mới, fonts/fixtures và nghiên cứu phiên khác.
- Đọc `AGENTS.md`, `PLAN.md`, [phạm vi đã duyệt](../DESIGN-PROPERTIES-IMPLEMENTATION.md)
  và [nghiên cứu Figma](../DESIGN-PROPERTIES-FIGMA-RESEARCH.md).
- Root chỉ điều phối/kiểm chứng/tài liệu; mọi sửa mã giao fresh subagent, một task
  mỗi agent, `fork_turns="none"`, không tái sử dụng worker cũ, không spawn đệ quy.
- Không ghi project/demo vào `~/.opencanvas`, không đụng port3847/volume/Docker,
  không restart server thật. Tests dùng data root tạm/random ports và dọn process.
- Giữ engine Chromium; không mở rich text/variable axes/blur/vector/responsive %.
- Bản tổng hợp chưa phát hành dùng **`scene-3` / `web-4`** cho cả text và paint;
  không cần bump mỗi lane một lần. Build đồng bộ trước khi sử dụng.

## Đã có từ task font trước đó

Font UI/API import/download, original bytes + SHA, hash CSS identity và SVG nhúng
font đã hoàn tất/kiểm độc lập ở phiên trước. Noto Sans TTF Google đã bundled sẵn;
Lato Regular/Bold/Italic có OFL và pinned provenance trong `fixtures/fonts/README.md`.
Không tự thêm font vào project thật. Xem `docs/FONTS.md`, `FONT-UI-REPORT.md`,
`FONT-PORTABILITY-REPORT.md` (báo cáo cũ ghi web-3 là mốc kiểm lịch sử).

## Tiến độ 5 nhóm

### Nhóm4 — static font faces: hoàn tất

[FONT-FACE-REPORT](../FONT-FACE-REPORT.md). Exact family/weight/style, duplicate/
missing errors, không synthetic bold, fontId cũ giữ nguyên; token/theme và instance
style override; compact resources đóng đủ effective fonts theo theme/definitions.

Các file chính: storage `validate/font_face.rs`, `validate/refs.rs`; server
`reads/compact.rs`, `reads/tree.rs`; tests `font_face.rs`, `font_face_context.rs`;
`spikes/pipeline/font-face-svg-offline.mjs`. Worker kiểm storage17, instances4,
server7, SVG offline3 mặt thật, strict Clippy. Root đã kiểm độc lập **22 ca** qua
6 suites: font_face, resolution, style_aliases, prop_catalog, font_portability,
bundle_roundtrip. Root chưa chạy lại server face suite trên paint cuối.

### Nhóm1/2 — text: phần lớn code có, chưa nghiệm thu cuối

Baseline half-leading theo metrics font, ceil64 width, textWrap, NBSP/narrow NBSP/
word joiner, grapheme emergency wrap, signed px letterSpacing, align, casing giữ
source, decoration; cmap4/12 diagnostics; scene inkBounds; canvas/preview/SVG adapters.

Các module mới: layout `model/text.rs`, `text_lines.rs`; renderer `glyph-coverage.ts`,
`text-wrap.ts`, `text-transform.ts`, `paint-text.ts`; preview `textContent.ts`;
server SVG `text.rs`. Storage text helper/catalog/validation đã tích hợp faces.
Đã sửa lỗi catalog cũ của Instance: check authored keys trước resolution và dùng
instance allowlist; giữ nguyên regression test thay vì nới test.

Worker đã chạy lại text-measure **4 pass**, gồm **1.440** tổ hợp Noto/Lato × text ×
size × tracking so DOM width/baseline/đo lại own width. Layout23 và renderer tsc đã
pass trước paint. Root độc lập glyph-coverage **3 pass**. Pixel worker ghi nhận:
preview24/24 và CSS strike12/12 chính xác; underline12/12 có5–15 pixel lệch1 kênh
ở vùng glyph đè lên underline, không lệch hình học. Test đã giới hạn dung sai vào
vùng overlap, vẫn kiểm endpoint và phần khác chính xác; cần chạy lại/đọc kết quả.

`crates/server/tests/text_rendering.rs` và `spikes/pipeline/text-svg-offline.mjs`
đã được viết để kiểm SVG production offline + source/context + pixels. Có ảnh ở
`docs/text-properties-evidence/{canvas,production-svg}.png`; **root chưa xem và chưa
nhận kết quả cuối suite này**. Chưa có `docs/TEXT-PROPERTIES-REPORT.md` lúc dừng.

Giới hạn phải giữ rõ: cmap là scalar mappings, không chứng minh complex shaping/
emoji/variation sequences; WOFF/WOFF2/format chưa kiểm báo GLYPH_CHECK_UNAVAILABLE;
missing glyph chỉ advisory, chưa có pinned fallback chain. Không hứa cross-OS pixels.

### Nhóm3 — discovery/diagnostics: có báo cáo, chờ root review cuối

[DESIGN-DISCOVERY-REPORT](../DESIGN-DISCOVERY-REPORT.md) đã lưu. Resource
`opencanvas://capabilities` sinh accepted keys từ catalog, facts riêng từng consumer;
semantic author/implement policy và notes intent. Scoped expanded/resolved text
fragmentation heuristic, measured overflow evidence/provenance trước pagination;
inspect node filter, context ready viewports, UI labels cho structured diagnostics.

Worker hoàn tất đã chạy MCP5, diagnostic lib4, capability lib1, UI label1,
web typecheck và scoped format. Root chưa kiểm độc lập nhóm này. Workflow hiện
80.922B; tools array25.151B/result25.199B. Baseline65.597B chỉ là capture worker cũ
không còn raw output, **không coi là baseline được tái chứng minh**; không claim giảm
payload/token. Paint có thể tiếp tục đổi props/capabilities bytes, đo lại sau cùng.

### Nhóm5 — paint: core có, đang viết/chạy AC; chưa hoàn tất

Storage `validate/{quad,paint,shadow}.rs`, layout `model/{fill,shadow}.rs` và
`paint_tests.rs`; CSS1–4 radius/stroke shorthand, scalar radius giữ shape, scene
arrays4, gradient tuple stops `[[position,color]]`, shadow0..8 outer/inset;
tokens/styles/themes, state gradients/short arrays normalize ở `jobs/states.rs`.
Renderer mới `corners.ts`, `fill.ts`, `shadow.ts`; paint/tree/stroke extents;
preview paintStyle/stateStyles/sceneDom; SVG `geometry.rs`, `paint.rs`.

Worker báo cargo check storage/layout/server + web typecheck xanh sau nối core.
Đang làm `crates/storage/tests/paint_props.rs`, layout paint tests và
`spikes/pipeline/paint-properties.test.mjs`/`paint-fixtures.mjs`. **Chưa nhận kết quả
cuối các tests này; chưa có PAINT-PROPERTIES-REPORT. Không coi nhóm5 đã xong.**

SVG hỗ trợ geometry/gradients; shadow hiện có per-file `SVG_SHADOW_NOT_EXPORTED`,
stroke giữ `SVG_STROKE_NOT_EXPORTED`; capability ghi diagnostic. Đây là giới hạn
được phép công bố theo roadmap, không được bỏ im lặng. PNG/preview phải hỗ trợ.
Export viewport vẫn layout bounds, outer effects ở mép target có thể bị crop;
test/document rõ, không nhầm với clip lỗi của group opacity layer.

## Việc cần làm tiếp

1. Fresh worker hoàn tất text tests/report, xem hai PNG evidence, kiểm production
   SVG offline thực. Chạy lại pixel tests trên paint đã tích hợp, giữ dung sai hẹp.
2. Fresh worker hoàn tất paint AC/tests/report: storage atomic invalids/themes/
   style/instance/bundle; direct layout guards; CSS/preview scale1/2; production
   SVG gradients/radii và shadow diagnostic, group opacity/effect extents.
3. Root đã gửi hai điểm review paint cần xác minh: **SVG alpha gradients** có thật
   premultiplied như CSS không (sRGB flag chưa đủ bằng chứng); **oversized asymmetric
   radii với unequal stroke widths** có clamp base đúng CSS không. Kiểm thực,
   không chỉ assertion shape. Root cũng yêu cầu union actual line inkBounds vào
   scene-tree text extents để opacity layer không cắt text tracking âm lớn;
   có `renderer/text-extent.test.mjs` mới nhưng chưa nhận kết quả.
4. Rà source/doc/capability cuối; không còn ghi corner/shadow/gradient unsupported
   chung trong props guide nếu đã làm. State shadow giữ reject nếu chưa hỗ trợ.
5. Build đồng bộ **một lần** `npm run build` debug khi source ổn định; không khởi
   động/restart server thật. Chạy targeted regression: font_face_context,
   font_portability/export_svg, text_rendering, text_discovery/text_overflow,
   tools_budget/skill_layer/compact_context, layout lib, renderer/pipeline mới,
   web unit/typecheck/preview và font UI smoke phù hợp. Browser/loopback tests
   có thể cần require_escalated; đã được auto review cho các lượt tạm trước.
6. Root review diff thô, format/Clippy/typecheck phù hợp, kích thước modules
   (mục tiêu100–200, đánh giá250, tránh350). Chưa có kiểm cuối toàn diff/size trên
   paint mới. `AGENTS.md` lúc dừng64 dòng/7.991B. Cập nhật báo cáo tổng hợp/LOG;
   không tự commit/push nếu chưa có lệnh người dùng.

## Trạng thái dừng

Các worker `finish_text`, `finish_discovery`, `finish_paint` đã interrupt theo yêu
cầu người dùng, không được xem là finished/accepted. Không còn process cargo/
node test/server/Vite khớp đợt kiểm tạm trong snapshot process khi dừng. Root chỉ
lưu tài liệu checkpoint/LOG sau yêu cầu dừng; không tiếp tục triển khai.
