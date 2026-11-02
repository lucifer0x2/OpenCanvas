# Task17 — review độc lập, 2026-11-02

Số đo và kết quả kiểm dưới đây là lịch sử của ngày ghi báo cáo. Raw logs,
JSON/JSONL, manifests và archives được sinh tự động không còn được version trong
cây hiện tại; xem [chính sách output và cách chạy lại](../spikes/realtime-db/evidence/README.md).

Kết luận: **không còn finding bắt buộc sửa** trong phạm vi spike Task17 sau
bản sửa NUL target ID. PostgreSQL vẫn là nguồn authoritative và COMMIT trước
ACK; Redis chỉ phục vụ dữ liệu đọc dẫn xuất. Review không xác nhận production
PostgreSQL adapter, migration dữ liệu thật, Kubernetes HA hay Redis-primary.

Reviewer chỉ đọc source/raw evidence và viết báo cáo này; không sửa implementation,
không build, không chạy Rust suite và không tạo traffic PostgreSQL/Redis. Kết quả
suite bên dưới được kiểm bằng log do integration worker chạy, không phải một lần
chạy độc lập khác của reviewer.

## Finding đã khép

**P2 — NUL trong update target làm đổi ordered error.** Trước sửa,
`core/src/delta.rs` phân loại `update_node` với ID `missing\u0000` là delta.
`postgres/src/delta.rs:60` bind toàn bộ IDs qua `TEXT_ARRAY` trước executor,
nên PostgreSQL trả lỗi bind và API thành `INTERNAL_ERROR`; đường full cũ trả
`VALIDATION_FAILED / Node does not exist`. NUL ở operation sau còn có thể che
lỗi `Props must be an object` của operation trước. Đây là lệch AC1 về error
parity, không làm partial commit.

Integration worker đã tái hiện đỏ bằng PostgreSQL thật trước sửa và ghi lại
`INTERNAL_ERROR` thay vì `VALIDATION_FAILED`.
Bản sửa ở `spikes/realtime-db/core/src/delta.rs:18` trả classifier fallback khi
target chứa NUL, để full executor quyết định lỗi theo thứ tự cũ. Regression tại
`spikes/realtime-db/postgres/tests/contract/delta_failures.rs:73` so code/message
với full `apply` cho ba trường hợp, đồng thời kiểm snapshot, certificate,
revision/history/idempotency không đổi. Reviewer đã kiểm source của remedy và
đọc suite xanh sau sửa. Không cần thay property rules hoặc certificate version.

## Các đường đã rà

- Certificate khớp cả locked revision và validator version mới được delta;
  thiếu/sai metadata, structural/mixed/malformed batch đi full. Property helper
  dùng chung giữ kiểm final candidate, repeated IDs và transient-invalid-then-fixed.
  Selected path chỉ materialize/clone/validate rows được chỉ định, không full tree.
- Project lock trước replay SELECT riêng trong READ COMMITTED giữ đúng snapshot
  sau lock wait. Replay/collision trước expected revision; stale trước node query.
  Dry run rollback; entities, certificate, revision, history, replay và NOTIFY
  cùng transaction; ACK sau COMMIT. Redis không xuất hiện trên write path.
- HOT read phải probe PostgreSQL; key gồm namespace/generation/revision/response
  version và v2 description hash. Miss đọc metadata/nodes qua REPEATABLE READ và
  fill revision/generation thực đọc. Hit không SELECT nodes. DB probe lỗi không
  dùng cached success.
- Redis envelope có digest gắn key, giới hạn transfer bằng STRLEN+GET nguyên tử,
  object cap, TTL, slots và deadline gồm lock/connect/response. Malformation,
  wrong-key copy, timeout, eviction, restart và outage đều về PostgreSQL.
- Migration generation/certificate additive/idempotent; startup initializer chạy
  trước phục vụ request, không DDL trong hot pool lease. Contract registry giữ
  các suite cũ và thêm delta/cache suites. Mọi module Task17 được rà đều dưới
  250 dòng; lớn nhất là delta contract 223 dòng.

## Kiểm bằng chứng

Reviewer đã đọc log tại thời điểm nghiệm thu: core **10 passed**,
PostgreSQL v1 **33 passed**, compat-v2 **34 passed**. Integration worker báo
strict locked/offline all-target Clippy cho core/v1/v2, fmt core/PostgreSQL và
default binary build đều pass; log Clippy/build có completion, fmt log rỗng như
mong đợi. Reviewer không diễn giải log rỗng riêng lẻ thành một lần tự chạy pass.

Raw review khi đó gồm lifecycle report, ba stage baseline/delta-only/delta-Redis,
các attempts/successes/reads/profiles JSONL và comparison verifier. Các outputs này
không còn nằm trong cây hiện tại; [Task17 report](TASK17-REPORT.md) giữ số đo,
lệnh kiểm và cách chạy lại.

Một script Node chỉ đọc của reviewer độc lập tính lại nearest-rank quantiles,
120 attempts/successes và revision liên tiếp mỗi cell, 100 measured samples,
seed/warmup/profile slicing, unique keys, zero retries và committed outcomes.
Đã kiểm cả **6 cells / 600 measured writes**, cùng **400 dedicated HOT reads**.
Selected profiles đều queried=validated=3, sortUs=0; baseline là 100/5.000 rows.
Driver/toolchain đồng nhất, baseline → delta-only → delta-Redis tuần tự, freeze
trước/sau khớp và off/on dùng cùng source/binary. Rehash từng source file thực
và binary cũng khớp manifest: current **96 files**, baseline **72 files**.

Nguồn cuối của lượt đo có fingerprint
`74512a42e342811c9916a7e843d6d90bf52bd3a595519ebe6eb257ac680c4b38`.

Lifecycle evidence có positive control: HOT HTTP thành công dưới exclusive nodes
lock, xóa đúng key rồi HTTP cold thực sự chờ lock; oversized/malformed entries
fallback; actual LRU eviction/restart; Redis-down write có mọi cache counter delta
bằng 0; dropped ACK retry qua gateway khác thêm đúng một commit; DB outage trả
500 dù snapshot đã cache; recreate cùng ID/revision dùng generation khác.
Raw reports ghi cleanup container/volume/process/credentials đều true, được
harness kiểm bằng ownership và existence assertions; reviewer không chạy lại
inventory container hoặc lifecycle tests.

## Giới hạn cần giữ trong kết luận

- Dedicated HOT reader-only p50 giảm **2,995 → 1,107 ms** ở 100 nodes và
  **54,990 → 21,155 ms** ở 5.000 nodes. Đây là workload có reuse snapshot.
- Redis không bảo đảm tăng tốc mọi chỉnh sửa: 100-node whole-operation p50
  delta-only **2,834 ms**, delta+Redis **3,985 ms**. 5.000-node continuous-reader
  p50 tương ứng **49,105 ms** và **66,189 ms**; cell Redis chỉ có **5 reads trong
  measured window**, 12 raw reads tổng cộng. Không dùng nhóm nhỏ đó làm SLA hoặc
  suy rộng tail latency. ContinuousReadMs gồm cả warmup và thời gian ngoài cửa
  sổ 100 writes; dedicated HOT load có 100 measured reads/cell riêng.
- Các số này là một lượt đo tuần tự dev build, một writer, host gateways không
  giới hạn CPU; source freeze không loại hết nhiễu host. Counters là số rows
  materialize/validate, không chứng minh số physical tuples PostgreSQL scan.
- Same-URL restore giữ nguyên generation/revision phải rotate generations hoặc
  namespace trước bật cache. Fixture restore hiện dùng DB URL khác; Task17 không
  thêm API restore cùng identity. Compat-v2 cần additive description migration
  có sẵn, ngoài initializer certificate mới.
- Digest phát hiện corruption/binding error, không chống actor có Redis write
  credentials. Scalar spike certificate không chứng minh validator parity cho
  text/layout/theme/instance của sản phẩm. Runtime thật vẫn SQLite, Task18 còn
  là backlog nghiên cứu.
