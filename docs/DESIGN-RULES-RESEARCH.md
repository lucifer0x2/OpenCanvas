# Nghiên cứu quy tắc thiết kế dùng chung cho AI

Ngày đối chiếu: 2026-10-29. Phạm vi: source trên nhánh
`codex/p24-components-instances`, tài liệu chính thức Figma và MCP OpenCanvas.
Không đọc project thật, không thay đổi runtime hoặc dữ liệu người dùng.

## Kết luận áp dụng

OpenCanvas đã có phần dữ liệu cần để hai AI dùng chung màu, typography, spacing
và component. Chưa có **chính sách thiết kế riêng theo project** được tự động nạp
hoặc bắt buộc bởi validator. Cần tách ba lớp:

1. **Dữ liệu chuẩn:** token/style/font/master trong project ở một revision cụ thể.
2. **Quy tắc sử dụng:** vai trò semantic, giá trị được phép, ngoại lệ và quyền sửa
   design system; dùng một tài liệu chuẩn có phiên bản.
3. **Protocol làm việc:** cả hai phiên phải đọc cùng nguồn trước khi ghi, giữ refs,
   đọc lại khi dependency đổi và kiểm kết quả ở cùng theme/viewport.

[Bản nháp protocol](DESIGN-RULES.md) dùng được với API hiện có. Nó chưa chọn palette
hay font cho bất kỳ project nào và không tự được MCP phát cho client.

## Figma thực sự cung cấp gì

| Nhóm | Cơ chế Figma đã xác minh | Bài học áp dụng |
| --- | --- | --- |
| Variables/tokens | Variable có ID, kiểu, `valuesByMode`, alias, description, scope và code syntax; collection nhóm variables dùng chung các mode. [Variable](https://developers.figma.com/docs/plugins/api/Variable/), [Collection](https://developers.figma.com/docs/plugins/api/VariableCollection/) | Cùng ID và vai trò semantic; giá trị theo theme chỉ là kết quả resolve. Tách primitive và semantic bằng naming/alias. |
| Typography | TextStyle tập hợp font, cỡ chữ, line height, letter spacing và các thuộc tính text; có binding variable ở các field hỗ trợ. [TextStyle](https://developers.figma.com/docs/plugins/api/TextStyle/) | Chốt cả bộ typography qua style; tên font đơn lẻ không đủ đảm bảo cùng metrics. |
| Layout/spacing | Auto layout mô tả direction, gap, padding, alignment và resizing; tài liệu hiện có cả grid và vertical wrap. [Auto layout](https://help.figma.com/hc/en-us/articles/360040451373-Guide-to-auto-layout-in-Figma) | Lưu ý đồ bố cục bằng constraints, kiểm nhiều viewport; không suy responsive từ ảnh hoặc x/y. |
| Components/variants | Component set tổ chức các biến thể theo thuộc tính như size/state; mỗi variant là một tổ hợp riêng. [Variants](https://help.figma.com/hc/en-us/articles/360056440594-Create-and-use-variants) | Định nghĩa rõ component nào, biến thể nào được phép; tránh hai AI tự dựng hai nút khác nhau. |
| Rules cho AI | Figma khuyến nghị rules theo project, tái sử dụng tokens/components và kiểm context + screenshot. Prompt `create_design_system_rules` giúp soạn rules. [Custom rules](https://developers.figma.com/docs/figma-mcp-server/add-custom-rules/) | Rules là nguồn hướng dẫn cho agent, không đồng nghĩa mọi giá trị hợp lệ đều bị kiểm theo policy. |
| Truyền context | `get_variable_defs` lấy variables/styles trong selection; catalog có tools tìm design system và Code Connect. [Tools and prompts](https://developers.figma.com/docs/figma-mcp-server/tools-and-prompts/) | Đọc system trước khi làm; mapping sang code phải có tên/path được xác nhận. OpenCanvas không có các tool Figma này. |

Các nguyên tắc primitive → semantic, allowlist, quyền sửa và protocol bên dưới là
đề xuất cho OpenCanvas, không phải cấu hình Figma được import tự động. Không có
một palette/font/spacing scale duy nhất do Figma bắt mọi project phải dùng.
Nguồn rules Figma hướng tới độ trung thực; tài liệu này không nâng nó thành lời
hứa pixel parity của OpenCanvas.

## Kiểm kê OpenCanvas bằng source

| Hạng mục | Hiện có | Khoảng trống/giới hạn |
| --- | --- | --- |
| Tokens | `upsert_token`, `list_tokens`; `color`, `dimension`, `number`; typed refs, alias cùng kiểu, default + named themes; validate mọi theme, cycle/type/range atomically. | Không có collection độc lập, string/boolean/font-family token, description, scopes, code syntax, publish library hay mode cục bộ từng subtree. `name` chỉ là tên; không phải semantic policy. |
| Styles | `upsert_style`, `list_styles`; `fill` và `text`. Text style nhận `fontId`, `fontSize`, `lineHeight`, `fill`; cỡ/line height có dimension refs. | Node props ghi đè style vẫn hợp lệ. Không có style inheritance, effect/grid styles, letter spacing, text alignment hoặc rich text. |
| Fonts | `list_fonts`; metadata family/weight/style/hash/status; `import_asset` nhận font bytes + metadata, text trỏ `fontId`; có `font_bundled`. | Node không có `fontFamily`/`fontWeight`; muốn weight khác cần font record phù hợp. Không suy ra font binary chỉ từ family name, không hứa variable-font axes hoặc cùng metrics giữa runtime. |
| Spacing/layout | Dimension refs cho gap/padding/sizes/radius; row/column/free, padding 1–4 cạnh, fixed/fill/hug, min/max, align/justify, row wrap opt-in. | Không enforcement spacing scale; không Grid, percentage, breakpoint variants hay column wrap. Không tương đương toàn bộ Auto Layout Figma hiện hành. |
| Components | Frame `component:{name}`, `instance` + `componentId`, flat overrides `text/fill/visible/styleId/assetId/name`; context reference/expand, virtual IDs chỉ đọc. | Không component set, axes/variant matrix, exposed typed component properties, arbitrary structure/layout override hoặc Code Connect. `props.states` chỉ hover/pressed/focus/disabled với visual overrides, không phải Figma variants. |
| Intent/guidelines | `create_note/reply_note/update_note/delete_note` là operations của `apply_batch`; context đọc notes frame + descendants + ancestors (`open/all/none`). | Note là văn bản, không tự chạy thành policy. Note ở page khác không tự được kế thừa; resolved note mặc định bị ẩn. Không có project instructions field hay `get_design_rules`. |
| MCP instructions | `CONNECT` hướng dẫn refs, stacks, conflict, render; ba resource `author-frame`, `implement-frame`, `props`. Props doc sinh từ validator. | Nội dung dùng chung cho server, chưa theo project; không tự đọc file repo `DESIGN-RULES.md`. Resource static đóng gói lúc build, không runtime hot reload. |
| Đồng bộ | Read trả revision, write có expectedRevision/idempotency; conflict có changed IDs; `get_changes` phân nhóm nodes/tokens/styles/fonts/notes. | Không khóa semantic hay phân quyền AI “design-system owner”; snapshot reads chỉ revision hiện tại, không arbitrary history. Structural validity không chứng minh tuân thủ policy. |

Nguồn kiểm trực tiếp:

- [DTO token/style/font/note](../crates/storage/src/model.rs),
  [notes model](../crates/storage/src/model/notes.rs),
  [operation schemas](../crates/server/src/schemas/operations.rs).
- [Tool schemas và font import](../crates/server/src/schemas.rs),
  [style validator](../crates/storage/src/validate/node.rs),
  [ref resolver và node-over-style precedence](../crates/storage/src/validate/refs.rs),
  [token resolution contract](RESOLUTION.md).
- [Property catalog](../crates/storage/src/props_catalog.rs),
  [components](COMPONENTS.md), [states](INTERACTION-STATES.md).
- [Connect instructions](../crates/server/src/instructions.rs),
  [resource registry](../crates/server/src/resources.rs),
  [notes scoping](../crates/server/src/reads/notes.rs),
  [current revision/cursor checks](../crates/server/src/reads/pagination.rs),
  [change API](../crates/server/src/reads/changes.rs).

Đây là kiểm chứng **năng lực của source**, không phải xác nhận project của người
dùng đã có tokens/styles/fonts đầy đủ. Chưa biết projectId, palette hay font đã
chốt; cần list và đọc context của project được giao trước khi điền manifest.
Server đang chạy có thể cũ hơn nhánh; lấy tool/resource schema thực tế làm cổng
khả năng, không giả định tính năng source đã được deploy.

## Nguồn chuẩn đề xuất theo project

**Ngay bây giờ, không cần runtime mới:** giữ protocol ở `docs/DESIGN-RULES.md`,
manifest riêng trong repo thiết kế được giao, ví dụ
`design-system/<projectId>/RULES.md`. Manifest chứa projectId, rulesVersion,
revision đã kiểm, theme, semantic token IDs, style IDs, font IDs/hash, master IDs,
viewports, ngoại lệ và người được đổi system. Đây là đường dẫn đề xuất, chưa tạo.
Giá trị authoritative ở project; manifest giải thích cách dùng ID và phạm vi.

Cả Opus và GPT phải được giao cùng manifest qua prompt/bootstrap hoặc file chỉ
dẫn của client. Không dựa vào memory riêng hay trông chờ client tự tìm tài liệu.
Lúc bắt đầu mỗi phiên, ghi nhận cùng `projectId + rulesVersion + revision + theme`.

**Nếu một AI chỉ có MCP:** tạo page `Design System`, một rules frame cố định và
note giữ protocol/ID mapping đã chốt. Truyền `rulesFrameId` cho cả hai; mỗi phiên
đọc `get_design_context` frame đó với `notes:"all"` trước frame công việc. Một note
tối đa 2.000 ký tự; nếu tách phần, đánh số, cùng rulesVersion và đọc hết pagination.
Không tự nhận note này lan ra tất cả page. Nếu lưu bản sao repo và MCP, chọn một
bản làm chuẩn; bản kia ghi version/con trỏ và phải khớp trước khi ghi thiết kế.
Không tạo note/project trong task nghiên cứu này.

## Những gì nên bổ sung sau, thành task riêng

1. **Discovery theo project:** một read trả policy version, nội dung/rules location,
   registry IDs và revision trong cùng snapshot; chỉ thêm resource template hay
   tool khi đã chốt contract. Generic MCP instructions nhắc đọc policy trước ghi.
2. **Lint có giải thích:** phát hiện literal lặp, token ngoài allowlist, typography
   override ngoài ngoại lệ, component duplicate; trả node/property/ruleId. Bắt
   đầu opt-in, không biến mọi literal thành lỗi (geometry/image crop vẫn cần số).
3. **Policy có cấu trúc:** versioned project policy + exception rationale, cập nhật
   cùng revision queue; audit dependencies của token/style/master khi policy đổi.
   Chỉ hứa enforcement sau khi có validator và test tương ứng.
4. **Mapping thiết kế → code:** registry tokenId/styleId/masterId → symbol/path
   trong repo implementation; chưa gọi nó là Figma Code Connect integration.

AC cho task policy tương lai: hai client mới đọc được cùng policy không qua
memory; update đồng thời không ghi đè; đổi token/style/master làm client đọc lại;
unknown/deprecated ID và ngoại lệ trả kết quả rõ; dữ liệu cũ vẫn đọc được; lint
không mutate; font thiếu và unsupported properties không bị thay thế âm thầm.

## Kiểm chứng và giới hạn nghiên cứu

Đã đối chiếu schema, validator, resolver, resources, notes, pagination và component
contract; mở nguồn Figma trực tiếp qua web ngày nêu trên. Trang Help Center
“Tokens, variables, and styles” không truy cập được bằng công cụ tại lần kiểm này;
không dùng nội dung chưa đọc, thay bằng Plugin API chính thức cho token/style.
Không gọi API dữ liệu thật, không import Figma, không chạy render hay test runtime
vì chỉ thêm tài liệu. Links local và diff whitespace được kiểm riêng khi bàn giao.
