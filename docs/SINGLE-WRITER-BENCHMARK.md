# PostgreSQL một writer — baseline và phân rã chi phí, 2026-11-02

Số đo và kết quả kiểm dưới đây là lịch sử của ngày ghi báo cáo. Raw logs,
JSON/JSONL, manifests và archives được sinh tự động không còn được version trong
cây hiện tại; xem [chính sách output và cách chạy lại](../spikes/realtime-db/evidence/README.md).

Với project 5.000 nodes và mỗi batch sửa 3 nodes, đường tải toàn bộ nodes,
decode JSONB và dựng/kiểm candidate chiếm **81,4–82,9% tổng thời gian server
đã đo**. Median toàn thao tác là **36,6–36,9 ms**. Phần ghi bulk SQL chiếm
2,7–2,8%, COMMIT 6,5–7,1%. Đây là hai lần lặp của **cùng baseline có profiler**;
chưa có bản tối ưu để so tốc độ. Bằng chứng này định vị vùng cần nghiên cứu,
chưa chứng minh một thiết kế thay thế cụ thể nhanh hơn.

## Bằng chứng và điều kiện đo

- Hai lượt baseline độc lập và một verifier đã đối chiếu lại raw JSONL tại thời điểm đo.
- Mỗi run tạo database/container/volume mới, chạy tuần tự 100 rồi 5.000 nodes;
  mỗi size dùng project mới, **20 warmup + 100 mẫu đo**, một writer duy nhất.
  Hai run tuần tự lúc 08:47:01–08:47:10 và 08:47:21–08:47:30 UTC.
- Cùng PostgreSQL 16.9 image ghim digest, giới hạn 2 CPU/2 GiB qua OrbStack;
  hai gateway native trên Apple M5/10 logical CPUs/24 GiB, không giới hạn CPU
  gateway. Dev build Rust 1.99.1, Node 24.21.0. Không build/test/load DB khác
  trong cửa sổ đo do nhóm agent điều phối; không kiểm soát mọi process của máy.
- Client `commitFixture` hiện có: GET revision nhẹ rồi POST sửa `sentinel-a`,
  `sentinel-b`, `n-3`. Mỗi edit đổi giá trị thật. Một reader liên tục tải full
  snapshot luân phiên qua hai gateway; SSE ở gateway thứ hai. Workload giữ
  reader/SSE của phép đo trước, không chạy writer-only hoặc matrix 4/8 writers.
- Baseline nguồn được chụp trước sửa backend, sau đó chỉ thêm profiler.
  Nguồn, binary, driver/toolchain được hash trước/sau mỗi cell và mỗi run.
  Hai run có cùng mọi hash; schema, indexes và thuật toán vẫn là bản bulk 6 SQL.

Fingerprint nguồn: `73dc2a1d588f452a212c5d7a1d0dbdd9d55c2c008f6563cdf53beb90735a97cc`.
Binary dev: `7ea05f6bc3b3c1de8c817173d018b8575453c934492e7453d4159bd4f6d9c327`.
Danh sách hash từng file đã được đối chiếu tại thời điểm đo; temporary source
snapshot không được giả định còn tồn tại.

## Latency: tách từng run, không gộp hoặc chọn mẫu tốt nhất

Đơn vị ms. Mỗi dòng có **100 mẫu đo**; warmup và seed bị loại khỏi quantile.
Nearest-rank p99 là mẫu thứ 99/100, không phải SLA hay khoảng tin cậy.
POST gồm serialize request, HTTP và parse JSON response. Toàn thao tác còn gồm
GET revision, tạo operations và bookkeeping client; server total là đoạn xử lý
batch có profiler, không gồm HTTP parser/response transport.

| Nodes | Run | POST p50 / p95 / p99 | Toàn thao tác p50 / p95 / p99 | Server p50 / p95 / p99 |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 1 | 3,32 / 4,01 / 4,36 | 3,88 / 4,88 / 5,19 | 2,66 / 3,32 / 3,70 |
| 100 | 2 | 3,06 / 3,93 / 4,72 | 3,68 / 4,49 / 5,06 | 2,44 / 3,10 / 4,25 |
| 5.000 | 1 | 36,01 / 57,01 / 61,87 | 36,87 / 58,46 / 62,70 | 35,32 / 56,34 / 61,25 |
| 5.000 | 2 | 35,76 / 59,71 / 87,01 | 36,63 / 60,27 / 89,38 | 35,01 / 58,64 / 70,71 |

5.000-node run 2 có whole-operation max 126,57 ms và server max 84,87 ms;
không loại outlier. Median tương tự giữa hai run nhưng tail biến động rõ.
Không lấy hiệu của các percentile để suy ra chi phí một pha hay overhead HTTP.

## Pha server và ý nghĩa cho nghiên cứu thiết kế

Mỗi median pha dưới đây lấy từ 100 batch đo, đơn vị ms. `apply` chứa các
subphase core: không cộng `validate`/`clone` vào `apply` lần nữa.

| Pha | 100 nodes R1 / R2 | 5.000 nodes R1 / R2 |
| --- | ---: | ---: |
| Query toàn bộ nodes | 0,315 / 0,300 | 9,834 / 9,307 |
| Decode rows/JSONB thành Node | 0,174 / 0,174 | 8,578 / 8,811 |
| Apply candidate tổng | 0,179 / 0,178 | 10,253 / 10,354 |
| ↳ Clone state | 0,022 / 0,022 | 1,092 / 1,132 |
| ↳ Thực thi operations | 0,012 / 0,012 | 0,073 / 0,074 |
| ↳ Validate candidate | 0,128 / 0,128 | 8,966 / 9,065 |
| ↳ Sort nodes | 0,001 / 0,001 | 0,047 / 0,047 |
| Encode response + persist bulk SQL | 0,325 / 0,300 | 0,876 / 0,846 |
| COMMIT | 0,927 / 0,828 | 1,421 / 1,477 |
| Lock project statement | 0,129 / 0,125 | 0,194 / 0,179 |

Tỷ trọng dùng **tổng elapsed từng pha / tổng server elapsed của cùng 100 mẫu**,
không cộng median. Raw đối chiếu lưu cả tổng tuyệt đối từng pha và p50/p95/p99.

| Nhóm elapsed | 100 nodes R1 / R2 | 5.000 nodes R1 / R2 |
| --- | ---: | ---: |
| nodeQuery + nodeDecode + apply | 26,73% / 27,25% | 82,87% / 81,38% |
| Trong đó validate candidate | 4,89% / 5,08% | 24,57% / 24,56% |
| Persist | 13,37% / 12,46% | 2,68% / 2,81% |
| COMMIT | 38,59% / 37,67% | 6,54% / 7,13% |

Ở 5.000 nodes, việc giảm số SQL ghi hoặc tối ưu tìm 3 changed IDs không nhắm
vào phần elapsed lớn nhất của workload này. Một đề xuất DB phải xem xét cách
đọc/validate dữ liệu cần cho mutation mà vẫn giữ atomicity và các bất biến.
Ở 100 nodes, tỷ trọng COMMIT lớn hơn; không suy kết luận project lớn cho mọi size.

Đây là **elapsed wall time**, không phải CPU profile. `nodeQuery` gồm PostgreSQL,
truyền rows, chờ async và scheduling; `nodeDecode` cũng có thể chịu scheduling.
`lock` là toàn bộ câu SELECT FOR UPDATE, không phải riêng lock-wait. Reader
không giới hạn tốc độ, cạnh tranh tài nguyên có chủ đích; measured reads lần lượt
201/77 và 201/79 theo size. Không dùng số đo này để khẳng định SQL execution,
JSON parser, network hoặc scheduler riêng lẻ là nguyên nhân duy nhất.

Profiler opt-in thêm Instant/tổng hợp rồi ghi một dòng stderr mỗi batch.
Chưa đo overhead so với profiler tắt; không coi latency này là release benchmark.
Ranh giới chi tiết ở [báo cáo instrumentation](../spikes/realtime-db/OPTIMIZATION-SINGLE-WRITER.md).

## Đúng dữ liệu, replay và cleanup

Mỗi run xác nhận 240/240 edits gồm warmup và 200 mẫu đo; 240 commits/SSE events
đúng thứ tự revision không trùng/thiếu. Thêm 11 seed commits thành **251 rows**
ở cả commits và idempotency, 2 projects. Mọi POST đo đều HTTP 200, không conflict,
không retry. Hai sentinel luôn bằng nhau trong snapshot reader; node count,
counter sửa thật, mọi node không liên quan và cuối chuỗi revision đều được kiểm.
Replay cùng request/key từ gateway thứ hai không đổi snapshot/revision; count
DB cuối run xác nhận không thêm commit/idempotency. Mỗi log writer có 251 profile
commit, log gateway thứ hai có 2 replay profile; raw không mang ID/key/payload.

`readCount` trong report chụp trước finalizer; raw reads JSONL có thể thêm một
request đang bay hoàn tất khi finalizer chờ reader. Validator kiểm chênh 0 hoặc
1 và ghi `rawReadCount` riêng; measuredReadCount không chịu chênh này. Giữ nguyên
driver/raw đã đo thay vì sửa số liệu sau đó.

Cleanup cả hai run kiểm Docker container vắng, anonymous volume vắng, PID của
gateway vắng; credential directory riêng bị xóa. Không đọc/đổi/dừng DB spike cũ,
không chạy dịch vụ native thật hoặc ghi `~/.opencanvas`. Container mới dùng label
sở hữu và cổng loopback ngẫu nhiên; không chép credential vào evidence.

## Tái lập và kiểm chứng

Source/driver archives và raw outputs lịch sử đã bỏ khỏi cây hiện tại, vẫn có
trong Git commit cũ `71e2142`. Không dùng đường dẫn snapshot tạm cũ làm input
cho checkout mới. Chạy lại trên nguồn hiện tại từ repo root:

```sh
cargo build --offline --locked --manifest-path spikes/realtime-db/postgres/Cargo.toml
EVIDENCE_DIR="$PWD/spikes/realtime-db/evidence/single-writer-current-r1-NEW" node spikes/realtime-db/harness/single-writer.mjs
EVIDENCE_DIR="$PWD/spikes/realtime-db/evidence/single-writer-current-r2-NEW" node spikes/realtime-db/harness/single-writer.mjs
node --test spikes/realtime-db/harness/single-writer.test.mjs
node spikes/realtime-db/harness/single-writer-summary.mjs spikes/realtime-db/evidence/single-writer-current-r1-NEW/report.json spikes/realtime-db/evidence/single-writer-current-r2-NEW/report.json /private/tmp/single-writer-recheck-NEW.json
```

Dựng binary trước cửa sổ đo, chạy hai lượt tuần tự, ngừng build/load cạnh tranh
do agent quản lý. Các lệnh này đo nguồn hiện tại có delta Task17, không tái tạo
baseline lịch sử chưa tối ưu. Muốn đo đúng baseline cũ cần khôi phục source/driver
archives từ commit cũ vào thư mục tạm, build và truyền `SPIKE_SOURCE`/`SPIKE_BINARY`
tương ứng; không coi binary hash build lại phải giống hash cũ. Kết quả Task17
và lệnh so delta/cache tại [Task17 report](TASK17-REPORT.md).

Node guard tests **5/5 pass**: quantiles, warmup/profile mapping, phase interval
validation/privacy và chống so sánh sai nguồn/resource/workload/cleanup.
Raw validator **4/4 cells pass**. Root chạy độc lập sau cửa sổ đo: core 7/7,
PostgreSQL contract 19/19, strict Clippy và fmt. Không gọi run này là full harness
recovery/migration hoặc test PostgreSQL production adapter/Kubernetes/HA.
Lệnh contract root đã chạy với PG_URL riêng là `cargo test --offline --locked
--manifest-path spikes/realtime-db/postgres/Cargo.toml --test contract --
--test-threads=1` (19 pass, 3,31 s; compile 2,22 s). Output đó nằm trong tool
trace của root, không có file raw riêng; đây là kết quả lịch sử của lượt kiểm đó.

Các module mới tách workload, lifecycle, provenance, profile parser, validator và
test; lần lượt 98/88/35/42/86/79 dòng. Không file mã mới nào quá 250 dòng.
Không commit/push, không đổi lựa chọn thiết kế DB trong task benchmark này.
