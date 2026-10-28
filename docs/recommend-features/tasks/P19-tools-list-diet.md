# P19 — Ăn kiêng `tools/list`: schema công bố gọn, mô tả không lặp, lỗi schema chỉ rõ chỗ sai
Ưu tiên: P1 · Kích cỡ: ~90′ dev (nếu vượt: tách mục lỗi thành P19b, xem "Ngoài phạm vi") · Phụ thuộc: không (chạm file nóng `schemas.rs`, `schemas/*.rs` — xếp tuần tự với P11b, P12a, P13a, P15) · Nguồn: tự đề xuất (ngân sách `tools/list`, gốc 23:50)

## Vấn đề

`tools/list` là chi phí token cố định mỗi phiên agent và đang tăng theo từng đề xuất (P02c-r2 +196, P11b +250, P12a +600, P13a–c +1 800, P14c +40, P15 +160 ≈ **+3 046 B** đang chờ). Đo ở `night/integration` `0847b4b` (16 tool): **24 525 B**. Hai phần ba là JSON Schema, phần lớn là ràng buộc lặp lại nguyên văn mà agent không cần đọc để gọi đúng (server vẫn kiểm). Phần mô tả lặp lại server instructions và giải thích tham số dài hơn cần thiết.

Mặt trái: khi agent gọi sai, mọi lỗi tầng schema đều trả **cùng một câu** không có field, nên agent phải đọc lại cả schema để đoán. Lỗi tầng ngữ nghĩa (`VALIDATION_FAILED` cho `props`) thì đã tốt: có `field`, `allowedKeys`, `opIndex`.

## Bằng chứng

**Số đo** (server tạm `--port 0`, data root tạm, binary integration build 02:05; `jq -c '.result.tools'`, cùng định dạng compact như `serde_json::to_vec` trong `tests/tools_budget.rs:23`):

| tool | tổng B | description | inputSchema |
| --- | ---: | ---: | ---: |
| apply_batch | 5 402 | 313 | 5 035 |
| get_design_context | 2 996 | 1 572 | 1 359 |
| find_nodes | 2 217 | 1 033 | 1 129 |
| export_nodes | 2 005 | 1 098 | 846 |
| render_frame | 1 919 | 1 010 | 854 |
| get_outline | 1 564 | 513 | 997 |
| get_subtree | 1 486 | 426 | 1 006 |
| inspect_layout | 1 406 | 624 | 723 |
| import_asset | 1 205 | 218 | 932 |
| get_changes | 977 | 534 | 389 |
| list_assets / tokens / fonts / styles | 683 / 678 / 654 / 652 | 147–178 | 451 mỗi tool |
| create_project / list_projects | 357 / 307 | 66 / 67 | 234 / 184 |
| **Tổng** | **24 525** | **8 122 (33 %)** | **~15 900 (65 %)** |

Chỗ thừa đo được:
- Mảnh ID `{"maxLength":128,"minLength":1,"pattern":"^[A-Za-z0-9_-]+$","type":"string"}` (76 B) xuất hiện **53 lần = 4 028 B (16,4 %)**: `crates/server/src/schemas.rs:38` `id()` được dùng cho mọi tham chiếu (`projectId`, `nodeId`, `frameId`, `parentId`, `jobId`, `aliasId`, mảng `ids`/`nodeIds`), không chỉ ID agent tự đặt. `apply_batch` riêng 19 lần. Agent chỉ cần biết pattern khi **tạo** ID (`id` trong op, `idempotencyKey`); tham chiếu thì copy từ kết quả đọc.
- `"minLength":1` 85 lần, `"additionalProperties":false` 40 lần (24 lần ở object lồng trong op), `"minimum":0` trên integer 22 lần. Không có schema thuộc tính nào có `description` (chỉ 16 mô tả tool), nên các ràng buộc này không mang nghĩa nghiệp vụ.
- `get_outline`/`get_subtree` công bố `viewport` (220 B) và `theme` (46 B) nhưng chỉ `get_design_context` dùng: `reads/tree.rs:40` và `:93-96` đều nằm trong `if context`. Agent đọc schema sẽ tưởng outline/subtree resolve theo theme.
- `token_value` lặp nhánh `number|string|{aliasId}` 3 lần (`schemas/operations.rs:64-67`): `upsert_token` 1 309 B.
- `CAPS` (322 B) chép vào 3 tool (`schemas.rs:74`). `get_design_context` lặp gần nguyên văn 2 đoạn của server instructions ("Write code against the authored reference…", đoạn `compact:true` 412 B). `find_nodes` lặp "totalMatches 0…", "Position…not searchable". `render_frame` (`schemas/render.rs:8`) giải thích `waitMs` hai lần.

**Lỗi chung chung** (kiểm live, cùng phiên, project thật):

| lời gọi sai | trả về |
| --- | --- |
| `create_node` id `"page.1"` | `INVALID_REQUEST` "Arguments do not match the published tool schema; check required fields, types and limits" |
| `create_node` thiếu `parentId` | như trên |
| op có thêm key `name` ngoài `props` | như trên |
| `find_nodes` `limit:500` | như trên |
| `find_nodes` `kinds:["rect"]` | như trên |
| `get_outline` `pageId` (key lạ) | như trên |
| `props.colour` (tầng ngữ nghĩa) | `VALIDATION_FAILED` `field:"props.colour"`, `allowedKeys:[…]`, `opIndex:0` ✓ |

Nguồn: `schemas/validation.rs:4-121` trả `invalid()` không tham số ở 14 chỗ, `:122-126` là câu duy nhất. `oneOf` của 11 op chỉ báo "khớp ≠ 1 nhánh", không biết op nào. `tests/tool_schemas.rs:11-12` khoá đúng câu này.

**Tiền lệ trong code:** `render_frame` đã tách schema công bố (phẳng, `render::published()`) khỏi schema kiểm (`render::validation()`, oneOf hai mode): `schemas.rs:17-21`, `schemas/render.rs:27-37`. P19 tổng quát hoá cách này cho mọi tool.

## Giải pháp

Ba thay đổi, không thêm tool, không đổi nghĩa field:

**1. Schema công bố = `lean(schema kiểm)`.** Giữ nguyên các builder hiện có làm schema kiểm (strict, như hôm nay). Thêm `schemas/publish.rs::lean(&Value) -> Value`, một phép biến đổi tất định chỉ dùng khi dựng `Tool`:
- Mảnh ID ở mọi key **trừ** `id` và `idempotencyKey` → `{"type":"string"}`. Pattern vẫn công bố ở 14 chỗ agent tự đặt ID.
- Bỏ `"minLength":1` (trừ trong mảnh ID còn giữ), bỏ `"minimum":0` của integer.
- Bỏ `"additionalProperties":false` ở object lồng; **giữ ở gốc mỗi `inputSchema`** (`tool_schemas.rs:50` vẫn đúng).
- Giữ nguyên `enum`, `const`, `required`, `maximum`, `maxItems`, `maxLength` ≠ 128 trên ID, `oneOf` của op, `pattern` của slug: đây là thông tin quyết định.
- `catalog()` (`schemas.rs:10-27`) lưu schema kiểm = builder gốc, `Tool.input_schema` = `lean(builder)`. `render_frame` = `lean(published())` công bố, `validation()` kiểm như cũ.

**2. Bỏ tham số không dùng khỏi schema công bố của `get_outline`/`get_subtree`:** `viewport`, `theme`. Schema kiểm vẫn nhận (tương thích lời gọi cũ, vẫn bị bỏ qua như hôm nay). Có thể làm bằng cách tách `tree` (`schemas.rs:72`) thành phần chung cộng phần riêng của context. Ghi một dòng trong `docs/SERVER.md`.

**3. Mô tả gọn.** Văn bản thay thế đã đo (byte cũ → mới):

| chỗ | cũ → mới | văn bản mới (dev được chỉnh chữ, giữ ý và ngân sách) |
| --- | ---: | --- |
| `CAPS` ×3 | 322 → 184 | ` depth (0–8) counts levels below the scope root (level 0); cut nodes carry omittedChildren. One revision, at most 200 nodes and 64 KiB per page; truncated:true comes with nextCursor.` |
| get_design_context, câu mục đích | 347 → 215 | `What implementing one frame needs: its subtree with authored props (token/style references: code against these) next to resolvedProps for the requested theme (today's values), plus the tokens and styles behind them.` |
| get_design_context, `compact` | 412 → 229 | ` compact:true keeps in resolvedProps only keys that differ from authored and lists only the tokens/styles/assets/fonts the frame references (marked resolvedPropsMode/resources); diagnostics and reflow still cover the whole frame.` |
| get_design_context, `viewports` | 334 → 280 | ` viewports (1–4, not with viewport, ≤16M pixels total) enqueue render jobs without waiting: while computed.status is pending, wait with render_frame {jobId, waitMs} and repeat the read; ready nodes get computedBounds per viewport and reflow (null until two viewports compare).` |
| find_nodes | 1 033 → 831 | `Find nodes by name, visible text, kind, ID or parent in one call; use get_outline for structure. Criteria are ANDed (kinds ORs within the list); at least one is required. Case-insensitive substring match, no accent folding; textContains matches text nodes' props.text; parentId matches direct children (parentId:<pageId> with kinds:["frame"] lists a page's root frames). Scope: project, or one nodeId/frameId subtree. Returns preorder matches with id, kind, name, parentId, depth, ancestor path (8 nearest, pathTruncated), textPreview and requested authored properties. totalMatches/scannedNodes cover the whole scope (0 = no match, not an error); truncated:true comes with nextCursor. Position is not searchable: read bounds with get_design_context viewports or inspect_layout. One revision, up to 200 matches and 64 KiB per page.` |
| export_nodes | 1 098 → 818 | bỏ danh sách 6 mã lỗi (lỗi tự mang mã), gộp câu: `Export image files of nodes (not a project backup): each node cut from its root frame at authored size, cropped to its own bounds, transparent background, drawn alone (own opacity applies, ancestor clip/opacity do not). settings (1–8, e.g. [{"scale":2}]) apply to every node; without them each node uses its exportSettings prop, else one PNG at 1x. A setting is {format?:"png", scale?:0.01–4 \| width? \| height? (one of the three), suffix? (default "@<scale>x" when scale≠1)}; file name = node name + suffix. Waits up to waitMs (default/max 10000). Per node: ready with files (url, fileName, mimeType, scale, width, height, bytes), pending (repeat the identical call; same jobId) or failed with its own error. Download urls right away: they live with the job record (last 32 jobs); no filesystem path is returned.` |
| render_frame | 1 010 → 594 | `Enqueue immutable exact-revision rendering; returns job and HTTP PNG metadata when ready. Two modes, not mixed: enqueue with projectId, revision, frameId and viewport (theme, state, scale optional), or poll with projectId and jobId plus waitMs (0–10000, default 0), which returns as soon as the job leaves pending. The PNG is the design's exact pixels at that revision and viewport, the reference to compare an implementation against. The computed scene (tens to hundreds of KB) is left out: read bounds and text lines with inspect_layout for the returned sceneKey, or pass includeScene:true.` |
| inspect_layout | 624 → 479 | `Frame-local pixel bounds and exact text lines of an existing job's scene key: the measured ground truth for that job's viewport (another viewport needs another render). No layout explanation or diff. Nodes in scene order, up to limit (default/max 200) per page; budget (bytes) uncapped unless given. fields picks bounds and/or lines (default both; ["bounds"] is much smaller). truncated:true comes with nextCursor: repeat the same arguments with cursor; the cursor binds the job.` |

Không cắt `apply_batch`, `import_asset`, `get_changes`, `list_*`: ngắn sẵn, và câu rebase/replay của `apply_batch` phải đúng cả với client không hiện server instructions. Quy tắc chọn: thông tin về **tham số và kết quả của tool** ở lại mô tả (agent đọc lúc gọi; Claude Code tải tool trễ qua ToolSearch); quy trình xuyên tool ở lại instructions. Không câu nào bị xoá mà mất ý; đoạn dài chỉ được rút gọn.

**4. Lỗi schema chỉ rõ chỗ sai** (0 B `tools/list`). `validation::validate` mang theo đường dẫn và trả lỗi đầu tiên dưới dạng:
```json
{"code":"INVALID_REQUEST","field":"operations[0].id","rule":"pattern",
 "expected":"1–128 chars of A-Z a-z 0-9 _ -","message":"operations[0].id: \"page.1\" must be 1–128 chars of A-Z a-z 0-9 _ -","retryable":false}
```
- `rule` ∈ `required | unknownKey | type | enum | const | pattern | minLength | maxLength | minimum | maximum | minItems | maxItems | maxProperties | oneOf`.
- `unknownKey` thêm `allowedKeys` (sắp theo tên, cùng shape với `VALIDATION_FAILED` agent đã quen); `enum`/`const` thêm `allowed`.
- `oneOf` có phân biệt bằng `properties.op.const` (11 op): chọn nhánh theo `op` rồi báo lỗi bên trong nhánh đó. `op` thiếu hoặc lạ → `field:"operations[i].op"`, `allowed:[11 op]`. `oneOf` không phân biệt (token value, 2 mode render_frame) → `rule:"oneOf"` kèm `expected` mô tả ngắn từng nhánh (`render_frame`: `"enqueue {projectId,revision,frameId,viewport} or poll {projectId,jobId,waitMs?}"`).
- Giá trị trích trong `message` cắt ≤ 64 ký tự. Không trích `base64`.
- Vì schema công bố không còn ghi các giới hạn trên, lỗi phải nói thay; ví dụ `limit:500` → `field:"limit"`, `rule:"maximum"`, `expected:200`.
- Message mở đầu bằng field, bỏ cụm "published tool schema" (schema kiểm giờ chặt hơn bản công bố).

**Vì sao không dùng `$defs`/`$ref`** để gộp mảnh ID và nhánh token: tiết kiệm tương đương, nhưng nhiều client MCP (bộ chuyển schema sang function-calling của một số SDK) chưa resolve `$ref`, hoặc inline lại nên không tiết kiệm. `lean` cho schema phẳng, mọi client đọc được. **Vì sao không bỏ hẳn `inputSchema` chi tiết** (chỉ mô tả bằng chữ): mất kiểm phía client và tên/enum máy đọc được.

## Phạm vi file

| file | việc | dòng (ước) |
| --- | --- | --- |
| `crates/server/src/schemas/publish.rs` (mới) | `lean()` + unit test nhỏ | ~70 |
| `crates/server/src/schemas.rs` (163) | `catalog()` lưu strict/publish; tách `tree` chung/context; `CAPS`, các mô tả | ±25, giữ ≤ 175 |
| `crates/server/src/schemas/render.rs` (107), `export.rs` (23) | `DESCRIPTION` mới | ±0 |
| `crates/server/src/schemas/validation.rs` (126) | mang path, trả `SchemaError{field,rule,expected,allowed}`, chọn nhánh theo `op` | → ~170 |
| `crates/server/src/schemas/validation_error.rs` (mới, nếu `validation.rs` > 200) | dựng `ApiError` + message từ `SchemaError` | ~60 |
| `crates/server/tests/tools_budget.rs` | cập nhật số gốc trong comment; hạ `TOTAL_MAX` xuống 24 KiB (~6 KiB headroom trên nền ~18,5 KB sau P19, đủ cho +3 046 B đang chờ) | ±3 |
| `crates/server/tests/tool_schemas.rs` (105) | `INVALID` thành kiểm `code` + `field`; thêm test lean | ~+40, hoặc file mới `tests/schema_errors.rs` ~120 |
| `docs/SERVER.md` | mục lỗi `INVALID_REQUEST` (field/rule/expected/allowedKeys); `viewport/theme` của outline/subtree bị bỏ qua | ~+10 |

## Tương thích MCP

- Tên tool, tên field, `required`, `enum`, nghĩa field: không đổi. Mọi lời gọi hợp lệ hôm nay vẫn hợp lệ, kể cả `viewport/theme` gửi cho outline/subtree. Lời gọi sai vẫn bị chặn y như cũ, vì schema kiểm không đổi.
- Schema công bố **lỏng hơn** schema kiểm ở: `minLength:1`, pattern của ID tham chiếu, `minimum:0`, `additionalProperties` lồng. Client tự kiểm phía mình sẽ để lọt các lỗi này lên server, và server trả lỗi có field. Đây đúng là trường hợp `render_frame` đã có hôm nay.
- Lỗi: `code` giữ `INVALID_REQUEST`, `retryable:false`. `message` đổi chữ, thêm field. Không client nào nên so khớp nguyên văn message (chỉ test nội bộ làm vậy).
- Cập nhật: `docs/SERVER.md`. Skill `author-frame.md` nên có một dòng "IDs you create: 1–128 of A-Z a-z 0-9 _ -" (đã có trong schema `id`, chỉ là nhắc lại).

## AC

Đo bằng `cargo test -p opencanvas-server --test tools_budget -- --nocapture` (dòng `tools/list: N tools, T B; per tool …`) trên nhánh rebase lên `night/integration` hiện tại. Gốc tham chiếu: **24 525 B @ `0847b4b`, 16 tool**. Nếu integration đã nhận thêm tool/field thì AC1–3 so trên cùng HEAD trước/sau P19.

1. Tổng `tools/list` giảm **≥ 5 500 B** so với cùng HEAD trước P19 (mô phỏng: 24 525 → ~18 500 B, −6 000 B, −24 %). Ròng với +3 046 B đang chờ vẫn **< 22 000 B** (âm ~2 950 B so với hôm nay).
2. Phần schema (`lean`) giảm ≥ 3 600 B; phần mô tả giảm ≥ 1 700 B; `get_outline` + `get_subtree` mỗi tool giảm ≥ 450 B. In ba số này trong report dev.
3. `apply_batch` ≤ 4 400 B, `get_design_context` ≤ 2 300 B, `render_frame` ≤ 1 300 B, `export_nodes` ≤ 1 550 B, `find_nodes` ≤ 1 700 B.
4. Test mới `published_schema_is_lean`: duyệt mọi `inputSchema` công bố: không có `"minLength":1` ngoài mảnh ID giữ lại; `pattern` ID chỉ dưới key `id`/`idempotencyKey` (đếm = 14 ở HEAD gốc); `additionalProperties:false` có ở gốc mỗi tool và không có ở object lồng; mọi `enum`/`const`/`required`/`maximum`/`maxItems` của schema kiểm còn nguyên trong bản công bố (so bằng cách áp `lean` lên schema kiểm rồi so với `tools/list`).
5. Tương thích: toàn bộ `cargo test -p opencanvas-server` xanh (trừ chỗ so nguyên văn message ở AC7). Thêm test: mọi lời gọi bị chặn trong `tool_schemas.rs`/`failures.rs` trước P19 vẫn bị chặn với `INVALID_REQUEST`. `get_outline` kèm `theme:"night"`, `viewport` vẫn trả như không kèm.
6. `get_outline`/`get_subtree` công bố không còn `viewport`, `theme`; `get_design_context` còn đủ.
7. Lỗi có field cho 7 lời gọi trong bảng Bằng chứng: `field` lần lượt `operations[0].id` (`pattern`), `operations[0].parentId` (`required`), `operations[0].name` (`unknownKey`, `allowedKeys` = `[id,index,kind,op,parentId,props]`), `limit` (`maximum`, `expected:200`), `kinds[0]` (`enum`, `allowed` 6 kind), `pageId` (`unknownKey`, `allowedKeys` có `nodeId`); `props.colour` giữ nguyên `VALIDATION_FAILED`.
8. `operations[3]` với `op:"create_nod"` → `field:"operations[3].op"`, `rule:"enum"`, `allowed` đủ 11 op. Op đúng nhưng sai bên trong → lỗi báo bên trong nhánh của op đó, không phải `oneOf`.
9. `render_frame` trộn mode (`jobId` + `revision`) → `rule:"oneOf"`, `expected` nêu hai mode; `waitMs` thiếu `jobId` cũng vậy.
10. Message ≤ 240 ký tự; giá trị trích ≤ 64 ký tự; `import_asset` với `base64` sai kiểu không trích chuỗi base64.
11. `TOTAL_MAX` hạ còn 24 KiB, test budget xanh. File sửa/tạo ≤ 200 dòng, trừ `schemas.rs` ≤ 180.

## Cách kiểm

```bash
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server --test tools_budget -- --nocapture   # AC1–3, 11
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server --test tool_schemas --test schema_errors --test failures --test multi_failures   # AC4–10
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server   # AC5
```
Oracle độc lập cho AC1/AC4 (kiểm thử chạy trên `tools/list` của **HEAD trước P19**, phải cho cùng hình dạng với bản dev, chênh ≤ 2 % do chữ mô tả):
```jq
def isid: type=="object" and .pattern=="^[A-Za-z0-9_-]+$" and .maxLength==128;
def keepkey: . == "id" or . == "idempotencyKey";
walk(if type=="object" and (.properties|type)=="object" then .properties |= with_entries(if (.key|keepkey|not) and (.value|isid) then .value={type:"string"} else . end) else . end)
| walk(if type=="object" and .type=="array" and (.items|isid) then .items={type:"string"} else . end)
| walk(if type=="object" and (.additionalProperties|type)=="object" and (.additionalProperties|isid) then .additionalProperties={type:"string"} else . end)
| walk(if type=="object" and .minLength==1 and (isid|not) then del(.minLength) else . end)
| walk(if type=="object" and .type=="integer" and .minimum==0 then del(.minimum) else . end)
| map(.inputSchema |= (walk(if type=="object" and .additionalProperties==false then del(.additionalProperties) else . end) | .additionalProperties=false))
| map(if .name=="get_outline" or .name=="get_subtree" then .inputSchema.properties |= del(.viewport,.theme) else . end)
```
Kết quả mô phỏng @`0847b4b`: chỉ schema 24 525 → 20 828 B; thêm bỏ `viewport/theme` → 20 340 B; thêm mô tả mới → ~18 514 B.

Kiểm live: server tạm `--port 0 --data-root $(mktemp -d)`, `initialize` → `tools/list` đo bằng `jq -c '.result.tools' | wc -c`; gọi lại 7 lời gọi sai của bảng Bằng chứng, so `field/rule`. Dọn process và data sau kiểm.

## Rủi ro

- **Agent bớt thông tin trước khi gọi** (ví dụ không thấy `minLength:1` của `cursor`, pattern của `parentId`). Giảm thiểu: chỉ bỏ ở tham chiếu (giá trị copy từ kết quả đọc) và ràng buộc hiển nhiên. Pattern vẫn công bố ở 14 chỗ agent tự đặt ID. Lỗi mới có `field/rule/expected` nên sửa được trong 1 lượt.
- **Client kiểm schema phía mình** (Codex/OpenAI strict) có thể đòi `additionalProperties:false` ở mọi object. MCP không bắt buộc; gốc mỗi tool vẫn giữ. Nếu QA thấy client cụ thể hỏng: giữ `additionalProperties:false` lồng (chỉ mất ~700 B).
- **Văn bản mô tả rút gọn làm đổi hành vi agent.** Mọi câu cũ có tương ứng trong câu mới (đối chiếu từng hàng của bảng). Test `skill_layer.rs`/`find_nodes.rs` có thể so chuỗi mô tả: dev cập nhật theo.
- **Xung đột file nóng** `schemas.rs` với P11b, P12a, P13a, P15 (đều thêm vào mô tả/op). Nên merge P19 **trước** để các đề xuất sau viết trên nền gọn; nếu sau thì `lean` tự áp lên op mới, chỉ mô tả mới cần tự giữ ngắn.
- Chọn nhánh `oneOf` theo `op` giả định mọi nhánh có `op.const`. Đúng cho `operations::schema()` hôm nay. Nhánh không có `op` thì rơi về `rule:"oneOf"`.

## Ngoài phạm vi / phase sau

- **P19b (nếu dev tách):** mục 4 (lỗi có field) thành task riêng ~60′. Khi đó P19a **không** bỏ `additionalProperties` lồng (để agent vẫn thấy key hợp lệ), chỉ làm 1–3: AC1 hạ còn ≥ 4 800 B.
- Gộp nhánh `token_value` (×3, ~350 B) thành một dòng mô tả: cần chắc client không mất kiểm `oneOf`; đo sau.
- Server instructions (4 476 B, cũng là chi phí cố định mỗi phiên) lặp với mô tả: đoạn `compact` (~430 B) và "Position…" (~150 B). Rút về một câu trỏ sang mô tả tool/skill, khoảng −600 B. Để phase sau vì chạm `instructions.rs`, cũng là file P06a/P07a sửa.
- Ngân sách cho từng đề xuất sau: mỗi field mới mang ràng buộc qua `lean` tự động. Có thể thêm vào test budget dòng in "schema vs description" để review thấy ngay phần nào tăng.
