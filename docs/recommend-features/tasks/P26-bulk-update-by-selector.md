# P26 — `update_matching`: sửa đồng loạt theo bộ chọn trong `apply_batch` (không thêm tool)
Ưu tiên: P2 · Kích cỡ: ~110′ dev → tách **P26a** (storage: op + matcher dùng chung + giới hạn, ~70′) và **P26b** (server: schema, `dryRun` preview, docs/props_doc, ~40′) · Phụ thuộc: không bắt buộc. Tiêu chí `styleId`/`tokenId` chỉ thêm sau P23 (lọc style/token) nếu P23 đưa chúng vào `find_nodes`; P12a (thay font) là người dùng đầu tiên. Merge sau P19 (cùng sửa `schemas.rs`).

## Vấn đề
Muốn đổi màu 300 text hoặc đổi `fontId` trên mọi text dùng mặt font cũ (sau P12a), agent phải:
1. `find_nodes` để lấy ID (≤200/trang, phân trang),
2. sinh N op `update_node` giống hệt nhau trừ `id`.

N op đó đi qua output của model (token GHI, chậm nhất và đắt nhất), nằm lại trong context, và bị cắt ở 500 op/batch. Giữa bước 1 và 2 còn có thể có commit khác chen vào, nên tập ID agent gửi có thể đã lệch so với ý định "mọi text trong trang X".

## Bằng chứng
- `crates/server/src/schemas/operations.rs:18-22`: `update_node` chỉ nhận `{id, props}`, một node mỗi op. Không có op nào nhận bộ chọn.
- `crates/server/src/schemas.rs:134`: `operations.maxItems: 500`. `crates/storage/src/project.rs:64-77` kiểm 1–500 op và 2 MiB.
- `crates/storage/src/validate/candidate.rs:137-148`: `UpdateNode` = merge `props` vào node, rồi `self.node(...)` validate và `changed.insert(id)`. Một op bộ chọn chỉ cần lặp đúng nhánh này trên các node khớp.
- `crates/storage/src/project.rs:99-103`: mọi op chạy trên `Candidate` rồi `validate()` mới persist. Như vậy nguyên tử và `opIndex` (`e.at(index)`) có sẵn, không phải viết thêm.
- `crates/storage/src/project.rs:86` + `db.rs`: replay theo key + hash body chạy trước kiểm revision. Op bộ chọn cũng hưởng idempotency như op thường.
- `crates/server/src/reads/find.rs:15-80`: matcher `find_nodes` (kinds OR, `nameContains`/`textContains` lowercase substring, `ids`, `parentId` con trực tiếp, AND giữa tiêu chí). Matcher nằm ở crate **server**, còn `Candidate` ở crate **storage**, nên phải chuyển nó xuống storage để dùng chung.
- `crates/storage/src/bundle_history.rs:53`: bundle import từ chối commit có hơn 500 op. Log lưu `request_value["operations"]` (`project.rs`, gọi `persist`).
- Đo byte (shell `${#s}`, ID/token cỡ thật):

  | Cách | Kích thước |
  |---|---|
  | 1 op `update_node` đổi `fill` | 84 B |
  | 300 op đổi màu | **25 200 B**, ≈6–7k token output, và phải chạy `find_nodes` trước |
  | 1 op `update_matching` tương đương | **148 B** (≈170× nhỏ hơn) |
  | Đổi font 18 mặt sau P12a | 18 × ~78 B = 1 404 B, cộng việc đọc ID |

## Giải pháp
Thêm một variant op trong `apply_batch`. Không thêm tool; `tools/list` chỉ tăng thêm một nhánh `oneOf` ≈ 450–550 B.

```json
{"op":"update_matching",
 "match":{"scopeId":"page-home","kinds":["text"],"nameContains":"price","textContains":"$",
          "parentId":"card-row","ids":["a","b"]},
 "set":{"fill":{"tokenId":"ink-strong"}},
 "expectCount":300, "maxNodes":500}
```

- **`match`** dùng đúng ngữ nghĩa `find_nodes` (AND giữa các tiêu chí; `kinds` là OR; lowercase substring; `parentId` = con trực tiếp). Thêm `scopeId` (một cây con, tương đương `nodeId`/`frameId` của `find_nodes`). Bắt buộc có ít nhất một tiêu chí **khác** `scopeId`, để chặn kiểu "khớp cả project" vô tình. `styleId`/`fontId`/`tokenId` (khớp giá trị authored props) chỉ thêm khi P23 đã đưa chúng vào `find_nodes`, để hai bên luôn cùng một matcher.
- **Đánh giá trên `Candidate` tại thời điểm op chạy**, theo thứ tự preorder. Op trước trong cùng batch (vd `create_node`) được tính. Vì đánh giá trong cùng transaction, khi đã khớp `expectedRevision` thì không có race giữa đọc và ghi.
- **`set`** merge như `update_node` (`candidate.rs:145`). Mỗi node đi qua `self.node()`. Nếu có node sai kind cho key (vd `fontId` trên rectangle), cả batch fail, lỗi kèm `opIndex`, `nodeId`, `field`, `allowedKeys` (B02). Không có chế độ "bỏ qua node sai": muốn vậy thì thu hẹp `kinds`.
- **Giới hạn:** `maxNodes` mặc định 500, trần cứng 2 000. Tổng số node bị chạm trong cả batch (op thường + node khớp) ≤ 2 000. Vượt thì trả `VALIDATION_FAILED field:"match"` kèm `matchCount`.
- **`expectCount`** (tuỳ chọn): số khớp khác giá trị này thì fail với `matchCount` thật. Đây là chốt an toàn, giống `expectedRevision` nhưng ở mức tập node. Khớp 0 node mà không có `expectCount:0` → fail `NO_MATCH` (không âm thầm commit rỗng).
- **`dryRun:true`** ở cấp request `apply_batch` (P26b): chạy hết `Candidate` + `validate()` nhưng không persist và không ghi idempotency key. Trả `{"dryRun":true,"wouldCommitRevision":R+1,"changedIds":[…≤200],"changedCount":N,"perOp":[{"opIndex":0,"matchCount":300}]}`. Agent xem số lượng trước, rồi gửi lại cùng body với `dryRun` bỏ đi và key mới.
- **Kết quả commit** giữ nguyên dạng (`changedIds` đầy đủ như hiện tại), thêm `perOp[].matchCount` cho các op `update_matching`.
- **Log/bundle:** lưu op **đúng như gửi** (bộ chọn), không bung ra N `update_node`. Đánh giá lại trên snapshot trước commit là tất định, và tránh vượt giới hạn 500 op/commit ở `bundle_history.rs:53`. Tăng phiên bản schema của bundle/log để bản cũ từ chối thay vì đọc sai (xem `docs/BACKUP.md`).

## Phạm vi file
- P26a, storage:
  - `crates/storage/src/model/requests.rs`: variant `UpdateMatching { r#match: Match, set: Value, expect_count, max_nodes }`.
  - Mới: `crates/storage/src/select.rs`: `Match` + `matches(&Node, &Candidate)`, chuyển từ `server/reads/find.rs:15-80`; `find.rs` gọi lại hàm này.
  - `crates/storage/src/validate/candidate.rs`: tách nhánh `UpdateNode` thành `fn merge_props()`, rồi nhánh mới lặp qua nó. File đang 218 dòng; nếu vượt 250 thì đưa nhánh mới sang `validate/bulk.rs`.
  - `crates/storage/src/project.rs`: `dryRun`, trả `perOp`.
  - `bundle_history.rs`: bump phiên bản.
- P26b, server:
  - `crates/server/src/schemas/operations.rs`: thêm variant.
  - `schemas.rs:134`: thêm `dryRun` và mô tả.
  - `props_doc.rs`, `docs/SERVER.md`, `docs/CONTRACT.md`.
- Tests:
  - mới `crates/storage/tests/update_matching.rs`;
  - mới `crates/server/tests/update_matching.rs`;
  - `find_nodes.rs` phải vẫn xanh sau khi chuyển matcher.

## AC
1. Seed 300 text + 50 rectangle trong `page-home`. Batch 1 op `{kinds:["text"],scopeId:"page-home"} set fill` commit 1 revision, `changedIds` đúng 300 ID text, rectangle không đổi; body request ≤ 300 B.
2. `find_nodes` với cùng tiêu chí trả `totalMatches` bằng `perOp[0].matchCount` (cùng matcher, test đối chiếu trên 3 bộ tiêu chí).
3. `set:{fontId:…}` mà match gồm cả rectangle → `VALIDATION_FAILED`, có `opIndex`, `nodeId` của rectangle đầu tiên, `field:"props.fontId"`; revision không tăng.
4. `expectCount:299` khi khớp 300 → lỗi có `matchCount:300`, không commit. Khớp 0 node mà không có `expectCount:0` → lỗi `NO_MATCH`.
5. Khớp 2 001 node → lỗi `field:"match"`, không commit. `match` chỉ có `scopeId` → lỗi schema/validation.
6. `dryRun:true`: revision không tăng; gửi lại đúng key **không** dùng dryRun thì commit thật (key chưa bị đốt); có `changedCount`, `perOp`.
7. Replay đúng body + key → `replayed:true`, cùng `committedRevision`. Đổi `set` với key cũ → lỗi idempotency như hiện tại.
8. Batch `[create_node t9 (text, page-home), update_matching kinds text scope page-home]` → t9 nằm trong `changedIds` và đã được đổi màu.
9. Export bundle rồi import vào project trống → cùng props trên cả 300 node. Bundle có op `update_matching` được ghi đúng như gửi.
10. Kích thước `tools/list` tăng ≤ 700 B (đo trước/sau, ghi vào report). Các file sửa ≤ 250 dòng, hoặc ghi rõ lý do nếu vượt.

## Cách kiểm
- Chạy `cargo test -p opencanvas-storage update_matching` và `cargo test -p opencanvas-server --test update_matching --test find_nodes --test tool_schemas`.
- Kiểm live bằng server tạm `--port 0` + data root tạm (không chạm cổng 3847): gọi `apply_batch` dryRun → commit → `get_changes includeOperations:true` để thấy op lưu dạng bộ chọn. Đo byte `tools/list` trước/sau bằng `wc -c`.

## Rủi ro
- **Ngữ nghĩa matcher trôi** giữa `find_nodes` và op ghi. Giảm bằng cách dùng chung một module `select.rs`, cộng AC2.
- **Sửa quá tay**: `textContains` khớp rộng hơn dự định. Giảm bằng `expectCount`, `dryRun`, trần node, và bắt buộc có tiêu chí khác `scopeId`.
- **Log lưu bộ chọn**: nếu sau này đổi ngữ nghĩa matcher (vd thêm accent folding) thì replay bundle cũ sẽ khác. Phải version hoá matcher trong op (`"matchVersion":1` ngầm định) hoặc cấm đổi ngữ nghĩa. Phương án thay thế là bung ra `update_node` khi persist, nhưng như vậy vướng giới hạn 500 op/commit và log to ra.
- **Commit lớn**: 2 000 node làm SSE workspace events/`get_changes` có `changedIds` dài. Các đường này đã có budget/truncated, cần kiểm lại khi viết test.
- **Hiệu năng**: quét preorder toàn scope cho mỗi op là O(n). Với project vài chục nghìn node vẫn chấp nhận được, nhưng nên đo `validate_us` trong `BatchMetrics`.
