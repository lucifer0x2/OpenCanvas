# Task17 — PostgreSQL delta commit và Redis HOT reads

Ngày 2026-11-02. Đã triển khai và kiểm trong spike `spikes/realtime-db/` theo
[Task17](../tasks/17-postgres-delta-redis-hot-reads.md). PostgreSQL vẫn là nguồn
dữ liệu authoritative và COMMIT thành công trước ACK. Redis chỉ cache response
snapshot; tắt hoặc mất Redis vẫn đọc từ PostgreSQL và ghi được bình thường.
SQLite runtime, dữ liệu người dùng, server thật và migration production không đổi.

## Kết quả và ranh giới

Batch chỉ update scalar trên revision có certificate hợp lệ chỉ tải, clone và
validate các nodes được chọn, rồi dùng persistence atomic hiện có. Structural,
mixed, malformed hoặc certificate thiếu/sai version/sai revision đi đường full.
Executor và property validator dùng chung giữ ordered semantics, repeated IDs,
final-candidate validation, replay/collision trước revision và stale reject trước
nodes. NUL trong target ID được đưa về full path để giữ lỗi và thứ tự lỗi cũ,
thay vì lỗi PostgreSQL TEXT khi bind parameter.

Đây là domain scalar của spike: name/text/fill/x/y/width/height/fontSize/opacity.
Không suy thành parity text/layout/theme/instance của validator sản phẩm.
Certificate chỉ đúng khi mọi mutation đi qua boundary này; trực tiếp sửa SQL
phải invalidate certificate. Chi tiết tại
[POSTGRES-DELTA-WRITE](../spikes/realtime-db/POSTGRES-DELTA-WRITE.md).

GET có Redis luôn probe identity/revision authoritative từ PostgreSQL trước.
Key gồm DB namespace, generation, revision và response version/metadata. Hit trả
encoded bytes, không SELECT nodes. Miss đọc metadata/nodes trong cùng transaction
REPEATABLE READ và cache dưới revision thực đọc. Probe→fill race, rollback, gateway
khác và old revision entries không cần tin NOTIFY invalidation. Xem
[REDIS-HOT-READS](../spikes/realtime-db/REDIS-HOT-READS.md).

Generation ngăn collision khi recreate cùng project ID/revision. **Restore vào
cùng DB URL giữ nguyên generation/revision cần rotate generation của mọi project,
đổi cache namespace hoặc xóa các cache keys thuộc DB đó trước khi bật lại cache.**
Task không thêm API restore cùng identity. Fixture restore đã có dùng DB URL
`/restored` khác namespace; không coi nó là bằng chứng restore cùng identity.
Digest SHA-256 của cache entry kiểm corruption/key binding, không phải MAC;
Redis credentials chỉ dành cho writer được tin cậy. Compat-v2 vẫn cần cài
`infra/migration-v2.sql` để có description; delta initializer không tự bật v2.

## Contract, lint và harness

Kiểm tra cuối sau sửa NUL target: core **10/10**, PostgreSQL default v1 **33/33**,
compat-v2 **34/34**; strict offline/locked all-target Clippy core/v1/v2 và fmt
core/PostgreSQL đạt. Binary default v1 được build cuối, trước mọi timing.
Lệnh, raw logs, failing regression trước fix và source checksum tại
[final checks](../spikes/realtime-db/evidence/task17-final-checks-20261102/VERIFICATION.md).

Node targeted **16/16** đạt với `--test-concurrency=1`: client SSE, retry,
single-writer quantiles/provenance, RESP probe và Task17 profiling guards. Syntax
checks cho năm entry/helper đã sửa đạt. Hai SSE tests ban đầu gặp EPERM bind
loopback trong sandbox; chạy lại với quyền loopback của fixture thì đạt, không
đổi implementation. [Node check record](../spikes/realtime-db/evidence/task17-final-checks-20261102/node-checks.json).

Phiên chính kiểm độc lập lại core10, PGv133/v234 và bộ Node harness26/26;
[raw root checks](../spikes/realtime-db/evidence/task17-root-checks-20261102/).
[Review độc lập](TASK17-REVIEW.md) kiểm lại raw quantiles/counts/profile slices,
600measured write samples và400dedicated HOT-read samples, cùng manifest96/72files
và hai binary hashes; không còn required finding.

Contracts kiểm differential scalar/full, certificate fallback, dry run, late
validation/SQL/COMMIT rollback, idempotent replay/lost response, cold/hot/size caps,
wrong-key/corrupted cache, race consistency, two clients, recreate và DB error.
Compat-v2 thêm description thay đổi không tăng revision. Không chạy lại matrix
4/8 writers hoặc toàn bộ product suite để nghiệm thu spike này.

## Lifecycle thật và HOT reads

[Raw lifecycle report](../spikes/realtime-db/evidence/task17-final-cache-20261102/report.json)
đạt trên hai gateway process, PostgreSQL và Redis container mới, dữ liệu tạm:

| Probe thực | Bằng chứng |
| --- | --- |
| Cold rồi 100 HOT GETs | Cold miss1/nodes query1/store1; HOT hit100/probe100/nodes query0; TTL3600s. |
| ACCESS EXCLUSIVE bảng nodes | HOT GET200 trong2,268ms; xóa đúng cache key, GET cold thực chờ lock trong `pg_stat_activity`, hoàn tất sau release160,651ms. |
| Malformed và entry quá cỡ | Mỗi probe fallback đúng revision, query1/store1; entry8.388.645bytes bị chặn bởi cap8MiB+36bytes envelope, không bị parse như response hợp lệ. |
| Eviction LRU thật | `evicted_keys` tăng50, key HOT biến mất sau64filler keys; GET kế tiếp miss/query1. |
| Stop/start Redis | Persistence tắt; DBSIZE0 sau restart; gateway tự phục hồi và đọc từ PG. |
| CLIENT PAUSE800ms | GET hoàn tất207,268ms; lookup/fill mỗi operation budget100ms, fallback2/query1. |
| Redis dừng | GET fallback7,684ms; text+paint write ACK6,210ms, mọi cache counter delta0 trên riêng write; gateway khác đọc nội dung mới. |
| ACK bị drop thật | Retry cùng key/body qua gateway khác replay; chỉ thêm1commit. |
| Structural + rollback | Create structural thành công; invalid update400 giữ snapshot trước đó. |
| PG dừng khi cache đã warm | GET500 trong0,763ms, không lấy cached success để che DB error. |
| Recreate cùng ID/revision1 | Generation đổi, GET chỉ trả node mới. |

Burst sau mỗi commit cho thấy cache cần lặp đọc cùng revision: 10commits với
1GET/commit cho hit0/miss10/query10; 10GET/commit cho hit90/miss10/query10.
p50 GET tương ứng2,827ms và0,985ms; raw latencies nằm trong report, không gộp với
workload một writer liên tục.

Đo riêng **zero writers, cùng revision**, tắt/bật Redis tuần tự trên cùng binary
và PG resources; mỗi cell warmup20 +100samples, hai gateways luân phiên:

| Nodes | Redis tắt p50 / p95 ms | Redis bật p50 / p95 ms | Bật: hit / probe / nodes SELECT |
| --- | --- | --- | --- |
| 100 | 2,995 / 3,726 | 1,107 / 1,643 | 100 / 100 / 0 |
| 5.000 | 54,990 / 61,964 | 21,155 / 23,291 | 100 / 100 / 0 |

HTTP latency gồm truyền và parse toàn JSON ở client; hit không làm response nhỏ
hơn. Mỗi cell tắt cache có100nodes SELECT. Raw100latencies/cell và counters được
lưu riêng trong `hotReadNoCache`/`hotReadRedis` của lifecycle report.

## So sánh một writer

Đo tuần tự baseline → delta-only → delta+Redis, mỗi stage DB/Redis mới, cùng image
digest, PG2CPU/2GiB, Redis1CPU/256MiB (maxmemory64MiB, allkeys-lru), hai gateways
host không cap, dev build không tối ưu. PostgreSQL fsync/synchronous_commit/
full_page_writes đều `on`. Redis được provision cả ba stage để giữ resource
configuration, chỉ stage cuối bật lookup. Không build/test/DB traffic khác trong
cửa sổ đo.

Client/load/retry helpers byte-identical baseline. Mỗi cell một writer, ba node
updates/batch, revision GET riêng, reader snapshot liên tục và SSE, warmup20 +
100samples. POST là HTTP batch latency; whole operation gồm revision GET, POST
và client handling. Nearest-rank quantiles trên từng cell, không pooled samples.

| Nodes | Stage | POST p50 / p95 ms | Whole operation p50 / p95 ms | Server p50 ms |
| --- | --- | --- | --- | --- |
| 100 | Baseline | 2,966 / 3,939 | 3,488 / 4,381 | 2,345 |
| 100 | Delta-only | 2,346 / 3,337 | 2,834 / 3,844 | 1,752 |
| 100 | Delta+Redis | 3,297 / 4,451 | 3,985 / 5,398 | 2,535 |
| 5.000 | Baseline | 36,614 / 67,418 | 37,518 / 68,518 | 35,810 |
| 5.000 | Delta-only | 2,509 / 5,532 | 2,973 / 6,068 | 1,908 |
| 5.000 | Delta+Redis | 2,071 / 4,053 | 2,486 / 5,579 | 1,543 |

Lợi ích chính là delta: whole-operation p50 giảm khoảng12,62lần ở5.000nodes,
1,23lần ở100nodes. Mọi100measured profiles mới đều query/validate đúng3nodes,
`sortUs=0`; baseline là100/5.000nodes. Ở5.000nodes, delta-only node query p50
10,112→0,142ms, decode9,121→0,007ms và apply10,549→0,030ms. Counts là rows
returned/materialized và validation targets, không phải physical tuples SQL scan.

Redis không đảm bảo tăng tốc thao tác ghi: 100nodes chậm hơn delta-only; ở5.000
nodes p50 thấp hơn nhưng whole-operation p99 là37,773ms so với21,795ms delta-only.
Chỉ một lượt/cell trên máy dev; đây không phải SLA hoặc kết luận production.

Cache counters trong toàn cell window (gồm seed/warmup/final/replay checks):

| Nodes | Delta-only nodes SELECT | Delta+Redis hit / miss / nodes SELECT | Hit share |
| --- | --- | --- | --- | --- |
| 100 | 209 | 100 / 114 / 114 | 46,7% |
| 5.000 | 16 | 5 / 10 / 10 | 33,3% |

Không có Redis fallback, malformed, oversize hoặc DB error trong các cells đó.
Continuous reader p50 baseline/off/on là1,754/1,792/2,701ms ở100nodes và
51,461/49,105/66,189ms ở5.000nodes. Raw reader sample counts tương ứng
252/206/211 và96/13/12 (gồm warmup và in-flight cuối); measured read counts là
207/162/164 và78/8/5. Delta rút ngắn cửa sổ ghi nên reader có ít samples hơn;
không suy cải thiện reader latency từ bảng này. Cache miss/fill thường xuyên
khi head liên tục đổi còn thêm chi phí. HOT counterfactual ở trên đo riêng.

Mỗi cell kiểm đủ120commits/SSE events, snapshot không mixed, untouched nodes và
idempotent replay. Mỗi stage đúng2projects/251commits/251idempotency rows.
[Raw comparison verifier](../spikes/realtime-db/evidence/task17-final-comparison-20261102.json)
đã đọc lại attempts/successes/reads/profiles, đối chiếu quantiles, workload,
source/binary/driver/toolchain freeze, resources, sequential timestamps và cleanup.

## Evidence, tái chạy và dọn tài nguyên

Raw reports: [baseline](../spikes/realtime-db/evidence/task17-final-baseline-20261102/report.json),
[delta-only](../spikes/realtime-db/evidence/task17-final-delta-only-20261102/report.json),
[delta+Redis](../spikes/realtime-db/evidence/task17-final-delta-redis-20261102/report.json).
Các raw cũ và baseline report cũ giữ nguyên; lượt cuối dùng chung Task17 wrapper
để có continuous-reader counters/latencies và strict comparison tương ứng.

Baseline source fingerprint `addb25db682b7514fa5556835b49a9903b581e99d92b02b591a187496f782f81`,
binary `e79ddc8cfaa0a512f607781a06ff340a0858c3d49b2a401534138994a93f2a59`.
Final fingerprint gồm core/postgres/harness/infra
`74512a42e342811c9916a7e843d6d90bf52bd3a595519ebe6eb257ac680c4b38`,
binary `8ac73be9d0d99eaf2f0faea61759568fe4e3e168fc05f8214c21090ed4893a91`.
Nguồn/binary/client/driver không đổi suốt các lượt đo. Hai source archives đã
extract vào thư mục tạm và so manifest từng file với report: baseline72files,
final96files, đều khớp. SHA-256 và danh mục raw ở
[evidence manifest](../spikes/realtime-db/evidence/task17-final-manifest-20261102.json).

Tái chạy từ repo root với evidence directory **mới** mỗi lần:

```sh
EVIDENCE_DIR=/absolute/new-cache-directory node spikes/realtime-db/harness/task17-cache.mjs
STAGE=baseline SPIKE_SOURCE=/absolute/baseline-source SPIKE_BINARY=/absolute/baseline-binary EVIDENCE_DIR=/absolute/new-baseline-directory node spikes/realtime-db/harness/task17-benchmark.mjs
STAGE=delta-only EVIDENCE_DIR=/absolute/new-off-directory node spikes/realtime-db/harness/task17-benchmark.mjs
STAGE=delta-redis EVIDENCE_DIR=/absolute/new-on-directory node spikes/realtime-db/harness/task17-benchmark.mjs
node spikes/realtime-db/harness/task17-compare.mjs /absolute/new-baseline-directory/report.json /absolute/new-off-directory/report.json /absolute/new-on-directory/report.json /absolute/new-comparison.json
```

Harness tự tạo PG/Redis cô lập, loopback ports ngẫu nhiên, nhãn ownership và
credential files private; không nhận DB người dùng làm fixture. Redis runtime
opt-in bằng `REDIS_URL`; không đặt biến này để tắt cache. TTL default/cap3600s,
object default8MiB/hard cap16MiB, operation timeout default100ms/cap2000ms,
16nonwaiting operation slots; maxmemory/eviction là cấu hình Redis ngoài gateway.
Không đưa credentials vào logs/evidence. Redis-primary/write-behind vẫn là
[Task18 research](../tasks/18-redis-primary-write-behind-research.md).

Bốn lượt cuối đều xác minh containers, anonymous volumes, gateway processes và
private credentials do chính lượt đó tạo đã xóa. Sau kiểm độc lập, fixture contract
dùng chung cũng đã xác minh lại đúng ownership labels rồi xóa PG/Redis containers,
anonymous volumes, private directories, credential file và fixture pointer:
[cleanup record](../spikes/realtime-db/evidence/task17-final-shared-fixture-cleanup-20261102.json).
Không đụng resource thừa kế khác, không ghi `~/.opencanvas`, không commit/push/restart3847.

Harness/infra mới hoặc sửa lớn nhất120dòng; mỗi module một trách nhiệm. Các
implementation modules core/PostgreSQL đều dưới200dòng; contract lớn nhất
223dòng là `delta.rs` (cache race contracts216dòng). Không file mã viết tay nào
vượt250dòng trong phạm vi Task17. Báo cáo này không áp tuyên bố size đó cho các
thay đổi sản phẩm khác đã có trong working tree dirty.
