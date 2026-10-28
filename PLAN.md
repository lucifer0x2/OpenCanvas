# OpenCanvas — kế hoạch triển khai v1

Trạng thái triển khai 2026-10-28: M0–M2 đã được nghiệm thu bằng fixture/module tests
và E2E local thật; M3 resolution/context/layout và M4 storage hardening đã được
kiểm trong phạm vi công bố. Bằng chứng và giới hạn tại `docs/ACCEPTANCE.md`;
hướng dẫn chạy tại `README.md`. Các AC bên dưới vẫn là nguồn yêu cầu.
Workspace hiện có Git và mã triển khai; thay đổi chưa commit/push.
Mục tiêu: agent tạo và đọc thiết kế qua MCP; người dùng nhìn kết quả trong browser local.
Ưu tiên sản phẩm hiện tại là hiển thị thiết kế, không phải xây công cụ chỉnh sửa bằng chuột.

## 1. Ràng buộc sản phẩm và lựa chọn triển khai

Đã chốt với người dùng:
- Backend Rust là nơi giữ mô hình thiết kế, revision, kiểm tra thay đổi và trạng thái project.
- Agent trò chuyện với người dùng trong Codex/Claude bên ngoài OpenCanvas.
- Agent đọc/ghi nodes, styles, layout, tokens và asset metadata qua MCP ngay trong MVP.
- Storage dùng SQLite; mỗi project là thư mục `~/.opencanvas/<project>/`, chứa `db.sql` và `assets/` cho mọi asset/font của project.
- File `db.sql` là SQLite binary; giữ nguyên đuôi `.sql` người dùng đã chọn.
- Browser local ưu tiên hiển thị thiết kế chỉ đọc; không có editing, chat hoặc terminal trong sản phẩm.
- Giao diện frontend dùng tone sáng theo hướng editor Figma, không yêu cầu clone pixel-perfect: panel/topbar trắng, canvas xám sáng, chữ tối rõ tương phản, divider mảnh và accent xanh cho selection/focus.
- UI gọn, trung tính để thiết kế nổi bật; bố cục topbar, layers bên trái, canvas chính và inspector chỉ đọc bên phải. Panel có thể thu gọn; trạng thái hiển thị gọn. Chưa cần dark theme hoặc toggle đổi theme giao diện.

Khuyến nghị triển khai; renderer/layout phải qua spike M0 trước khi khóa:
- SQLite WAL đã chốt cho MVP; benchmark kiểm hiệu năng theo workload dự kiến và xác định giới hạn vận hành.
- Một backend instance thường trực phục vụ cả MCP lẫn viewer, dùng cùng state và writer queue.
- MCP dùng Streamable HTTP; viewer dùng HTTP để đọc snapshot và SSE để nhận thay đổi.
- Asset/font của project ở `~/.opencanvas/<project>/assets/`; không ghi asset binary vào node JSON.
- Frontend React + TypeScript + Konva chạy trong browser local, chỉ đọc thiết kế.
- Schema ứng dụng độc lập Konva; không lưu cây Konva làm mô hình thiết kế gốc.
- Batch mutation phải atomic, có `expectedRevision` và khóa idempotency.
- Backend commit durable trước khi báo thành công hoặc publish SSE.
- Revision cơ bản cần từ đầu; lịch sử trực quan, diff và inspect nâng cao làm sau lát cắt hiển thị.
- Không hứa pixel-perfect giữa mọi hệ điều hành, browser hay font chưa được cung cấp.

## 2. Đề xuất cần spike và giới hạn phạm vi

Khuyến nghị dùng một module renderer web chung cho viewer và browser worker chạy headless.
Worker đo text và render PNG; Rust quản lý layout, scene và toàn bộ trạng thái ứng dụng.
Chromium cùng runtime JavaScript của worker là dependency có chủ đích của phương án này.
Backend Rust không đồng nghĩa toàn bộ hệ thống chỉ có Rust hoặc không cần browser runtime.
Spike đầu tiên phải kiểm chứng phương án trước khi khóa thư viện và giao thức worker.
Nếu spike không đạt, dừng mở rộng layout và ghi quyết định thay thế cụ thể trước triển khai tiếp.
Phần viewer chỉ đọc và contract dữ liệu không phụ thuộc việc đã khóa engine layout hay chưa.

MVP hỗ trợ page, frame, group, rectangle, text và image; group dùng layout hoặc tọa độ tự do.
Layout gồm row/column, padding (một số hoặc 1–4 cạnh kiểu CSS), gap, `align`/`justify` (P08a), fixed/fill/hug, min/max và row wrap opt-in đã duyệt ngày 2026-10-28.
Frame gốc có kích thước cố định; không hỗ trợ chu kỳ phần trăm hoặc fill trong trục hug mơ hồ.
Text dùng một font/style mỗi node, plain text, line height tường minh và wrap theo chiều rộng.
Image chỉ cần fit/contain/cover, crop đơn giản, opacity và clip theo frame.
MVP có literal tokens kiểu color/dimension/number, primitive styles và typed refs ở property được hỗ trợ.
Resolver v1 trả giá trị literal/style đã resolve; alias/theme nâng cao và UI quản lý token làm sau.
Asset/font metadata đọc/list được qua MCP; không đòi UI quản lý resource để dùng MVP.
Component/variant tối giản là giai đoạn sau MVP; chưa triển khai override hoặc inheritance phức tạp.
Responsive v1 chỉ là agent chọn viewport hữu hạn và bố cục trong phạm vi row/column đã công bố.
Không cam kết tự động mô phỏng CSS media query hoặc mọi hành vi responsive của website.

### Task04 — row wrap được duyệt trước triển khai

Phạm vi theo `docs/TASK04-PROPOSAL.md`, người dùng duyệt rõ “Duyệt, triển khai row wrap”.
`layout.wrap` là literal boolean, mặc định false và bỏ khỏi serialization khi false.
Thiếu/false giữ hành vi cũ ở row/column/free; true chỉ cho container page/frame/group
với mode row. Sai kiểu (null/string/number/ref) bị từ chối; storage vẫn reject unknown keys.
Wrapping row cần width số hữu hạn >0 hoặc fill qua chuỗi ancestor fill tới width số >0;
width thiếu/hug hoặc chỉ có min/max không definite. Direct layout chốt selected frame
bằng viewport hữu hạn; storage kiểm mọi theme, chốt root frame fixed cả hai trục.
Child trực tiếp width fill cần minWidth tường minh >0 (dimension ref resolve hợp lệ);
fixed/hug child hợp lệ, không áp guard này cho text fill nằm trong card column.
Row rỗng hợp lệ; giữ guard fill trong hug hiện có trên cả hai trục.

Min/max hữu hạn 0…100000, min≤max trên cả hai trục; equality hợp lệ. Fixed ngoài
interval vẫn clamp. Không reject tổng minima vượt viewport: child quá lớn vẫn overflow.
Storage validate candidate cuối, mọi theme và update/token/theme/reparent atomically;
lỗi VALIDATION_FAILED có field, không partial snapshot/revision/log/idempotency.
Direct guard chạy trước measurement, lỗi LAYOUT_INVALID; sai kiểu deserialize INVALID_JSON.

Mapping Taffy FlexWrap::Wrap + AlignContent::START chỉ khi wrap true, align-items START;
giữ nguyên default no-wrap. Sibling order không đổi; gap giữa card và giữa hàng.
Fill grow1/shrink1/basis0 chia theo từng hàng; hàng cuối có thể fill toàn width, không Grid.
Hug height = padding + tổng max height từng hàng + row gaps; fixed/min/max vẫn clamp,
có thể overflow dọc. Giữ callback cache-only, batch measurement mỗi pass, tối đa6 passes,
text cache key/line list/renderer và pixel/node/depth budgets hiện có; input bất biến.

AC: fixture mới 1040→3 card/1 hàng,640→2+1,390→1+1+1, min280/gap16/padding24;
geometry row hug80/156/212, Vietnamese long/unbroken text128/224/320, không overflow
frame/card/text hoặc overlap. Biên920/919/624/623; fixed row height align start,
nested definite width, fixed/hug children, equal min/max/clamp, oversized-min overflow
control, omitted/false no-wrap compatibility, repeat cache0 batch và input bất biến.
Negative direct/storage guards gồm sai mode/type, indefinite width, thiếu/nonpositive
fill minimum, min>max hai trục; token/theme/update/reparent và late invalid op rollback.
MCP temp root create/apply/read/context/reflow/inspect/render ba viewport, restart giữ
wrap/revision/bounds/PNG; viewer GET-only dùng shared scene/render ở1040 và390.
Giữ đủ pipeline11 và E2E15 nhóm cũ, thêm wrap; chạy layout tests, full storage offline
fault-injection (2 threads), server wrap + multi_viewport/skill_layer, E2E, strict library
Clippy layout/storage/server, fmt, JS syntax/format, diff/size checks (mã≤250 dòng).

Không DB migration: props JSON additive; DB schema2, snapshot/API/scene schema1 giữ nguyên.
Scene/job key gồm resolved input và startup pipeline fingerprint; build layout mới rồi
restart backend mới đổi cache identity. Backend/layout phải nâng cùng nhau; engine cũ
có thể bỏ unknown field, không hứa mixed-version/downgrade/hot replacement. Task không
restart3847, không đổi renderer/web behavior, real demo hoặc no-wrap golden.
Không thêm %/breakpoint/column-wrap/wrap-reverse (align/justify thêm sau ở P08a). Wrap không tự sửa dashboard:
sidebar/body minima, fixed chart/progress và fixed text heights vẫn cần author riêng.

Ngoài phạm vi: user editing, drag/drop sửa node, undo UI, chat, terminal và AI provider API.
Ngoài phạm vi: cộng tác nhiều người, cloud hosting, plugin Figma và full Figma clone.
Ngoài phạm vi: full CSS/Grid, rich text, vector path editor, prototype tương tác (trừ link điều hướng khai báo `links`/`flowStart`: click/hover → navigate/overlay/back/close) và animation.
Không xây Postgres adapter, kiến trúc đa dịch vụ hoặc event sourcing tổng quát trước khi cần.

## 3. Luồng kiến trúc và quyền sở hữu dữ liệu

```text
Codex / Claude ── MCP Streamable HTTP ─┐
                                     v
                              Rust backend
                          domain + revision queue
                           /                  \
                  SQLite + assets       compute/render jobs
                           \                  /
                            HTTP snapshot + SSE
                                     |
                           React / Konva viewer

Rust compute jobs <── measure/render protocol ── headless browser worker
Viewer và worker cùng dùng web-renderer module + font manifest + scene version.
```

Authored model là dữ liệu agent viết: tree, properties, layout rules, references và text.
Computed scene là kết quả của snapshot revision cụ thể: bounds, lines, transforms và diagnostics.
Rust xuất computed scene; viewer không tự chạy một engine layout thứ hai.
Module web chung vẽ scene và cung cấp text measurement; không tự sửa authored model.
Taffy là ứng viên layout row/column, dùng custom measure; Taffy không tự đo hay shape text.
Không trả số đo placeholder như computed result khi worker hoặc font chưa sẵn sàng.
Scene chưa tính xong phải trả `pending`, hoặc `failed` kèm lỗi rõ; authored snapshot vẫn đọc được.
Job cũ có thể hoàn thành nhưng không được thay thế scene mới đang được viewer yêu cầu.
Cache theo project/revision/frame/viewport/theme/state và phiên bản pipeline; mất cache vẫn dựng lại được.

## 4. Storage và schema v1

### Quyết định engine và điều kiện đánh giá lại

Workload là thiết kế có tree/refs/revision và batch nhiều entity cần commit cùng nhau, đọc snapshot thường xuyên.
Nhiều agent gọi cùng backend đi qua một writer queue; không đồng nghĩa có nhiều writer độc lập vào file.
SQLite WAL là lựa chọn MVP đã chốt; benchmark kiểm hiệu năng và giới hạn vận hành, không phải bước chờ chọn engine.
Quyết định này không khẳng định SQLite tối ưu tuyệt đối; các alternative dưới đây được giữ làm cơ sở đánh giá lại khi có số đo.
Agent/runtime/chat nằm ngoài app; MVP không cần agent-memory hoặc vector database để giữ thiết kế.

| Ứng viên | Đánh giá theo contract thiết kế |
| --- | --- |
| [SQLite WAL](https://www.sqlite.org/wal.html) | Transaction/constraints/query phù hợp snapshot + revision + idempotency; một writer, cần đo queue và checkpoint. |
| [redb](https://github.com/cberner/redb/blob/master/docs/design.md) | Đối thủ thực sự: pure Rust ACID KV/MVCC; vẫn tuần tự một writer, tree và secondary indexes phải tự thiết kế. |
| JSON atomic replace hoặc envelope + op log | Khả thi; phải tự bảo đảm durability, recovery, compaction và consistency nhiều file, không mặc định đơn giản hơn. |
| [DuckDB](https://duckdb.org/docs/stable/connect/concurrency) / [LanceDB](https://docs.lancedb.com/faq/faq-oss) | DuckDB hướng phân tích OLAP; LanceDB hướng retrieval/fragments, không trực tiếp thay contract transaction tài liệu thiết kế trong một file. |
| [AgentFS](https://github.com/tursodatabase/agentfs) | Có thật, dùng SQLite cho filesystem/KV/tool calls của agent runtime, còn beta; không tự cung cấp semantic contract nodes/refs/revisions. |
| [Memvid](https://github.com/memvid/memvid) | Có thật, single-file recall/memory; chưa xác minh guarantee transaction nhiều entity cần cho thiết kế. |

Revisit khi đo thấy write queue bão hòa, hoặc tree/index representation là bottleneck của project lớn.
Nếu cần nhiều backend/máy cùng ghi, đánh giá server database; không đặt SQLite WAL trên network filesystem.
Chỉ đổi engine sau phép đo và quyết định riêng; không xây đồng thời hai backend hoặc hứa migration trong MVP.

### Persistence, durability và assets

Từ M1, project slug chỉ gồm ký tự cho phép, có giới hạn độ dài; không nhận đường dẫn từ client làm slug.
Backend canonicalize slug/path theo cùng một quy tắc khi tạo và mở project; resolve project root bên trong data root.
Chặn `..`, separator trong slug và symlink dẫn ra ngoài root; mọi đường dẫn bên trong project phải qua containment guard theo project root đã canonicalize.
Khởi tạo project chỉ khi gọi API tương ứng; PLAN này không tạo `~/.opencanvas`.
Mỗi project có cấu trúc:

```text
~/.opencanvas/
└── <project>/
    ├── db.sql          # SQLite binary
    ├── db.sql-wal      # WAL sidecar khi có
    ├── db.sql-shm      # shared-memory sidecar khi có
    └── assets/         # mọi asset/font của project
```

Project ID ổn định độc lập slug; manifest DB chứa ID, slug, schema version và revision hiện tại.
Chỉ một process được sở hữu data root; startup instance thứ hai phải lỗi rõ bằng lock.
Mỗi project có writer queue duy nhất, đọc dùng connection riêng; bật WAL và foreign keys.
File `~/.opencanvas/<project>/db.sql` là SQLite binary, không phải SQL dump; sidecars `db.sql-wal` và `db.sql-shm` nếu có nằm cạnh `db.sql` trong cùng project root.
Bundle/pin SQLite đã vá WAL-reset: >= 3.51.3 hoặc backport được xác minh như 3.44.6/3.50.7.
Kiểm version runtime thực dùng lúc build/test; không dựa vào SQLite có sẵn của hệ điều hành.
Chốt `synchronous=FULL`, `foreign_keys=ON`, busy timeout hữu hạn và checkpoint policy qua spike storage.
Busy timeout có giới hạn; lỗi bận phải phân biệt với revision conflict và lỗi validation.
Đo ngưỡng checkpoint/WAL growth và khả năng checkpoint tiến sau khi reader kết thúc; không checkpoint cưỡng bức trên mỗi batch.
Không giữ SQLite transaction, kể cả read transaction, qua SSE, đo text, render, tải asset hay gửi network.

| Bảng | Cột chính và trách nhiệm |
| --- | --- |
| `project_meta` | Một hàng: `project_id`, `slug`, `schema_version`, `current_revision`, timestamps. |
| `nodes` | `id`, `parent_id`, `sort_key`, `kind`, `props_json`; parent FK, root page hợp lệ. |
| `styles` | `id`, `kind`, `name`, `value_json`; primitive style có version schema. |
| `tokens` | `id`, `name`, `type`, `value_json`; MVP chỉ literal và binding bằng ID đúng kiểu. |
| `assets` | `id`, `content_hash`, `media_type`, `byte_size`, `relative_key` tương đối với `<project>/assets/`, dimensions/status. |
| `fonts` | `id`, family/weight/style, `asset_id` hoặc bundled ID, hash/license metadata nếu có. |
| `commits` | `revision` PK, `base_revision`, `batch_id`, `ops_json`, `summary_json`, timestamp. |
| `idempotency` | `key` PK, request hash, committed revision, response JSON; retention công bố. |

`props_json` chứa geometry/layout/style refs/text/image refs theo discriminated schema của `kind`.
Page là root node; frame/group chứa children qua `parent_id` và thứ tự `sort_key` ổn định.
Index `(parent_id, sort_key)` phục vụ đọc children; candidate tree trong RAM được validate trước commit.
SQLite không tự tối ưu traversal mọi tree; representation/index/serialization phải được đo riêng với engine overhead.
Không đổi sang một document blob lớn chỉ từ nghiên cứu engine khi chưa đo workload cập nhật/reparent.
V1 dùng JSON có schema và validation; chưa tách bảng cho từng property hoặc tạo generic EAV.
Node tree hiện tại là snapshot authored đã materialize; `commits` là op log để replay thông báo/diff.
Cập nhật snapshot, tăng revision, ghi commit và idempotency result trong cùng một transaction.
Không cần lưu một bản full snapshot cho mọi commit; không bắt hệ thống rebuild DB từ op log.
Retention log/idempotency phải có metadata để client biết khi nào cần refetch hoặc key đã hết hạn.
Migration versioned chạy trước khi mở project cho ghi; lỗi migration giữ DB gốc và báo lỗi cụ thể.
Mọi asset/font binary của project nằm trong `~/.opencanvas/<project>/assets/`; `relative_key` resolve từ thư mục này, không nhận absolute path hoặc traversal.
Containment guard kiểm đường dẫn đã canonicalize nằm trong `assets/` của đúng project và trong project root, kể cả khi có symlink; không cho đọc/ghi sang project khác.
Asset binary không atomic cùng SQLite: lưu immutable theo content hash, ghi temp, fsync rồi atomic publish và fsync thư mục trước commit DB ref.
Chỉ ACK import sau khi binary và metadata đã durable; crash trước DB commit có thể để lại orphan.
GC trì hoãn phải giữ assets của revision manifest và handles render/export đang dùng; MVP chưa cần GC nền phức tạp.
Backup dùng [SQLite backup API](https://www.sqlite.org/backup.html) hoặc quiesce/checkpoint hợp lệ; không copy live main file.
Backup/export bao trùm project folder: snapshot `db.sql` tại một revision, manifest hashes và đủ binary trong `assets/` của revision đó, giữ chúng tới khi hoàn tất.
Sidecars đang hoạt động không phải bản backup độc lập; tạo snapshot DB nhất quán bằng cơ chế SQLite nêu trên, không copy nguyên project folder đang ghi rồi coi là backup hợp lệ.
Restore kiểm manifest/hash để phát hiện thiếu binary; snapshot DB thành công riêng chưa chứng minh backup project nhất quán.
Export/import là bundle versioned của project folder, chứa snapshot `db.sql` với authored data/refs, asset manifest và asset/font cần thiết trong `assets/`.
Import/restore validate schema/hash/path và containment trước khi activate project folder đích; import thành project mới, giữ node IDs nhưng revision bắt đầu lại.
Chốt format bundle ở giai đoạn hardening; không dùng SQL dump làm public interchange contract.

## 5. Transaction, revision và đồng bộ

Mỗi batch áp dụng cho một project và một expected revision; thành công tăng revision đúng một lần.
Trong writer queue: tìm idempotency key trước, rồi kiểm revision, validate candidate và commit.
Cùng key + cùng request hash trả lại kết quả cũ kể cả expected revision hiện đã cũ.
Cùng key + payload khác trả `IDEMPOTENCY_KEY_REUSED`; không thực hiện một phần batch.
Request hash dùng canonical payload, bao gồm project, expected revision và operations.
Từ M1, validation kiểm schema, giới hạn batch, ID, tree cycle, refs, loại parent, giá trị hữu hạn và layout hỗ trợ.
Token/style/asset/font ref phải có record; sai kiểu hoặc binding chưa hỗ trợ bị từ chối, không âm thầm đổi ref.
Record asset/font tồn tại nhưng binary chưa ready có trạng thái missing riêng; alias chưa thuộc MVP.
Validation structural thực hiện trước commit; measure/layout/render chạy từ snapshot bất biến ngoài transaction.
Lỗi render sau commit không rollback authored design; response/readback thể hiện trạng thái render riêng.
Commit thành công rồi mới enqueue compute và publish event; crash sau commit vẫn replay được từ log.
Client mất response có thể retry cùng key; không tạo node hay tăng revision lần thứ hai.
Revision conflict trả current revision cùng hướng dẫn đọc lại; backend không tự merge thay đổi ngầm.

SSE không là nguồn dữ liệu duy nhất: HTTP snapshot và committed revision là mốc tin cậy.
Snapshot ban đầu chứa `projectId`, `revision`, authored scope và trạng thái scene tương ứng.
Kết nối event có cursor `projectId:revision`; server replay mọi commit mới hơn cursor còn trong log.
Đăng ký stream trước khi chụp snapshot R, rồi replay từ durable log sau R để không rơi event giữa hai bước.
Client bỏ qua revision đã xử lý; gặp khoảng trống phải replay, hoặc refetch khi log không còn.
`reset_required` nêu revision hiện tại khi cursor sai project, hết retention hoặc không hợp lệ.
Event `scene_ready` mang source revision và scene key; không thay thế revision của authored model.
Sau reconnect viewer chỉ hiện scene khớp snapshot đang chọn; trạng thái pending/stale hiển thị rõ.
Heartbeat, giới hạn buffer và reconnect backoff ngắn có jitter tránh giữ connection chết vô hạn.

## 6. Contract API và MCP

HTTP API có version `/api/v1`; MCP endpoint `/mcp` bind localhost mặc định, origin kiểm soát.
Schema MCP dùng JSON Schema rõ kiểu, required fields, enums, bounds và response/error envelope.
Không đưa filesystem path tuyệt đối hoặc secret vào kết quả tools, log gửi client hay diagnostics.
Project listing chỉ trả ID/slug/name/revision; asset trả opaque ID hoặc local HTTP URL có kiểm soát.
M1 cố định outline/context tối đa 200 nodes, depth 8, context 64 KiB; resource list tối đa 100 items.
Batch/pixel budget chốt từ spike; mọi giới hạn có server cap, truncation hoặc lỗi riêng.

| MCP tool | Input chính | Output chính |
| --- | --- | --- |
| `list_projects` / `create_project` | cursor hoặc slug/name | Project refs, revision 0 cho project mới. |
| `get_outline` | project, page/frame, depth, limit | Tree rút gọn, IDs/kinds/names, truncation rõ; cursor bổ sung M3. |
| `get_subtree` | project, node, depth, property filter | Authored subtree tại một revision nhất quán. |
| `get_design_context` | project, frame, viewport, budget | Semantic tree, resolved literal/style values, token/asset/font refs, diagnostics. |
| `list_tokens` / `list_styles` / `list_assets` / `list_fonts` | project, IDs/filter, limit | Typed values hoặc metadata/hash/readiness tại revision nhất quán; không trả binary. |
| `apply_batch` | project, expectedRevision, idempotencyKey, operations | committedRevision, changed IDs, render status. |
| `import_asset` | project, bounded binary/resource input, media type | Validated asset ref; không nhận arbitrary local path. |
| `render_frame` | project, revision, frame, viewport, theme/state, scale | PNG resource hoặc job ID, provenance, readiness/errors. |
| `inspect_layout` | cùng scene key, node scope | Computed bounds/lines/rules và diagnostics. |
| `find_nodes` | project, page/frame/node scope, kinds/nameContains/textContains/ids, limit, cursor | Danh sách match phẳng có path tổ tiên, totalMatches, truncation rõ. Bổ sung chỉ đọc, người dùng duyệt 2026-10-28 (Task06). |
| `get_changes` | project, sinceRevision, limit, budget | Commits + changed ids phân loại theo snapshot hiện tại; conflict mang `changedSince`. Người dùng duyệt 2026-10-28 (Task07D). |

`inspect_layout` bản đầu chỉ bounds/status; giải thích sâu và diff tools mở ở giai đoạn sau.
HTTP tối thiểu: list/create project, GET snapshot, GET events, GET asset và GET render job/resource.
Viewer chỉ dùng HTTP đọc; không gọi mutation và không cần tích hợp MCP client vào browser.
Scene/snapshot trả `schemaVersion`, `revision`, scope và `truncated`; không cắt âm thầm theo token budget.
Context mặc định là outline trước, subtree theo nhu cầu; không luôn trả cả project hoặc base64 mọi ảnh.
MCP PNG có resource ref/mime/dimensions, và có thể image content theo khả năng client, không chỉ đường dẫn file.

Ví dụ batch đầu tiên, ID do caller tạo để dùng ngay trong cùng batch:

```json
{
  "projectId": "p_demo",
  "expectedRevision": 0,
  "idempotencyKey": "demo-first-frame-v1",
  "operations": [
    {"op":"create_node","id":"page_1","kind":"page","parentId":null,"props":{"name":"Home"}},
    {"op":"create_node","id":"frame_1","kind":"frame","parentId":"page_1","props":{"width":800,"height":600,"layout":{"mode":"column","padding":24,"gap":16}}},
    {"op":"create_node","id":"text_1","kind":"text","parentId":"frame_1","props":{"text":"OpenCanvas","width":"fill","height":"hug","fontId":"font_bundled","fontSize":32,"lineHeight":40}}
  ]
}
```

Các op v1: create, update properties, reparent/reorder, delete subtree, upsert literal token/primitive style.
Token/style upsert qua `apply_batch` dùng cùng validation/revision/idempotency; không dùng arbitrary JSON Patch.
Operation được xử lý theo thứ tự trong candidate model; ref forward chỉ cho phép nếu được quy định rõ.
Response thành công: `committedRevision: 1`, changed IDs, `renderStatus: pending`, diagnostic refs nếu có.
Envelope lỗi: `code`, `message`, `retryable`, current revision nếu liên quan và chi tiết field/op bị lỗi.
Mã chính: `VALIDATION_FAILED`, `REVISION_CONFLICT`, `IDEMPOTENCY_KEY_REUSED`, `CONTEXT_LIMIT`, `RENDER_FAILED`.
Geometry dùng logical pixel; authored x/y thuộc parent-local, computed bounds phải ghi coordinate space.
Render/inspect dùng frame-local origin (0,0), transform rõ ràng; không trộn canvas viewport pan/zoom vào design.
Viewport/theme/state luôn được resolve thành giá trị cụ thể trong provenance, kể cả khi caller dùng default.
Authored và computed nằm ở các field riêng; không ghi computed width/line breaks ngược vào authored props.

## 7. Layout, typography và render readiness

Spike dùng fixture font bundled cố định, text Latin/Vietnamese, wrap nhiều dòng, image và nested row/column.
Kiểm license font trước khi bundle; font hệ thống không là mặc định cho fixture reproducible.
Worker nạp font đúng hash/weight/style, đợi `document.fonts` và image decode trước đo hoặc chụp.
Font thiếu dùng fallback đã khai báo và trả diagnostic; chế độ strict render phải fail readiness.
Asset thiếu/hỏng có placeholder viewer có nhãn lỗi; strict PNG không báo success như thể asset đầy đủ.
Một implementation wrapping/line-break chung cung cấp measurement và line list cho scene.
Không để Taffy callback và Konva Text tính wrapping độc lập từ cùng chuỗi.
Frontend vẽ line list/positions đã tính; module chung giữ font loading, measurement và drawing thống nhất.
Taffy measure callback đồng bộ, không thể await browser RPC; không gọi roundtrip blocking cho từng leaf.
M0 thử premeasure theo batch và cache text/style/font hash/width/renderer version; pass thiếu cache thu constraints.
Rust gửi cả batch misses rồi chạy lại layout có giới hạn; scene chỉ publish khi mọi số đo cần thiết đã sẵn sàng.
Đây là protocol cần chứng minh qua fixture; nếu constraints không hội tụ phải quyết định lại sau spike.
Vòng layout/measure có giới hạn pass, timeout và chẩn đoán non-convergence; không block writer queue.
Spike phải kiểm tra width constraint → wrap → hug height và fill/hug trong nested container.
Quy tắc overflow/clip, rounding, baseline và line height phải được ghi thành fixture trước khi mở rộng.
Taffy chỉ được khóa sau khi custom measurement và vòng tính này đáp ứng fixture thực tế.

Render result mang project ID, revision, frame ID, viewport, theme/state và scale đã resolve.
Kèm `sceneVersion`, `rendererVersion`, browser/runtime version và font/asset manifest hashes.
Kèm readiness từng font/asset, diagnostics, error code và source scene key; job có timeout/cancel.
Viewer và PNG cùng scene, font manifest và renderer version; mismatch phải invalidation/refetch.
PNG có thể hoàn thành cho revision cũ khi caller yêu cầu chính xác revision còn sẵn; gắn nhãn provenance.
V1 không hứa render mọi revision lịch sử: nếu snapshot requested không còn thì trả `REVISION_UNAVAILABLE`.
Render concurrency/pixel budget giới hạn; worker crash chỉ làm job fail/retry, không mất committed design.

## 8. Viewer chỉ đọc

- Dùng visual đã chốt: topbar và panel trắng, canvas xám sáng, chữ tối rõ tương phản, divider mảnh; accent xanh biểu thị selection/focus. Giữ UI gọn, trung tính theo hướng editor Figma để thiết kế nổi bật, không yêu cầu clone pixel-perfect.
- Bố cục gồm topbar, layers bên trái, canvas chính và inspector chỉ đọc bên phải; cho phép thu gọn panel khi cần không gian xem. Trạng thái/revision được trình bày gọn, vẫn phân biệt rõ các lỗi và scene đang chờ.
- Chọn project → page → frame; chọn viewport hữu hạn phù hợp frame và phạm vi responsive đã công bố.
- Canvas pan/zoom/fit; trạng thái này là local view state, không tạo design revision.
- Layers tree chọn/highlight node; inspect cơ bản tên, loại, authored props và computed bounds khi sẵn sàng.
- Không có input sửa design, resize handle, mutation shortcut, property editing hoặc lịch sử undo.
- Loading/pending/reconnecting/missing font/missing asset/render failed phải có trạng thái phân biệt.
- Hiện revision đang xem và báo đang chờ scene mới; không gắn revision mới cho hình cũ.
- Reload giữ project/page/frame hợp lệ; nếu target bị xóa chọn fallback rõ ràng, không crash.
- Hiển thị semantic design context dạng inspect cơ bản; PNG không thay thế dữ liệu node cho agent.
- Tương tác với thiết kế chỉ gồm chọn/highlight, pan/zoom/fit và chọn viewport; không thêm resize handle, user editing, chat hoặc terminal. Chưa cần dark theme/toggle; không tạo design system riêng hoặc screenshot giả để minh họa kế hoạch.

## 9. Milestone theo lát cắt và tiêu chí nghiệm thu

### M0 — spike pipeline trước khi mở rộng

Tạo skeleton Rust + web module + worker nhỏ nhất, chưa xây toàn bộ backend/framework phụ.
Fixture một frame gồm text wrap, rectangle và image; thử custom measure với ứng viên Taffy.
AC: viewer và PNG dùng cùng computed scene/font/renderer manifest; text không bị wrap lần hai.
AC: đổi width làm đổi line list/hug height đúng fixture; font/asset chưa ready không cho strict success.
AC: chứng minh batched measurement/cache với callback đồng bộ và bounded relayout, không một RPC mỗi leaf.
Kiểm tra targeted: một font fixture có dấu, hai width, một image decode chậm và một font thiếu.
Đầu ra: quyết định engine layout/protocol, runtime/version pin, fixture và giới hạn layout được xác minh.
Spike storage kiểm hiệu năng SQLite đã chốt; ghi quy mô fixture nhỏ/lớn, phần cứng và latency budget trước khi benchmark; không đặt số pass giả khi chưa đo.
Đo batch create/update/reparent trên hai quy mô tree: p50/p95, bytes ghi, CPU/RAM, query/serialization và thời gian commit.
Đo 1/4/8 clients gửi batch khi viewer đọc: queue wait, revision conflicts, WAL growth/checkpoint và engine overhead tách riêng.
Correctness bắt buộc: snapshot nhất quán, tree/refs hợp lệ, revision/idempotency đúng; kết quả latency so với budget đã ghi.
Không qua M0 thì chưa mở rộng tính năng layout hoặc tạo abstraction renderer tổng quát.

### M1 — agent tạo thiết kế và nhìn thấy ngay

Xây project create có slug/path guard, schema SQLite, atomic/idempotent apply batch, snapshot, SSE và viewer frame.
Nối `render_frame` qua worker M0; thêm outline/context có cap, resource reads và literal token/style upsert/resolution.
AC: MCP tạo page/frame/text/rect/image → commit → browser hiển thị → readback/PNG khớp revision.
AC: viewer dùng tone sáng theo hướng editor Figma đã chốt: topbar/panel trắng, canvas xám sáng, chữ tối rõ tương phản, divider mảnh, accent xanh cho selection/focus; bố cục topbar + layers trái + canvas chính + inspector chỉ đọc phải, panel thu gọn được khi cần và status gọn.
AC: viewer chỉ cho chọn/highlight/pan/zoom/fit/chọn viewport đối với thiết kế; không có resize handle, user editing, chat hoặc terminal; chưa cần dark theme/toggle giao diện.
AC: MCP đọc nodes/layout, resolved styles/tokens và asset/font metadata; đổi literal token cập nhật scene phụ thuộc.
AC: project tạo đúng cấu trúc `<project>/db.sql` và `<project>/assets/`; restart backend và reload browser vẫn thấy node IDs, nội dung, asset refs và revision cũ.
AC: batch invalid/missing ref/sai kiểu không để lại commit; retry cùng key không tăng revision; slug/path sai bị chặn.
Kiểm tra targeted: một E2E MCP → DB → viewer → render → restart, dùng project tạm trong test root.
Đầu ra: bản chạy local đầu tiên, hướng dẫn chạy Rust/viewer/worker và fixture tái lập được.

### M2 — consistency và phục hồi trong sử dụng thực

Kiểm sâu contract expectedRevision/idempotency/atomicity đã có từ M1; hoàn thiện replay SSE, job guard và lỗi resource.
AC: hai batch cùng base revision chỉ một batch thắng; loser nhận conflict và có thể đọc lại.
AC: retry cùng key sau mất response trả kết quả cũ; payload khác cùng key bị từ chối.
AC: ngắt SSE quanh lúc commit rồi reconnect nhận đủ thay đổi; cursor hết retention refetch được.
AC: job R hoàn thành sau R+1 không ghi đè scene hiện tại; missing font/asset có status đúng.
Kiểm tra targeted: transaction integration, conflict/retry, SSE reconnect/gap và worker failure fixtures.
Thêm workload crash trước/sau commit và retry mất response: không partial batch, không double revision, không ACK mất dữ liệu.
Crash quanh asset publish/DB commit: committed ref có binary durable; orphan được nhận diện, không xóa asset đang dùng.
Đầu ra: viewer dùng ổn định qua reconnect/restart; không chỉ demo happy path.

### M3 — context và resolution nâng cao trong phạm vi công bố

Thêm pagination/tối ưu context budgets, token alias/theme resolution, inspect giải thích và layout constraints còn thiếu.
AC: outline/subtree/context thống nhất revision; truncation có cursor; authored/computed phân biệt rõ.
AC: khi thêm alias phải chặn cycle/refs sai kiểu trước commit; theme/alias đổi invalidates đúng scene phụ thuộc.
AC: fixture nested row/column, fixed/fill/hug, padding/gap, overflow và text line height đều đạt.
Kiểm tra targeted: semantic contract, dependency invalidation và font/render regression bằng fixture cố định.
Đầu ra: danh sách tính năng được hỗ trợ và lỗi có cấu trúc cho các trường hợp chưa hỗ trợ.

### M4 — hardening dữ liệu và mở rộng có điều kiện

Thêm migration/backup/export/import versioned; inspect/diff sâu hoặc component/variant chỉ khi cần thật.
AC: export/import project folder round-trip giữ authored data và resource hashes trong `db.sql` cùng `assets/`; migration lỗi không phá DB gốc.
Workload backup/restore project folder khi có writes và render: DB revision, resource manifest và toàn bộ binary cần thiết phải khớp.
AC: restore mở được project tại revision đã chụp, kiểm đủ hashes; GC không làm hỏng backup/export đang chạy.
AC: mở rộng failure tests cho containment/slug guard có từ M1, symlink/archive traversal và asset vượt giới hạn.
Kiểm tra targeted: một fixture migration, round-trip bundle, malformed archive và containment guards.
Đầu ra: tài liệu vận hành local, giới hạn đã biết, quy tắc nâng schema và khôi phục project.

## 10. Cách kiểm chứng và điều kiện hoàn tất MVP

MVP hoàn tất khi M0–M2 đạt; M3/M4 có thể giao từng phần theo nhu cầu, không chặn việc nhìn thiết kế.
Mỗi milestone lưu lệnh đã chạy, fixture, kết quả và revision/provenance; không ghi “pass” nếu chưa chạy.
Ưu tiên unit test validator/resolver và integration/E2E vào các seam có hậu quả dữ liệu hoặc hiển thị.
So sánh scene/line list trước, dùng PNG fixture trên runtime cố định với tolerance công bố khi cần.
Không coi pixel snapshot đơn lẻ là bằng chứng persistence, font readiness hoặc SSE không mất event.
Không chạy full build/test suite lặp lại nếu thay đổi chỉ cần targeted check; giới hạn render concurrency.
Không gọi provider AI để test backend; MCP client fixture đủ kiểm contract agent.
Rủi ro đầu: typography, font fallback, wrap/hug convergence và runtime worker; M0 xử lý trước mở rộng.
Rủi ro tiếp: revision/render race, reconnect gap và asset lifecycle; M2 có AC riêng từng trường hợp.
Rủi ro storage: writer saturation, read transaction kéo dài gây WAL growth và DB/assets lệch durability; benchmark/recovery kiểm riêng.
Không thêm checklist deployment/security đồ sộ; mặc định local, path containment và lỗi không lộ secret là bắt buộc.

## 11. Nguồn kỹ thuật đã đối chiếu

- [Rust MCP SDK chính thức](https://github.com/modelcontextprotocol/rust-sdk): nền tảng `rmcp`, HTTP/stdio.
- [MCP tools specification](https://modelcontextprotocol.io/specification/2025-11-25/server/tools): schema và tool results.
- [Taffy](https://docs.rs/taffy/latest/taffy/): layout và custom measurement; không thay thế text engine.
- [SQLite WAL](https://www.sqlite.org/wal.html): reader/writer đồng thời, một writer tại một thời điểm.
- [Konva state management](https://konvajs.org/docs/data_and_serialization/Best_Practices.html): giữ application state riêng.
