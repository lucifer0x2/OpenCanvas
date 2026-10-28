# P06b — Tool `get_navigation`: bản đồ điều hướng của project/page/frame trong một lần gọi
Ưu tiên: P1 · Kích cỡ: ~55 phút dev · Phụ thuộc: P06a (bắt buộc: `links`, `flowStart`, `opencanvas_storage::links`, `reads/navigation.rs`) · Nguồn: U6

Phase thứ hai của P06. P06a cho AI coder biết link của **một** frame. P06b cho cả đồ thị trong một lần đọc nhỏ, để:
- AI coder dựng router (danh sách route, route khởi đầu, đâu là overlay);
- AI designer tự kiểm flow: màn nào chưa nối, link nào thiếu.

## Vấn đề

Sau P06a, muốn có toàn bộ đồ thị điều hướng, agent phải:
- gọi `get_design_context` cho từng frame (5,6–7,9 KB mỗi frame, chủ yếu là node và resource không liên quan); hoặc
- quét `get_subtree`/`find_nodes` cả project rồi tự lọc `props.links`, tự tìm frame gốc chứa nút, tự đảo chiều để có link vào.

Đo trên server tạm (`night/integration@51f6368`, 1 page, 6 root frame, 157 node, chưa tính byte của link):

| Cách đọc | Lần gọi | Byte |
| --- | --- | --- |
| `get_subtree` từng frame (6 frame) | 6 | 28 779 |
| `find_nodes` mọi kind (trừ page), `properties:["name"]` | 4 trang | 43 028 |
| `get_outline nodeId:<page> depth:1` (chỉ danh sách màn, không có link) | 1 | 834 |
| **`get_navigation` đề xuất** (6 screen, 12 link, 1 flow; JSON dựng tay theo schema dưới) | 1 | **1 521** |

Một cạnh trung bình 88 byte. Đồ thị cỡ QueryNest (18 frame, ước 40 link) ≈ 4,8 KB trong 1 lần gọi. Cách quét `get_subtree` tốn khoảng 60–90 KB.

## Bằng chứng

- Read tools chung một đường:
  - dispatch `reads::read` (`crates/server/src/mcp.rs:119-120`);
  - `pagination::scope` cho cursor theo revision và tham số (`crates/server/src/reads/pagination.rs:9-29`);
  - `pagination::page` (`:63-120`) phân trang entries theo field, trần 100 mục cho field không phải `nodes` (`:88-94`), tạo field khi có entry đầu (`:95-98`).
  - Một tool mới chỉ cần dựng entries `flows`/`screens`/`links` rồi gọi `page`.
- `ReadRequest` dùng chung, `deny_unknown_fields` (`crates/server/src/reads.rs:16-44`). Field mới phải `skip_serializing_if` để hash cursor của tool cũ không đổi, giống `parent_id`/`compact` (`reads.rs:32-43`).
- Schema read được dựng trong `schemas.rs:71-99`. `find_nodes` là mẫu tool riêng có mô tả dài (`schemas.rs:101-109`).
- Có bốn test ghim số tool = 15:
  - `crates/server/tests/tool_schemas.rs:18`;
  - `crates/server/tests/protocol_2026.rs:90`;
  - `crates/server/tests/skill_layer.rs:74`;
  - `crates/server/tests/transport.rs:11`.
  P02c (`list_notes`) và P03a (`export_nodes`) cũng thêm tool, nên cùng chạm bốn chỗ này.
- `TreeIndex` (`reads/index.rs:13-80`) có `preorder` và `ancestors` để tìm root frame của nút nguồn. P06a đã đặt `edges(snapshot, index)` trong `reads/navigation.rs`.
- Tham khảo:
  - Figma `PageNode.flowStartingPoints: [{nodeId, name}]`, mục đầu là mặc định (https://developers.figma.com/docs/plugins/api/properties/PageNode-flowstartingpoints/).
  - Penpot `Flow {name, startingBoard}` trên page (https://doc.plugins.penpot.app/interfaces/Flow).
  - Không bên nào có API "đồ thị phẳng". Agent phải đi từng node, đúng chi phí P06b loại bỏ.

## Giải pháp

### Tool mới `get_navigation`

Input (mọi field tuỳ chọn trừ `projectId`):
```json
{"projectId":"…","pageId":"page-app","frameId":"home","limit":100,"budget":65536,"cursor":"…","revision":12}
```
- **Phạm vi:**
  - mặc định cả project;
  - `pageId`: link có nút nguồn trên page đó;
  - `frameId` (root frame): link ra hoặc vào frame đó.
  - `pageId` và `frameId` loại trừ nhau → `INVALID_REQUEST` "Choose one scope ID" (như `reads/tree.rs:26-28`).
- **Lỗi:**
  - `pageId` không phải page → "pageId must identify a page";
  - `frameId` không phải root frame → "frameId must identify a root frame".

Output:
```json
{"schemaVersion":1,"projectId":"prj_…","revision":12,"scope":"page-app",
 "flows":[{"name":"Main","start":"home","pageId":"page-app"}],
 "screens":[{"id":"home","name":"Home","pageId":"page-app"},
            {"id":"db-menu","name":"Open databases","pageId":"page-app","overlay":true}],
 "links":[{"from":"home-row0-btn","fromFrame":"home","on":"click","action":"navigate","to":"search"},
          {"from":"home-menu-btn","fromFrame":"home","on":"click","action":"overlay","to":"db-menu"},
          {"from":"search-back","fromFrame":"search","on":"click","action":"back"}],
 "unlinked":["settings"],
 "truncated":false,"nextCursor":null}
```

**`flows`:** mọi root frame có `flowStart` trong phạm vi, theo thứ tự page rồi `sortKey`. Mục đầu là flow mặc định (route khởi đầu).

**`screens`:**
- Root frame trong phạm vi, theo thứ tự page rồi `sortKey`, cộng đích khác page (để `to` luôn tra được `name`/`pageId`).
- Chỉ ghi `name` khi có.
- `overlay:true` khi mọi link vào màn đó đều là `overlay` (và có ít nhất một). Router không tạo route cho màn này mà dựng nó thành modal/popover.

**`links`:**
- Thứ tự: frame nguồn (page, `sortKey`), preorder nút trong frame, rồi thứ tự trong mảng.
- `on`/`action` luôn ghi rõ; `to` chỉ có với navigate/overlay.
- Không lặp `name`, vì đã có trong `screens`. Đây là khác biệt cố ý với `get_design_context.navigation`, nơi chỉ có một frame nên ghi `toName` cho tiện.

**`unlinked`:**
- Root frame trong phạm vi không có link ra, không có link vào và không có `flowStart`.
- Chỉ xuất hiện khi khác rỗng.
- Designer dùng nó để kiểm "đã nối hết chưa"; coder dùng để biết màn nào không có đường vào.

**Phân trang:**
- Entries theo thứ tự `flows` → `screens` → `links` → `unlinked`, qua `pagination::page` (mỗi field ≤ 100 mục/trang, `limit` mặc định 100, `budget` 1 024–65 536).
- Cursor gắn revision và tham số: commit mới làm cursor cũ trả `INVALID_CURSOR`, giống mọi read khác.
- `revision` khác revision hiện tại → `REVISION_UNAVAILABLE`.

### Vì sao tool mới, không phải tham số

Phương án đã cân nhắc:
- **`get_outline links:true`:** outline là cây id/kind/name; trộn danh sách cạnh làm mô tả và phân trang (`nodes` không có trần 100) rối hơn, và client cũ không biết field mới.
- **`find_nodes hasProps:["links"]`:** chỉ trả nút nguồn, không có frame gốc, không có link vào, không có flow. Coder vẫn phải tự lắp.
- **Tool mới:** tốn thêm khoảng 900–1 200 byte `tools/list` mỗi phiên (≈5 % của 20 475). Bù lại, chỉ cần một lần gọi là được cả đồ thị, rẻ hơn ≥ 18 lần so với quét `get_subtree`. Mô tả nói rõ mục đích "dựng router / kiểm flow", nên agent tự tìm đến.

### Mô tả tool (≤ 700 ký tự)

"Navigation map declared by props.links and flowStart: flows (name, start frame; the first is the default entry), screens (root frames; overlay:true when only opened as an overlay), links (from node, fromFrame, on click|hover, action navigate|overlay|back|close, to root frame) and unlinked root frames. Scope is the project, one pageId (links whose source is on it) or one root frameId (links into or out of it). Use it to build a router or to check that every screen is reachable; get_design_context gives one frame's links with its nodes. One revision, up to 100 items per list per page; truncated with nextCursor means more."

### Hướng dẫn

- **`instructions.rs`:** câu P06a thêm ", and get_navigation maps every screen and link for a router" (≤ 80 ký tự).
- **`implement-frame.md`**, mục "Building navigation" (≤ 6 dòng):
  - gọi `get_navigation` một lần;
  - mỗi screen không phải overlay là một route;
  - flow đầu là route khởi đầu;
  - screen `overlay:true` dựng thành modal/popover;
  - `back` dùng history;
  - route path, guard và transition thì hỏi người dùng.
- **`author-frame.md`:** sau khi nối link, gọi `get_navigation pageId` và xử lý `unlinked`.
- **`docs/SERVER.md`:** mục con "get_navigation" trong "Navigation links" của P06a.

## Phạm vi file

| File | Việc | Ước lượng dòng |
| --- | --- | --- |
| `crates/server/src/reads/navigation_map.rs` (mới) | scope, `flows`/`screens`/`links`/`unlinked` entries, gọi `pagination::page` | ~140 |
| `crates/server/src/reads/navigation.rs` (P06a) | export thêm helper root-frame-of/thứ tự nếu cần | +10 |
| `crates/server/src/reads.rs` | `page_id` trong `ReadRequest` (`skip_serializing_if`), nhánh `get_navigation` | 92 → ~100 |
| `crates/server/src/mcp.rs` | thêm tên tool vào nhánh read | +1 |
| `crates/server/src/schemas.rs` | `add("get_navigation", …)` | 159 → ~170 |
| `crates/server/src/instructions.rs`, `skills/*.md` | văn bản | +1, +8 |
| `crates/server/tests/navigation_map.rs` (mới) | AC 2–8 | ~190 |
| 4 test ghim số tool | 15 → 16 (hoặc +1 so với nhánh nền nếu P02c/P03a đã merge) | +4 |
| `docs/SERVER.md` | mục `get_navigation` | +25 |

Không file nào vượt 200 dòng. `reads.rs` vẫn dưới 110.

## Tương thích MCP

- `tools/list` thêm 1 tool. Các tool cũ giữ nguyên schema và mô tả, trừ câu instructions ở trên.
- `ReadRequest.page_id` dùng `skip_serializing_if`: cursor của `get_outline`, `get_design_context`, `find_nodes`… không đổi hash.
- Client cũ không gọi tool mới thì không có gì khác.
- Bốn test ghim số tool phải cập nhật. Thứ tự merge với P02c/P03a quyết định con số; dev rebase rồi sửa thành giá trị thực.

## AC

1. **Danh mục.**
   - `tools/list` có `get_navigation` với `required:["projectId"]`, `additionalProperties:false`, các property `projectId,pageId,frameId,limit,budget,cursor,revision`.
   - Số tool = số của nhánh nền + 1.
   - `tools/list` tăng ≤ 1 200 byte (đo bằng `serde_json::to_vec`, ghi vào `.night/dev/P06b.md`).
2. **Project scope.**
   - Fixture: 2 page; 7 root frame; 13 link gồm navigate, overlay, hover-overlay, back, close và 2 link khác page; 2 `flowStart` (`Main` trên page 1, `Admin` trên page 2); 1 frame không link.
   - Response bằng đúng JSON mong đợi (so nguyên object): thứ tự, mặc định ghi rõ, `overlay:true`, `unlinked`.
3. **Page scope.**
   - `pageId:page-2` chỉ có link có nguồn trên page 2.
   - `screens` gồm root frame của page 2 và đích bên page 1 (mang `pageId` page 1).
   - `flows` chỉ có `Admin`.
4. **Frame scope.** `frameId:home` chỉ có link có `fromFrame==home` hoặc `to==home`; `screens` là `home` và các màn kề.
5. **Lỗi.** Mỗi ca trả lỗi có message như mục Giải pháp:
   - `pageId` là frame;
   - `frameId` là frame lồng;
   - có cả hai;
   - `revision` cũ → `REVISION_UNAVAILABLE`.
6. **Phân trang.**
   - Project có 150 link: trang 1 có 100 `links`, `truncated:true`; theo `nextCursor`, trang 2 có 50 và `truncated:false`.
   - Cursor dùng với `pageId` khác → `INVALID_CURSOR`.
   - Sau một commit, cursor cũ → `INVALID_CURSOR`.
   - Mọi trang ≤ `budget`.
7. **Không có link.** Project chưa có link → `flows:[]`, `screens` đủ root frame, `links:[]`, `unlinked` = mọi root frame. Không lỗi.
8. **Ngân sách byte.**
   - Seed giống script đo (6 frame, 157 node, 12 link, 1 flow) → `get_navigation` ≤ 2 000 byte.
   - Tổng `get_subtree` 6 frame ≥ 28 000 byte.
   - Hai số ghi vào `.night/dev/P06b.md`.
9. **Nhất quán với P06a.** Với mỗi root frame F của fixture AC 2, tập `(from,on,action,to)` trong `get_navigation frameId:F` có `fromFrame==F` bằng đúng tập `get_design_context(F).navigation.out`, kể cả thứ tự.
10. **Hướng dẫn.**
    - `instructions::CONNECT` chứa `get_navigation`; độ dài tăng ≤ 80 ký tự so với sau P06a.
    - `implement-frame.md` có mục "Building navigation".
11. **Modular, test.**
    - File mới ≤ 200 dòng.
    - `cargo test -p opencanvas-server` (các test dưới) và `cargo clippy` sạch ở file đã chạm.

## Cách kiểm

```bash
export CARGO_BUILD_JOBS=4
cargo test -p opencanvas-server --offline --lib
cargo test -p opencanvas-server --offline --test navigation_map --test navigation_context --test tool_schemas --test protocol_2026 --test skill_layer --test transport --test find_nodes --test outline_depth -- --test-threads=2
cargo clippy -p opencanvas-server --offline --all-targets
wc -l crates/server/src/reads/navigation_map.rs crates/server/src/reads.rs crates/server/src/schemas.rs
```

Kiểm live: server tạm `--port 0 --data-root $(mktemp -d)`, seed như AC 8, gọi `get_navigation` (project, page, frame), đo byte, rồi dọn process và thư mục.

## Rủi ro

- **Thêm tool tốn `tools/list` mỗi phiên** (~1 KB). Chấp nhận vì thay một chuỗi đọc 28–90 KB. Mô tả giữ ≤ 700 ký tự.
- **Xung đột số tool và `schemas.rs`** với P02c, P03a. Mỗi nhánh +1 rồi rebase. Dev orchestrator xếp tuần tự.
- **`overlay:true` là suy luận.** Màn vừa được navigate tới vừa được mở overlay thì không có cờ, coder thấy cả hai loại link trong `links`. Ghi trong mô tả.
- **Đồ thị rất lớn** (20 000 node, mỗi node 2 link) phải phân trang nhiều. Vẫn đúng nhờ cursor, chỉ tốn lượt gọi.

## Ngoài phạm vi / phase sau

- **Chẩn đoán flow nâng cao:** màn không tới được từ flow start (BFS), `close` trên màn không bao giờ là overlay, vòng lặp. Có thể thêm `diagnostics` sau khi có phản hồi sử dụng.
- **Xuất đồ thị dạng khác** (Mermaid, route table sinh sẵn): agent tự làm từ JSON.
- **Viewer:** P06c (flow map), P06d (hotspot).
