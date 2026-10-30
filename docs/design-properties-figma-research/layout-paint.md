# Layout, paint, components và tokens

[Tóm tắt](../DESIGN-PROPERTIES-FIGMA-RESEARCH.md) · [Audit source](audit.md) · Nguồn Figma truy cập **2026-10-30**. Đề xuất ở đây là subset cho agent author/viewer/coder, không là cam kết clone Figma.

## 1. Layout/constraints: ma trận

| Nhóm | Figma chính thức | OpenCanvas thực tế | Mapping CSS / lựa chọn |
| --- | --- | --- | --- |
| Mode | [layoutMode](https://developers.figma.com/docs/plugins/api/properties/nodes-layoutmode/) NONE/HORIZONTAL/VERTICAL/**GRID**, default NONE | free/row/column đã có, defaultfree; chưa Grid | Absolute/flex row/column; Grid Figma hiện có thật, không nhầm với layout-grid guides |
| Fixed/hug/fill | [layoutSizingHorizontal/Vertical trên FrameNode](https://developers.figma.com/docs/plugins/api/FrameNode/) là shorthand UI sizing | Width/height numeric/hug/fill đã có; root fixed | Numeric sizes, content sizing, flex1 1 0; không đồng nhất mọi trường hợp với CSS auto |
| Grow/shrink | [layoutGrow](https://developers.figma.com/docs/plugins/api/properties/nodes-layoutgrow/) chỉ0/1, trục chính | Fill main-axis grow1/shrink1/basis0; fixed/hug shrink0 | Chưa cần public arbitrary flexGrow/flexShrink/basis. Chỉ mở khi có fixture unequal shares thật |
| Align parent | [counterAxisAlignItems](https://developers.figma.com/docs/plugins/api/properties/nodes-counteraxisalignitems/) / primaryAxisAlignItems | align start/center/end/stretch đã có | align-items; baseline alignment chưa có, cần baseline contract text trước |
| Justify | [primaryAxisAlignItems](https://developers.figma.com/docs/plugins/api/properties/nodes-primaryaxisalignitems/) MIN/MAX/CENTER/SPACE_BETWEEN/SPACE_EVENLY/SPACE_AROUND | start/center/end/space-between đã có | P08b thêm around/evenly; gap là minimum, không nhầm khoảng cách phân phối với authored gap |
| Align child | [layoutAlign](https://developers.figma.com/docs/plugins/api/properties/nodes-layoutalign/) INHERIT/STRETCH; MIN/CENTER/MAX đã deprecated | Chưa authored alignSelf; chỉ giữ hug khỏi stretch nội bộ | P08b align-self là tiện ích CSS, **không** lý do “Figma đang có mọi alignSelf value” |
| Padding/gap | [FrameNode](https://developers.figma.com/docs/plugins/api/FrameNode/) padding từng cạnh/itemSpacing/counterAxisSpacing | Padding1–4 và một gap cho hai trục đã có | CSS padding/gap; rowGap/columnGap riêng phase sau nếu cần |
| Wrap | [layoutWrap](https://developers.figma.com/docs/plugins/api/properties/nodes-layoutwrap/) NO_WRAP default, WRAP cho HORIZONTAL/VERTICAL | Row wrap opt-in đã có; chưa column/reverse | flex-wrap:wrap; align-content:start; giữ definite width và fill-min guards hiện tại |
| Positioning | [layoutPositioning](https://developers.figma.com/docs/plugins/api/properties/nodes-layoutpositioning/) AUTO default hoặc ABSOLUTE trong auto-layout | Free parent tạo absolute child; stack không có out-of-flow child | Badge/close button trong card cần wrapper free hoặc phase overlay child; không đổi cả card sang free chỉ vì một badge |
| Constraints | [REST LayoutConstraint](https://developers.figma.com/docs/rest-api/file-property-types/#layoutconstraint) anchors/stretch/scale hai trục | Chỉ x/y under free + sizing/min/max; chưa right/bottom anchors/scale constraints | CSS inset/percent tùy semantics; anchors khác flex rules |
| Min/max | [REST FRAME](https://developers.figma.com/docs/rest-api/file-node-types/#frame) min/max optional/null | Finite0…100000, min≤max; fixed clamp có thật | min-/max-width/height; chưa cần đề xuất lại; không hứa tất cả minima đều vừa màn hình |
| Percent/breakpoints | Figma sizing/constraints không phải hợp đồng CSS media queries tự động | Chưa có, P28 chờ | Tái dùng P28 sau khi sửa spec; phase sau; không yêu cầu trước để sửa typography |
| Aspect ratio | [REST FRAME](https://developers.figma.com/docs/rest-api/file-node-types/#frame) targetAspectRatio thay preserveRatio cũ | Fit giữ ratio **nội dung ảnh**, chưa node aspectRatio constraint | aspect-ratio là feature khác object-fit; phase sau nếu card media cần |

Cụ thể `crates/layout/src/tree.rs:29` cố ý shrink0 cho fixed/hug, `:47` chỉ fill có grow/shrink1; `stack.rs:35` dùng cùng gap cho hai chiều. Đây là hành vi đã có, không mô phỏng đầy đủ browser flex defaults. Thêm “flex properties” chung chung dễ tạo các tổ hợp cycle mới không cần thiết.

## 2. Paint, effects, geometry và image/vector: ma trận

| Nhóm | Figma / semantics chính thức | OpenCanvas | CSS/frontend |
| --- | --- | --- | --- |
| Solid/multiple paints | [Paint](https://developers.figma.com/docs/plugins/api/Paint/) solid/gradient/image…; fills là paints | Một solid fill + typed refs/styles; chưa layered fills | background/color hiện có; chỉ thêm linear/radial trước, không arrays mọi paints ngay |
| Gradient | [Paint GradientPaint](https://developers.figma.com/docs/plugins/api/Paint/#gradientpaint) transform + stops0…1 | Chưa có, P09a chờ | CSS angle/radial center+radius; Figma transform cần adapter, không copy số matrix thành CSS angle |
| Stroke | [REST FRAME](https://developers.figma.com/docs/rest-api/file-node-types/#frame) inside/center/outside, dashes… | Scalar/4 sides color+width, 3 align đã có; inside lấy stack space; **SVG bỏ stroke** | CSS border cho inside, paint-only outline/spread cho outside/center; chưa dashes/path stroke |
| Corners | [REST FRAME](https://developers.figma.com/docs/rest-api/file-node-types/#frame) radius chung/4 góc, smoothing | Radius đều; P04b/P17 chưa làm | border-radius theo TL/TR/BR/BL; smoothing/squircle khác circle radius, chưa nên làm |
| Group opacity | [REST opacity](https://developers.figma.com/docs/rest-api/file-node-types/#frame) overall opacity | Đã composite nhóm ở canvas, có budget/fallback | CSS opacity; không nhân opacity trực tiếp từng con có overlap |
| Shadows | [Effect](https://developers.figma.com/docs/plugins/api/Effect/) outer/inner, offset/radius/spread (default0), visible/blend | Chưa có, P09b chờ | Chọn CSS box-shadow semantics, không hứa Figma spread/clip mọi tổ hợp giống CSS |
| Blur | [Effect](https://developers.figma.com/docs/plugins/api/Effect/#blureffect) layer/background, radius≥0 | Chưa có, P09c chờ | filter:blur / backdrop-filter; snapshot backdrop và clip/order/budget là phần việc thực, không chỉ thêm ctx.filter |
| Blend/mask | [REST BlendMode/MaskType](https://developers.figma.com/docs/rest-api/file-property-types/#blendmode) normal/pass-through, alpha/vector/luminance masks | Chưa authored blend/mask | mix-blend-mode/isolation, mask/clip-path tùy loại; không coi clip hiện có là mask đầy đủ |
| Clip | [FrameNode clipsContent](https://developers.figma.com/docs/plugins/api/FrameNode/) giới hạn children | Đã có clip box bo góc; ảnh luôn clip bounds | overflow:hidden/rounded clip; text clip không đồng nghĩa ellipsis |
| Transform | [relativeTransform](https://developers.figma.com/docs/plugins/api/properties/nodes-relativetransform/) matrix vị trí, rotation có semantics riêng | Chưa static matrix/rotation/scale authored; animation translate/rotate/scale **đã có** ở preview | CSS transform/pivot; scene bounds/hit-test/clipping cần chốt local vs world trước static transform |
| Image | [Paint ImagePaint](https://developers.figma.com/docs/plugins/api/Paint/#imagepaint): Figma dùng image paint, không image node; FILL/FIT/CROP/TILE | Image node có fit(stretch)/contain/cover + crop normalized; đủ subset; chưa tile/rotation/filter | Figma FILL gần object-fit:cover; FIT gần contain. **OC fit = CSS fill**, không phải Figma FIT |
| Vector | [VectorNode](https://developers.figma.com/docs/plugins/api/VectorNode/) paths/networks | Chưa vector node, P14a/b/c chờ | Path+viewBox+fillRule/Path2D và SVG; ưu tiên icons, chưa vector editor/boolean ops |

Không phải mọi hàng “chưa” đều nên triển khai. Layered paints, arbitrary masks, mesh gradients, progressive blur, font path editor, blend catalog toàn bộ, boolean vector và corner smoothing có chi phí lớn hơn lợi ích cho UI handoff hiện tại.

## 3. Subset ưu tiên và schema dự kiến

Các phạm vi dưới đây **tái dùng task đang có**, không tạo task trùng. Tất cả defaults phải omit được để giữ dữ liệu cũ gọn. Token refs phải resolve trên mọi theme; enum chưa nhận string tokens vì hệ thống chưa có type ấy.

| Nhóm / use case | Draft schema và default | Compatibility/consumer/dependency |
| --- | --- | --- |
| P17→P04b: tab chỉ bo trên | `radius:number\|number[1..4]\|dimension refs`, default0; góc TL/TR/BR/BL theo CSS shorthand, đơn vịpx | Scalar giữ nguyên. Không tái dùng nhầm thứ tự cạnh T/R/B/L. Sửa layout/scene/stroke/clip/preview/SVG đồng bộ, kể cả ảnh; P27 radius hiện scalar cần quyết định có mở arrays trong cùng lát không |
| P09b: modal elevation, keycap inset/ring | `shadow:[{x,y,blur?,spread?,color,inset?}]`, x/y/spread signed px/number refs; blur≥0 dimension; defaults blur0/spread0/insetfalse, absent=[] | Box-only; không đổi layout, mở render extent và offscreen budgets/export bounds. Chọn CSS-first. Không tự cho P27 states.shadow hay P24 override.shadow khi chưa AC |
| P09a: brand button/glow | `fill:{type:"linear",angle?:180,stops:[[offset,color/ref],...]}`; radial `{center?:[.5,.5],radius?:...}` theo task | Solid/ref cũ giữ nghĩa. Resolve discriminated paint union trước typed-ref helper; fill style nhận gradient, color token vẫn chỉ color. Scene fill hiện String phải đổi type + version; preview/SVG cần gradients riêng |
| P14a→b→c: icons nét và đổi màu | kind vector, path string/viewBox/fillRule, fill solid; rồi safe SVG import và stroke cap/join/miter | Bắt đầu path geometry, không arbitrary embedded markup. Mở NODE_KINDS/MCP enum là byte cost có thật; có raster fallback trước khi importer làm xong |
| P08b: button tự căn phải trong form | alignSelf start/center/end/stretch, absent inherit parent; justify thêm around/evenly; đơn vịkhông | Taffy geometry-only, scene bounds đủ; không cần field mới trong SceneNode. Giữ explicit size/hug thắng stretch; instance root keys không tự mở alignSelf |

AC và cache/version chung nằm ở [roadmap](roadmap.md). Draft shadow ở đây tóm subset; khi triển khai dùng chính task P09b để chốt tên/bounds chi tiết, không coi ví dụ này là schema đã nhận.

**Vấn đề phải sửa khi rebase task:** P09a tài liệu từng nói server/viewer không đọc fill; preview DOM mới đã đọc nó. P04b/P17 phải xét states.radius và SVG export. Các số đo prototype trong backlog là lịch sử, không phải kiểm lại ở revision hiện tại. Mọi feature paint phải chọn hỗ trợ SVG đầy đủ hay diagnostic/fallback rõ ràng; không âm thầm mất hiệu ứng.

## 4. Layout phase sau, không mở cả CSS engine

- **P28 responsive:** tách percent sizing khỏi breakpoint; numeric/hug/fill giữ nguyên. Percent lấy content box đã definite, chặn chu kỳ hug; breakpoint override chỉ là compute input, không ghi revision. Draft hiện dùng `hidden` chưa phải key hợp lệ (hiện `visible`), nhắc “HTML export” dù sản phẩm chỉ PNG/SVG, thiếu validation storage/instance/theme → cần sửa trước giao.
- **Overlay child trong stack:** nếu wrapper free đang làm cấu trúc rối, đề nghị scoped `positioning:"auto"|"absolute"` defaultauto + x/y trên direct child stack. Absolute không góp hug parent; width/height explicit/guards cần định nghĩa. Anchors right/bottom không tự có; không nhập cả constraints matrix vào cùng PR. Chưa có task riêng, chỉ đề xuất nghiên cứu sau.
- **Aspect ratio:** optional positive ratio (absent không constraint), khi một trục definite thì suy trục kia; hai trục fixed ưu tiên explicit. Phải định nghĩa min/max và fill/hug conflict trước; không mở do object-fit thiếu.
- **Grid:** Figma đã có Grid auto-layout, nhưng OpenCanvas row wrap đủ cho nhiều card. Chỉ mở khi cần columns thẳng hàng qua nhiều rows/table sizing; phải có fixture mà wrap không giải được. Không ưu tiên từ tên tính năng.
- **Grow/shrink tự do, column wrap, independent gaps:** chưa có nhu cầu được lượng hóa; sử dụng fill/min/max hiện có và theo dõi fixture trước. Không tăng tổ hợp validation/cycle chỉ để đạt parity danh mục.

## 5. Components/variables/styles: ma trận

| Khả năng | Figma | OpenCanvas / hướng giữ |
| --- | --- | --- |
| Master/instances/overrides | [InstanceNode](https://developers.figma.com/docs/plugins/api/InstanceNode/) và main component | P24 **đã có** reference expand, virtual IDs, validation cycles, limited flat overrides; giữ model gọn |
| Exposed props | [Component properties](https://developers.figma.com/docs/plugins/working-with-component-properties/) BOOLEAN/TEXT/INSTANCE_SWAP/VARIANT/SLOT | Chưa có definitions/exposed names; text/visible overrides là một phần công năng, chưa tương đương typed public props |
| Variant sets | Cùng [component guide](https://developers.figma.com/docs/plugins/working-with-component-properties/), VARIANT thuộc ComponentSet | Chưa có; P27 states là paint-only interaction variants, **không** size/layout variants |
| Design variables | [Variable](https://developers.figma.com/docs/plugins/api/Variable/) valuesByMode có thể giữ aliases; resolveForConsumer khác read raw | Tokens aliases/themes và authored/resolved đã có; tái dùng, không làm lại nền token |
| Modes/collections | [VariableCollection](https://developers.figma.com/docs/plugins/api/VariableCollection/) nhóm variables chung modes; [resolved modes](https://developers.figma.com/docs/plugins/api/properties/nodes-resolvedvariablemodes/) theo node/collection | OC một theme request toàn scene; chưa per-node/per-collection modes/inheritance. Hai trục brand×density chưa diễn đạt độc lập |
| Style binding | [TextStyle](https://developers.figma.com/docs/plugins/api/TextStyle/) và paint styles | Một top styleId + fill typed style ref; text style chỉ bốn metrics/color keys; không style aliases. Mở props text phải mở style validator tương ứng |
| Code mapping | [Variable codeSyntax](https://developers.figma.com/docs/plugins/api/Variable/#codesyntax) | P20 đang chờ; hữu ích cho coder hơn variable collections phức tạp; chưa claim có Code Connect |

**Phase tiếp đáng cân nhắc:** exposed component properties nhỏ `{label:TEXT, iconVisible:BOOLEAN}` ánh xạ descendant props hiện có, rồi named size variants nếu có nhu cầu button sm/md/lg. Chưa cần full component-set/slot/inheritance. Defaults lấy từ master; names ổn định phải sống qua đổi descendant IDs có kiểm migration. Đây cần quyết định product riêng sau P24, không gắn lén vào text props.

**Mode strategy:** tiếp tục một theme cho MVP. Nếu thực tế cần brand/density độc lập, khảo sát collection-scoped mode selection trước; schema draft `modes:{collectionId:modeId}` absent dùng default phải có authored/resolved context riêng và cache selection map sorted. Không lấy states làm themes hoặc theme làm breakpoint.

**Tác động feature mới lên instances/states:** mở validator base không mở instance root/OVERRIDE_KEYS/STATES. Ưu tiên cho master và text style mang typography/paint, instance đổi styleId để tái dùng. Không thêm font/spacing vào paint-only state, vì sẽ làm wrap/geometry thay đổi sau layout. State precedence trong preview phải giữ cùng scene behavior, không để CSS inline override state.
