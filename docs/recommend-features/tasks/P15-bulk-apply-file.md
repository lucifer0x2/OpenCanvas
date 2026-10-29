# P15 — Bulk apply: POST file NDJSON, server tự chia batch (không thêm tool)
Ưu tiên: P2 · Kích cỡ: ~85′ dev · Phụ thuộc: B02 (đã merge), B01 (đã merge) · Nguồn: ý tưởng "Đường bulk chính thức" trong `/Users/luis/Documents/opencanvas.md`, phần "Ý tưởng" (dòng 85–86), và cách làm ở dòng 6–7

## Vấn đề

Khi cần vài trăm đến vài nghìn op (ví dụ chuyển DOM capture thành `create_node`, lưới dữ liệu, icon), agent chỉ có hai đường, và đường nào cũng tệ:

1. **Nhét op vào tool call `apply_batch`.**
   - Mỗi op đi qua output của model và nằm lại trong context suốt phiên.
   - Mỗi lần gọi tối đa 500 op, nên 1 200 op cần 3 lần gọi. Agent phải tự xâu `expectedRevision` và tự đặt `idempotencyKey` cho từng lần.
2. **Tự viết script gọi HTTP MCP**, như người dùng đã làm với batch 450 op. Script phải tự lo:
   - `initialize` → `notifications/initialized` → header `mcp-session-id`;
   - `tools/call`, rồi bóc SSE `data:`;
   - tự chia batch, tự xâu revision.

   Cách này không có tài liệu, mỗi agent phải viết lại từ đầu.

Kết quả là không có **đường chính thức có tài liệu** để "áp op từ file": lỗi phải chỉ đúng dòng, và khi đứt giữa chừng phải chạy tiếp được mà không tạo trùng.

## Bằng chứng

**Giới hạn hiện có**
- `crates/storage/src/model.rs:6-7`: `MAX_BATCH_OPS = 500`, `MAX_BATCH_BYTES = 2 MiB`.
- `crates/storage/src/project.rs:64-77`: kiểm 1–500 op và kiểm byte trên `canonical_json` của cả request.
- `crates/server/src/schemas.rs:134`: schema `operations.maxItems: 500`. Gửi 501 op chỉ nhận `INVALID_REQUEST` chung "Arguments do not match the published tool schema", đã đo.
- `crates/storage/src/bundle_history.rs:53`: bundle import từ chối commit có hơn 500 op. Vì vậy **không thể nâng giới hạn op/commit**; muốn áp nhiều op hơn thì phải chia thành nhiều commit.

**Idempotency hỗ trợ chạy tiếp**
- `crates/storage/src/project.rs:86`: `replay()` chạy **trước** khi kiểm revision (`:94`).
- `crates/storage/src/db.rs:93-105`: tra theo key và hash body.
- `crates/storage/src/persistence.rs:82`: key chỉ được ghi trong transaction commit, nên chunk lỗi không "đốt" key.
- `docs/SERVER.md` mục "Consistency and limits": idempotency giữ suốt đời project.
- Hệ quả: POST lại đúng request thì các chunk đã commit được replay, còn chunk hỏng được áp lần đầu.

**Lỗi theo op đã có sẵn**
- `crates/storage/src/error.rs:12,61`: `opIndex` (`e.at(index)` ở `project.rs:101`).
- B02 thêm `field`, `nodeId`, `nodeKind`, `allowedKeys`, `suggestion` (`crates/storage/src/error.rs:14-34`).
- Đo thật trên server tạm: `{"code":"VALIDATION_FAILED","field":"props.cornerRadius","nodeId":"e6","nodeKind":"rectangle","opIndex":6,"suggestion":"radius","allowedKeys":[…]}`.

**Đường commit và các hạn chế**
- `crates/server/src/domain.rs:22-91`: `mutate` gồm commit, sự kiện SSE `commit`, rồi prerender tối đa 4 frame (`:73`) cho **mỗi** commit chưa replay. Batch chỉ chứa note thì bỏ prerender (`:24`).
- `crates/server/src/schemas.rs:28-33`: `validate("apply_batch", args)` là bộ kiểm schema mà MCP dùng. Route HTTP mới phải gọi lại nó, vì storage không kiểm `maxProperties`, `maxLength`…
- `crates/server/src/http.rs`:
  - routes ở `:37-47`;
  - body cap 15 MiB ở `:51`;
  - `guard` ở `:132-159` chặn Host khác loopback và Origin lạ. curl không gửi Origin nên qua; trình duyệt khác origin bị 403.
- `crates/server/src/main.rs:33-52`: CLI chỉ có option server, chưa có subcommand và chưa có HTTP client. `reqwest` chỉ là dev-dependency (`crates/server/Cargo.toml:22`).
- Bản Docker (`compose.yaml:20-22`) publish `127.0.0.1:3847`. Binary nằm trong container, nhưng curl trên host vẫn gọi được.

**Số đo** trên server tạm (`--port 0`, data root tạm, debug build `ec1fc01`). Batch 450 op mô phỏng lưới QueryNest: 300 text và 150 rectangle, mỗi op có `x`, `y`, `width`, `fill`, font…

| Đo | Byte |
| --- | --- |
| Tham số `tools/call apply_batch` (model phải sinh) | **96 441 B** (trung bình 214 B/op), ước lượng ≈ 27–32k token với 3–3,5 B/token JSON |
| Kết quả tool (`changedIds` gồm 452 ID ngắn) | 3 169 B text, cộng 3 169 B `structuredContent`; JSON-RPC thô 7 406 B. ID thật ~20 ký tự thì ≈ 10 KB |
| Cùng op ở dạng NDJSON trên đĩa | 96 330 B, **không vào context** nếu script sinh ra |
| Lệnh curl đề xuất, cộng response tóm tắt 1 chunk | ≈ 170 B + ≈ 420 B |
| Thời gian commit 450 op qua MCP HTTP | 306 ms |

- Phần tiết kiệm lớn nhất (~96 KB → ~0,6 KB, khoảng **160×**) chỉ có khi op do script sinh: DOM capture, converter, lưới lặp.
- Nếu model tự gõ từng op vào file thì output token gần như không đổi. Khi đó lợi ích còn lại là: không giới hạn 500, tự xâu revision và key, lỗi theo dòng, chạy tiếp được.

## Giải pháp

### Route mới

`POST /api/v1/projects/{project}/batches?expectedRevision=R&idempotencyKey=K[&fromLine=N][&chunkOps=M]`

- Body là NDJSON. Mỗi dòng không trống là **đúng một op** như một phần tử của `apply_batch.operations`.
- Không cần content-type riêng, nên `curl --data-binary @ops.ndjson` là đủ.

```bash
curl -sS --data-binary @ops.ndjson \
  'http://127.0.0.1:3847/api/v1/projects/p_123/batches?expectedRevision=7&idempotencyKey=qn-grid'
```

```jsonl
{"op":"create_node","id":"grid","kind":"frame","parentId":"page","props":{"width":1440,"height":900,"layout":{"mode":"column","gap":4}}}
{"op":"create_node","id":"cell-1","kind":"text","parentId":"grid","props":{"text":"customer_1","width":"fill","height":"hug","fontId":"font_bundled","fontSize":13,"lineHeight":16}}
```

### Thuật toán (`crates/server/src/bulk/`)

**1. Đọc và kiểm toàn bộ file trước khi commit**
- Tách dòng theo `\n`, bỏ `\r` cuối dòng, bỏ dòng trống. Giữ số dòng vật lý (bắt đầu từ 1) cho mỗi op; bỏ các dòng `< fromLine`.
- Mỗi dòng: parse JSON object, rồi `schemas::validate("apply_batch", {projectId, expectedRevision, idempotencyKey, operations:[op]})`. Dùng đúng bộ kiểm của MCP nên **không có đường vòng qua schema**.
- Lỗi cú pháp hoặc schema → `400 INVALID_REQUEST` kèm `line` và **không commit gì**.
- File không có op → `INVALID_REQUEST "No operations"`.

**2. Chia chunk tham lam theo thứ tự**
- Mỗi chunk ≤ `chunkOps` (mặc định và tối đa 500).
- Tổng `serde_json::to_vec(op).len()+1` ≤ `MAX_BATCH_BYTES − 4096`. 4 KiB dành cho envelope; `canonical_json` chỉ sắp key nên không đổi độ dài.
- Một op đơn lẻ vượt ngân sách → `VALIDATION_FAILED field:"operations"` kèm `line`, kiểm ở bước 1.

**3. Key tất định theo dòng đầu chunk**
- Key chunk = `"{K}-L{firstLine}"`. `K` hợp lệ theo `valid_id` và dài ≤ 120 ký tự, để key chunk vẫn ≤ 128 (`project.rs:132`).
- Vì key gắn với dòng chứ không với số thứ tự chunk, chạy tiếp bằng `fromLine` với **cùng K** không đụng key cũ.

**4. Commit tuần tự**
- Chunk 0 dùng `expectedRevision = R`. Chunk n dùng `committedRevision` mà chunk n−1 trả về, kể cả khi chunk đó là replay.
- Mỗi chunk gọi `domain::commit_batch`, tách từ `mutate`. `conflict::checked` vẫn gắn `changedSince`.
- Sự kiện SSE `commit` vẫn gửi theo từng chunk chưa replay, như hiện nay, để viewer theo đúng từng revision.

**5. Prerender một lần ở cuối**
- Sau chunk cuối, chỉ khi có ít nhất một chunk chưa replay: lấy hợp `changedIds` của các chunk, bỏ chunk chỉ có note (`is_note_batch`), rồi gọi `prerender_frames(…, 4)`.
- Hiện nay 3 batch tạo ≤ 12 prerender; route mới tạo ≤ 4.

**6. Lỗi giữa chừng thì dừng, không tự rollback** (mỗi chunk là một commit nguyên tử)
- Lỗi có `opIndex` (vị trí trong chunk) được chuyển thành `opIndex` theo file (0-based, đếm op từ đầu file, không tính dòng trống) và thêm `line` (dòng vật lý).
- Lỗi cuối chunk không gắn op (`candidate.validate()`, `project.rs:103`) → thêm `lines:[first,last]` của chunk.
- Mọi field B02 giữ nguyên.

### Response thành công (200), nhỏ, không có `changedIds`

Muốn đọc lại thì dùng `get_changes {sinceRevision:R}` hoặc `find_nodes ids`.

```json
{"projectId":"p_123","fromRevision":7,"committedRevision":9,"operations":900,
 "chunks":[{"firstLine":1,"lastLine":500,"operations":500,"committedRevision":8,"replayed":false},
           {"firstLine":501,"lastLine":900,"operations":400,"committedRevision":9,"replayed":false}],
 "changedCount":902,"renderStatus":"pending"}
```

### Response lỗi

HTTP status lấy theo mã lỗi như hiện có (`crates/server/src/error.rs:54-63`). Thêm `progress` ở cấp trên:

```json
{"error":{"code":"VALIDATION_FAILED","field":"props.cornerRadius","nodeId":"e6","nodeKind":"rectangle",
          "allowedKeys":["name","x","…","radius"],"suggestion":"radius","opIndex":507,"line":508,"retryable":false},
 "progress":{"fromRevision":7,"committedRevision":8,"committedOperations":500,"nextLine":501,
             "chunks":[{"firstLine":1,"lastLine":500,"operations":500,"committedRevision":8,"replayed":false}]}}
```

### Chạy tiếp (tài liệu hoá trong skill)

- **Sửa dòng lỗi** rồi POST lại **y nguyên query**. Các chunk trước đó có cùng dòng đầu, cùng body và cùng `expectedRevision`, nên được replay (`replayed:true`). Chunk hỏng chưa ghi key, nên được áp lần đầu.
- **Mất kết nối hoặc server restart**: POST lại y nguyên; những chunk đã commit sẽ được replay.
- **`REVISION_CONFLICT`** (có writer khác chen giữa hai chunk): đọc `error.changedSince` như với `apply_batch`, rồi POST `fromLine=progress.nextLine&expectedRevision=<toRevision>` với cùng K.
- **Đổi nội dung** các dòng đã commit với cùng K → `IDEMPOTENCY_KEY_REUSED` kèm `line`. Khi đó dùng K mới.

### Khám phá cho agent: con trỏ ngắn, không thêm tool

- Thêm vào cuối mô tả `apply_batch` (`schemas.rs:134`) khoảng 110 B:
  `" For hundreds of script-generated ops, POST NDJSON to /api/v1/projects/{id}/batches (skill://opencanvas/author-frame.md)."`
- Thêm mục "Many operations from a file" (~18 dòng) vào `crates/server/skills/author-frame.md`: lệnh curl, quy tắc key/fromLine, cách đọc `line`/`progress`, và lưu ý "chỉ đáng dùng khi op do script sinh; ≤ 50 op cứ dùng `apply_batch`".
- Thêm mục "Bulk apply from a file" vào `docs/SERVER.md`.

### Vì sao chọn route HTTP, không dùng các phương án khác

| Phương án | Lý do chọn hoặc loại |
| --- | --- |
| **A. Route HTTP NDJSON (chọn)** | 0 dependency mới; curl có sẵn trên macOS/Linux; chạy được với bản Docker; đi qua cùng `commit`, SSE, prerender và guard Host/Origin; agent có shell chỉ tốn ~170 B lệnh. |
| B. Subcommand `opencanvas-server apply --file` | Binary không có trên host khi chạy Docker (`compose.yaml`). Cần HTTP client runtime (reqwest đang chỉ là dev-dep) hoặc tự viết HTTP thô. Nếu ghi thẳng SQLite từ process thứ hai thì bỏ qua SSE/prerender và vi phạm "một writer mỗi project" (`docs/SERVER.md`, Consistency). Để phase sau, làm wrapper mỏng của A. |
| C. `apply_batch {file:"/path"}` hoặc tool mới | Server đọc đường dẫn tuỳ ý trái nguyên tắc "never arbitrary paths or URLs" (`docs/SERVER.md`, Consistency). Container không thấy file trên host. Tool mới làm tăng `tools/list`. |
| D. Nâng `MAX_BATCH_OPS` | Không giảm token. Phá `bundle_history.rs:53` (bundle import). |

## Phạm vi file

| File | Thay đổi | Dòng |
| --- | --- | --- |
| `crates/server/src/bulk.rs` (mới) | handler axum: query, gọi `parse`/`chunk`, vòng commit, ghép `progress`/lỗi, prerender cuối | ~130 |
| `crates/server/src/bulk/lines.rs` (mới) | parse NDJSON (dòng vật lý, `fromLine`), kiểm schema từng dòng, chia chunk theo op/byte, tạo key chunk | ~110 |
| `crates/server/src/domain.rs` | tách `commit_batch(state, req) -> Value` và `announce(state, &result, prerender: bool)` khỏi `mutate`; hành vi `mutate` giữ nguyên | 104 → ~120 |
| `crates/server/src/http.rs` | 1 route `.route("/api/v1/projects/{project}/batches", post(bulk::apply))` + import | 179 → ~182 |
| `crates/server/src/lib.rs` | `mod bulk;` | +1 |
| `crates/server/src/schemas.rs` | thêm 1 câu vào mô tả `apply_batch` | +0 dòng (~110 B) |
| `crates/server/skills/author-frame.md` | mục "Many operations from a file" | 106 → ~125 |
| `docs/SERVER.md` | mục "Bulk apply from a file" (route, query, chunk, key, lỗi/progress, chạy tiếp, giới hạn 15 MiB body) | +~35 |
| `crates/server/tests/bulk_apply.rs` (mới) | test AC 1–11 | ~190 |

Không có file nào vượt 200 dòng. File nóng: `domain.rs` và `schemas.rs`; P02a đã merge; P14b sẽ thêm op `import_svg` và tự chạy được qua route này. Nên rebase sau các nhánh đang sửa `schemas.rs`.

## Tương thích MCP

- **Không thêm tool, không đổi schema.** `tools/list` chỉ tăng phần mô tả `apply_batch`: ≤ 160 B, đo bằng `serde_json::to_vec`, xem AC 12.
- `apply_batch` qua MCP giữ nguyên hành vi, kể cả prerender mỗi commit. `mutate` chỉ được tách hàm.
- Route mới là HTTP API, không thuộc MCP. Lỗi dùng cùng envelope `{"error":{…}}` (`crates/server/src/error.rs:32-40`), thêm `line` và `lines`, cùng `progress` ở cấp trên.
- Skill `author-frame.md` thay đổi nội dung, `skill_layer.rs` tự kiểm `size` theo file. Server instructions (`instructions.rs`) không đổi.

## AC

1. **Một chunk.** POST NDJSON 450 op (sinh như phần đo) trên project ở revision R.
   - Kết quả: 200, `chunks.length == 1`, `committedRevision == R+1`, `operations == 450`.
   - Snapshot có đủ 450 node mới.
   - `serde_json::to_vec(response).len() ≤ 600`.
2. **Chia theo số op.** 1 200 op → 3 chunk với `firstLine` 1/501/1001, 500/500/200 op, revision R+1..R+3.
   - `get_changes {sinceRevision:R}` trả 3 commit.
   - Kênh SSE/broadcast nhận đúng 3 sự kiện `commit`, theo thứ tự.
3. **Chia theo byte.** 600 op, mỗi op ~5 KB (text dài) → mọi chunk có request canonical ≤ `MAX_BATCH_BYTES`, không chunk nào nhận "Batch exceeds 2 MiB"; số chunk ≥ 2.
   - Một dòng vượt ngân sách một mình → 400 kèm `line`, **revision không đổi**.
4. **Lỗi B02 theo dòng.** File 900 op, dòng 508 có `cornerRadius`.
   - Kết quả: 400 `VALIDATION_FAILED`; `field:"props.cornerRadius"`, `nodeId`, `nodeKind`, `allowedKeys`, `suggestion:"radius"`; `line == 508`, `opIndex == 507`.
   - `progress.committedRevision == R+1`, `progress.nextLine == 501`; revision project là R+1.
5. **Sửa rồi POST lại cùng query.** Chunk 1 `replayed:true`, chunk 2 commit, revision R+2.
   - Không node trùng, `get_changes` có đúng 2 commit sau R.
6. **POST lại y nguyên sau khi thành công.** Mọi chunk `replayed:true`, revision không đổi, 0 sự kiện `commit` mới, 0 prerender mới (đếm như `tests/prerender_targets.rs:22`).
7. **Lỗi trước khi commit gì.** Dòng JSON hỏng, dòng `kind:"ellipse"`, hoặc dòng có 51 key props.
   - Kết quả: 400 `INVALID_REQUEST` kèm `line` đúng; revision không đổi.
   - Body rỗng hoặc chỉ dòng trống → 400 "No operations".
   - Thiếu `expectedRevision`, hoặc K không hợp lệ hay > 120 ký tự → 400.
8. **Conflict và chạy tiếp.**
   - `expectedRevision` cũ → 409 `REVISION_CONFLICT` có `changedSince`, `progress.nextLine == 1`, `committedOperations == 0`.
   - Chạy tiếp bằng `fromLine=501&expectedRevision=<hiện tại>` với cùng K → chỉ áp các dòng ≥ 501, key chunk `K-L501`.
   - Đổi nội dung dòng đã commit, rồi POST lại với cùng K và R cũ → 409 `IDEMPOTENCY_KEY_REUSED` kèm `line` của chunk đó.
9. **Prerender một lần.** Bulk 1 200 op chạm 6 frame → tổng prerender được tạo ≤ 4 và chỉ sau chunk cuối.
   - Ba `apply_batch` riêng tương ứng thì tạo > 4.
   - File chỉ có op note → 0 prerender.
10. **Bảo mật.**
    - `Origin: http://evil.example` → 403 `ORIGIN_REJECTED`.
    - `Host: example.com` → 403 `HOST_REJECTED`.
    - Body > 15 MiB → 413. Không có dữ liệu nào được commit trong cả ba trường hợp.
11. **Tương đương MCP.** Cùng 450 op qua `apply_batch` MCP và qua route mới, trên hai project mới, cho snapshot giống nhau: so `nodes` sau khi bỏ `projectId`.
12. **`tools/list`.** Tăng ≤ 160 B so với `ec1fc01`, đo `serde_json::to_vec(tools/list result).len()`. Tên tool không đổi (`tool_names::assert_listed`).
13. **Kích thước module.** `bulk.rs` và `bulk/lines.rs` ≤ 200 dòng; `domain.rs` ≤ 130; `http.rs` ≤ 185. `cargo clippy -p opencanvas-server --all-targets -- -D warnings` sạch.
14. **Kiểm live.** Trên server tạm, curl 450 op từ file → 200 trong ≤ 1 s (debug; MCP đo được 306 ms). Lệnh curl trong `author-frame.md` chạy đúng nguyên văn sau khi thay project ID.

## Cách kiểm

```bash
cd <worktree>; export CARGO_BUILD_JOBS=4
cargo test -p opencanvas-server --test bulk_apply
cargo test -p opencanvas-server --test tool_schemas --test skill_layer --test prerender_targets --test conflict_changes
cargo clippy -p opencanvas-server --all-targets -- -D warnings
wc -l crates/server/src/bulk.rs crates/server/src/bulk/lines.rs crates/server/src/domain.rs crates/server/src/http.rs
```

**Test mới** `tests/bulk_apply.rs`, theo khung server tạm của `tests/conflict_changes.rs`:
- sinh NDJSON trong test;
- kiểm AC 1–11;
- dùng `reqwest` dev-dep với `.body(String)` và header Origin/Host tuỳ chỉnh.

**Kiểm live**
1. `target/debug/opencanvas-server --port 0 --data-root $(mktemp -d)`.
2. `POST /api/v1/projects` tạo project.
3. curl file 450 op; sửa một dòng hỏng rồi POST lại.
4. `get_changes` qua MCP.
5. Dọn process và data.

## Rủi ro

- **Không nguyên tử cả file.** Lỗi ở chunk 3 để lại chunk 1–2 đã commit. Cách giảm:
  - lỗi cú pháp và schema được chặn trước khi commit (AC 7);
  - response có `progress`, skill dạy cách chạy tiếp hoặc sửa;
  - người cần nguyên tử thì giữ ≤ 500 op hoặc dùng `apply_batch`.

  Một dry-run kiểm toàn file bằng `Candidate` mà không commit cần API storage mới, để phase sau.
- **Chunk trung gian phải hợp lệ.** `candidate.validate()` chạy cuối mỗi chunk (`project.rs:103`), nên chuỗi op chỉ hợp lệ khi đã áp hết (ví dụ tạo text thiếu font rồi `update_node` sau) có thể hỏng ở ranh giới chunk. Skill dặn: mỗi op tạo node phải đầy đủ props, và token/style tạo trước node dùng chúng. Lỗi có `lines` để biết chunk nào.
- **Ranh giới chunk theo byte phụ thuộc kích thước op kế tiếp.** Sửa dòng đầu của chunk k có thể dịch ranh giới chunk k−1 khi ngân sách byte là ràng buộc (hiếm: trung bình > 4 KB/op). Khi đó chunk k−1 đổi body → `IDEMPOTENCY_KEY_REUSED` kèm `line`. Dùng `fromLine` để chạy tiếp; ghi trong docs.
- **Route ghi mới không có xác thực**, cùng mức với `/mcp` và `POST /api/v1/projects`. Guard Host/Origin hiện có áp dụng nguyên. AC 10 kiểm.
- **Agent không có shell** (chỉ MCP) không dùng được. Họ vẫn có `apply_batch`; mô tả chỉ thêm một câu.
- **Xung đột rebase** ở `domain.rs` và `schemas.rs` (file nóng). Nên làm sau các nhánh đang sửa mô tả tool.

## Ngoài phạm vi / phase sau

- **P15b (tuỳ chọn):** subcommand `opencanvas-server apply --url … --project … --file ops.ndjson [--key K]`. Là wrapper mỏng của route này: in tiến độ, tự `fromLine` khi conflict nếu `changedSince` không chạm ID trong file. Chỉ đáng làm nếu có người dùng không có curl.
- `?dryRun=true`: kiểm toàn file trên một `Candidate` không commit. Cần thêm `validate_batch` ở storage.
- Response stream NDJSON tiến độ theo chunk cho file rất lớn. Nâng body cap riêng cho route quá 15 MiB.
- Nhận định dạng `{"operations":[…]}` (JSON thường) hoặc nhiều project trong một file.
