# Task 01 — Tầng skill của MCP: instructions, resources và diagnostics

**Trạng thái:** hoàn tất, root kiểm độc lập 2026-10-28.
**Phụ thuộc:** không. Đây là task nên làm trước.
**Phạm vi sở hữu:** `crates/server/src/instructions.rs`, `crates/server/src/mcp.rs`,
`crates/server/src/reads/tree.rs`, thư mục mới `crates/server/skills/`, `docs/SERVER.md`.
Không sửa `crates/layout`, `crates/storage`, `web/`.
**Workspace:** `/Users/luis/Workspace/OpenCanvas`, branch `main`. Không reset, không checkout,
không commit/push khi chưa được yêu cầu.

## Bối cảnh

Người dùng nối MCP OpenCanvas vào Codex và nhờ agent dựng lại một frame thành HTML. Kết quả
giống design gần như tuyệt đối nhưng cứng hoàn toàn: mọi node là `position:absolute` với toạ độ
px, không co giãn theo màn hình.

Điều tra cho thấy agent không làm sai. Frame mẫu `welcome-dashboard` được author bằng
`layout.mode:"free"` với `x`/`y` tuyệt đối cho cả 71 node, nên `get_design_context` trả về đúng
một thứ duy nhất có thể dịch sang code: toạ độ tuyệt đối.

`instructions::CONNECT` hiện dặn agent:

> re-read at another viewport to see how the design reflows instead of inventing breakpoints

Với frame `free` thì lời dặn này không dùng được. Đọc lại ở viewport khác cho ra **bounds y hệt**
(xem Task 02 phần bằng chứng): `crates/layout/src/tree.rs:60-64` ép root frame bằng viewport, còn
node con `free` giữ nguyên `x`/`y`, nên đổi viewport chỉ **cắt** chứ không reflow. Agent làm đúng
lời dặn, thấy hai lần đọc giống nhau, và kết luận hợp lý rằng thiết kế này cố định.

Chỉ dẫn đúng nhưng gặp dữ liệu không trả lời được câu hỏi. Task này làm cho sự im lặng đó trở
thành một thông báo tường minh.

## Mục tiêu

Agent nối vào MCP phải biết ba điều mà hôm nay nó không có cách nào biết:

1. Khi nào một frame **không mang thông tin responsive**, nói thẳng bằng dữ liệu chứ không để
   agent tự suy ra từ hai lần đọc giống nhau.
2. Phải làm gì trong trường hợp đó: nêu rõ giả định hoặc hỏi người dùng, không được coi
   absolute positioning là bản dịch đã hoàn thành.
3. Chi tiết dài (checklist, template, bẫy đã biết) nằm ở đâu để đọc khi cần, thay vì nhồi hết
   vào instructions.

## Việc phải làm

### 1. `diagnostics` cho `get_design_context`

`crates/server/src/reads/tree.rs:90` hiện có `response["diagnostics"] = json!([])` — trường đã tồn
tại trong schema và luôn rỗng. Điền vào đó:

- Khi mọi node trong scope đều `free` (hoặc không khai báo `layout`, vì mặc định là `free`):
  phát một diagnostic nêu rõ frame được author ở kích thước cố định nào và rằng context này
  không mang thông tin reflow.
- Khi frame có dùng `row`/`column`: không phát diagnostic này.

Diagnostic là dữ liệu có cấu trúc (code + message), không phải câu văn tự do, để client khác
còn xử lý được. Đặt logic ở một module riêng, không nhét thêm vào `tree.rs` đang 139 dòng.

### 2. Sửa `instructions::CONNECT`

File `crates/server/src/instructions.rs` (27 dòng). Giữ nguyên tinh thần hiện có, bổ sung:

- Một frame `free` mà bounds không đổi giữa các viewport là **một bức tranh ở một kích thước**,
  không phải đặc tả responsive. Agent phải nói rõ giả định hoặc hỏi, không được im lặng chọn
  absolute positioning rồi coi là xong.
- Phần authoring: mặc định dùng `row`/`column`; `free` chỉ dành cho overlay và trang trí.
- Trỏ tới skill resources ở mục 3 cho phần chi tiết.

CONNECT được nhét vào system prompt của **mọi** phiên client nên phải ngắn. Chỉ thêm ý mới, không
diễn giải dài.

### 3. Bật resources và expose skill

`crates/server/src/mcp.rs:16` hiện là:

```rust
ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
```

Thêm `enable_resources()`, cài `list_resources` / `read_resource` trong `impl ServerHandler for Mcp`,
và expose hai skill dưới dạng tài liệu markdown nhúng sẵn (`include_str!`):

| URI | Nội dung |
| --- | --- |
| `skill://opencanvas/implement-frame.md` | Đọc context → dựng code. Checklist, map `row`/`column`/`fill`/`hug` sang flex, cách xử lý `height:"hug"`, bẫy baseline text, khi nào phải hỏi thay vì đoán. |
| `skill://opencanvas/author-frame.md` | Tạo design qua `apply_batch`. Mặc định stacks, khi nào được dùng `free`, cách đặt `minWidth`/`maxWidth`, kiểm bằng hai viewport. |

Đặt file tại `crates/server/skills/*.md`. Chúng là tài liệu, không áp ngưỡng 250 dòng của code,
nhưng viết gọn — đây là thứ agent phải đọc hết.

Mô hình tham chiếu: `skill://cua-driver/SKILL.md` mà người dùng đã dùng trong môi trường của họ.

## Acceptance Criteria

- `get_design_context` trên frame toàn `free` trả về `diagnostics` không rỗng, nêu đúng kích thước
  authored và nói rõ không có thông tin reflow.
- `get_design_context` trên frame có `row`/`column` **không** phát diagnostic đó.
- `resources/list` trả về đúng hai URI trên; `resources/read` trả về nội dung markdown.
- `ServerCapabilities` công bố `resources`; client cũ chỉ dùng tools vẫn chạy bình thường.
- CONNECT đã có hướng xử lý cho trường hợp frame không mang thông tin responsive.
- Mọi file mã viết tay trong phạm vi ≤250 dòng. `tree.rs` không phình thêm.
- `docs/SERVER.md` ghi phần resources và danh sách diagnostic codes.

## Cách kiểm

```bash
cargo test -p opencanvas-server --offline
cargo clippy -p opencanvas-server --offline -- -D warnings
cargo fmt --all --check
```

Thêm integration test cho: diagnostic có/không theo hai loại frame, `resources/list`,
`resources/read` với URI hợp lệ và không hợp lệ.

Kiểm bằng MCP thật: dùng data root tạm, tạo hai frame (một `free`, một stacks), gọi
`get_design_context` cho cả hai. Không chạm `~/.opencanvas`, không restart server của người dùng
ở cổng 3847.

## Đầu ra

- Delta mã trong phạm vi sở hữu ở trên.
- Hai file skill markdown.
- `docs/SERVER.md` cập nhật.
- Báo cáo: lệnh thật đã chạy, kết quả test, số dòng mỗi file đã sửa/tạo, và một ví dụ response
  `get_design_context` có diagnostic.

## Ngoài phạm vi

- Không sửa schema layout (xem Task 04).
- Không thêm tham số viewport mới cho tool (xem Task 03).
- Không author lại design mẫu (xem Task 02).

## Kết quả triển khai — 2026-10-28

- Thêm `reads/diagnostics.rs`: `NO_REFLOW_INFORMATION` trên toàn bộ scope trước depth,
  property filter và pagination; trả `frameId`, `authoredSize`, `message` ở mọi trang.
  Stack nằm ngoài frame không làm mất diagnostic; stack lồng sâu bên trong có làm mất.
- `resources.rs` nhúng hai Markdown qua `include_str!`; MCP công bố resources, trả đúng
  hai URI và MIME `text/markdown`. URI sai trả resource-not-found; tools cũ vẫn hoạt động.
- CONNECT và `docs/SERVER.md` đã cập nhật; diagnostics rỗng không chứng nhận responsive.
- Root chạy thật `rtk bypass cargo test -p opencanvas-server --offline -- --test-threads=1`:
  **12/12 integration tests pass**, gồm 2 tests mới trên MCP HTTP thật/data root tạm.
  Loopback/Chromium chạy ngoài sandbox; không chạm server3847 hoặc project người dùng.
- Root `cargo clippy -p opencanvas-server --offline -- -D warnings`,
  `cargo fmt --all --check`, `git diff --check`: pass. Shared test helper vẫn có
  warning dead_code khi mỗi test binary chỉ dùng một phần; production strict lint sạch.
- File code: instructions32, lib17, mcp175, reads63, tree139 (không tăng),
  diagnostics28, resources39, skill_layer test136 dòng. Không file vượt250.
- Ví dụ trên authored960×720 dù request viewport320×480:

```json
{"code":"NO_REFLOW_INFORMATION","frameId":"free","authoredSize":{"width":960,"height":720},"message":"Frame 'free' has authored size 960 × 720. All scoped nodes use free layout, explicitly or by default; this context contains no reflow information. State responsive assumptions or ask the user before treating an absolute-positioned implementation as complete."}
```

Worker đã build binary mới. Khi triển khai mẫu thật, preflight phát hiện không có
process3847; instance mới đã được khởi chạy và đang phục vụ code mới. Root dùng
connected MCP xác nhận frame cũ trả diagnostic authored1040×720 thật; không restart
hay kill instance nào đang chạy.
