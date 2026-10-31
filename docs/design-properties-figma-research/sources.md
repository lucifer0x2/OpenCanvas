# Nguồn chính thức và giới hạn chứng cứ

Tất cả URL dưới đây được đọc bằng web trong phiên nghiên cứu ngày **2026-10-30** (Asia/Ho_Chi_Minh). Ngày này là **ngày truy cập**, không phải ngày phát hành của từng API. Tài liệu online có thể đổi; bảng giữ chính semantics đã dùng trong báo cáo. Không dùng bài tổng hợp làm chứng cứ kỹ thuật.

## Figma Plugin API — text

| Nguồn | Điều dùng để đối chiếu |
| --- | --- |
| [TextNode](https://developers.figma.com/docs/plugins/api/TextNode/) | Whole text và ranges; căn ngang/dọc, textCase/decoration, fontWeight read-only; không đồng nhất mixed với thiếu hỗ trợ |
| [LetterSpacing](https://developers.figma.com/docs/plugins/api/LetterSpacing/) | Tagged value PIXELS/PERCENT |
| [LineHeight](https://developers.figma.com/docs/plugins/api/LineHeight/) | PIXELS/PERCENT hoặc AUTO, không gán auto=120% |
| [textAutoResize](https://developers.figma.com/docs/plugins/api/properties/TextNode-textautoresize/) | NONE fixed, HEIGHT auto-height/wrap, WIDTH_AND_HEIGHT không soft-wrap; TRUNCATE deprecated |
| [textTruncation](https://developers.figma.com/docs/plugins/api/properties/TextNode-texttruncation/) | DISABLED/ENDING; resize cùng maxHeight/maxLines ảnh hưởng khi nào truncate |
| [maxLines](https://developers.figma.com/docs/plugins/api/properties/TextNode-maxlines/) | Chỉ với ENDING, giá trị≥1, null bỏ giới hạn |
| [paragraphSpacing](https://developers.figma.com/docs/plugins/api/properties/TextNode-paragraphspacing/) | Paragraph-level range style; không là spacing của mọi soft line |
| [textWrapStyle](https://developers.figma.com/docs/plugins/api/properties/TextNode-textwrapstyle/) | Per-paragraph wrap style; khác khả năng nowrap/auto-width |
| [FontName](https://developers.figma.com/docs/plugins/api/FontName/) | Family/style và variable variationSettings; named-instance axis defaults, axis không tồn tại bị lỗi |
| [StyledTextSegment](https://developers.figma.com/docs/plugins/api/StyledTextSegment/) | Inclusive start/exclusive end và attributes của style ranges; báo cáo chọn UTF-16 cho OC là draft riêng |
| [openTypeFeatures](https://developers.figma.com/docs/plugins/api/properties/TextNode-opentypefeatures/) | Feature controls khác letter spacing; không đề xuất copy toàn interface |
| [TextStyle](https://developers.figma.com/docs/plugins/api/TextStyle/) | Text style chứa nhiều typography fields hơn style subset OC |
| [Working with Text](https://developers.figma.com/docs/plugins/working-with-text/) | Mixed styles/loading/missing fonts phải phân biệt; load font trước khi sửa layout-affecting props |

## Figma Plugin API — layout, paint, components, variables

| Nguồn | Điều dùng để đối chiếu |
| --- | --- |
| [FrameNode](https://developers.figma.com/docs/plugins/api/FrameNode/) | Auto-layout/sizing/padding/clip container; shape và computed bounds khác nhau |
| [layoutMode](https://developers.figma.com/docs/plugins/api/properties/nodes-layoutmode/) | NONE/HORIZONTAL/VERTICAL/**GRID**, default NONE; auto-layout đổi positions/sizes |
| [layoutGrow](https://developers.figma.com/docs/plugins/api/properties/nodes-layoutgrow/) | Chỉ0/1, primary-axis stretch; không arbitrary CSS flex-grow |
| [layoutAlign](https://developers.figma.com/docs/plugins/api/properties/nodes-layoutalign/) | Default INHERIT; STRETCH, MIN/CENTER/MAX deprecated cho counter alignment |
| [primaryAxisAlignItems](https://developers.figma.com/docs/plugins/api/properties/nodes-primaryaxisalignitems/) | MIN/MAX/CENTER/BETWEEN/EVENLY/AROUND; extra spacing distribution |
| [counterAxisAlignItems](https://developers.figma.com/docs/plugins/api/properties/nodes-counteraxisalignitems/) | MIN/MAX/CENTER/BASELINE, baseline chỉ horizontal |
| [layoutWrap](https://developers.figma.com/docs/plugins/api/properties/nodes-layoutwrap/) | NO_WRAP default; WRAP HORIZONTAL/VERTICAL; cross-axis spacing liên quan |
| [layoutPositioning](https://developers.figma.com/docs/plugins/api/properties/nodes-layoutpositioning/) | AUTO default; ABSOLUTE không tham gia flow trong auto-layout |
| [relativeTransform](https://developers.figma.com/docs/plugins/api/properties/nodes-relativetransform/) | Unit axes, không node scaling; container-parent/group semantics và rotation khác width/height |
| [Paint](https://developers.figma.com/docs/plugins/api/Paint/) | Gradients/stops/transform; image là paint, scale modes FILL/FIT/CROP/TILE; không map OC fit theo tên giống nhau |
| [Effect](https://developers.figma.com/docs/plugins/api/Effect/) | Inner/drop shadow, offset/blur/spread, layer/background blur; không tự coi CSS shadow mọi ca bằng Figma |
| [VectorNode](https://developers.figma.com/docs/plugins/api/VectorNode/) | Paths và vector networks; subset icon paths khác editor vector đầy đủ |
| [InstanceNode](https://developers.figma.com/docs/plugins/api/InstanceNode/) | Main component, overrides/swap/componentProperties; OC P24 chỉ subset |
| [Working with Component Properties](https://developers.figma.com/docs/plugins/working-with-component-properties/) | BOOLEAN/TEXT/INSTANCE_SWAP/VARIANT/SLOT; variant sets khác interaction paint states |
| [Variable](https://developers.figma.com/docs/plugins/api/Variable/) | valuesByMode không auto resolve aliases; resolveForConsumer; codeSyntax WEB/ANDROID/iOS |
| [VariableCollection](https://developers.figma.com/docs/plugins/api/VariableCollection/) | Variables chung collection modes, defaultModeId; OC themes chưa tương đương multi-collection |
| [resolvedVariableModes](https://developers.figma.com/docs/plugins/api/properties/nodes-resolvedvariablemodes/) | Resolved mode theo node và collection; không là một theme string toàn project |

## Figma REST và Help Center

| Nguồn | Điều dùng để đối chiếu |
| --- | --- |
| [REST node types](https://developers.figma.com/docs/rest-api/file-node-types/) | FRAME corners/strokes/min/max/aspect; TEXT characters/styleOverrideTable/characterStyleOverrides. Plugin và REST không cùng representation |
| [REST property types](https://developers.figma.com/docs/rest-api/file-property-types/) | TypeStyle letterSpacing px, lineHeightPx và đơn vị; LayoutConstraint, BlendMode, MaskType |
| [Explore text properties](https://help.figma.com/hc/en-us/articles/360039956634-Explore-text-properties) | Tracking khác kerning; line height % theo font size; paragraph spacing px; balance/pretty và cảnh báo browser line breaks có thể khác Figma |
| [Add right-to-left text](https://help.figma.com/hc/en-us/articles/4972283635863-Add-right-to-left-text) | Bidi auto direction và paragraph overrides; font fallback khác font load success |

## Chuẩn frontend làm cơ sở mapping

| Nguồn chính | Vai trò / giới hạn |
| --- | --- |
| [CSS Text3](https://www.w3.org/TR/css-text-3/) | Whitespace, line breaking, alignment, transformation và tracking; bản CR Draft được trang công bố, không phải mọi browser đã giống nhau |
| [CSS Fonts4](https://www.w3.org/TR/css-fonts-4/) | Font matching/kerning/features/variations; Working Draft, dùng để phân biệt semantics, không làm compatibility table browser |
| [WHATWG Canvas2D text styles](https://html.spec.whatwg.org/multipage/canvas.html#text-styles) | letterSpacing nhận length, default0px; direction/baseline/font state. Browser capability thực được probe riêng |

Không suy runtime browser support từ một bảng tương thích nhúng có thể cũ; phiên này chỉ xác minh canvas letterSpacing tồn tại trong Chromium156.0.8078.4. Không ghi Firefox/Safari pass.

## Chứng cứ repo và provenance

- Source revision: `8931abd0ad5de2538d433079f7200f65eaf11d03`, branch `codex/p24-components-instances`.
- Yêu cầu đọc: `PLAN.md`, `README.md`, `docs/ACCEPTANCE.md`, `AGENTS.md`, `docs/agents/LOG.md`; RTK tại đường dẫn người dùng đã chỉ định. Bảng trạng thái cũ được đối chiếu lại source, không được coi là code đã triển khai.
- Task sources: `tasks/01`–`08` cho authoring/context/layout/read workflow; `tasks/09`–`13` nhận diện hướng Rust đã gác lại; `docs/recommend-features/README.md` và P10a/b/c, P04b/P17/P08b/P09a–c/P11a/P12b/P14/P24/P27/P28/P29 liên quan. Link truy cập task theo [audit](audit.md) và [roadmap](roadmap.md).
- [Probe source](probe-text.mjs) + [kết quả cuối](probe-text-results.json): Node24.21.0, Chromium156.0.8078.4, macOS; font/source hashes, widths/baselines/whitespace/tracking. Script không tạo user project và không sửa bundle sản phẩm.
- Không chạy/copy lại các script cũ từ task rồi ghi nhận như kết quả mới. Bằng chứng Linux, pixel và phút/byte cũ được gọi là lịch sử khi nhắc đến.
- Kiểm tài liệu cuối: `node --check docs/design-properties-figma-research/probe-text.mjs` exit0; Node kiểm7 Markdown files,47 local links tồn tại,59 source references có file/dòng hợp lệ, không trailing whitespace và hash measurement khớp probe; `git diff --check` exit0. Kiểm link này không thay nghiệm thu semantics hay HTTP health mọi URL.

## Phần chưa kiểm và câu hỏi cần nghiên cứu tiếp

1. QueryNest thật, capture-characters script và font/styles trên Air: chỉ có báo cáo chuyển giao, chưa tự truy cập.
2. Runtime Docker/Linux font hinting, fallback glyph/emoji fonts và pixel parity giữa OS: chưa probe.
3. Figma app output/API response từ một file mẫu: chưa có; nghiên cứu semantics public docs, không đo pixel Figma so Chrome.
4. Preview/SVG pixel equivalence cho feature hiện có: audit consumer code, chưa chạy render regression ở phiên này.
5. Rich runs/RTL/variable axes/render byte budgets: mới proposal + fixtures; chưa spike hay benchmark, chưa hứa tốc độ/chính xác.
6. Ước lượng implementation: chưa có; chỉ thứ tự/phạm vi/dependencies/AC. Cần rebase task và chốt product scope trước khi ước lượng.

Một số URL property suy theo tên không truy cập được qua web tool (letterSpacing/lineHeight property pages, nodes-clipscontent, nodes-opentypefeatures). Đã dùng **data-type pages, FrameNode và link TextNode-opentypefeatures chính thức** thay thế. Không lấy lần mở thất bại làm chứng cứ semantics. Báo cáo không yêu cầu đăng nhập hay cài Figma plugin.
