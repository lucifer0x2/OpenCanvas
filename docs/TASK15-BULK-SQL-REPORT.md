# Task15 — ghi PostgreSQL theo lô, 2026-11-02

Đã bỏ vòng gọi SQL theo từng changed ID trong PostgreSQL spike. Một batch ghi
thành công dùng **6 lệnh cố định**, gồm BEGIN/COMMIT, thay vì **N+9**. Candidate
validation, khóa project, revision, replay, dry run và indexes giữ nguyên.
Giảm số truy vấn không đồng nghĩa mọi workload có latency thấp hơn; matrix mới
có kết quả hỗn hợp, được giữ đầy đủ bên dưới.

## Cách ghi mới

- `postgres/src/batch_write.rs` chia candidate cuối thành hai tập ID rời nhau:
  JSONB recordset để UPSERT và text array để DELETE. Không còn tìm lại toàn cây
  cho từng ID, không còn chờ một câu SQL cho mỗi node.
- Một writable CTE ghi nodes, revision, commit history, response idempotency và
  transactional NOTIFY. FK composite vẫn deferred, COMMIT lỗi thì rollback toàn
  bộ. Không dựa vào thứ tự chạy giữa hai CTE ghi/xóa node.
- Lock, replay, đọc nodes và câu ghi dùng typed-query API của tokio-postgres để
  tránh prepare riêng mỗi lần. `db::state`/`revision` giữ mapper/isolation cũ.
- Lock project và đọc replay vẫn tách statement: replay phải thấy snapshot mới
  sau khi đợi writer trước. Không gộp chúng để giảm query nhưng phá đồng thời.

Chi tiết và các đường success/replay/conflict/dry-run:
[implementation report](../spikes/realtime-db/OPTIMIZATION-BULK-SQL.md).
Semantics CTE đối chiếu với [PostgreSQL16 WITH](https://www.postgresql.org/docs/16/queries-with.html#QUERIES-WITH-MODIFYING).

## Số lệnh và đồng bộ giao thức đã đo

Probe loopback PostgreSQL v3 riêng đếm Execute/SimpleQuery và Sync, đối chiếu
ReadyForQuery. Không đưa proxy vào matrix latency; không lưu auth, Bind values,
row payloads hoặc raw SQL. Cả hai phía dùng cùng runner/hash và DB mới riêng:
[before](../spikes/realtime-db/evidence/query-counts-baseline-1793608180/query-counts.json),
[after](../spikes/realtime-db/evidence/query-counts-optimized-1793608180/query-counts.json).

| Changed IDs | SQL executions trước → sau | Protocol completion cycles trước → sau |
| ---: | ---: | ---: |
| 1 | 10 → 6 | 26 → 6 |
| 3 | 12 → 6 | 32 → 6 |
| 500 | 509 → 6 | 1.523 → 6 |

Cùng kết quả cho update-only, delete-only và mixed500. Các counts gồm BEGIN và
COMMIT, không đếm mỗi CTE thành một lần gửi riêng. Prepare/execute/close cũ tạo
nhiều completion cycles hơn số SQL executions. Đây không phải số TCP packets,
không suy thành số lần RTT nối tiếp chính xác. Category labels chỉ là phụ:
DELETE được phân nhóm candidate-read và CTE được phân nhóm notify do thứ tự
matcher; tổng protocol counters không dựa vào tên nhóm đó.

## Matrix trước/sau trên database mới

Baseline nguồn trước sửa được chụp vào `.runtime/query-baseline.kRJExD`, fingerprint
`a792088d464857c9c551608463a7df857014a84f25503d8f85c45efcd3e19aed`.
[Baseline mới](../spikes/realtime-db/evidence/query-baseline-1793601110/report.json)
đã chạy đủ6cells/312edits và cleanup. Không lấy riêng số baseline cũ tốt/xấu để so.

Root chạy độc lập bản mới bằng full harness:
[pg-1793608137042-b80957ec](../spikes/realtime-db/evidence/pg-1793608137042-b80957ec/report.json),
fingerprint `d661d3bc3ec25ac560f0e508520f33d1e4ba34a3d797a3aa28b7b987ce71d7eb`.
Nguồn và binaries không đổi trong run. [Comparison root](../spikes/realtime-db/evidence/pg-1793608137042-b80957ec/comparison-root.json)
kiểm cùng client, nodes/writers/iterations, dev build, host/toolchain và PG16.9
2CPU/2GiB, hai database mới độc lập, chạy tuần tự. Client/retry/load, core,
schema/indexes và Cargo.lock giữ nguyên byte; continuous reader và SSE vẫn chạy.

| Nodes | Writers | Median toàn thao tác trước → sau (ms) | p99 trước → sau (ms) |
| ---: | ---: | ---: | ---: |
| 100 | 1 | 6,87 → 4,76 | 50,48 → 40,23 |
| 100 | 4 | 5,79 → 4,00 | 230,62 → 206,76 |
| 100 | 8 | 6,04 → 4,04 | 600,70 → 338,47 |
| 5.000 | 1 | 35,25 → 42,11 | 69,76 → 115,04 |
| 5.000 | 4 | 38,01 → 46,62 | 1.215,51 → 1.518,43 |
| 5.000 | 8 | 47,82 → 51,39 | 3.353,47 → 2.609,90 |

Matrix này sửa3nodes/batch. Với5.000/8, throughput20,59→21,30edits/s,
conflicts278→295; không tuyên bố số conflict giảm. Với5.000/1, median POST
34,64→41,31ms, p99 POST68,93→90,04ms. Kết quả chưa chứng minh giảm roundtrips
giải quyết phần lớn chi phí của batch nhỏ trên project lớn.

Mỗi writer có12samples; p99 ở1writer là mẫu chậm nhất. Đây là một cặp lượt đo,
không phải kết luận thống kê hay SLA. Reader không giới hạn tốc độ nên số lần đọc
có thể khác giữa hai run; chưa có profile chia lock/SQL/deserialize/validation/
commit/scheduling. Không tự gán phần thời gian còn lại cho một nguyên nhân.

## Kiểm độc lập

- Root full harness **9/9 pha**:19tests contract (18DB scenarios và1blob unit),
  semantics/matrix, jobs/resources/staging, rolling gateway, sole DB restart,
  history1000, additive migration/rollback/backup restore và index EXPLAIN.
- Sáu regression mới: mixed create/update/delete/reparent, recreate cùng ID,
 500operations,1operation xóa1000nodes, same-key replay thực sự đợi project lock,
  deferred FK lỗi tại COMMIT không để lại dữ liệu/notification rồi retry thành công.
- Node **18/18**, strict offline Clippy all-targets/all-features và fmt đạt.
  Lượt Node đầu trong sandbox lỗi EPERM ở2ca mở loopback; chạy ngoài sandbox
  đầy đủ đã pass, không bỏ assertions hoặc ghi lượt lỗi là pass.
- Review độc lập không có finding bắt buộc. Rust module sửa/tạo lớn nhất152dòng;
  modules đo bổ sung cũng dưới250dòng. Không đổi dependencies/schema/indexes.

Lệnh root fullrun:

```sh
env -u PHASES -u EVIDENCE_DIR NODE_COUNTS=100,5000 WRITERS=1,4,8 \
  ITERATIONS=12 CLIENT_MODES=optimized HISTORY_COMMITS=1000 \
  CONTRACT_TESTS=1 SKIP_BUILD=0 node spikes/realtime-db/harness/run-all.mjs
```

## Giới hạn

Batch hợp lệ vẫn tải, clone và kiểm toàn bộ candidate dưới khóa project. Nhiều
writer cùng project vẫn tranh revision. PostgreSQL production adapter, migration
dữ liệu thật và Kubernetes/HA chưa triển khai. Không sửa/chạy lại dịch vụ thật,
không thay DB spike cũ người dùng đang xem, không commit/push.
