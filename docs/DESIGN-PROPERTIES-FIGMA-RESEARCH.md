# Thuộc tính thiết kế OpenCanvas: đối chiếu Figma

Nghiên cứu ngày **2026-10-30** (Asia/Ho_Chi_Minh). Chỉ nghiên cứu và đề xuất; chưa triển khai sản phẩm.

## Kết luận dành cho người dùng

OpenCanvas nên ưu tiên **text còn nguyên ý nghĩa, đo đúng và đọc lại được để code**, trước khi mở rộng thành một bộ thuộc tính ngang Figma. Ca QueryNest cho thấy ba vấn đề cộng hưởng: thiếu tracking; đo chữ/ngắt dòng chưa sát trình duyệt; agent được phép dùng những node hợp lệ nhưng đánh mất cấu trúc đoạn văn. Chỉ thêm `letterSpacing` không giải quyết hai vấn đề còn lại.

Nền tảng hiện tại đã có nhiều hơn bảng backlog cũ: row/column, padding 1–4 cạnh, align/justify, row wrap, min/max, viền từng cạnh, group opacity, crop ảnh, component instances P24, states P27, preview motion. Không nên đề xuất lại chúng như tính năng chưa có. Tuy nhiên, “ghi vào model được” chưa đồng nghĩa viewer, preview và mọi export đều thể hiện đầy đủ: SVG hiện **bỏ stroke có diagnostic**, text SVG phụ thuộc font bên ngoài.

Thứ tự đề nghị:

1. **P10a + P11a:** sửa baseline, chốt width/wrap/whitespace semantics và báo font thiếu glyph. Rebase AC P10a: lỗi tự vỡ ở width hug đã được sửa một phần trong code hiện tại.
2. **P10b:** tracking âm/dương, căn dòng, decoration, transform; làm trọn measurement → scene → canvas → preview → SVG. Bắt đầu bằng px theo task có sẵn; quyết định riêng nếu cần giữ đơn vị em/% trong design system.
3. **P19/P25/P29:** cập nhật hướng dẫn, capability theo tầng, cảnh báo text phân mảnh và overflow có bằng chứng. Chạy cùng các lát text; không cấm mọi node một ký tự.
4. **P10c:** chọn mặt font theo family/weight/style, giữ fontId cũ, lỗi rõ khi thiếu mặt. Sau đó mới P12b variable fonts.
5. **P17→P04b, P09b/P09a:** góc riêng, shadow, gradient giúp loại nhiều rect giả. Tiếp đến P14 vector; P08b khi có nhu cầu layout cụ thể. Blur, responsive %, rich runs và variants là phase sau với AC riêng.

Không đề nghị mở lại hướng engine Rust đã gác lại; cơ sở là Rust layout + Node/Chromium đo/vẽ hiện tại. Không cần Figma plugin hay tài khoản Figma cho nghiên cứu này.

## Đọc báo cáo

| Tài liệu | Nội dung |
| --- | --- |
| [Audit code](design-properties-figma-research/audit.md) | Mốc revision, bằng chứng file:dòng theo từng tầng; trạng thái đầy đủ/một phần/chưa có; probe tự chạy |
| [Typography](design-properties-figma-research/typography.md) | Ma trận Figma ↔ OpenCanvas ↔ CSS; text sizing, tracking, rich runs, shaping; thiết kế đề xuất |
| [Layout, paint và design system](design-properties-figma-research/layout-paint.md) | Auto-layout/constraints, effects/vector/image, components/tokens; subset phù hợp sản phẩm |
| [Authoring và diagnostics](design-properties-figma-research/authoring-policy.md) | Phân tích QueryNest; draft hướng dẫn và capability/validation policy |
| [Lộ trình và AC](design-properties-figma-research/roadmap.md) | Lát cắt độc lập, dependencies, cache/version/MCP cost, fixture và quyết định cần chốt |
| [Nguồn chính thức](design-properties-figma-research/sources.md) | URL trực tiếp Figma Plugin/REST/Help và web standards, ngày truy cập, giới hạn nghiên cứu |

## Cơ sở và mức tin cậy

- **Tự xác minh:** code tại `8931abd0ad5de2538d433079f7200f65eaf11d03`, nhánh `codex/p24-components-instances`; working tree ban đầu có LOG sửa và tài liệu EXPORT/PHOTOCRAFT của phiên khác. Đã đọc PLAN, README, ACCEPTANCE, LOG, tasks và backlog liên quan; dùng source thực để phân xử trạng thái cũ.
- **Tự chạy:** [probe-text.mjs](design-properties-figma-research/probe-text.mjs), [kết quả JSON](design-properties-figma-research/probe-text-results.json), trên macOS/Node 24.21.0/Chromium 156.0.8078.4, NotoSans bundled có hash. Nạp source measurement hiện tại qua bundle trong RAM; không dùng bundle sản phẩm có thể cũ.
- **Nguồn ngoài:** đọc web Figma chính thức trong phiên này; các link cạnh ma trận chỉ ra semantics API, không chứng minh OpenCanvas đã thực hiện hay Figma bằng pixel với Chrome.
- **Bối cảnh do người dùng chuyển:** QueryNest rev50/58/61, 377 text nodes/147 ký tự đơn, script capture và CSS trên máy Air. Chưa tự đọc project thật, script ấy hoặc máy Air; không nâng chúng thành kết quả probe của phiên này.
- **Suy luận/đề xuất:** mọi schema mới, thứ tự ưu tiên và AC trong các chương là draft cần chốt phạm vi trước triển khai. Không có ước lượng giờ/phút mới; số phút trong backlog là lịch sử, không phải cam kết.

Probe xác minh ba điều có thể hành động:

| Ca | Kết quả thực |
| --- | --- |
| Noto 14/20; 32/40; 15/15 | Baseline OpenCanvas thấp hơn DOM lần lượt **2, 4, 2 px** |
| `Prefer` hug | Width OpenCanvas 40.359375, DOM 40.375; đo lại ở width của chính nó vẫn **một dòng** |
| `A NBSP B`, width20 | OpenCanvas hai dòng, DOM `white-space:pre-line` một dòng tràn ngang; narrow NBSP tương tự |
| Cả chuỗi so tổng ký tự, tracking0 | `Your data, without the noise.`: 188.999802 so 189.699799 px; tách ký tự làm mất tương tác shaping/kerning ngay cả khi tracking0 |

Không chạy full build/suite; không đo Docker/Linux, Safari/Firefox, pixel diff preview/SVG, rich runs hoặc dự án QueryNest. Không restart server/Docker, không sửa design/data thật, không commit/push. Xem [AC còn phải chạy](design-properties-figma-research/roadmap.md#fixture-nghiệm-thu).
