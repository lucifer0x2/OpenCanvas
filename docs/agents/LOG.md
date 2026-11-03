# Nhật ký điều phối OpenCanvas

Các mục đã đóng về push main/ruleset và khảo sát app: [archive](history/2026-11-02-publish-notes.md).

Các mục đã đóng về build, design memory và P24: [archive](history/2026-11-03-closed-build-p24.md).

Append-only. Cách ghi theo `AGENTS.md` mục "Nhật ký và memory": mỗi task một heading có ngày, mỗi lần ghi vài dòng (quyết định, kết quả cuối, blocker, link báo cáo).
Lịch sử đến hết Task01–08 và commit `215313c`: `docs/agents/history/2026-10-28.md`.

- Các mục đã đóng 2026-10-28 → 2026-11-02 (design rules, P27, export Rust, thuộc tính Figma, public viewer, storage/PG spike): [archive](history/2026-11-03-log-1028-1102.md).

### Task19 — PostgreSQL runtime backend và Redis binary cache — 2026-11-03

- Người dùng duyệt triển khai bước1: PostgresBackend đầy đủ qua StorageBackend, tích hợp Redis nếu phù hợp; ưu tiên một key/project và zero-copy thay JSON. Root khảo sát domain/contract, chốt [handoff](../../tasks/19-postgres-runtime-backend.md), chia fresh workers backend, binary cache và runtime contracts.
- Redis dùng authoritative identity/revision trước cache và chỉ là read acceleration; API Snapshot owned còn bước materialization. Không migration/cutover, CLI engine selection, restart dịch vụ thật hoặc commit/push.
- Người dùng chốt dùng bitcode thay zero-copy; root chuyển yêu cầu cho cả ba workers và cập nhật handoff. Cache giữ một binary snapshot key/project, decode về owned Snapshot, PostgreSQL vẫn authoritative.
- Nhận adapter PostgreSQL đã đóng băng: full domain/shared validation, entity delta cùng durable revision/history/replay/resources, bounded reusable workers và cache namespace binding. Worker kiểm unit/SQLite resource regression/strict Clippy; root đang kiểm độc lập và giao fresh reviewer chỉ đọc cho source Task19.
- Root kiểm unit13 và SQLite regression19 đạt. Review độc lập phát hiện hai điểm cần sửa trong PG mới: PartialEq có thể bỏ thay đổi dấu của float0 và sort tài nguyên phụ thuộc localeDB. Giao fresh fix worker cho cùng Task19; chưa nghiệm thu trước sửa/lượt kiểm cuối. Bộ chạy Docker tạm được gia cố sau lỗi startup không output, không có lỗi backend trong lượt17 bài trước đó.
- Nhận binary cache và contract lanes hoàn tất; fresh fix worker đã sửa serialized delta và Rust ordering, thêm fidelity regression/ICU fixture. Root đọc delta và chạy độc lập final:20 PG/Redis +1 HTTP/MCP,13unit,19SQLite regression đạt; strict production Clippy/fmt/diff sạch. Xem [báo cáo Task19](../TASK19-REPORT.md).
- Điểm dừng: bước1 và Redis bitcode opt-in trong runtime source đã nghiệm thu phạm vi báo cáo; mọi source/test mới ≤250dòng, không giữ generated output. Docker Task19 đã dọn, inventory label rỗng. Chưa CLI chọn engine/bước2, migration/cutover/bước3, TLS/Kubernetes, benchmark runtime mới hoặc commit/push; không restart dịch vụ hay chạm dữ liệu thật.

### Task19b — Số đo runtime một writer — 2026-11-03

- Người dùng hỏi số liệu; root chốt [phạm vi đo](../../tasks/19b-runtime-single-writer-benchmark.md): release runtime thật, SQLite/PG/PG+Redis,100/5.000nodes, một writer và HOT reads, byte bitcode/JSON. Giao fresh benchmark worker và reviewer chỉ đọc; không sửa production engine hoặc dùng số spike thay runtime mới.
- Phiên tiếp tục tìm thấy đủ raw của hai lượt release. Đã kiểm lại percentile, fingerprint nguồn hiện tại, snapshot/revision, kích thước và Redis HOT counters. Docker inventory Task19 trống. Bổ sung lượt B và kết luận đối chiếu vào [báo cáo runtime](../RUNTIME-STORAGE-BENCHMARK.md).
- Điểm dừng: Task19b đã chốt benchmark một writer. SQLite dẫn về median đọc; Redis HOT làm chậm snapshot trực tiếp ở 5.000 node trong cả hai lượt. Chỉ cập nhật tài liệu tiến độ và báo cáo; CLI selection và migration vẫn ngoài Task19b.

- Mốc Redis 8, Moka và nghiên cứu Ferra đã đóng. Xem [nhật ký lưu trữ](history/2026-11-03-moka.md).

### JSON dạng hàng và GET snapshot revalidation — 2026-11-03

- Người dùng duyệt thử JSON theo schema và yêu cầu cache GET bằng Moka, kiểm revalidation kỹ. Giao hai worker mới cho MCP + encoder chung và HTTP + authoritative cache identity. Root sẽ kiểm parity, hành vi revision/ETag và số đo.
- Giao worker riêng bổ sung benchmark release với GET 200 và conditional 304 cho từng format. Nhận harness đã compile/lint, chờ nguồn HTTP đóng băng. MCP đã qua parity và paging; golden cũ lệch trường `effectiveTheme` có từ trước.
- Nhận HTTP + cache: kiểm unit/SQLite/PostgreSQL đạt, gồm race, nguồn lỗi, asset mất và đổi incarnation. Root chạy release và kiểm raw, 304 body rỗng và HOT vẫn probe nguồn. [Báo cáo](../RUNTIME-STORAGE-BENCHMARK.md#results--release-run-d-with-http-response-cache-2026-11-03).
- Điểm dừng: opt-in rows và cache GET đã kiểm, default body giữ tương thích. Dọn fixture thành công, chưa phát hành. Xem [contract](../CONTRACT.md#http-viewer-surface).

### Chọn backend và cổng qua env — 2026-11-03

- Lưu điểm bàn giao sau benchmark và revalidation. Người dùng duyệt `DATABASE_PROVIDER` mặc định SQLite, `DATABASE_URL` bắt buộc cho PostgreSQL + Moka và `PORT` cho cổng riêng. Giao worker mới nối startup và kiểm bằng binary thật với storage tạm.
- Nhận startup hoàn tất: test cấu hình và smoke debug hai backend đạt; PG có bằng chứng Moka snapshot dùng chung giữa hai format HTTP. Root rebuild server release và kiểm `npm start` trên PG/SQLite tạm, xác nhận ghi đúng engine, cổng env và ưu tiên CLI.
- Điểm dừng: đã dọn fixture, giữ tiến trình người dùng đang chạy. Không migration hay commit/push. [Cách chạy](../../README.md).

### Tối ưu tích hợp PostgreSQL theo review — 2026-11-03

- Root review chỉ đọc storage/postgres, hot cache, snapshot HTTP và SSE; người dùng duyệt sửa cả 7 mục: SSE đọc full snapshot chỉ để lấy revision, lỗi kết nối PG bị coi là không retry được, thiếu write-through/gộp miss cache, đường ghi O(kích thước project), prepare lại câu lệnh mỗi lần và probe lặp, dry run giữ `FOR UPDATE`, cấu hình pool/timeout qua env, clone sâu snapshot khi hit, readiness font O(n²).
- Chia 6 worker tuần tự trên cùng working tree (thay đổi Task19 chưa commit nên không dùng worktree): lỗi kết nối → SSE → env/readiness → round trip → cache → đường ghi/dry run. Root chạy baseline release `task19-bench.sh` trước khi sửa và đo lại sau cùng. Không commit/push, không chạm cổng 3847/3848 hay `~/.opencanvas`.
- Mục 2 xong: classifier `postgres/failure.rs`, worker bỏ client sau `STORAGE_UNAVAILABLE`, test refused/reconnect PG thật; root chạy lại unit 8/8. Phát hiện HTTP trả 400 cho `STORAGE_UNAVAILABLE`, gộp sửa 503 vào worker SSE.
- Mục 1 xong: SSE và import font dùng `project_meta`, `STORAGE_UNAVAILABLE` → 503; test `events_metadata` bắt hồi quy. Root chạy lại đạt. Suite server 194/196: golden outline lệch có từ trước và `workspace_events` flaky trên bản gốc (5/6 lần), không thuộc thay đổi này. Clippy all-targets còn dead code test support có sẵn.
- Mục 7 (vận hành) xong: `DATABASE_POOL_SIZE` và 4 biến `DATABASE_*_TIMEOUT_MS` (module `startup/postgres_env.rs`, README), readiness font dùng `partition_point`. Worker smoke binary debug với PG tạm; root chạy lại `startup_env` 9/9.
- Mục 5 xong: `postgres/statements.rs` (Session + cache prepared statement theo worker, xoá khi 0A000/26000), gộp probe và đọc miss vào một `pool.run`. Round trip: hit 2→1, apply_batch 16→9. Release p50 5.000 node: snapshot pgonly 7,0→5,2 ms, Moka hit 1,33→0,90 ms; commit gần như không đổi. Root chạy lại lib 17/17. Test `stroke_props` fail là của thay đổi props_catalog đang dở, ngoài phạm vi.
- Mục 3 + Arc xong: `postgres/write_through.rs` dựng snapshot sau commit từ đúng payload đã ghi, put trên thread caller sau commit thành công; replay/dry run/lỗi không put; cache trả `Arc<Snapshot>`, `snapshot_with_cache_identity` trả Arc. Bỏ single-flight vì chờ trong worker có thể cạn pool. Root chạy lại lib 20/20. `docs/MOKA-SNAPSHOT-CACHE.md` cần cập nhật cuối đợt.
- Mục 4 + 6 xong: đường ghi so payload mới với text đã lưu (`snapshot/stored.rs`), bỏ serialize `before` và clone Candidate thứ hai, gộp 5 câu ghi thành một CTE; dry run chạy RR read-only không `FOR UPDATE`, test `postgres_dry_run` chứng minh không bị khoá. Không bỏ serialize `after` vì không chứng minh được tập thay đổi đủ.
- Root kiểm độc lập: lib 24/24, 12 binary PG storage 24/24, server `postgres_backend` 2/2, events/startup/snapshot_http/storage_backend đạt, clippy storage all-targets + server lib/bins sạch, file chạm ≤233 dòng, không còn container. Release p50 5.000 node: commit PG 30,2→25,4 ms, snapshot PG 7,0→5,2 ms, Moka hit 1,33→1,05 ms. [Run E](../RUNTIME-STORAGE-BENCHMARK.md#results--release-run-e-after-the-postgresql-review-fixes-2026-11-03).
- Điểm dừng: 7 mục đã triển khai, chưa commit/push/restart dịch vụ thật. Còn mở: validate tăng dần (~10 ms/commit, dùng chung SQLite), `get_changes` vẫn đọc full snapshot, single-flight miss bị bỏ có chủ đích.
- Người dùng bỏ kế hoạch validate tăng dần (scout bị dừng, không để lại file) và yêu cầu commit/push. Nhánh hiện tại chứa `claude` và lịch sử cũ, nên root tạo `feat/postgres-runtime-storage` từ `origin/main` (cùng tree với HEAD) và commit toàn bộ thay đổi thành 2 commit: spike/research (giống hệt nhánh spike đã push) và runtime storage. Đã quét secret/attribution, push branch; chưa mở PR. Test `stroke_props` của props_catalog vẫn fail trong commit.
