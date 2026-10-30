# Typography: ma trận và thiết kế đề xuất

Mốc code và evidence: [audit](audit.md). Ngày đọc nguồn Figma/web: **2026-10-30**. Các schema ở chương này là **draft**, chưa phải API được hỗ trợ.

## 1. Ma trận Figma ↔ OpenCanvas ↔ frontend

| Khả năng | Figma chính thức | OpenCanvas hiện tại | CSS/frontend và hướng chọn |
| --- | --- | --- | --- |
| Nội dung + mixed style | TextNode có character ranges; [StyledTextSegment](https://developers.figma.com/docs/plugins/api/StyledTextSegment/) mang start inclusive/end exclusive và style | Plain string, một style/node; chưa runs | Một paragraph với spans khi style khác; không gán x/y cho mỗi ký tự |
| Tracking px/% | [LetterSpacing](https://developers.figma.com/docs/plugins/api/LetterSpacing/) là `{value, unit:PIXELS\|PERCENT}` | Chưa có; P10b px đang chờ | CSS letter-spacing dùng length, không `%`; Figma p% → p/100 em → fontSize×p/100 px; không lấy % của container |
| Tracking âm/em | Thang tương đối và px không nên nhập nhằng với kerning; [Help](https://help.figma.com/hc/en-us/articles/360039956634-Explore-text-properties#letter-spacing) phân biệt tracking/kerning | Chưa có; signed number token là hướng P10b | `.17em=17%`, `−.055em=−5.5%`; px resolve theo size. Giữ relative unit có lợi cho style dùng nhiều cỡ chữ |
| Line height | [LineHeight](https://developers.figma.com/docs/plugins/api/LineHeight/) có PIXELS/PERCENT/AUTO | Bắt buộc px1…10000 hoặc dimension ref; chưa auto/relative | `line-height:20px`, `1.5` hoặc150%; auto không mặc định coi bằng120% |
| Paragraph spacing/indent | [paragraphSpacing](https://developers.figma.com/docs/plugins/api/properties/TextNode-paragraphspacing/) là paragraph-level; [Help](https://help.figma.com/hc/en-us/articles/360039956634-Explore-text-properties#paragraph-spacing) xác nhận px | LF có dòng mới, chưa spacing/indent | Khoảng cách giữa paragraphs; không thay tất cả bằng line-height. Paragraph DOM/margins cần tránh margin-collapse khác nghĩa |
| Căn ngang | [TextNode](https://developers.figma.com/docs/plugins/api/TextNode/) có LEFT/CENTER/RIGHT/JUSTIFIED | Chưa có, mọi line.x0 | P10b left/center/right trước; start/end cùng direction ở phase quốc tế hóa; justify chưa cần |
| Căn dọc | [TextNode](https://developers.figma.com/docs/plugins/api/TextNode/) TOP/CENTER/BOTTOM trong textbox | Chưa có; stack align chỉ dời box | `textAlignVertical` là offset block lines trong fixed-height box; không map CSS vertical-align trực tiếp |
| Fixed box | [textAutoResize NONE](https://developers.figma.com/docs/plugins/api/properties/TextNode-textautoresize/) độc lập content | width/height numeric có thật; text có thể vượt bounds | `width/height` fixed; overflow/clip là quyết định khác, không ngầm truncation |
| Auto height | [textAutoResize HEIGHT](https://developers.figma.com/docs/plugins/api/properties/TextNode-textautoresize/) width fixed, height theo wrapped content | Numeric/fill width + hug height đã có | Dùng paragraph width đã chốt, height auto; đây là fixture chính |
| Auto width | [WIDTH_AND_HEIGHT](https://developers.figma.com/docs/plugins/api/properties/TextNode-textautoresize/) không soft-wrap | Hug có thể bị available width giới hạn và wrap | P10a thêm nowrap riêng; hug đơn thuần chưa tương đương auto-width Figma |
| Wrap policy | [textWrapStyle](https://developers.figma.com/docs/plugins/api/properties/TextNode-textwrapstyle/) áp trong paragraph | Word splitting tự viết; grapheme split từ dài | Tách `textWrap` wrap/nowrap khỏi `overflowWrap` và whitespace. Balance/pretty để sau; Figma [cảnh báo breaks có thể khác browser](https://help.figma.com/hc/en-us/articles/360039956634-Explore-text-properties#wrap-style) |
| Ellipsis/maxLines | [textTruncation](https://developers.figma.com/docs/plugins/api/properties/TextNode-texttruncation/) DISABLED/ENDING; [maxLines](https://developers.figma.com/docs/plugins/api/properties/TextNode-maxlines/) chỉ tác dụng khi ENDING | Chưa có; clip chỉ cắt paint | text-overflow một dòng cần width/nowrap/overflow; nhiều dòng cần measured line clamp; không giả bằng sửa authored text |
| Font weight/style | [FontName](https://developers.figma.com/docs/plugins/api/FontName/) family/style; TextNode fontWeight read-only | Font manifest có weight/style nhưng node chỉ chọn fontId | P10c chọn mặt có thật, map family/weight/style; không coi weight metadata là font bytes đúng |
| Variable fonts | [FontName variationSettings](https://developers.figma.com/docs/plugins/api/FontName/#variationsettings) có axis values; axis thiếu giữ named-instance default | Chưa có axes/ranges; P12b chờ | `font-weight`, optical sizing, rồi font-variation-settings khi cần; không thay weight bằng scaleX |
| Decoration/case | [TextNode](https://developers.figma.com/docs/plugins/api/TextNode/) có textDecoration/textCase | Chưa có, nguồn thường phải uppercase sẵn hoặc rect gạch | P10b underline/line-through và transform, giữ source text; advanced skip-ink/thickness sau |
| Shaping/kerning/features | [OpenType features](https://developers.figma.com/docs/plugins/api/properties/TextNode-opentypefeatures/); [CSS Fonts](https://www.w3.org/TR/css-fonts-4/) phân biệt kerning/ligatures/variations | Browser shape cả line và fontKerning normal; chưa controls/features | Đủ một phần; tránh per-character drawing và cộng width độc lập; feature settings chỉ sau spike |
| Fallback/Unicode | [Working with Text](https://developers.figma.com/docs/plugins/working-with-text/) phân biệt missing fonts; [RTL Help](https://help.figma.com/hc/en-us/articles/4972283635863-Add-right-to-left-text#font-fallback) mô tả fallback font | Font bytes load có kiểm; glyph coverage/fallback không xác nhận | P11a cảnh báo coverage; sau đó fallback chain đã ghim, không hứa font hệ thống bất kỳ |
| Bidi/RTL | [Figma RTL](https://help.figma.com/hc/en-us/articles/4972283635863-Add-right-to-left-text) tự nhận hướng, có override per paragraph | Canvas có shaping browser nhưng không authored direction/lang, wrapper chưa bidi-aware | Phase riêng direction auto/ltr/rtl + lang, logical align; không đảo chuỗi hay sibling order để giả RTL |

Figma REST có representations khác Plugin: [TypeStyle](https://developers.figma.com/docs/rest-api/file-property-types/#typestyle) trả letterSpacing **px**, lineHeightPx/lineHeightPercentFontSize/lineHeightUnit; lineHeightPercent cũ đã deprecated. [TEXT node](https://developers.figma.com/docs/rest-api/file-node-types/#text) dùng characterStyleOverrides + styleOverrideTable. Không copy nguyên Plugin union vào REST importer và không biến character-style entries thành character nodes.

Không suy ra defaults của mọi Figma property từ tên enum. Tài liệu cụ thể xác nhận nghĩa nhưng không luôn ghi default; OpenCanvas phải công bố default riêng. Defaults OpenCanvas draft bên dưới không được gán ngược cho Figma.

## 2. P10a: độ trung thực trước thuộc tính mới

**UI hưởng lợi:** shortcut, nav label, h1, mô tả card và paragraph fixed-width/hug-height. Bất kỳ baseline sai đều ảnh hưởng mọi thiết kế, kể cả có đủ font/size/lineHeight.

Giữ contract một owner tính lines trong Chromium worker; Rust chỉ dùng cache đã đo. Không để preview hoặc viewer wrap lại độc lập. Sửa/chốt ba trách nhiệm tách được:

- `text-metrics`: width quantization, ascent/descent/leading, ink bounds nếu thêm; phân biệt advance width với ink rect. Dùng prototype P10a làm giả thuyết, kiểm lại font/runtime hiện tại trước khi chốt công thức.
- `text-wrap`: whitespace, forced break, word/grapheme boundaries và nowrap; NBSP/U+202F không collapse như U+0020. Chính sách từ quá dài là quyết định riêng: overflow-wrap:anywhere khác CSS normal.
- `measurement`: orchestration request→metrics→lines, không chứa tất cả Unicode/font policy. Không cần tách hình thức nếu module vẫn ngắn và một trách nhiệm.

P10a đã đề nghị `textWrap:"wrap"|"nowrap"`, default wrap. Retain existing wrapping cho dữ liệu cũ trong lần đầu; khi sửa whitespace sai cần release note/before-after fixture vì lines/height có thể đổi dù schema không đổi. `nowrap` vẫn tôn trọng forced LF; width numeric không tự mở box, text có thể overflow. Hug+nowrap dùng max-content, không lấy khoảng còn lại làm soft wrap.

**Tương thích:** fontId/style/token cũ giữ nguyên; measurement correction có thể đổi pixels/geometry cũ, không được gọi là byte-identical compatibility. Không thêm sizing keys vào P27 states. Rich runs chưa ở lát này. [Probe](audit.md#6-probe-tự-chạy-và-cách-đọc-kết-quả) xác minh baseline/NBSP; chưa xác minh toàn bộ giải pháp.

## 3. P10b: đề nghị giữ subset nhỏ nhưng làm đủ consumer

| Key draft | Schema / default / đơn vị | Resolution, scene và CSS |
| --- | --- | --- |
| letterSpacing | finite number px hoặc number-token, mặc định0; theo P10b bounds−1000…1000 | Signed number vì dimension token hiện không phù hợp giá trị âm; resolve style→node→px; CSS `<n>px`; reset canvas state mỗi request/node |
| textAlign | left/center/right, mặc định left | Tính line.x từ box width và measured line width; giữ raw source text; không set align-items để thay thế |
| textDecoration | none/underline/line-through, mặc định none | Scene giữ decoration geometry hoặc contract metric đủ rõ; paint canvas/preview/SVG cùng cách đặt; CSS text-decoration là starting mapping, không hứa nét bằng font metric khác |
| textTransform | none/uppercase/lowercase/capitalize, mặc định none | Transform trước measure/wrap, authored text không đổi; xử lý chữ đổi độ dài như ß→SS, giữ mapping source nếu sau này runs |

Không thêm `kerning` làm alias đồng nghĩa tracking trong hướng dẫn: P10b hiện liệt kê nó như suggestion; cần chỉnh ngữ nghĩa để agent không tưởng đây là điều khiển cặp glyph. Letter spacing không thay font kerning normal. [WHATWG](https://html.spec.whatwg.org/multipage/canvas.html#dom-context-2d-letterspacing) định nghĩa canvas letterSpacing là CSS length, default0px, giá trị không parse được bị bỏ qua: kiểm capability trước, không im lặng bỏ spacing.

**Đơn vị tương đối:** đề nghị ship P10b px trước theo task đã có; author adapter chuyển `.17em` bằng fontSize đã resolve. Nhãn14px tracking .17em →2.38px; build-tag .09em →1.26px **nếu fontSize14**; h1−.055em →−3.52px **nếu fontSize64**. Đây là ví dụ quy đổi, không xác minh cỡ chữ QueryNest.

Nếu người dùng cần typography style scale cùng fontSize, mở rộng chính P10b bằng union `number | {value:number,unit:"em"|"percent"} | {tokenId}`; không tạo một task tracking cạnh tranh. number giữ nghĩa px. `percent` lấy theo effective fontSize của run, không theo viewport. Với relative token, lưu number ref trong `value` (phải chốt explicit schema) và unit bên ngoài; không đổi nghĩa number token hiện tại. Đây là phạm vi rộng hơn draft P10b vốn cấm em/%.

Ưu điểm relative: bảo toàn intent design system. Nhược: phải thêm branch vào resolve_value, style validator, canonicalization/cache, compact readback và diagnostics. Không tự mở union chỉ để cover một screenshot có fontSize cố định.

**Tương thích P24/P27:** styleId override đã đủ đổi cả typography style; chưa cho instance patch trực tiếp letterSpacing/runs. Nếu muốn mở patch phải cập nhật OVERRIDE_KEYS, validate master/descendant và default byte budget riêng. P27 giữ paint-only; metrics properties không vào states.

## 4. P10c và P11a: chọn đúng font, biết khi nào font chưa đủ

- P10c hiện có draft chọn mặt exact `(family,weight,style)`, default400/normal khi chọn family, fontId cũ vẫn chạy; cấm authored fontId+fontFamily đồng thời, node thắng style theo luật đã ghi. Không synthetic bold/italic, không đoán weight gần nhất.
- Font record metadata đang có, nên static face selection có thể resolve thành fontId mà không mở SceneNode. Nhưng kiểm compact referenced resources phải gom **resolved face**, không chỉ font authored; kiểm context/style/instance và font manifest cache.
- Sửa AC P10c: nhiều records trỏ **cùng bytes Noto** chỉ kiểm selection/metadata; muốn xác nhận chữ bold/italic phải có font files thật khác hash. Không coi mock metadata là visual proof.
- P11a nên dựa font cmap/coverage để báo unsupported code points, rồi probe shaped clusters; width comparison hoặc document.fonts.check không đủ xác nhận glyph nguồn. Emoji ZWJ/variation selectors/combining marks có thể cần cluster-level diagnostics; thiếu mapping một code point không đồng nghĩa cluster hỏng.
- Diagnostic đề nghị ghi nodeId/fontId + phạm vi source/cluster bị nghi, không dump toàn text mặc định. Chính sách mặc định **warn có bằng chứng**; strict export chỉ fail khi resource thật không sẵn sàng hoặc user bật strict coverage sau khi capability ấy có thật.
- P12b variable wght/opsz sau P10c: phải đưa effective axes/optical policy vào font identity hoặc measure request. Một fontId/hash bytes không đủ phân biệt hai axis settings. Giữ named-instance semantics rõ; custom axes phase sau. Đây là điều cần kiểm trong task P12b, không mặc định task đã giải đúng mọi consumer.

## 5. Phase tiếp: textbox và rich text có nghĩa

**Textbox extension draft:** `textAlignVertical:top|center|bottom` defaulttop; `textOverflow:visible|clip|ellipsis` defaultvisible; `maxLines` absent nghĩa không giới hạn; `paragraphSpacing` px/dimension-token≥0 default0. Text phải giữ source toàn bộ khi ellipsis. Effective visible lines là giao giữa maxLines và fixed/maxHeight; clipping không tự thêm dấu. Với hug-height, maxLines giới hạn measured height khi overflow ellipsis; không sửa authored height. Căn dọc tính trên block lines còn hiển thị; oversized block căn top để tránh âm khó hiểu (cần chốt so với Figma nếu muốn unsafe center).

`lineHeight` relative/auto và paragraph indent không ưu tiên trước core correctness. Cần quyết định LF là paragraph break hay hard line break riêng trước khi spacing; dữ liệu cũ LF tiếp tục khoảng cách lineHeight. Nếu thêm paragraphSpacing, ghi rõ spacing chỉ giữa đoạn, không cuối đoạn cuối. Không bịa auto=1.2 vì font metrics khác.

**Rich runs draft** cho câu có link/in đậm/accent giữa dòng và heading đổi màu: giữ một node text, `text` là canonical string, `textRuns:[{start,end,styleId?,fontId?,fontSize?,fill?,...}]` là các khoảng không chồng lấn; default[] lấy base style. Offset chuẩn dự kiến **UTF-16** để hợp JS/DOM; Rust phải chuyển đổi có kiểm, reject boundary giữa surrogate pair hoặc grapheme khi style tách cluster không có nghĩa. Giới hạn số runs phải đo trước, không sao chép budget node2048 thành budget runs.

Nên bắt đầu runs chỉ fill/face/style, chưa lists/inline images/arbitrary nested markup. Kế thừa base→run style→run literal; text replacement kèm runs phải atomic. Update text mà runs hết range phải lỗi rõ hoặc yêu cầu gửi cả runs, không ngầm dời indices. P24 override text hiện tại có thể làm master runs sai indices: cần luật **text override xóa runs kế thừa hoặc bắt override cả text+textRuns**, chốt trước khi giao task. P27 không override metrics/runs.

**Spike bắt buộc:** một paragraph layout có mixed runs, bidi và ligatures. Đo từng run độc lập rồi cộng advance là chưa đủ; runs có cùng shaping properties nên shape chung dù khác màu. Có thể đo DOM paragraph + Range fragments rồi phát scene fragments/source ranges, nhưng phải kiểm font/runtime và line wrapping parity với pipeline trước khi khóa. Không quay lại “một authored node mỗi glyph”. Scene có fragments theo line/run/cluster khi cần; coder vẫn nhận source+styles, không nhận hàng trăm tọa độ làm cấu trúc thiết kế.

Rủi ro rich text lớn hơn bốn props P10b: scene schema mới, resource closure đa font, grapheme/source mapping, cache byte cost và SVG shaping. Đây là phase sau có spike và quyết định phạm vi theo PLAN (rich text hiện ngoài scope), chưa tạo handoff triển khai trong nghiên cứu này.

## 6. Unicode/RTL và công bố khả năng

Không gọi hiện trạng là “không hỗ trợ Unicode”: UTF-8 plain strings, browser shaping và grapheme split đã tồn tại. Cũng không gọi là “đã hỗ trợ đầy đủ RTL”: whitespace wrapper, line direction/align và fallback chưa được mô hình hóa. [CSS Text](https://www.w3.org/TR/css-text-3/) phân biệt whitespace, line breaking, alignment và transform; đây là các semantics cần tách ở schema/testing.

Phase RTL chỉ công bố sau fixture có Arabic/Hebrew + số/Latin/dấu câu, font đã ghim; direction/lang trong authored+request+scene+SVG/preview; logical start/end. Không reverse text, không tách từng ký tự nối Arabic, không normalize Unicode làm mất source khi chưa có yêu cầu. Vietnamese NFC/NFD và emoji graphemes là fixture ngay trong P10a/b dù chưa mở RTL authored.
