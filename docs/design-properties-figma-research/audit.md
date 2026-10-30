# Audit source hiện tại

Quay lại [tóm tắt](../DESIGN-PROPERTIES-FIGMA-RESEARCH.md). Mốc tất cả file:dòng dưới đây: **8931abd0ad5de2538d433079f7200f65eaf11d03**, 2026-10-30. Đây là audit source, không phải nghiệm thu lại toàn bộ app đang chạy trên3847.

## Quy ước trạng thái

- **Đủ trong subset:** có model, validation, resolution và đường tiêu thụ liên quan; không hàm ý bằng toàn bộ Figma hay đã chạy lại E2E.
- **Một phần:** chỉ có subset hoặc các đường tiêu thụ khác nhau.
- **Chưa có:** không có key/kiểu hợp lệ và đường thực hiện trong nguồn đã kiểm.
- **Có task, chưa làm:** task mô tả ý định; source ở mốc này chưa có. Hai trạng thái cuối có thể cùng đúng.

## 1. Authored model, catalog, validator, styles/tokens

| Bằng chứng | Trạng thái thực và hệ quả |
| --- | --- |
| `crates/storage/src/model.rs:5`, `:13`, `:39` | DB schema4, 7 kinds gồm instance; node props là JSON, không phải một schema text phong phú sẵn có |
| `crates/storage/src/props_catalog.rs:99` | Catalog duy nhất cho keys; không có key không đồng nghĩa được chuyển nguyên xuống canvas |
| `crates/storage/src/validate/prop_keys.rs:11`, `:42` | Unknown/wrong-kind bị reject với allowedKeys/suggestion; required text là text/fontId/fontSize/lineHeight sau merge style |
| `crates/storage/src/props_catalog.rs:115`, `validate/properties.rs:29`, `:91` | Text plain string≤64KiB; fontSize/lineHeight số px1…10000; chưa tracking/alignment/nowrap/overflow/runs/direction |
| `crates/storage/src/validate/node.rs:20` | Text style chỉ fontId/fill/fontSize/lineHeight. Thêm key vào node mà quên style vẫn chưa đủ |
| `crates/storage/src/validate/refs.rs:59` | Merge text style trước, node props đè sau; resolve typed refs theo từng property. Style aliases bị từ chối (`:35`) |
| `crates/storage/src/validate/refs.rs:95` | Dimension refs cho sizes/fontSize/lineHeight/radius, number cho x/y/opacity; key mới phải được thêm đúng kiểu, không tự resolve mọi object |
| `crates/storage/src/validate/tokens.rs:11`, `:48`, `:83`; `token_graph.rs:9` | **Đã có** literal color/dimension/number, aliases, default/themes, giới hạn themes và cycle guard xuyên nhánh; chưa collection modes độc lập hay string/boolean tokens |
| `crates/storage/src/props_catalog.rs:108`, `:114` | Fixed/fill/hug, min/max, row/column/free, padding1–4, gap, row wrap, align/justify đã có; %/Grid/constraints/alignSelf chưa có |
| `crates/storage/src/props_catalog.rs:119`, `:122`, `:123` | Solid fill, radius đều, stroke đơn hoặc **đúng4** cạnh đã có. Gradient, shadow, góc riêng, blur chưa có |
| `crates/storage/src/validate/properties.rs:43` | Crop normalized0…1 ở trong ảnh nguồn; fit vocabulary là fit/contain/cover. Không có image paint transform/tile/filter |
| `crates/storage/src/instances.rs:6`, `:7` | P24 có overrides text/fill/visible/styleId/assetId/name; instance root có danh sách keys riêng, không tự nhận tất cả base props |
| `crates/storage/src/validate/instances.rs:69`, `:91` | Override từng descendant được kiểm; subtree component không mang links/animations/flowStart. Không có variant sets, exposed property definitions, swap/slots |
| `crates/storage/src/validate/states.rs:6`, `:8`, `:47` | P27 đã có hover/pressed/focus/disabled; chỉ fill/opacity/radius/stroke. Thêm fontWeight vào base không tự cho phép trong states |

“Không có” được kết luận từ allowlist và consumer types trên đây, không chỉ từ search không tìm thấy chuỗi. Chưa audit mọi lỗi cực biên của các tính năng đã có.

## 2. Resolution → layout → measurement

| Bằng chứng | Điều đã xác minh |
| --- | --- |
| `crates/storage/src/validate/refs.rs:132` | Component expand **trước** style/token resolve |
| `crates/storage/src/instances.rs:71`, `:101` | Root lấy master props, instance keys đè; descendant virtual ID `instance/masterChild`. Phải giữ ID ổn định khi thêm rich text hay diagnostics |
| `crates/server/src/jobs/input.rs:9`, `:21`, `:24` | Resolve/expand snapshot theo theme; frame tối đa2048 render nodes; gom fonts từ fontId cấp node. Rich runs sẽ cần gom fonts bên trong runs |
| `crates/layout/src/model.rs:55`, `:112` | Layout Props và MeasureRequest hiện chỉ plain text/font/size/lineHeight/width. Mở catalog một mình không truyền tính năng mới vào đo |
| `crates/layout/src/tree.rs:20`, `:44`, `:64` | Fixed/hug mặc định shrink0; main-axis fill = grow1/shrink1/basis0; free child absolute left/top. Không author được flex grow tùy ý hay absolute child trong stack |
| `crates/layout/src/stack.rs:23`, `:35`, `:39`, `:58` | Row wrap, một gap dùng cả hai trục, align/justify; self-align nội bộ chỉ giữ hug khỏi stretch, **chưa phải authored alignSelf** |
| `crates/layout/src/engine.rs:36`, `:51`, `:110` | Callback cache-only; sizing đo theo width known/available rồi dựng lines ở width layout cuối; vì vậy consistency hai lượt là AC bắt buộc |
| `crates/layout/src/util.rs:33`, `:58` | Request cache hash gồm renderer version và toàn request: font hash/family/weight/style, text, size, lineHeight, width lượng tử1/64 |
| `renderer/measurement.ts:42`, `:43` | Canvas font lấy từ manifest; fontKerning normal; không có điều khiển tracking/features/lang/direction |
| `renderer/measurement.ts:47`, `:59` | **Đã sửa một phần P10a:** fits và reported width đều round1/64. Không còn so raw width với width đã round như bằng chứng task cũ |
| `renderer/measurement.ts:54` | Baseline vẫn `fontSize + (lineHeight-fontSize)/2`; không dùng ascent/descent thật |
| `renderer/measurement.ts:61` | CR/LF normalize; trim và regex whitespace collapse; NBSP/narrow NBSP cũng bị collapse/break sai semantics web |
| `renderer/measurement.ts:81` | Có Intl.Segmenter vi/grapheme khi từ dài bị split; đây là bảo vệ cluster cục bộ, **không phải** toàn bộ Unicode line-break/bidi engine |
| `renderer/resources.ts:77`, `:94` | Hash check + FontFace load + readiness có thật. Không có glyph coverage check; comment “no implicit system fallback” không chứng minh glyph nào cũng từ font được import |
| `worker/runtime.mjs:57` | Chromium launch chưa có font-render-hinting flag. Chưa tự kiểm Linux trong nghiên cứu này; kết quả Linux ở P10a là bằng chứng lịch sử |

## 3. Computed scene và version/cache

| Bằng chứng | Hiện trạng |
| --- | --- |
| `crates/layout/src/model.rs:127`, `:151`; `renderer/types.ts:18`, `:47` | TextLine chỉ text/x/y/width; SceneNode chỉ một fontId/fontSize/lineHeight/fill. Không có spans, glyph clusters, paragraph source offsets, direction hay decoration geometry |
| `crates/layout/src/engine.rs:121` | Scene dựng từ bounds đã lượng tử và measured lines; paint không tự layout lại |
| `crates/layout/src/lib.rs:20`; `renderer/types.ts:1` | **scene-2/web-2**, không còn scene-1/web-1 trong nhiều backlog cũ; MAX_PASSES6/MAX_NODES2048/MAX_PIXELS16M |
| `crates/server/src/jobs.rs:123` | Job key hash resolved input, scale, pipeline identity; export spec thêm vào basis. Input đã gồm revision/viewport/theme/state |
| `crates/server/src/fingerprint.rs:4`, `:8`, `:22` | Fingerprint lúc startup gồm worker, built renderer, lockfiles, layout binary/runtime identity; thay runtime/source phải build đồng bộ và restart backend khi triển khai, không hot replacement |
| `crates/server/src/jobs/states.rs:1`, `:31` | State visual áp **sau layout**, không đổi geometry/wrap. Không thể mở font-weight state bằng cách thêm nó vào allowlist |

## 4. Viewer/preview/export

| Đường | Bằng chứng | Mức hỗ trợ |
| --- | --- | --- |
| Viewer canvas | `web/src/components/CanvasView.tsx:4`, `:157` | Dùng paintScene chung; text/stroke/ảnh/opacity subset hiện tại có consumer |
| PNG worker | `renderer/drawing.ts:13`, `:71`; `renderer/paint.ts:18` | Cùng paintScene, dùng lines sẵn, không wrap lại; version guard chặn pipeline lệch |
| Text paint | `renderer/paint.ts:31` | Set font/kerning/baseline/fill rồi fillText nguyên dòng; shaping giao browser, không có property control mới |
| Group/clip | `renderer/layers.ts:77`, `:92`, `:124` | Opacity composite có offscreen pixel budget/fallback; extent mới phải tính shadows/blur/transforms nếu bổ sung |
| Image | `renderer/paint.ts:55`, `renderer/image-fit.ts:1` | Crop source rồi placeImage fit/contain/cover và clip rounded box; đủ trong subset này |
| Motion preview | `web/src/preview/sceneDom.ts:43`, `:97` | **Renderer thứ hai:** DOM absolute boxes + SVG text theo scene lines; không browser reflow. Cần cập nhật text/paint properties riêng |
| SVG export | `crates/server/src/jobs/export_svg.rs:1`; `svg/elements.rs:8` | Rust writer từ scene, text giữ nguyên `<text>` theo từng line; font không outline/embed tại đây |
| SVG stroke | `crates/server/src/jobs/export_svg.rs:91` | `SVG_TEXT_NOT_OUTLINED`, `SVG_STROKE_NOT_EXPORTED`; không gọi stroke là end-to-end đầy đủ cho SVG |
| SVG image | `crates/server/src/svg/elements.rs:43`, `:59`, `:73` | Embedded image bytes, preserveAspectRatio mapping, normalized crop bằng nested viewBox |

PNG/viewer chung mã chỉ giảm một nguồn sai lệch; cùng sai baseline vẫn có thể pixel-identical. Preview/SVG là các consumer riêng phải được đưa vào AC feature.

## 5. MCP đọc/ghi/docs và author/implement

| Bằng chứng | Kết luận |
| --- | --- |
| `crates/server/src/schemas/operations.rs:11` | props là generic object≤50 keys; property mới thường không tăng tool schema nếu giữ API này. Validator mới quyết định được phép |
| `crates/server/src/props_doc.rs:23`, `:40` | props.md sinh từ catalog; hiện liệt kê text alignment/tracking/effects là unsupported (`:61`) |
| `crates/server/src/resources.rs:4`, `:17` | Hai skills tĩnh + props resource; chưa machine-readable capability theo consumer |
| `crates/server/src/reads/compact.rs:1`, `:75` | compact differing và recursive referenced-resource walker đã có; object/runs mới phải test closure và payload thực |
| `crates/server/src/reads/instances.rs:34`, `:77` | P24 reference context giữ definitions một lần; expanded geometry ở computed children. Không nên ép mọi read expand vì tăng payload |
| `crates/server/src/reads/states.rs:4` | Resolved states opt-in; base key mới không tự hiện thành state support |
| `crates/server/src/reads/diagnostics.rs:4` | NO_REFLOW_INFORMATION phát hiện all-free hoặc bounds không đổi; chưa phát hiện character fragmentation/text overflow/font fallback |
| `crates/server/skills/author-frame.md:38`, `:53`, `:96` | Có stack-first, catalog-first, hug-height; chưa policy giữ paragraph/runs và tránh glyph positioning workaround |
| `crates/server/skills/implement-frame.md:80`, `:93` | Có CSS mapping, không copy measured lines; câu “line positions come from font metrics” (`:98`) **quá mạnh so code baseline hiện tại** |
| `README.md:63` | Đoạn implement vẫn nói design chỉ default, không interactions/animation; đã cũ so source P27/P07/preview. Đề xuất đồng bộ docs trong P25, chưa sửa trong nghiên cứu |

## 6. Probe tự chạy và cách đọc kết quả

Lệnh thành công: `node docs/design-properties-figma-research/probe-text.mjs > docs/design-properties-figma-research/probe-text-results.json` (exit0). Source measurement được esbuild trong RAM rồi nạp vào browser page tạm; FontFace load bytes bundled; finally đóng browser. Không backend/process listener/data root mới.

Lượt đầu thất bại vì import Playwright trước khi đặt browser-path env; đã đổi thứ tự import và chạy lại thành công. Không cài dependency/browser mới. Lượt cuối thêm đối chứng DOM whitespace, exit0; JSON trong repo là lượt cuối.

| Quan sát | Số đo / giới hạn |
| --- | --- |
| Baseline Noto14/20,32/40,15/15 | DOM15/32/13; scene17/36/15 → sai +2/+4/+2px |
| Width Prefer | Canvas raw40.361969; scene40.359375; DOM40.375. Mismatch1/64 vẫn còn, nhưng tự wrap lại không xảy ra ở 5 chuỗi probe |
| NBSP/narrow NBSP, width20 | Cả hai scene2 lines/h40; DOM pre-line h20. Đây là mismatch whitespace có tái lập, không phải cần tracking |
| Whole run vs từng code point | Chênh ~0.70px trên câu Latin probe ở spacing0/+2.38/−0.77; minh họa mất tương tác giữa glyph khi phân mảnh, không chẩn đoán font QueryNest |
| Platform | darwin, Node24.21.0, Chromium156.0.8078.4; font/source SHA trong JSON |

Không có probe glyph fallback, RTL, emoji, pixel diff, container measurement integration hay runtime Linux. Các kiểm ấy nằm trong [AC dự kiến](roadmap.md), không được ghi pass ở đây.

## 7. Backlog phải rebase

- P10a/b/c: chưa có các properties mới; P10a lỗi round đã sửa một phần, đường vẽ đã chuyển từ drawing.ts sang paint.ts; version web-2/scene-2, baseline chưa sửa.
- P04b/P17/P08b/P09a–c/P11a/P12b/P14a–c/P28/P29: file task có thật, source allowlist/consumer chưa có feature tương ứng; không lấy số phút/bằng chứng runtime cũ làm kết quả mới.
- P24/P27: **đã có code**; phần task mô tả chưa làm hoặc cần thêm state inheritance không còn là nguồn trạng thái. Root/override restrictions hiện tại vẫn có hiệu lực.
- Tasks09–13 engine Rust: tài liệu tồn tại nhưng người dùng đã gác lại; không đưa làm dependency cho text/features trên pipeline hiện hành.
