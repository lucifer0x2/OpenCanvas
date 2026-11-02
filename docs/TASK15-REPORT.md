# Task15 — PostgreSQL spike, nghiệm thu 2026-11-02

Đây là mốc **trước tối ưu**. Phần tối ưu fast path/retry/index và kết quả kiểm mới
nằm ở [`TASK15-OPTIMIZATION-REPORT.md`](TASK15-OPTIMIZATION-REPORT.md); số đo và raw
evidence dưới đây giữ nguyên lịch sử, không đại diện nguồn mới.

Người dùng đã chốt **PostgreSQL**. Spike PostgreSQL đã được đưa vào
[`spikes/realtime-db/`](../spikes/realtime-db/README.md) và root kiểm độc lập trong
workspace chính. **Runtime sản phẩm vẫn SQLite**; chưa port toàn bộ storage/MCP,
chuyển dữ liệu thật hoặc deploy workload Kubernetes sản phẩm.

## Bằng chứng quyết định

- Run nghiệm thu: [`pg-1793595561297-6e08b047/report.json`](../spikes/realtime-db/evidence/pg-1793595561297-6e08b047/report.json),
  `status: pass`, đủ 8 pha, không bỏ pha hoặc giảm matrix/history.
- Bắt đầu 11:59:10, hoàn tất 12:00:57 ngày 2026-11-02, giờ Việt Nam.
- Nguồn không đổi trong run; baseline `438254936823cb4ef8639d85c1404302c608f7f7`,
  fingerprint `a26d059225c86212e490bac395516b7d6c0281ec603c2a939362c234deba7d24`.
  [Provenance](../spikes/realtime-db/evidence/pg-1793595561297-6e08b047/provenance.json)
  chứa hash từng source và hai binary thực đã chạy; không suy Git HEAD bao trùm dirty tree.
- [Topology](../spikes/realtime-db/evidence/pg-1793595561297-6e08b047/topology.json):
  PostgreSQL 16.9/Debian arm64, image pin digest, Docker OrbStack, 2 CPU/2 GiB;
  hai gateway Rust process độc lập trên host Apple M5/10 logical CPUs/24 GiB,
  không áp CPU/RSS cap cho gateway. Rust/Cargo 1.99.1, Node 24.21.0.
  Đây là phiên bản test được ghim, không phải khuyến nghị phiên bản mới nhất.

## Mã đã triển khai

| Module | Trách nhiệm |
| --- | --- |
| `core/` | Fixture Rust thuần: canonical request hash, candidate tree validation, create/update/reparent/delete. |
| `postgres/src/{db,batch,events}.rs` | Project row lock, transaction atomic, replay trước revision check, consistent snapshot, commit log, LISTEN wakeup và polling repair. |
| `postgres/src/{jobs,leases,blobs,staging,durable_http}.rs` | Job input bất biến, DB-time lease/attempt fencing, resources/artifacts dùng chung, bounded project-scoped upload ID. |
| `postgres/src/{http,transport,main,compatibility}.rs` | HTTP JSON fixture, giới hạn body, SIGTERM/SIGINT drain, DB-backed health, hai bản compile v1/v2 cho migration fixture. |
| `harness/`, `infra/` | Lifecycle cô lập, semantic/load/fault tests, migration/backup, manifest/provenance và raw evidence. |

Chỉ giữ PostgreSQL và harness trong workspace chính; không đưa nhánh SpacetimeDB
vào đây. Các báo cáo lane: [contract](../spikes/realtime-db/POSTGRES-CONTRACT.md),
[durability](../spikes/realtime-db/POSTGRES-DURABILITY.md),
[harness](../spikes/realtime-db/HARNESS-REPORT.md).

## Đối chiếu AC

| AC Task15 | Kết quả và giới hạn |
| --- | --- |
| 1 | **Đạt fixture.** Hai gateway race cùng revision chỉ một thắng; held DB lock của project A không chặn B; candidate lỗi và SQL lỗi ở insert cuối đều rollback nodes/revision/commits/idempotency. |
| 2 | **Đạt fixture.** Cắt HTTP response sau commit, replay qua gateway khác giữ kết quả/revision; changed body cùng key bị collision. |
| 3 | **Đạt fixture.** SSE qua hai gateway/reconnect gapless, snapshot nhiều SELECT nhất quán, dry-run không commit/event/job/reserve key. Có kiểm LISTEN reconnect, polling khi không listener và cursor/reset. Chưa port MCP protocol/session. |
| 4 | **Đạt hai bài process recovery; chưa đạt bằng chứng Kubernetes/DB upgrade.** Gateway lần lượt bị kill/restart khi write/read/SSE tiếp tục. PostgreSQL đơn lẻ stop/start có lỗi và khoảng ngừng được ghi riêng. Publish module không áp dụng. Chưa kiểm HA/failover, nâng version DB hay rolling host trên cluster. |
| 5 | **Đạt durability fixture.** Gateway chết khi job pending và sau completion; worker process thật claim rồi SIGKILL; lease hết hạn được retry, stale attempt/owner bị chặn. Job khác revision/pipeline không ghi đè output nhau. Bytes/hash còn đọc qua gateway khác. Không chạy renderer thật; asset/font chỉ là byte fixtures. |
| 6 | **Đạt migration/backup fixture.** Additive column, hai binary v1/v2 cùng dùng DB, thay v2 bằng v1 thật rồi ghi tiếp, v2 đọc lại dữ liệu cũ. pg_dump/restore sang DB khác giữ snapshot, 7 blob hashes, 3 resources, 2 jobs và 2 uploads. Chưa import SQLite hoặc backup object store ngoài DB. |
| 7 | **Đã đo đủ matrix được đề xuất.** 100/5.000 nodes × 1/4/8 writers, 12 edits/writer; thêm 1.000 commits để đo history/memory/restart. Có raw attempts và retry-to-success; không đặt SLA hoặc suy thành tải người dùng. |
| 8 | **Đạt upload-ID fixture.** Upload ở gateway A, execute ở B; scope project, tối đa 500 ops/2 MiB, inline/staged cùng idempotency, dry-run tương thích. Không dùng inbox path của Pod khác. Chưa port `apply_batch_file` MCP hoặc quotas/GC sản phẩm. |

## Số đo của lượt root

[Contract/matrix JSON](../spikes/realtime-db/evidence/pg-1793595561297-6e08b047/contract.json)
và các file `postgres-*-attempts.jsonl`/`*-successes.jsonl` cùng thư mục giữ raw data.
Thời gian dưới đây là **retry-to-success**, gồm đọc snapshot và conflict retries,
khác latency của riêng request thành công. Cùng project vẫn tuần tự theo revision.

| Nodes | Writers | Edits thành công | Conflicts | p95 (ms) | p99 (ms) |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 100 | 1 | 12 | 0 | 9,2 | 9,2 |
| 100 | 4 | 48 | 74 | 128,6 | 374,0 |
| 100 | 8 | 96 | 440 | 856,2 | 1.456,2 |
| 5.000 | 1 | 12 | 0 | 135,2 | 135,2 |
| 5.000 | 4 | 48 | 74 | 1.735,4 | 4.672,7 |
| 5.000 | 8 | 96 | 365 | 11.044,3 | 17.505,6 |

Đủ **312 edits**, số commits/revisions khớp; không mixed snapshot, không thiếu/trùng
SSE commit. Có assert giá trị cuối theo writer và geometry các node không sửa.
Latency contention ở fixture lớn còn cao: mỗi retry đọc snapshot đầy đủ, batch
validate candidate và các writer cùng tranh một project. Chưa profile tách chi phí
để quy nguyên nhân cho engine; chưa có bằng chứng hiệu năng sản phẩm sẵn sàng.

Đọc lại raw ngày 2026-11-02: mẫu 17.505,6 ms là writer4/iteration0, có **78 retries**;
mỗi batch chỉ sửa 3 nodes trong project 5.000 nodes. p99 của từng POST attempt
(bao gồm cả conflict responses) là 350,5 ms khi có 8 writers; với 1 writer là
72,5 ms, còn p99 toàn thao tác là 135,2 ms. Đây không phải số đo riêng SQL commit.
Code có hai lãng phí xác định được: client GET toàn snapshot chỉ để lấy revision
mỗi retry; adapter đã khóa/đọc revision nhưng vẫn SELECT toàn nodes trước khi
`apply` từ chối revision cũ. Retry không backoff và background reader liên tục
đọc full snapshot làm workload nặng thêm. Chưa sửa/đo lại các điểm này; không coi
đề xuất tối ưu là kết quả đã đạt, không bỏ expectedRevision hoặc tự merge request.

| Recovery riêng | Writes thành công | Raw request lỗi | SSE thiếu/trùng | Ghi nhận |
| --- | ---: | ---: | --- | --- |
| Hai gateway kill/restart lần lượt | 34 | 49 | 0/0 | 76 requests overlap disruption; SSE reconnect gap 299,3 ms, retry p99 96,5 ms. |
| Stop/start PostgreSQL duy nhất | 26 | 26 | 0/0 | 40 requests overlap disruption; control stop/start/readiness 1.466,2 ms, retry p99 1.080,2 ms; SSE socket còn mở và catch up sau DB phục hồi. |

Client thử trực tiếp hai URL và retry; không có load balancer thật. **Có raw errors**
nên không gọi đây là zero downtime. Với Kubernetes, Deployment có rolling strategy
vẫn cần được kiểm qua routing/readiness/session và database topology thật;
xem [Kubernetes Deployment](https://kubernetes.io/docs/concepts/workloads/controllers/deployment/).

History thêm 1.000 commits trên project 5.000 nodes: DB toàn fixture tăng
15.012.323 → 15.790.563 bytes; Docker memory 31,16 → 36,61 MiB.
Sum process VmRSS tăng 197.586.944 → 211.509.248 bytes, có thể đếm lặp shared pages.
Gateway restart + snapshot 161,8 ms; DB restart + snapshot/replay 674,6 ms,
đọc lại đúng 1.000 commits. Đây là tăng trưởng hữu hạn, chưa đo retention/capacity dài hạn.

Backup dùng `pg_dump -Fc`, restore vào DB khác và đối chiếu toàn bộ fixture blob
hashes/resources/jobs/uploads cùng snapshot. Database snapshot nhất quán là tính
chất được mô tả trong [pg_dump PostgreSQL 16](https://www.postgresql.org/docs/16/app-pgdump.html);
test này không chứng minh backup tài nguyên nằm ngoài PostgreSQL.

## Kiểm độc lập và tái chạy

Root đã chạy trong workspace chính:

- Core tests **6/6**, PostgreSQL blob/hash unit **1/1**;
  [DB contract **10/10**](../spikes/realtime-db/evidence/pg-1793595561297-6e08b047/rust-contract.txt).
- Node parser/empty-traffic tests **3/3**; 17 JavaScript syntax checks khi tích hợp.
- Strict Clippy core all-targets và PostgreSQL all-targets/all-features; fmt cả hai crate.
- Default full runner **8/8 pha đạt**, không skipped; source fingerprint trước/sau khớp.

Lệnh runner đã chạy, với các override được bỏ khỏi môi trường:

```sh
env -u PHASES -u SKIP_BUILD -u CONTRACT_TESTS -u NODE_COUNTS \
  -u WRITERS -u ITERATIONS -u HISTORY_COMMITS -u EVIDENCE_DIR \
  node spikes/realtime-db/harness/run-all.mjs
```

Không chạy lại full app/frontend vì chỉ thêm spike độc lập. 38 file mã viết tay,
file lớn nhất 176 dòng; không file nào quá 250 dòng. Code/docs whitespace sạch;
raw Rust test logs giữ nguyên blank lines của output gốc.

Root kiểm sau run: không còn container/gateway thuộc run này, container anonymous
volume đã dọn; không có mật khẩu test trong evidence. Runtime files riêng phục vụ
debug nằm trong `.runtime/` bị ignore; không đưa credentials/builds vào mã nguồn.
Các workload test kế thừa từ phiên trước được giữ nguyên, không dọn nhầm.
Không restart dịch vụ thật, ghi `~/.opencanvas`, commit, push hoặc đổi nhánh gốc.

## Phần còn cần làm để chuyển sản phẩm

Spike chưa port tokens/styles/notes/instances/themes, toàn bộ validator Task14,
asset/font metadata và các API đọc/export/backup thực. Cần đưa command/validation
core thật qua PostgreSQL adapter mà giữ contract, bổ sung job/render pipeline thật,
MCP session behavior, shared resources với lifecycle/quotas, auth/ingress/readiness,
chọn database topology rồi kiểm Kubernetes rollout/backup/failover tương ứng.
SQLite import phải có snapshot/resource manifest, kiểm hash/revision và cutover
riêng; rollback sau khi đã nhận writes mới cần phương án đã kiểm. Chưa thao tác dữ liệu thật.
