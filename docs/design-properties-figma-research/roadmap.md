# Lộ trình theo lát cắt và AC tái lập

[Tóm tắt](../DESIGN-PROPERTIES-FIGMA-RESEARCH.md). Các lát dưới đây là đề xuất chuẩn bị handoff; không mở task trùng, không triển khai code sản phẩm trong phiên nghiên cứu.

## 1. Làm sớm

| Lát / backlog tái dùng | Giá trị cụ thể | Phụ thuộc và điểm bàn giao | AC độc lập |
| --- | --- | --- | --- |
| A — P10a, bổ sung P11a | H1/body/shortcut đúng baseline; biết font thiếu glyph thay vì nới width | Rebase source hiện tại trước; A-metrics và A-font diagnostics review riêng. Không phụ thuộc engine Rust mới | Paragraph fixed-width/hug-height có line list/height nhất quán giữa hai lượt đo; baseline theo oracle DOM; NBSP cases có semantics được công bố; no raw/rounded fit regression |
| B — P10b | Label tracking, empty state center, deleted row gạch ngang; bỏ character/rect giả | Sau metrics của A; B1 tracking+align, B2 decoration+case là hai lát kiểm độc lập; scalar px trước | Positive/negative/zero tracking+wrap sát biên, left/center/right multi-line, transform source intact; canvas/preview/SVG đều áp dụng hoặc diagnostic rõ |
| C — P19/P25/P29 docs/capability và lint | Agent khám phá support trước workaround, coder giữ paragraph | Draft policy có thể chuẩn bị song song A; publish instructions theo capability thật từng release. Font rule thuộc P11a | Resources/validator keys không drift; fake unsupported prop trả lỗi đúng; fragmentation fixtures thường/art cho kết quả giải thích được, không blanket reject; workflow byte baseline/after thật |
| D — P10c | Coder/author chọn đúng600 italic và style override, khỏi nhớ ID từng mặt | Sau B để tránh file text validator xung đột; static font selection không cần chờ rich runs | Exact face/duplicate/missing errors; real font bytes/hash cho visual evidence; resolved font trong compact resources, master+instance styleId override render đúng |
| E — P17→P04b | Tab/segmented control bo đúng góc | Sau base text hoặc paint lane riêng nhưng cùng catalog/version phải integrate tuần tự | Radius scalar compatibility, shorthand1–4, clamp CSS corners; stroke/clip/image/preview/SVG; P27 radius quyết định rõ, không nhận arrays rồi bỏ |
| F — P09b rồi P09a (có thể đảo khi use case cần) | Elevation/ring/inset và brand gradients, giảm fake rects | Thứ tự do shared paint/scene modules; không phụ thuộc nhau về sản phẩm | Multiple outer/inset shadows + alpha; linear/radial stops/token theme; opacity+clip+export bounds; SVG fallback/diagnostic được công bố |

A không cần chờ B/C để mang lại giá trị; B không đợi rich runs. E/F có thể dùng nhánh độc lập nhưng không giao hai worker sửa cùng catalog/model/paint mà thiếu integration owner. Trong repo này triển khai sau phải do fresh worker theo AGENTS, root điều phối; đây chưa phải lệnh bắt đầu triển khai.

**Thứ tự ưu tiên có lý do:** A/B chạm mọi text và trực tiếp loại workaround QueryNest; C bảo vệ vòng lặp agent với chi phí schema nhỏ; D ngăn đổi spacing để bù font sai; E/F loại cấu trúc giả ở UI phổ biến. Không dùng số lượng property của Figma làm thước đo tiến độ.

## 2. Phase tiếp và chưa nên làm

| Nhóm | Lát đề nghị / điều kiện mở | AC cốt lõi |
| --- | --- | --- |
| P14a→P14b→P14c | Vector path primitive→safe SVG importer→path stroke; icons là use case | Nét khi zoom/PNG2×, màu token; viewBox/fillRule; rejected markup không thực thi; export đúng subset |
| P08b | alignSelf/around/evenly khi form/toolbar cần | Exact geometry + inherit/hug/fixed rules; no-op free parent được ghi rõ; preserve old fixture |
| P12b sau P10c | Variable wght/opsz có font metadata thật | Hai weights cùng file vẫn khác measure/cache identity; pinned instance pixels/metrics; out-of-range error; preview/SVG cùng axes |
| Textbox extension | Mở rộng P10b follow-up cho vertical align/ellipsis/maxLines/paragraphSpacing; chốt semantics trước | Source không mất, ellipsis grapheme-safe; fixed box/hug height; paragraph spacing không cộng sau đoạn cuối |
| Rich text runs | Spike và quyết định mở ngoài scope PLAN; không giả glyph nodes | Một paragraph nhiều style vẫn wrap/search/edit có nghĩa; boundary/font fallback/bidi và source mapping; P24 text override không giữ ranges sai |
| P28 | Rebase percent spec trước, rồi breakpoint riêng; không gộp với text fidelity | Definite-axis/cycle validation ở storage và direct layout; computed breakpoint không mutate revision; clone code mapping rõ |
| P20/P23/P30 | Code syntax, filtered resource reads, token analysis; sau correct typography styles | Giữ refs + resolved; scoped resource payload nhỏ, không tự token hóa design khi chưa author yêu cầu |
| P09c | Blur/backdrop chỉ khi modal/glass cần thật | Clip/composite order, budget cảnh nhiều lớp, screenshot parity; unsupported SVG rõ |
| Component public props/variants, modes | Sau P24, theo design-system nhu cầu thật | Defaults/override precedence, virtual IDs, resource closure/cache map; không lẫn interaction states với size variants |

**Chưa nên làm:** toàn bộ CSS Grid/constraints/grow-shrink tùy ý, full mask/blend catalog, arbitrary OpenType controls, vector editing/boolean operations, corner smoothing, shader/glass/noise/mesh paints hoặc typography editor. Figma có các khả năng lớn hơn subset UI cần; chưa có bằng chứng lợi ích đủ cho độ phức tạp. Giữ notes/assets/raster fallback có công bố. Tasks09–13 và tối ưu engine Chromium đang gác lại, không bị kích hoạt bởi roadmap này.

## 3. Hợp đồng chung cho mọi feature ưu tiên

1. **Authored:** catalog/scopes/default/units phải rõ, style validator cùng subset. Không ép mọi default vào JSON cũ. Schema props additive không tự đòi DB migration; DB hiện4, nhưng release phải kiểm snapshot/bundle/import/old validator. Không hứa downgrade hay mixed-version writers.
2. **Resolve:** style→node và typed refs trên mọi theme; signed spacing dùng number, nonnegative lengths dùng dimension; discriminated paint objects không bị nhận nhầm typed refs. Khóa canonical resolved values ổn định để không nhân cache bởi nhiều dạng equivalent.
3. **P24:** master expansion trước resolve, virtual IDs giữ nguyên; instance root và descendant override allowlists cập nhật có chủ đích. Mặc định typography/paint mới dùng master/styleId; direct patches mở theo task riêng nếu cần.
4. **P27:** giữ paint-only sau layout. Metrics/runs/font size/weight/tracking/layout không tự vào state. Feature như radius arrays/gradient phải chốt có hỗ trợ trong state không; không silent degrade.
5. **Layout/measure:** chỉ geometry-affecting inputs vào request đúng chỗ; tất cả input đổi glyph/advance/line breaks phải vào cache identity (tracking/transform/width/white-space/face/axes/lang/direction/runs). Căn dòng có thể tính sau measure nếu chỉ offset, nhưng scene key vẫn phản ánh input.
6. **Scene/version:** baseline/rounding change ít nhất đổi renderer semantics/version và fingerprint; không nhất thiết bump scene shape khi fields không đổi. Thêm fields/types/fragments thì đánh giá bump scene version, đồng bộ Rust/TS/worker/viewer/preview/SVG. Dùng version kế tiếp thực tại lúc merge, không hardcode scene-3 từ báo cáo này.
7. **Paint/export:** PNG+viewer canvas chung renderer; preview DOM và Rust SVG writer riêng. Có fixture qua cả hai, provenance cùng revision/viewport/theme/state. Geometry+ink/effect extents/clip khác nhau: export bounds/hit testing/layer budgets phải xét shadow/blur/transforms.
8. **MCP/read:** compact differing không chèn defaults dư; referenced resources bao gồm effective fonts/styles/runs/gradient stops; instances reference-first. Không trả glyph coordinate lists trong context mặc định nếu chưa cần; inspect opt-in có pagination/source spans có giới hạn.
9. **Release:** không thay running backend/runtime files nóng rồi coi cache mới tự cập nhật. Build đủ components, restart được điều phối ở lần triển khai có quyền; phiên nghiên cứu không restart3847. Golden changes phải lý giải, không cập nhật ảnh để che regression.

## 4. Module boundaries dự kiến

| Trách nhiệm | Vùng sở hữu / tách khi cần |
| --- | --- |
| Text prop schema & validation | storage `props_catalog`, module `validate/text_props`; style/node dùng cùng helper; không dồn toàn bộ vào properties.rs |
| Face resolution/coverage | storage `font_face`/font metadata; coverage module tách khỏi layout policy; P11 diagnostics có source spans |
| Metrics/wrap/transform | renderer modules `text-metrics`, `text-wrap`, `text-transform`; measurement điều phối; giữ source/display strings riêng |
| Rust contracts/line placement | layout model types và `text_lines` helper nếu mở align/decoration/truncation; engine chỉ tổ chức compute |
| Paint/corners/effects | renderer paint helpers riêng; layers/scene-tree chỉ composite/extent; image-fit vẫn một trách nhiệm |
| Preview/SVG | Preview text/paint adapters + server svg elements riêng; shared normalized contract/fixtures, không copy toàn measurement engine sang preview |
| Capabilities/lint/read | catalog metadata + generated resource; diagnostics module tách heuristic khỏi API paging; compact resource walker giữ small API |

Mục tiêu100–200 dòng/file, >250 đánh giá tách, tránh >350 hand-written; kiểm kích thước tại mỗi handoff. Không refactor source lớn không liên quan chỉ vì nghiên cứu này.

## 5. Cache, scene và MCP byte cost

| Thay đổi | Tác động dự kiến / cái chưa biết |
| --- | --- |
| P10a correctness/nowrap | Font/text/width vẫn nền cache; thêm wrap flag vào key. Correction invalidates semantic output. Chưa đo hit rate sau sửa |
| P10b px/tracking/case | Operations props generic nên có thể giữ tools/list schema **không đổi** nếu không sửa mô tả; request/scene thêm fields O(text nodes), không O(characters). Phải đo serialized bytes thực, không gọi token savings từ RTK |
| P10c static face | Resolve ra fontId/manifest cũ có thể không cần scene shape mới; compact phải include face đã chọn. Metadata/fonts response có thể đổi theo scope |
| P09/P04 paint | Fill union/radii/shadow arrays tăng scene bytes theo feature dùng; absence omit. Shadow layers có memory/pixel cost, chưa có benchmark mới |
| Rich runs | O(style ranges) authored, O(line fragments) computed; có thể lớn với bidi/styles; cần budget riêng. Không mặc định “luôn rẻ hơn” trước fixture real design |
| Capability resource | Tăng resources/list/read, có thể cache; chưa cần tool mới. Đo whole session workflow P25, không chỉ byte tools/list |

Các số phút và tools/list delta cũ trong backlog không được lấy làm số đo ở revision này. Phiên này chỉ đo widths/baseline/whitespace ở [JSON probe](probe-text-results.json), chưa làm benchmark MCP.

## Fixture nghiệm thu

Đây là **cần chạy khi triển khai**, trừ những phần probe đã ghi rõ trong audit. Không ghi chúng pass từ nghiên cứu source.

| Fixture | Kiểm tra và oracle |
| --- | --- |
| T01 paragraph | Một text node fixed width240/320/480, hug-height; câu tiếng Việt có spaces/LF; layout lần sizing và lần scene cho cùng lines/height; parent column cũng tăng đúng |
| T02 tracking | 0,+0.5,+2.38,−0.77px và vài giá trị theo fontSize; font bundled+font thật dự án; raw source nguyên vẹn, whole-run paint; width/height phù hợp DOM oracle cùng styles |
| T03 sát biên | Lấy actual advance W từ font đã ghim; thử W−1/64,W,W+1/64 và hug đo lại; `Prefer`, `customers`, `exploring`, AV/ffi; dòng không đột biến giữa measure/cache passes |
| T04 whitespace | Space runs, tab, LF/CRLF, NBSP, U+202F, từ dài; oracle CSS white-space/overflow-wrap được **ghi tường minh**; không dùng DOM default khác policy để “chứng minh” parity |
| T05 vertical metrics | Nhiều font thật, size12/14/32/64 và lineHeight nhỏ/lớn hơn font size; baseline DOM marker + ink bounds/clip; không chỉ box height |
| T06 Unicode | Vietnamese NFC/NFD, emoji skin-tone/ZWJ/flags, combining marks; không cắt cluster. Missing glyph/fallback báo riêng và giữ font manifest/hash; không hứa emoji pixels qua OS |
| T07 mixed runs (sau spike) | Một paragraph có bold/accent/link, style boundary qua ligature candidate, source offsets UTF-16; text replacement/P24 override; source và wraps vẫn đúng, không tăng authored nodes theo glyph |
| T08 RTL (khi mở support) | Arabic/Hebrew + Latin/123/punctuation, auto/ltr/rtl, start/end; font có coverage và pinned fallback. Chưa công bố hỗ trợ thì báo test chưa trong phạm vi |
| T09 box behavior | Fixed box/auto width/auto height; nowrap+LF, ellipsis/maxLines/vertical alignment khi có; source không bị cắt; oversized minima/intentional overflow có diagnostic đúng |
| V01 consumer parity | Cùng scene+fonts: PNG1×/2×, viewer canvas, preview DOM snapshot, SVG rasterized trong Chromium; compare bounds/line lists trước rồi pixels, svg unsupported feature được diagnostic |
| D01 design system | Token theme update, text style/node override, two instances shared master, styleId patch, P27 visual state; invalid late op rollback revision/idempotency; referenced resources đủ |
| D02 author policy | Paragraph thường/char capture giả + wordmark/monogram/per-letter animation/control label hợp lệ; cảnh báo giải thích được, không false hard reject |

**Điều kiện môi trường:** lưu revision/build fingerprint, OS, Node/Playwright/Chromium exact versions, DPR/scale, font bytes/hash/face/axes, fallback và CSS oracle styles. Không dựa tên family. DOM oracle và worker phải cùng browser/runtime; Linux/Docker đo riêng nếu release phục vụ Docker.

**Tiêu chí đề nghị:** lines/source/wrap và sizing semantics phải khớp oracle đã chọn; bounds trong lưới1/64 dùng tolerance được công bố theo lượng tử (đề nghị≤1/64px khi oracle dùng cùng grid). Baseline marker nên khớp ở fixture font/runtime cụ thể. Pixels đặt ngưỡng sau spike từng consumer, không dùng một “≤0.5% toàn ảnh” che sai cả một dòng chữ; báo diff vùng text/ink edge và mismatch geometry riêng. Không hứa pixel-perfect đa nền tảng hay mọi font/browser.

**Kiểm nhắm vùng:** renderer typecheck + targeted text pipeline fixtures; storage validation/style/token/instance/state tests tương ứng; server MCP/context/export fixture liên quan; viewer/preview snapshot parity cho features bị chạm. Không chạy full suite chỉ vì nghiên cứu. Khi implementation đổi contract cần chọn checks lớn hơn theo rủi ro thực và repo rules.

## 6. Quyết định thực sự cần người dùng chốt

| Quyết định | Đề nghị mặc định | Hệ quả lựa chọn khác |
| --- | --- | --- |
| Tracking units | P10b px trước; adapter giữ note intent em/% | Native relative units trong P10b mở rộng resolver/style/schema và AC scaling, nhưng giữ design-system intent tốt hơn |
| Mức fidelity text | Geometry/wrap sát Chrome trên runtime/font đã ghim; pixel threshold theo spike | Đòi Figma và Chrome cùng breaks/pixels mọi font cần nghiên cứu khác; ngay Figma không bảo đảm browser breaks giống |
| Rich runs có mở scope PLAN không | Sau A/B/D, spike nhỏ trước; chưa mở full rich text | Nếu mixed-style paragraphs là yêu cầu ngay, đẩy spike sớm và hoãn một số paint features; không thay bằng glyph capture |
| Unsupported effect/export | Notes + diagnostic rõ; PNG khi SVG thiếu feature | Strict export fail hoặc raster embed SVG phải là lựa chọn rõ về editability/portability, không tự quyết âm thầm |
| Font fallback | Warn khi coverage có bằng chứng; strict pinned fallback là phase riêng | Fail mọi font coverage gap có thể chặn emoji/icon hợp lệ; cần font assets/fallback policy đã xác nhận |
| Responsive expansion | Giữ stacks/min/max/wrap hiện tại; P28 review riêng | Mở breakpoints/Grid/anchors thay đổi tổ hợp layout và validation; không gắn vào việc sửa letter spacing |

Không cần hỏi lại để hoàn tất báo cáo này. Những quyết định trên dùng lúc chốt task triển khai, không phải blocker giả cho nghiên cứu. Không cần người dùng chốt từng tên module hay micro-API trước khi có spike/evidence.

## 7. Liên kết backlog để tái dùng/chỉnh, không tạo bản trùng

| Nhóm | Handoff hiện có | Cần làm với tài liệu trước khi giao |
| --- | --- | --- |
| Text correctness | [P10a](../recommend-features/tasks/P10a-text-measure-fidelity.md), [P11a](../recommend-features/tasks/P11a-missing-glyph.md) | Rebase round fix, web-2/scene-2, baseline/whitespace AC; coverage không suy từ load success |
| Text style và faces | [P10b](../recommend-features/tasks/P10b-text-style-props.md), [P10c](../recommend-features/tasks/P10c-font-family-weight.md) | Thêm preview/SVG consumers; chốt units; dùng font thật để kiểm visual face |
| Variable fonts | [P12b](../recommend-features/tasks/P12b-variable-font.md) | Kiểm effective axes/opsz cache + font identity, không chỉ metadata |
| Corners | [P17](../recommend-features/tasks/P17-four-side-shorthand.md), [P04b](../recommend-features/tasks/P04b-corner-radii.md) | Include states.radius, image clip, SVG; không nhầm thứ tự góc/cạnh |
| Effects | [P09b](../recommend-features/tasks/P09b-box-shadow.md), [P09a](../recommend-features/tasks/P09a-gradient-fill.md), [P09c](../recommend-features/tasks/P09c-blur-backdrop.md) | Rebase paint.ts + preview DOM + SVG, offscreen/effect extent budgets |
| Vector | [P14a](../recommend-features/tasks/P14a-vector-path-node.md), [P14b](../recommend-features/tasks/P14b-import-svg.md), [P14c](../recommend-features/tasks/P14c-vector-stroke.md) | Path subset trước importer/stroke; giữ explicit supported export surface |
| Layout | [P08b](../recommend-features/tasks/P08b-align-self-space.md), [P28](../recommend-features/tasks/P28-responsive-percent-breakpoints.md) | P28 sửa hidden/visible, bỏ HTML-export giả định, thêm storage/themes/instances validation |
| Discovery/workflow/lint | [P19](../recommend-features/tasks/P19-tools-list-diet.md), [P25](../recommend-features/tasks/P25-agent-workflow-e2e.md), [P29](../recommend-features/tasks/P29-design-lint-diagnostics.md) | Capability theo consumer, semantic text policy, no false hard reject và baseline byte đo thật |
| Nền đã có | [P24](../recommend-features/tasks/P24-components-instances.md), [P27](../recommend-features/tasks/P27-interaction-states.md) | Giữ code hiện tại làm baseline, không giao triển khai lại; phân biệt allowlists và paint-only states |
| Design tokens | [P20](../recommend-features/tasks/P20-token-code-syntax.md), [P23](../recommend-features/tasks/P23-token-style-filter.md), [P30](../recommend-features/tasks/P30-auto-tokenize.md) | Chỉ mở sau typography/resource closure đúng; không cần collection modes chỉ để thêm tracking |

Các nội dung chưa có task (rich runs, textbox extensions, overlay child, collection modes) là đề xuất phase sau; chưa tạo file tasks mới và chưa đổi scope PLAN/backlog trong nghiên cứu này.
