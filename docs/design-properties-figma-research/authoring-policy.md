# QueryNest, authoring và capability policy — draft

[Tóm tắt](../DESIGN-PROPERTIES-FIGMA-RESEARCH.md) · [Audit](audit.md). Chương này đề xuất sửa hướng dẫn/policy, **chưa sửa** skills hoặc validator sản phẩm.

## 1. Phân biệt ba nguyên nhân

| Loại | Dấu hiệu và bằng chứng | Cách xử lý đúng |
| --- | --- | --- |
| Thiếu property model | Catalog không có tracking/textAlign; unknown key bị reject; source `props_catalog.rs:115`, `prop_keys.rs:11` | Bổ sung semantic property xuyên pipeline, hoặc ghi hạn chế + note/fallback được chấp nhận |
| Sai measurement/render dù đủ property | Font/size/lineHeight có đủ nhưng baseline thấp; wrap NBSP sai; probe phiên này tái lập | Sửa metrics/wrapper/font policy và fixture. Không cần property y-offset để che lỗi engine |
| Agent workaround / discovery | Agent dùng nhiều node hợp lệ để bắt chước khoảng cách/line break; hướng dẫn chưa yêu cầu giữ paragraph | Sửa hướng dẫn, capability và diagnostics; không kết luận agent đã thấy/hay cố tình bỏ qua docs nếu chưa có transcript |

Một screenshot có thể chứa cả ba loại. Bounds gần đúng không chứng minh semantic model đúng, và semantic model đúng cũng chưa chứng minh PNG bằng Chrome.

## 2. Điều biết và chưa biết về QueryNest

**Do người dùng cung cấp, chưa tự xác minh trong project:** rev61 có377 text nodes/147 text một ký tự, IDs dạng `s0_n20_char0...`, tập trung welcome-copy, DATABASE WORKSPACE, h1 “Your data, without” + “the noise.” và BROWSER PREVIEW trên01 Connections. Rev50 chia theo dòng; rev58 đo rộng hơn Chrome khiến “exploring” xuống dòng; rev61 capture-characters.cjs lấy DOM Range rect từng ký tự, bỏ spaces, min box20px chồng nhau. Transcript nói “Editable character layers carry source tracking and exact DOM line breaks.” CSS nguồn trên Air: eyebrow.17em, build-tag.09em, h1−.055em; welcome-copy.02em chưa chắc selector.

Đây là một **dấu hiệu phân mảnh đáng điều tra**, chưa đủ để kết luận mọi char node sai hoặc tự gộp lại design thật. Cần source text, font file/hash/face, computed styles, DOM width/line-height/white-space và viewport nguồn để tái dựng đúng. Không suy từ tên ID ra chính xác reading order.

Vì sao thêm letterSpacing vẫn chưa đủ:

1. **Font metrics/measurement:** tracking không sửa baseline thấp, face khác, hinting/fallback, width quantization hay whitespace. Probe bundled font ở đây xác nhận baseline/NBSP lỗi độc lập tracking; không xác nhận nguyên nhân cụ thể chữ exploring ở rev58.
2. **Mất nguồn:** bỏ spaces và lưu mỗi glyph riêng mất paragraph string, word boundaries và khả năng search/copy/replace như text có nghĩa. Gộp bằng sort x/y không khôi phục chắc chắn bidi, intentional breaks hoặc spaces.
3. **Shaping:** mỗi glyph độc lập không chia sẻ kerning/ligatures/cluster shaping; probe Latin chênh tổng widths0.70px ngay tracking0. Với Arabic/emoji/graphemes rủi ro lớn hơn, chưa probe tại đây.
4. **Đóng băng layout:** Range rects phản ánh một width/runtime/font. Dòng hard-coded và glyph x/y không reflow như paragraph khi viewport/fontSize/text thay đổi.
5. **Bounds và lựa chọn:** min box20px cho glyph nhỏ có thể overlap, làm selection/highlight/hit test và overlap lint khó hiểu; glyph ink có thể đúng nhưng authored box không còn mô tả layout.
6. **Chi phí:** O(N) authored nodes/ops/IDs thay O(1) paragraph + O(k) style runs; render2048-node budget, snapshots/context/resources và sửa nội dung đều chịu thêm chi phí. Chưa đo payload rev61 nên không gán số token tiết kiệm giả.

“Editable” ở mức mỗi glyph vẫn sửa được không đồng nghĩa paragraph editable. “Source tracking” chỉ hữu ích nếu lưu được source string + styles/units + break intent; tọa độ kết quả render không thay thế các dữ liệu ấy.

## 3. Paragraph, line nodes, runs và glyph art

| Representation | Dùng khi | Giới hạn |
| --- | --- | --- |
| Một paragraph text node | Nội dung UI cùng vai trò, cùng style, cần wrap theo width | Kiến trúc hiện tại đã hỗ trợ plain paragraph; tracking/correctness cần bổ sung |
| Một node + rich runs | Đổi weight/color/link giữa câu hoặc heading accent | Chưa triển khai; phải spike shaping/source ranges, không giả bằng per-glyph authored tree |
| Nhiều paragraph nodes | Các đoạn semantic riêng, heading/body, mỗi mục list tách biệt | Là cấu trúc hợp lý, không gộp vì cùng font |
| Line nodes | Layout nghệ thuật có dòng cố ý, line animation riêng, source yêu cầu hard breaks | Chấp nhận có lý do; ghi việc breaks authored cố ý và responsive limits; không dùng vì measurement bug |
| Character/glyph nodes | Wordmark, letter animation, monogram, puzzle, typography nghệ thuật có placement riêng | Giữ exception; cluster nguyên vẹn, có note intent/source nếu cần; không cấm node ký tự đơn toàn cục |

QueryNest h1 có “the noise.” đổi màu/weight thì mục tiêu tự nhiên là một paragraph với runs hoặc hai block intentional theo source, **không tự suy đoán** từ ảnh. Đến khi runs có thật, author nên nêu limitation và chọn approximation có review; không claim faithful editable paragraph khi chỉ có positioned characters.

## 4. Draft thêm vào author-frame.md

Đặt sau authoring checklist/text sizing. Nội dung tiếng Anh dưới đây để dùng cùng skill hiện tại:

> Keep UI copy as semantic text. Use one text node per paragraph or coherent label with its full source string, including meaningful spaces and intentional hard breaks. Use a fixed/fill width and hug height for wrapping paragraphs. A measured line break is output for one viewport; do not turn it into authored line nodes just to match a screenshot.
>
> Read the property reference and consumer capabilities before approximating an unsupported text property. When tracking, alignment, mixed styles or font behavior cannot be represented, record the limitation in a node note and report the approximation. Do not invent unsupported props or silently replace a paragraph with positioned character layers to hide a measurement problem.
>
> Character or line layers are appropriate for intentional lettering, wordmarks and per-letter motion. Preserve grapheme clusters and record that intent. A single-character label, icon or monogram is not by itself an error. Keep source copy and visual fragments distinguishable; do not automatically merge existing layers without evidence of their source and intent.
>
> Verify at the intended and a second relevant width using the same revision, fonts, theme and state. Inspect text lines and visible overflow as well as pixels. Report which parts are semantic and responsive, which are intentionally fixed, and which are approximations.

**Trong thời gian P10b chưa có:** câu “read capabilities” không được liệt kê letterSpacing như đã hỗ trợ. Sau ship mới thêm ví dụ tracking thật và CSS mapping. Không thêm `sourceText` hay `textIntent` vào props khi catalog chưa có; sử dụng `create_note` đã có để ghi ngoại lệ nếu cần, không text node phụ.

## 5. Draft sửa implement-frame.md và connect instructions

- Giữ hướng dẫn đọc authored/resolved, token/style refs, referenced resources và P24 definitions. Sửa câu khẳng định baseline đã theo font metrics: giải thích line.y là **computed rendering output**, không là CSS contract; font metrics correction đang chờ P10a.
- Khi gặp nhiều line/char nodes: đọc notes và source đầy đủ nếu có; tìm grouping/intent trước khi triển khai. Không bắt coder tái dựng hàng trăm positioned spans chỉ vì chúng có tọa độ; cũng không tự gộp art/animation thật.
- Code paragraph từ source string + semantic styles/layout, dùng measured lines để **đối chiếu** một width. Nếu source mất spaces/order và không khôi phục có chứng cứ, báo limitation, không tự tạo copy mới.
- Typography sau P10b: map spacing/alignment/case/decoration rõ; sau P10c map effective face đã resolve. Notes giữ capability gaps, không trở thành yêu cầu sản phẩm mới do agent tự bịa.
- Đồng bộ README đoạn default-only với P27/P07/preview hiện có; không quảng cáo mọi state có layout/font variants. Chỉnh connect message ngắn, chi tiết ở resource để tránh tools/list phình.

## 6. Capability discovery ở mức draft

Catalog hiện là nguồn tốt cho **accepted authored keys**, nhưng thiếu câu hỏi “key này hiển thị ở đâu?”. Đề nghị mở rộng chính metadata catalog và sinh một resource nhỏ, thay vì tạo một tool gọi cho mỗi property.

Ví dụ shape đề xuất, không phải response hiện có:

```json
{
  "capabilityVersion": 1,
  "sceneVersion": "scene-2",
  "rendererVersion": "web-2",
  "properties": {
    "stroke": {
      "authored": true,
      "canvasPng": "supported",
      "preview": "supported",
      "svg": "omitted-with-diagnostic"
    },
    "letterSpacing": {"authored": false}
  }
}
```

- Đây phải được sinh/kiểm chéo với catalog và feature fixtures; không parse prose “Not supported” để importer quyết định.
- Tách static accepted feature khỏi runtime readiness: browser support/font coverage/asset readiness/job provenance. Capability JSON không chứng minh từng glyph dùng đúng font.
- Read once theo version/fingerprint, cache theo session; không lặp full capabilities trên mỗi context node. Resource mới làm tăng resources/list/read bytes, phải đo thực; thêm props thường không cần tăng operations schema.
- API write vẫn reject unknown keys với field/allowedKeys. Không biến “tool schema là generic object” thành dấu hiệu tùy ý viết CSS props.

## 7. Diagnostics và validation policy

Tái dùng P29 cho diagnostics; P11a sở hữu font coverage; P19/P25 sở hữu discovery/byte budget/docs. Các code dưới đây là **tên draft**, không có trong sản phẩm hiện tại.

| Diagnostic draft | Điều kiện có bằng chứng | Severity / output |
| --- | --- | --- |
| TEXT_FRAGMENTATION_SUSPECTED | Một nhóm sibling text rất ngắn, cùng baseline/style gần nhau, ID/order/notes gợi ý một câu bị tách; phải kết hợp nhiều tín hiệu | Warning/advisory; list vài nodeIds + counts + reason, không “invalid design” |
| TEXT_CONTENT_OUTSIDE_BOX | Measured line/ink range vượt box/visible clip đã biết | Warning với sceneKey/font hash/viewport; phân biệt intentional overflow, advance width và ink overhang |
| FONT_COVERAGE_UNCONFIRMED / MISSING_GLYPH | Cmap/shape checks có kết quả xác định; không suy từ font family name | Chỉ rõ node/font/range bị ảnh hưởng; không dump toàn source mặc định |
| UNSUPPORTED_EXPORT_FEATURE | Feature accepted nhưng export target bỏ/approximate | Diagnostic/fallback hoặc fail theo explicit strict export policy; tiếp tục giữ SVG diagnostics hiện có |

Không hard reject character nodes, negative tracking, lineHeight nhỏ hơn fontSize hoặc overflow chỉ vì chúng có thể khó đọc. Reject dành cho schema/type/range sai, reference/cycle/budget invalid và unsupported key. Warnings cần phân biệt content có chủ đích với lỗi cấu trúc; threshold fragmentation phải tune bằng fixture art/monogram/toolbar/paragraph thật, không gán ngưỡng chung tùy tiện.

Diagnostics text phải chạy trên scope đầy đủ trước pagination/property filters như NO_REFLOW_INFORMATION hiện tại; message đủ ngắn, examples có giới hạn, hỗ trợ scoped read khi cần thêm. Không quét/gửi toàn project sau mỗi op. Không tự sửa hoặc merge nodes khi warning. Tool/client instructions chỉ khuyến nghị author hành vi, không tự trở thành validator.
