# Task 15 — Database realtime và deploy Kubernetes

**Trạng thái:** PostgreSQL đã chốt; spike và tối ưu fast path/retry/index đã được root kiểm độc lập ngày 2026-11-02. Xem [báo cáo tối ưu](../docs/TASK15-OPTIMIZATION-REPORT.md), [mốc trước tối ưu](../docs/TASK15-REPORT.md). Runtime/migration/Kubernetes sản phẩm chưa triển khai.
**Yêu cầu:** Người dùng muốn redeploy liên tục, giữ dữ liệu; yêu cầu nghiên cứu
SpacetimeDB và các database tương tự trước khi chọn kiến trúc.
**Workspace:** `/Users/luis/Workspace/OpenCanvas`, giữ nguyên checkout và mọi thay đổi đang có.
**Đầu ra:** Spike PostgreSQL cô lập, harness/evidence và báo cáo; phần shortlist bên dưới giữ bối cảnh quyết định.
Được cài/chạy engine cô lập và tạo dữ liệu test tạm cho spike; không migration,
restart dịch vụ thật hoặc ghi dữ liệu người dùng. Kubernetes workload sản phẩm chưa thuộc phạm vi.

## Quyết định triển khai — 2026-11-02

- Người dùng chốt PostgreSQL. Không cần tiếp tục so sánh hoặc triển khai SpacetimeDB.
- Kế thừa code spike có sẵn ở worktree bên dưới, kiểm lại và hoàn thiện các AC
  áp dụng cho PostgreSQL; giữ nghiên cứu/benchmark cũ làm lịch sử, không coi là kiểm mới.
- AC4 tách rolling gateway và restart/nâng cấp database; publish module là không áp dụng
  với PostgreSQL. Không coi restart một PostgreSQL đơn lẻ là HA hoặc zero downtime.
- Phạm vi vẫn là spike cô lập ở `spikes/realtime-db/`, manifest, harness và báo cáo.
  Port toàn bộ storage/MCP, migration dữ liệu thật và deploy Kubernetes sản phẩm là
  công việc tiếp sau, không được coi là đã hoàn tất chỉ nhờ spike.
- Mã PostgreSQL/harness đã tích hợp vào workspace gốc. Root chạy đủ matrix
  100/5.000 nodes × 1/4/8 writers, history thêm 1.000 commits và toàn bộ pha recovery/
  migration/backup: run `pg-1793595561297-6e08b047`, nguồn giữ nguyên trong run,
  cleanup đạt. Các AC còn giới hạn được ghi riêng trong báo cáo; không gọi kết quả
  process restart là Kubernetes HA hoặc zero downtime.

## Bài toán đã xác minh trong repo

### Ưu tiên một writer — 2026-11-02

Người dùng yêu cầu tiếp tục tiến độ phiên khác, tạm gác đo nhiều writer và ưu tiên
xử lý nhanh một writer. Đợt này chỉ chạy tải một writer trên project100/5.000nodes;
giữ matrix4/8writers cũ làm lịch sử, không chạy lại để nghiệm thu hiệu năng mới.

- Chụp nguồn baseline trước sửa; đo chi phí các pha trước khi chọn tối ưu.
- Giữ revision/idempotency, full candidate validation, atomic rollback, durable ACK,
  snapshot consistency và SSE. Không bỏ kiểm để lấy số latency tốt hơn.
- Đối chiếu tuần tự trên database mới, cùng client/build/resources, có warmup và
  đủ mẫu hơn12edits cũ; báo p50/p95/p99, POST và toàn thao tác, số mẫu và cleanup.
- Module nhỏ, kiểm regression sát đường thay đổi và root kiểm độc lập. Phạm vi vẫn
  là spike PostgreSQL; runtime SQLite, dữ liệu và dịch vụ thật giữ nguyên.

Đã thêm profiler opt-in và đo hai baseline100samples/cell, không chạy4/8writers.
Ở5.000nodes, query/decode/apply chiếm81–83%elapsed trong batch; đây là chẩn đoán,
chưa là kết quả tối ưu. [Research](../docs/DATABASE-WRITE-DESIGN-RESEARCH.md)
đối chiếu [proposal người dùng gửi](sources/postgresql-schema-proposal.md),
đề xuất delta-write với validation có phạm vi được chứng minh và cache RAM GET
theo identity/revision; Redis/ltree/GIN chưa có bằng chứng cần thêm. Thiết kế
delta/cache chưa triển khai; [baseline](../docs/SINGLE-WRITER-BENCHMARK.md) giữ
raw/provenance/cleanup và giới hạn workload chỉ sửa ba tên.

### Tối ưu số lượt SQL được duyệt — 2026-11-02

Người dùng yêu cầu bỏ truy vấn nối tiếp theo từng `changed_id` và giảm các lượt
trao đổi cố định trong batch. Phạm vi vẫn là PostgreSQL spike, giữ nguyên
revision/idempotency/atomic validation/dry run/SSE và indexes hiện có.

- Ghi node theo tập hợp; số lượt SQL không tăng theo số node thay đổi. Gộp các
  cập nhật revision/history/idempotency/notify khi an toàn, không bỏ durability.
- Không gộp khóa project với đọc replay bằng một snapshot SQL cũ khi phải đợi
  writer khác; kiểm replay đồng thời và rollback cả batch khi SQL lỗi.
- Chụp nguồn trước sửa, đo trước/sau tuần tự cùng build/resources/workload và
  client tối ưu, giữ reader/SSE. Báo p50/p95/p99 và số lượt SQL thực, không suy
  mọi cải thiện đều do database hoặc thay build profile giữa hai bên.
- Kiểm bulk create/update/delete/reparent, biên500 operations, replay/conflict/
  dry run/rollback và recovery. Root kiểm độc lập; không chuyển runtime thật.

Đã triển khai bulk SQL và typed queries; root fullrun9/9 pha trên nguồn đóng băng
`pg-1793608137042-b80957ec` đạt,19contract tests và18Node tests. Probe giao thức
thực xác minh batch1/3/500changedIDs dùng6lệnh cố định, giữ semantics và indexes.
Matrix latency có kết quả hỗn hợp, chưa giải quyết chi phí batch nhỏ trên5.000nodes;
xem [báo cáo bulk SQL](../docs/TASK15-BULK-SQL-REPORT.md) và evidence được giữ nguyên.

### Tối ưu PostgreSQL được duyệt — 2026-11-02

Người dùng yêu cầu tối ưu tiếp và dùng index. Phạm vi là spike PostgreSQL hiện có;
không chuyển runtime/dữ liệu thật trong bước này. Mốc so sánh cũ là run
`pg-1793595561297-6e08b047`; phải giữ nguyên raw evidence và ghi nguồn mới riêng.

- Từ chối revision cũ sau idempotency lookup, trước khi tải nodes; giữ replay,
  collision, dry-run, rollback và snapshot consistency.
- Thêm đọc revision nhẹ; client retry có backoff/jitter hữu hạn và không bỏ kiểm
  revision. Giữ workload sửa 3 nodes/project 100 hoặc 5.000 nodes, 1/4/8 writers,
  reader snapshot và SSE; giữ mode client cũ để tách cải thiện server/client.
- Kiểm index sẵn có và bổ sung index phù hợp truy vấn/FK thực; tránh index trùng PK.
  Lưu EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) với dữ liệu đủ chọn lọc, ANALYZE bảng,
  không ép planner tắt sequential scan để báo dùng index giả.
- Chụp nguồn baseline trước sửa, đo baseline/server-only/server+client tuần tự
  cùng engine/resources/build profile. Báo p95/p99 từng POST và retry-to-success,
  conflicts/retries, throughput và correctness; không giảm tải để báo nhanh hơn.
- Kiểm regression chứng minh stale request không cần đọc bảng nodes; chạy lại
  contract/durability/recovery/backup và ghi report tối ưu riêng.

Root chia ba lane: PostgreSQL fast path/API; indexes/query-plan evidence;
baseline/retry harness/integration. Mỗi lane dùng fresh worker, root kiểm cuối.

**Kết quả:** root full run `pg-1793597580146-67e95999` đạt9/9 pha, hai matrices
và history1000; đo client tối ưu trên DB mới ở `pg-1793598033043-9d396896`.
Nguồn/binaries/tài nguyên đã đối chiếu; p99 5.000nodes/8writers baseline đo lại
13,25s →2,63s, vẫn còn tail contention. Hai index có EXPLAIN tự nhiên; giới hạn,
raw evidence và cách so sánh ba biến thể ở báo cáo tối ưu. Chưa chuyển runtime.

- `crates/storage/src/store.rs`: `.instance.lock` giữ độc quyền toàn data root.
- `crates/storage/src/db.rs`, `persistence.rs`: mỗi project một SQLite; nodes,
  tokens/styles, assets/fonts, notes, revision, commits và idempotency cần nhất quán.
- `crates/storage/src/batch.rs`: canonical request hash; replay trước kiểm revision;
  candidate validation; tối đa 500 operations/2 MiB; dry run không ghi.
- `crates/server/src/http.rs`: MCP dùng `LocalSessionManager`; không tự chia sẻ session giữa Pod.
- `crates/server/src/events.rs`, `state.rs`: durable commit replay nhưng broadcast
  đánh thức là process-local; stream ở Pod B chưa tự nhận commit từ Pod A.
- `crates/server/src/jobs.rs`: queue, job records, PNG và export files ở RAM của process.
- `crates/storage/src/assets.rs`, `batch_file.rs`: assets/fonts và batch inbox dùng filesystem.
- `main.rs` hiện đợi SIGINT; Dockerfile đặt `STOPSIGNAL SIGINT`. Không được kết luận
  container hiện tại chắc chắn bỏ qua shutdown chỉ vì Kubernetes thường gửi SIGTERM.
- Host/Origin và resource URL cần thiết kế cho ingress; contract local hiện không phải API public có auth.

SQLite là runtime MVP một host hiện tại. Yêu cầu nhiều backend cùng ghi kích hoạt
đánh giá lại trong `PLAN.md` §4; người dùng đã chọn PostgreSQL cho hướng tiếp theo.
Spike này chưa thay thế runtime hoặc chuyển dữ liệu sản phẩm.

## Shortlist và đánh giá kiến trúc

Các nhận định về mức phù hợp/chi phí chuyển đổi là suy luận từ repo và tài liệu,
không phải benchmark hay kết quả deploy đã đo. Bảng này lưu nghiên cứu trước khi
người dùng chốt PostgreSQL, không phải đề xuất tiếp tục triển khai các engine khác.

| Ứng viên | Bằng chứng chính | Đánh giá cho OpenCanvas |
| --- | --- | --- |
| SpacetimeDB | Module Rust/WASM, reducer transaction, subscription/cache client; publish hot-swap và incremental migration [S1–S5]. | Ứng viên spike đáng ưu tiên khi muốn lõi collaborative realtime; cần chuyển command core thành module, giữ MCP adapter và render worker riêng. |
| Convex | Mutations và OCC serializable; self-host mặc định SQLite, hỗ trợ Postgres/MySQL [S6–S8]. | Hợp app reactive theo framework Convex; domain mutations thường viết TS, nên cần chuyển nhiều logic Rust hoặc thêm ranh giới tích hợp. Không mặc định suy cloud HA sang self-host. |
| SurrealDB | Document/graph, transactions, live queries và changefeeds; tài liệu multi-node mô tả storage riêng [S9–S12]. | Đáng xét nếu graph traversal là nhu cầu chính; live notification chưa thay được project commit log/revision/idempotency. |
| PostgreSQL, realtime hiện có hoặc bổ sung | Row locks, snapshot isolation, JSONB; LISTEN/NOTIFY [S13–S16]. | Baseline ít thay domain Rust nhất; có thể giữ MCP/SSE contract, nhưng vẫn cần sửa state process-local. |
| InstantDB | Có SDK reactive/collaboration; Cloud đã đóng đăng ký và công bố dừng 2027-08-31, vẫn có self-host [S17]. | Không ưu tiên làm nền tảng cloud mới; self-host là lựa chọn riêng cần đánh giá duy trì. |

## SpacetimeDB: điểm phù hợp và phần còn phải chứng minh

- Một reducer có thể đại diện toàn batch: kiểm expectedRevision và idempotency,
  validate candidate rồi ghi entities + commits + response trong cùng transaction.
  Đây là thiết kế đề xuất, chưa port hoặc chạy contract tests.
- Rust là lợi thế tái sử dụng pure validation/operations. Crate storage hiện kéo
  SQLite/filesystem nên không thể đưa nguyên crate vào module WASM.
- Tài liệu mô tả hot-swap module và duy trì client với thay đổi tương thích. Có
  ngoại lệ migration: gián đoạn scheduled reducer, xóa table có thể ngắt client,
  thay reducer/schema có thể phá client cũ [S3–S4].
- Hot-swap module, rolling MCP gateway và nâng cấp SpacetimeDB host là ba bài kiểm
  riêng. Chưa xác minh topology self-host HA/rolling host phù hợp cluster người dùng;
  không lấy khả năng Maincloud làm bằng chứng cho self-host Kubernetes.
- Dữ liệu bảng ở RAM, có commit log bền vững [S2]. Phải đo RSS/restart với history
  và idempotency tăng dần; không cho ảnh/font/PNG vào bảng mặc định.
- Tài liệu upgrade 2.0 mô tả confirmed reads mặc định; spike phải xác minh cả
  response ACK và client-visible state bền vững với phiên bản pin thực tế [S5].
- Subscription/cache không tự thay `get_changes` hay chính sách project revision.
  Reducer SDK result cũng không tự là JSON `BatchResult`/error contract hiện có;
  phải thiết kế readback/procedure/adapter tương thích và kiểm replay sau mất response.
- Giữ API agent `apply_batch` với structured JSON. Module được developer biên dịch
  và publish; không chuyển sang mô hình agent sinh/chạy code cho từng lần edit.
- Layout/text measurement, Chromium/native rendering, font/asset bytes và export
  vẫn có vòng đời riêng, cần job ownership/retry/artifact store khi Pod biến mất.

## PostgreSQL baseline và điểm dễ làm sai

- Dùng bảng có `project_id`; khóa/FK/idempotency phải có scope project, vì IDs
  hiện chỉ cần duy nhất trong SQLite riêng của mỗi project.
- Ghi batch trong transaction với project row lock hoặc CAS được kiểm chứng;
  replay được ưu tiên trước expectedRevision. Cùng project vẫn tuần tự theo revision,
  nhiều project có thể ghi độc lập. Không tăng revision bằng sequence có gap.
- Snapshot qua nhiều SELECT cần transaction snapshot nhất quán, chẳng hạn
  `REPEATABLE READ READ ONLY`; mặc định `READ COMMITTED` chưa đủ [S13].
- JSONB giữ thuộc tính linh hoạt; giữ canonical hash của request ở tầng ứng dụng,
  không dựa vào thứ tự key/format JSONB [S15].
- LISTEN/NOTIFY chỉ đánh thức; commits table là nguồn replay. Listener cần kết
  nối riêng, đăng ký đúng thứ tự, catch up sau reconnect và cơ chế polling sửa gap [S16].
- Chưa cần thêm Redis/Kafka chỉ để có realtime. Job table/leases có thể là bước đầu,
  nhưng cần pin immutable render input và pipeline fingerprint, fencing stale worker,
  shared output cùng đường retry; đổi DB không tự cung cấp các phần này.

## Handoff spike — người dùng đã cho triển khai

Run đầu ở worktree `/private/tmp/opencanvas-task15-spike`, nhánh
`codex/task15-realtime-db-spike`, baseline `438254936823cb4ef8639d85c1404302c608f7f7`.
Giữ dirty tree gốc nguyên vẹn. Run đầu từng tách lane PostgreSQL, SpacetimeDB và
harness. Sau quyết định PostgreSQL, các lane hiện tại là contract/core,
jobs/resources/shared staging và harness/lifecycle/evidence. Gateway thử nghiệm
kiểm HTTP JSON semantics của batch; chưa đồng nghĩa MCP session/protocol đã được port.
Fixtures chỉ dùng tập operations công bố rõ; chưa coi là full storage replacement.

**Mục tiêu hiện tại:** Kiểm chứng PostgreSQL đã được người dùng chọn bằng contract và workload OpenCanvas.
**Phạm vi:** fixture/gateway tối thiểu và báo cáo ở `spikes/`; không đổi backend mặc định.
Giữ engine core, adapters, subscription và test harness thành module nhỏ; lý giải file >250 dòng.
**Workspace/branch:** khi được giao, dùng nhánh/worktree `codex/...` tách từ baseline
đã xác minh; không checkout/stash hoặc gom các thay đổi Task14/frontend đang dirty.
Phiên chính giao mỗi phần độc lập cho một worker mới, worker không tạo agent tiếp.

**AC và cách kiểm:**

1. Hai gateway khác process cùng ghi một project: revision conflict đúng, không mất
   update; hai project không khóa nhau ở tầng ứng dụng. Batch sai rollback toàn bộ.
2. Cắt response sau durable commit rồi gửi nguyên request/key qua gateway khác:
   trả đúng kết quả cũ, không thêm revision; key/body khác trả collision.
3. Client đang subscribe thấy thay đổi từ cả gateway; disconnect/reconnect catch up
   đủ theo revision, không dựng snapshot pha trộn; dry run không ghi/event/job.
4. Đo riêng publish module tương thích, rolling gateway, rolling/nâng cấp DB host.
   Giữ write/read/subscription traffic liên tục; ghi raw errors, reconnect gap,
   p95/p99 và duplicate/missing commits. Không gọi chung ba kết quả là zero downtime.
5. Kill gateway/render worker lúc job pending; job/artifact theo ID còn đọc hoặc
   retry được, completion cũ không ghi đè pipeline/revision mới. Asset/font bytes còn nguyên.
6. Migration additive cho phép client v1/v2; thử rollback code và backup/restore
   DB + resources. Import SQLite lần đầu có cutover riêng; chưa hứa migration liên tục.
7. Fixture dự kiến 100/5.000 nodes, 1/4/8 writers; pin versions/resources trước đo.
   Đo latency cả retry-to-success, RSS khi history tăng và thời gian restart.
   Đây là test matrix đề xuất, chưa phải tải thật của người dùng hay SLA đã chốt.
8. Với `apply_batch_file`, không giả định inbox Pod-local xuất hiện ở Pod khác;
   quyết định upload ID/shared staging có giới hạn và giữ inline batch tương thích.

Generated manifests/logs/raw outputs không version trong cây hiện tại; xem
[chính sách output](../spikes/realtime-db/evidence/README.md). Báo cáo giữ kết luận,
giới hạn và lệnh tái chạy; không coi đường dẫn output lịch sử là input sẵn có.

**Đầu ra cần có:** code spike tối thiểu, version/topology manifest, lệnh chạy,
raw evidence và bảng đạt/chưa đạt từng AC; nêu self-host support còn thiếu và chi phí
port domain. Engine đã chốt PostgreSQL theo quyết định trên; không tự provision sản phẩm.

## Nguồn đã mở để nghiên cứu

Tài liệu được truy cập ngày 2026-11-02; trang SpacetimeDB đang gắn nhãn docs 2.0.0,
không suy nhãn đó là server release mới nhất. Phải pin server/SDK khi spike. Tương
tự, tài liệu SurrealDB có mô tả theo phiên bản/engine; không suy mọi bản có cùng khả năng.

- S1: https://spacetimedb.com/docs/intro/key-architecture/
- S2: https://spacetimedb.com/docs/intro/faq/ — architecture, RAM/log, subscriptions, deploy.
- S3: https://spacetimedb.com/docs/databases/automatic-migrations/ — safe/breaking/forbidden và client compatibility.
- S4: https://spacetimedb.com/docs/databases/incremental-migrations/
- S5: https://spacetimedb.com/docs/upgrade/ — confirmed reads từ 2.0; https://spacetimedb.com/docs/databases/transactions-atomicity/ ; https://spacetimedb.com/docs/functions/reducers/ ; https://spacetimedb.com/docs/functions/procedures/
- S6: https://docs.convex.dev/database/advanced/occ ; https://docs.convex.dev/functions/mutation-functions
- S7: https://github.com/convex/convex-backend/blob/main/self-hosted/README.md
- S8: https://github.com/convex/convex-backend/blob/main/self-hosted/advanced/postgres_or_mysql.md ; https://github.com/convex/convex-backend/blob/main/self-hosted/advanced/upgrading.md
- S9: https://surrealdb.com/docs/what-is-surrealdb
- S10: https://surrealdb.com/docs/learn/querying/real-time/live-queries
- S11: https://surrealdb.com/docs/learn/querying/real-time/changefeeds
- S12: https://surrealdb.com/docs/running/multi-node
- S13: https://www.postgresql.org/docs/current/transaction-iso.html
- S14: https://www.postgresql.org/docs/current/explicit-locking.html
- S15: https://www.postgresql.org/docs/current/datatype-json.html
- S16: https://www.postgresql.org/docs/current/sql-notify.html ; https://www.postgresql.org/docs/current/sql-listen.html
- S17: https://www.instantdb.com/essays/instant_team_joins_openai
- Bổ sung: https://supabase.com/docs/guides/realtime ; https://www.sqlite.org/wal.html ; https://kubernetes.io/docs/concepts/workloads/controllers/deployment/

**Giới hạn nghiên cứu ban đầu:** Khi lập shortlist chưa cài/chạy/benchmark engine nào.
Kết quả spike phải đọc ở báo cáo/evidence của run đã kiểm, không suy từ shortlist.
Chưa chứng minh HA hay rolling upgrade trên Kubernetes thật. Các vị trí code trên
phản ánh repo đang dirty; baseline của spike được ghi riêng ở handoff/manifest.
