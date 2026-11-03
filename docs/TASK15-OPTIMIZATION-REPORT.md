# Task15 — tối ưu PostgreSQL và index, 2026-11-02

Số đo và kết quả kiểm dưới đây là lịch sử của ngày ghi báo cáo. Raw logs,
JSON/JSONL, manifests và archives được sinh tự động không còn được version trong
cây hiện tại; xem [chính sách output và cách chạy lại](../spikes/realtime-db/evidence/README.md).

Đã tối ưu spike PostgreSQL theo yêu cầu người dùng và kiểm độc lập. Với cùng
project 5.000 nodes/8 writers, p99 toàn thao tác giảm **13,25 → 2,63 giây** trong
lượt đối chiếu trên database mới, khoảng **5,04 lần**; throughput 6,65 → 20,27
edits/giây. Một writer giảm p99 130,3 → 66,2 ms. Đây là Cargo dev/debug và fixture
có contention, chưa phải SLA hoặc latency SQL riêng.

## Thay đổi

- Backend tra idempotency trước, rồi reject expectedRevision cũ **trước SELECT
  nodes**; giữ replay/collision/dry-run/rollback và toàn bộ candidate validation.
- `GET /projects/:id/revision` chỉ đọc project row; SSE kiểm project tồn tại bằng
  đường đọc nhẹ. Snapshot đầy đủ vẫn dùng transaction snapshot nhất quán.
- Thêm `nodes_parent(project_id,parent_id)` và partial index
  `render_jobs_artifact(artifact_id) WHERE artifact_id IS NOT NULL` trong
  [`indexes.sql`](../spikes/realtime-db/postgres/indexes.sql). Các PK đã có cho
  revision/node ID/commits/idempotency được dùng tiếp, không thêm index trùng.
- Client fixture đọc revision nhẹ, backoff exponential equal-jitter 2…64 ms khi
  conflict; giới hạn retry. Mất response POST giữ nguyên body/key khi thử lại.
  Không tự merge authoring intent; rebuild request sau conflict chỉ áp dụng cho
  các phép gán độc lập đã biết trong benchmark.
- Giữ mode `legacy` nguyên đường ghi cũ, continuous full-snapshot reader và SSE;
  mode `optimized` sửa cùng 3 nodes/batch. Không giảm số writers/nodes/edits.

Các module vẫn tách DB/batch/HTTP, SQL indexes, retry, workload và query-plan
fixture. Xem [backend report](../spikes/realtime-db/OPTIMIZATION-POSTGRES.md),
[harness report](../spikes/realtime-db/OPTIMIZATION-HARNESS.md) và
[index report](../OPTIMIZATION-INDEXES.md).

## Đối chiếu ba biến thể

Nguồn baseline đã chụp trước sửa, fingerprint
`a26d059225c86212e490bac395516b7d6c0281ec603c2a939362c234deba7d24`.
Nguồn mới `a792088d464857c9c551608463a7df857014a84f25503d8f85c45efcd3e19aed`.
Mỗi run xác minh source trước/sau không đổi, lưu binary hashes/configuration.
Các run tuần tự, không benchmark đồng thời với build; PostgreSQL 16.9 cùng image
digest, 2 CPU/2 GiB, cùng host/toolchain và Cargo dev profile.

1. Baseline đo lại:
   server/client cũ, đủ 6 cells, 312 edits, exit0 và cleanup đạt.
2. Run đầy đủ mã mới:
   9/9 pha đạt, 624 edits cho hai modes, history1000 và toàn bộ recovery/backup/index.
   Sáu cells `legacy` chạy đầu tiên được lấy làm cột **chỉ backend mới**.
3. Client tối ưu trên DB mới:
   đủ 6 cells/312 edits và DB contract; các pha recovery/history/index cố ý bỏ vì
   đã kiểm trong run đầy đủ cùng nguồn/binaries. Cột **backend + client mới** dùng
   run riêng này để tránh trộn dữ liệu tích lũy của mode legacy chạy trước.

comparison-isolated.json
kiểm cùng cấu hình/tài nguyên/binary mới, đủ traffic, nonempty reader, revisions/
commits khớp; chứa cả p95/p99 POST, toàn thao tác, throughput, conflicts và max retry.

| Nodes | Writers | p99 baseline (ms) | Chỉ backend mới (ms) | Backend + client mới (ms) |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 1 | 35,8 | 34,5 | 7,6 |
| 100 | 4 | 315,8 | 231,5 | 295,8 |
| 100 | 8 | 966,9 | 654,2 | 507,8 |
| 5.000 | 1 | 130,3 | 108,6 | 66,2 |
| 5.000 | 4 | 4.036,9 | 2.329,8 | 1.733,9 |
| 5.000 | 8 | 13.253,2 | 5.620,7 | 2.630,3 |

Đây là p99 **toàn thao tác**, tính cả revision reads, mọi retry và backoff/scheduler
delay. Với 5.000/8, POST attempt p99 223,4 → 87,7 ms; max retries 84 → 30.
Conflicts baseline335, backend-only520, client mới338: **không** tuyên bố giảm
conflict so baseline; fast rejection có thể làm số lần thử tăng. Trong cả ba
biến thể, dữ liệu/revisions/commit ranges/SSE đạt và không mixed snapshot.

Mode optimized trong run đầy đủ cũng đạt (p99 5.000/8 là 3,53 giây), giữ làm số đo
đối chứng bổ sung. Không chọn riêng số tốt nhất để thay thế raw evidence: bảng trên
dùng trọn một matrix riêng trên DB mới. Mốc 17,5 giây trong report trước vẫn giữ
nguyên; baseline đo lại lần này là 13,25 giây. Timing có biến động host/cache/jitter;
mỗi cell chỉ có 12 edits/writer, nên p99 ở cell96 mẫu gần với mẫu chậm nhất.

## Index đã được xác minh

Index evidence của lượt root
chứa 20 EXPLAIN trước/sau, actual rows/buffers/timing, SQL/params, inventory/index
sizes và planner settings. Fixture riêng 100.000 nodes, 10.000 projects, 20.000
commits/keys, 10.000 jobs; chạy sau matrix trong DB dùng một lần rồi dọn.
`enable_seqscan/indexscan/bitmapscan=on`; ANALYZE và warmup giống nhau, không ép index.

| Truy vấn | Trước | Sau |
| --- | --- | --- |
| Parent có 101 children | PK lấy5.000 entries, lọc4.899 | `nodes_parent` lấy101 entries, không lọc thừa |
| Parent leaf không children | Lọc5.000 nodes,128 buffer hits | `nodes_parent`,0 rows,2 buffer hits |
| Artifact chọn lọc | Seq Scan lọc9.999 jobs,114 hits | Index Only Scan `render_jobs_artifact`,2 hits |

Index mới có kích thước 745.472 và 16.384 bytes trên fixture. Project/node point,
commit cursor và idempotency dùng PK hiện có. Snapshot vẫn trả đủ5.000 nodes;
full-table control vẫn Seq Scan hợp lệ. Một lần EXPLAIN không phải workload p99;
không quy toàn bộ cải thiện latency cho index. Cơ sở phương pháp ở
[PostgreSQL EXPLAIN](https://www.postgresql.org/docs/16/using-explain.html).

## Kiểm độc lập và tái chạy

- Node tests **12/12**: SSE, retry bounds/jitter, mất response giữ body/key,
  lỗi không được rebase, guards cho comparison/index evidence.
- Rust target **13/13**:
  12 DB contract scenarios +1 blob/hash unit, không gọi nhầm thành13DB tests.
- Test giữ `ACCESS EXCLUSIVE` trên nodes: stale/replay/collision/dry-run stale,
  revision HTTP và SSE headers vẫn trả; valid write thực sự chờ khóa rồi commit.
- Reparent/delete_subtree thật, invalid reparent atomic và deferred FK từ chối
  dangling parent sau khi thêm index.
- Default full runner **9/9 pha**, history1000, worker SIGKILL, gateway/DB restart,
  migration hai binary v1/v2, backup/restore và query plans đều đạt.
- Root chạy strict Clippy core all-targets + PostgreSQL all-targets/all-features,
  fmt cả hai crate; code/docs whitespace và kích thước được soát riêng.

```sh
# Full regression, cả legacy và optimized matrices:
node spikes/realtime-db/harness/run-all.mjs
# Đối chiếu client mới bằng một runtime/database sạch riêng:
PHASES=rustContract,contractAndMatrix CLIENT_MODES=optimized \
  node spikes/realtime-db/harness/run-all.mjs
```

Khi root chạy đã bỏ các override counts/build/history/evidence khỏi môi trường.
Cleanup verification:
ba run baseline/full/isolated không còn container/gateway riêng; không mật khẩu
test trong evidence. Thư mục `.runtime/` vẫn ignore các credential/cache/dump để
debug, không đưa chúng vào source. Các workload kế thừa không bị chạm.

46 file mã viết tay trong spike, lớn nhất193 dòng; không file nào quá250.
Các lượt điều tra thất bại được ghi riêng tại thời điểm đo và không được tính
là nghiệm thu; generated outputs hiện đã bỏ khỏi cây versioned. Lượt bổ sung từng bị hệ thống duyệt lệnh chặn do usage limit;
sau người dùng yêu cầu tiếp tục, cùng hành động được duyệt/chạy và hoàn tất.

## Giới hạn còn lại

Đọc lại raw để giải thích chi phí: với1writer, median toàn thao tác35,1ms,
p99/max12mẫu66,2ms; mẫu chậm nhất không retry, revision GET0,63ms và POST65,5ms.
Batch sửa3nodes nhưng vẫn tải/deserialize5.000nodes, clone/validate/sort cả
candidate và thực hiện các lệnh SQL nối tiếp trong khi giữ project row lock.
Không có phase timing để tách chính xác lock wait, decode, validation, writes,
COMMIT/fsync và scheduling; không được quy65,5ms thành SQL commit time.

Với8writers, mẫu2.630,3ms có30conflicts/31POST attempts,31revision reads
(155,1ms tổng), requested backoff1.258,0ms và POST attempts1.220,4ms tổng.
Requested sleep không phải actual elapsed sleep nên không cộng như phase timing
chính xác. Median toàn thao tác69,4ms; p99 gần max96mẫu. Writers gửi liên tục
vào cùng project và cùng2sentinel nodes, có thêm full-snapshot reader liên tục;
không tương đương8CRUD độc lập. Những số đo này chưa gồm render/PNG.

5.000 nodes/8 writers vẫn có tail2,63 giây khi tranh cùng revision; chưa đạt ngưỡng
latency sản phẩm nào và không bảo đảm fairness. Backoff cũng không thắng mọi cell
so backend-only (100nodes/4writers trong bảng). Valid batch vẫn đọc/clone/validate
full candidate; SQL commit/lock/validation/serialization chưa có profile tách riêng.
Index DDL startup của spike chưa phải migration online cho production.
Phạm vi vẫn là spike PostgreSQL; runtime/storage/MCP sản phẩm, migration dữ liệu
thật và Kubernetes chưa chuyển. Không commit/push, đổi nhánh hoặc restart dịch vụ thật.
