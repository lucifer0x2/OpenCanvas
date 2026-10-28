# P02c-r2 — Rà note toàn project/page bằng `find_nodes notes` (thay tool `list_notes`)
Ưu tiên: P2 · Kích cỡ: ~45 phút dev · Phụ thuộc: **P02a** (bắt buộc, sau khi merge) · Nguồn: U4

Bản rà lại của `P02c-list-notes.md` (HOÃN 23:50 vì ngân sách `tools/list`). Nhu cầu giữ nguyên: liệt kê note theo project/page/node, lọc open/resolved/all, có `frameId` để gọi `get_design_context`, phân trang. Cách làm mới: **không thêm tool**, thêm tiêu chí `notes` vào `find_nodes`.

## Vấn đề

- Sau P02a, note chỉ đọc được qua `get_design_context`, và tool này bắt buộc `frameId` là frame (`crates/server/src/reads/tree.rs:19-36`). Muốn biết "page/project còn note mở nào" thì phải đọc từng frame.
- Đo trên project 318 node (3 page × 5 frame × 20 node, 12 note): đọc `get_design_context compact:true` cho 15 frame = **60 527 B**.
- `list_notes` (bản cũ) tốn **+1 065 B** `tools/list` cho mọi phiên, kể cả phiên không đụng tới note.

## Bằng chứng

Code ở `night/P02a@12a8b83` (P02a chưa merge vào `night/integration@dcfb7ad`):
- `find_nodes` có sẵn phạm vi project/`nodeId`/`frameId`, preorder, `path`, `totalMatches`, tiêu chí AND (`crates/server/src/reads/find.rs:15-85`, `:87-126`).
- Note theo node, theo ID: `crates/server/src/reads/notes.rs:19-50`. "Frame ngoài cùng chứa node" đã là quy tắc của prerender (`crates/server/src/reads/targets.rs:1-2`).
- `pagination::page` giới hạn 100 mục cho mọi field khác `nodes` (`reads/pagination.rs:88-94`). Nhờ vậy `notes` trong `find_nodes` được phân trang từng note như ở `get_design_context`.
- **Bẫy 1:** `reads.rs:52` chuẩn hoá `notes:"open"` về vắng mặt **trước** nhánh `find_nodes` (`:54`). Nếu để nguyên, tiêu chí `open` sẽ biến mất.
- **Bẫy 2:** `find.rs:114` đặt `totalMatches = entries.len()`. Khi xen entry note thì con số này sai.
- **Bẫy 3:** test ngân sách P02a (`crates/server/tests/notes.rs:149-179`) cộng mọi **câu** mô tả có chữ "note" của mọi tool, trần 2 000 B, đang 1 816 B. Câu mới của `find_nodes` (151 B) đưa tổng lên **1 967 B**, chỉ còn 33 B dư.

Prototype: bản sao tạm của `night/P02a` trong scratchpad, đã xoá sau khi đo. Kết quả:
- 4 test mới pass;
- các test hiện có `find_nodes` (5), `notes` (4, bỏ qua test prerender vì cần browser), `tool_schemas` (2) và `compact_context` (5) đều pass, không phải sửa.

Số đo `serde_json::to_vec` trên mảng `tools`:

| Phương án | Byte thêm vào `tools/list` | Response cho fixture 318 node / 12 note |
| --- | --- | --- |
| **A. `find_nodes` + `notes` (chọn)** | **+196** (find_nodes 2 217 → 2 413; tổng 22 519 → 22 715) | 4 439 B. Hỏi "còn note mở?" với `limit:1`: 459 B |
| B. `get_outline` + `notes` | +186 (cần thêm ~40 B để dạy mẹo `depth:0`, thành ≈ 226) | Outline đầy đủ 29 773 B; `depth:0` + notes 2 419 B |
| C. `get_design_context` nhận page/không frame | đổi `required`, mô tả dài | Kéo theo nodes, tài nguyên, viewport: sai mục đích |
| D. `get_changes` | 0 | Chỉ cho delta theo revision, không cho trạng thái open/resolved theo phạm vi |
| E. Tool `list_notes` (bản hoãn) | **+1 065** | 2 098 B, nhưng chỉ có ID: muốn biết tên/kind/vị trí phải gọi thêm `find_nodes ids` (~2,6 KB) |
| Hôm nay (lách) | 0 | 60 527 B (15 lần `get_design_context compact`) |

Tham khảo: Figma `GET /v1/files/:key/comments` liệt kê comment toàn file (https://developers.figma.com/docs/rest-api/comments-endpoints/); Penpot ẩn thread đã resolve theo mặc định (https://doc.plugins.penpot.app/interfaces/CommentThread).

## Giải pháp

### Lý do chọn A

- Rẻ ngang B, rẻ hơn E 869 B mỗi phiên.
- "Tìm node có note mở" đúng nghĩa của `find`. Không thêm quy tắc mới: phạm vi, AND, `totalMatches 0`, cursor đều theo luật `find_nodes` mà instructions đã dạy.
- Kết hợp được với tiêu chí khác, ví dụ `kinds:["text"]` + `notes`, hoặc `parentId`.
- Mỗi match có sẵn `name/kind/path`, nên agent nói được note nằm ở đâu mà không cần gọi thêm.

B bị loại vì: `depth` lọc node nhưng không lọc note (quy tắc lẫn lộn), không có `totalMatches`, và outline vốn để đọc cấu trúc.

### Request
```json
{"projectId":"p_…","nodeId":"page-checkout","notes":"open"}
```
- `notes`: `"open" | "resolved" | "all"`. Đây là **tiêu chí**: node khớp khi có ≥1 note ở trạng thái đó. Tiêu chí này đủ để thoả điều kiện "cần ít nhất một tiêu chí".
- Khác `get_design_context` (`open|all|none`, bộ lọc đầu ra, mặc định `open`). Hai giá trị `open` và `all` cùng nghĩa ở cả hai tool.
- Phạm vi theo `find_nodes`: project, hoặc cây con `nodeId`/`frameId` **gồm gốc, không gồm tổ tiên**. Note của tổ tiên vẫn đọc qua `get_design_context` như cũ.

### Response (đo thật từ prototype, rút gọn)
```json
{"criteria":{"notes":"open"},"totalMatches":2,"matches":[
  {"id":"P","kind":"page","name":"P","parentId":null,"depth":0,"path":[],"matchedOn":["notes"]},
  {"id":"a1","kind":"rectangle","name":"a1","parentId":"A","depth":2,"path":[…],"matchedOn":["notes"],"frameId":"A"}],
 "notes":[{"id":"note-p","nodeId":"P","text":"…","author":"designer"},{"id":"note-a1","nodeId":"a1","text":"…","author":"designer"}],
 "truncated":false,"nextCursor":null, …}
```
- `notes` ở cấp trên, cùng shape với `get_design_context`. Mỗi note đứng ngay sau match của nó trong danh sách entry: thứ tự preorder, trong một node thì theo ID. Mỗi trang ≤100 note (`pagination::page`). Note có thể sang trang sau match của nó; nối lại qua `nodeId`.
- `frameId` (chỉ có khi dùng `notes`, như `textPreview` chỉ có khi dùng `textContains`) là frame ngoài cùng chứa node, tính cả chính node. Không có key này khi node là page hoặc không nằm trong frame nào.
- `totalMatches` đếm **node** khớp trên toàn phạm vi.
- `limit` đếm cả match và note (như `get_design_context`). Ghi trong `docs/SERVER.md`; với `limit` mặc định 50, một node có 120 note cần 3 trang.
- Không dùng `notes` thì response giống hệt hiện nay: không có key `notes`/`frameId`.
- Cursor ràng theo `notes`: `open` và `all` là hai phạm vi khác nhau, dùng lẫn cursor → `INVALID_CURSOR`.

### Triển khai (theo prototype)
- `reads.rs`: chuyển dòng chuẩn hoá `req.notes` xuống **sau** nhánh `find_nodes`; sửa comment field.
- `reads/notes.rs`:
  - `Filter` thêm `Resolved` và `fn keeps(&Note)`;
  - tách `by_node(s, filter) -> HashMap<&str, Vec<&Note>>` (dùng lại trong `entries`);
  - thêm `outer_frame(index, node)` (`ancestors(id, None)` + node, lấy frame đầu tiên) và `entry(note)`.
- `reads/find.rs`:
  - `Criteria::new(req, s)` thêm `notes: Option<HashMap>`; điều kiện "ít nhất một" và thông báo lỗi thêm `notes`; `matched_on` thêm `"notes"`;
  - thay `filter_map` bằng vòng lặp đẩy match, kèm `frameId` khi dùng `notes`, rồi đến các note của match;
  - `totalMatches = matched`; `echo` thêm `notes`.
- `schemas.rs`:
  - `find_nodes` thêm `"notes":{"enum":["open","resolved","all"]}`;
  - thêm một câu mô tả (151 B): ` notes (open, resolved or all) matches nodes with such notes, listed in notes as in get_design_context; each match then has frameId, its outermost frame.`
  - Schema `get_design_context` giữ `open|all|none`: validator từ chối `resolved` ở đó và `none` ở `find_nodes` (prototype đã kiểm).
- `instructions.rs` thêm ≤70 ký tự vào câu Notes: `find_nodes notes:open lists them across a page or the project.`

## Phạm vi file

| File | Việc | Dòng (prototype) |
| --- | --- | --- |
| `crates/server/src/reads/find.rs` | tiêu chí, `frameId`, entry note, `totalMatches` | 182 → ~205 |
| `crates/server/src/reads/notes.rs` | `Resolved`, `by_node`, `outer_frame`, `entry` | 50 → ~80 |
| `crates/server/src/reads.rs` | dời chuẩn hoá | 97 → 98 |
| `crates/server/src/schemas.rs` | property + câu mô tả | 161 (chỉ dài dòng) |
| `crates/server/src/instructions.rs` | +1 câu | 50 → 51 |
| `crates/server/tests/find_notes.rs` (mới) | AC 1–8 | ~170 |
| `crates/server/tests/notes.rs` | prose P02a bỏ `find_nodes` (đo riêng ở AC8) | +1 |
| `docs/SERVER.md` | bảng/response `find_nodes`; mục Notes thay dòng "`list_notes` (P02c) later" | +8 |
| `crates/server/skills/implement-frame.md`, `author-frame.md` | mỗi file +1 dòng: `find_nodes {nodeId:<page>, notes:"open"}` để rà, `frameId` để đọc | +2 |

## Tương thích MCP

- Không thêm tool. `find_nodes` thêm 1 property tuỳ chọn và 1 câu mô tả (+196 B). Field và nghĩa cũ không đổi. Response không dùng `notes` thì giống hệt byte.
- `get_design_context` không đổi gì: schema, mặc định và hash cursor đều giữ nguyên, vì chuẩn hoá `open` vẫn chạy cho tool này.
- Không đổi storage hay schema DB, không migration.
- Cập nhật: `docs/SERVER.md`, `instructions.rs`, 2 skill.

## AC

Fixture:
- page P (frame A chứa `a1`, `a2` và frame lồng `inner` chứa `i1`; frame B);
- page P2 chứa frame `bare`;
- note open trên P, A, `a1`, `i1`, B; note resolved trên `a2`.

1. `{notes:"open"}`:
   - `matches` lần lượt là P, A, `a1`, `i1`, B;
   - `notes` lần lượt là note-p, note-a, note-a1, note-i1, note-b;
   - `totalMatches:5`;
   - `matchedOn:["notes"]`;
   - `criteria:{"notes":"open"}`.
2. `frameId`: P không có key; A, `a1`, `i1` → `"A"` (frame ngoài cùng, không phải `inner`); B → `"B"`.
3. Phạm vi và AND:
   - `{nodeId:"A",notes:"open"}` → A, `a1`, `i1`;
   - `{notes:"open",kinds:["frame"]}` → A, B;
   - `{nodeId:"P2",notes:"open"}` → `totalMatches:0`, không có key `notes`.
4. Trạng thái:
   - `resolved` → chỉ note-a2, có `resolved:true`;
   - `all` → `totalMatches:6`.
5. Phân trang: 120 note trên một frame, `limit:200`:
   - trang 1 có 100 note, `truncated:true`;
   - trang 2 có 20 note, 0 match, `truncated:false`;
   - cursor trang 1 dùng với `notes:"all"` → `INVALID_CURSOR`;
   - mọi trang ≤ 65 536 B.
6. Không đổi hành vi cũ:
   - `find_nodes {kinds:["frame"]}` trên project có note → JSON không chứa `notes` hay `frameId`;
   - `tests/find_nodes.rs`, `tests/notes.rs`, `tests/compact_context.rs` pass không sửa (trừ dòng prose ở AC8).
7. Schema:
   - `find_nodes {notes:"none"}` → lỗi;
   - `get_design_context {notes:"resolved"}` → lỗi;
   - `get_design_context` mặc định vẫn có note-p, note-a, note-a1, note-i1 với frame A.
8. Ngân sách:
   - property `notes` của `find_nodes` + các câu mô tả `find_nodes` có chữ "note" ≤ **260 B** (prototype: 42 + 151);
   - toàn mảng `tools` tăng ≤ **260 B** so với commit cha (ghi số trước/sau vào `.night/dev/P02c.md`, đo bằng `cargo test --test tools_budget -- --nocapture` nếu F-TOOLS đã merge);
   - test prose của P02a loại `find_nodes` và vẫn ≤ 2 000 B;
   - instructions thêm ≤ 70 ký tự.
9. Modular: `find.rs` ≤ 215, `notes.rs` ≤ 120, test mới ≤ 200 dòng; báo số dòng.

## Cách kiểm

```bash
export CARGO_BUILD_JOBS=4
cargo test -p opencanvas-server --offline --test find_notes --test find_nodes --test notes --test tool_schemas --test compact_context --test context -- --test-threads=2
cargo clippy -p opencanvas-server --offline --lib --bins --test find_notes -- -D warnings
cargo fmt --all --check
wc -l crates/server/src/reads/{find,notes}.rs crates/server/tests/find_notes.rs
```

Kiểm live trên server tạm (`--port 0 --data-root $(mktemp -d)`):
1. `apply_batch` tạo page/frame/rect + 2 `create_note`.
2. Gọi `find_nodes {notes:"open"}`, `{nodeId:<page>,notes:"all"}`, `{notes:"open",limit:1}`, đối chiếu AC 1–4.
3. `tools/list`: `find_nodes` có property `notes`.
4. Dọn process và data.

## Rủi ro

- **File nóng** `schemas.rs`, `find.rs`: rebase sau P03a (`export_nodes`), P06b (`get_navigation`) và các task khác sửa `schemas.rs`. P02c không đổi số tool.
- Ngữ nghĩa `limit` (đếm cả note) khác cách hiểu "matches per page". Đã ghi tài liệu; agent chỉ cần theo `nextCursor`.
- Một note đơn lẻ lớn hơn `budget` (2 000 ký tự + 100 reply × 2 000) trả `CONTEXT_ITEM_TOO_LARGE`. Rủi ro này có từ P02a (`get_design_context`), không mới.
- Response của A lớn hơn E khoảng 2,3 KB mỗi lần rà 12 note, vì mỗi match có `path`/`name`. Đổi lại agent không cần gọi thêm để định vị note.

## Ngoài phạm vi / phase sau

- Đọc note theo **note ID**: P02c cũ có `ids` theo note; ở đây `ids` vẫn là node ID. Note ID từ `get_changes changed.notes` đọc bằng `find_nodes {notes:"all"}` hoặc `get_changes includeOperations`. Thêm `noteIds` chỉ khi có nhu cầu thật.
- Lọc theo `author`/`textContains` trong note.
- Đếm note theo page trong `get_outline`.
- `resolved` cho `get_design_context`.
- `GET /notes` HTTP cho viewer (P02b).
