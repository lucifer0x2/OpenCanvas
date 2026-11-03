# Task19 — PostgreSQL runtime backend và Redis bitcode snapshot

Đây là báo cáo nghiệm thu Task19 ở thời điểm dùng Redis/bitcode. API và lệnh
kiểm cache runtime hiện tại nằm ở [Moka snapshot cache](MOKA-SNAPSHOT-CACHE.md).

Ngày kiểm: 2026-11-03. Phạm vi là bước 1: adapter runtime đầy đủ, injection qua
`Storage`/`AppState` và cache snapshot opt-in. CLI vẫn chọn SQLite; chưa migration,
cutover dữ liệu thật, Kubernetes hay Redis-primary.

## Cách tích hợp

```rust,ignore
let backend = PostgresBackend::connect(postgres_url)?;
let cache = SnapshotCache::new(RedisCacheConfig::new(
    redis_url,
    backend.cache_namespace(),
))?;
let storage = Storage::new(backend.with_cache(cache)?);
let state = AppState::new(storage, server_config);
let app = router(state);
```

Không cấu hình cache thì `Storage::new(backend)` đọc trực tiếp PostgreSQL.
`with_cache` từ chối namespace khác bằng `INVALID_CACHE_CONFIG`; namespace được
đọc từ registry PostgreSQL. Mỗi snapshot có PG probe authoritative trước cache.
Redis miss/lỗi không đổi nguồn authoritative và không tạo SQLite fallback.

PostgreSQL lưu domain hiện có, không dùng domain đơn giản của spike Task15/17.
Validator/candidate được dùng lại; project row lock tuần tự hóa mutation; replay
được đọc sau row lock và trước expectedRevision. Entity delta, revision, history,
response/hash replay và asset bytes cùng transaction; chỉ trả success sau COMMIT.
Payload domain là escaped JSON TEXT và tên project là UTF-8 BYTEA để giữ U+0000
hợp lệ với model SQLite. Các bảng thuộc schema `opencanvas_runtime_v1`.

Cache dùng **bitcode 0.6.9**, với DTO bảo toàn loại số JSON, envelope namespace /
project / generation / revision, bounded object/TTL/network và một key mới nhất
mỗi project. Decode tạo `Snapshot` owned trước consumer. Không có tuyên bố
zero-copy end-to-end hoặc benchmark hiệu năng. Chi tiết ở
[`REDIS-BINARY-CACHE.md`](REDIS-BINARY-CACHE.md).

## Bộ kiểm chứng

Các test integration được đánh dấu ignored có chủ đích: chạy thường không được
âm thầm bỏ qua DB hoặc chuyển sang SQLite. Chạy bằng runner dưới đây để cung cấp
PG/Redis thật, mới hoàn toàn. Những test so parity tạo thêm SQLite trong tempfile
riêng; mọi thao tác PG vẫn đi qua `PostgresBackend` thật.

| Suite | Hành vi kiểm |
| --- | --- |
| `postgres_contract` (2) | ID/slug/project scope, exact replay sau commit sau và reconnect, key collision, late rollback, dry run không giữ key, whole/ordered history, ahead/retention reset, hai backend cùng base revision chỉ một winner, snapshot không trộn entity giữa commit. |
| `postgres_domain` (3) | So SQLite: token alias/themes, styles, instances/scoped themes/paint overrides, notes/reply/resolve/re-anchor, reparent/reorder và cascade; error code/field/opIndex, invalid NUL ID; U+0000 ở tên project/token/note/text và số trong range domain qua reconnect. |
| `postgres_transactions` (3) | Hai backend cùng body/key chỉ một commit và một replay; xmin của entity không đổi giữ nguyên khi sửa một node; deferred constraint trigger cố ý fail tại COMMIT không để entity/revision/history/replay key, retry cùng key thành công sau gỡ fault. |
| `postgres_resources` (2) | Font import bytes/hash/media, bundled font, isolation/reconnect, response import cũ nguyên vẹn dù metadata hiện tại bị thay đổi qua SQL control; invalid signature/media/size/font registration atomic, corrupted stored bytes bị phát hiện; staging trả `STAGING_UNSUPPORTED`. |
| `postgres_cache` (3) | Cold/hot equality, binary payload, U+0000/float 0.0, HOT thành công khi khóa ACCESS EXCLUSIVE cả entities/binaries trong khi cold control lỗi; hai backend cập nhật; racing old/new fill; generation mới khi tái tạo cùng ID/revision; một key và TTL hữu hạn. |
| `postgres_cache_failures` (3) | Malformed/oversize/missing/closed Redis fallback PG; CLIENT PAUSE timeout hữu hạn; writer hoàn tất trong khi Redis vẫn pause; warm cache đã chứng minh có dữ liệu nhưng PG container pause vẫn trả retryable error. |
| `postgres_namespace` (2) | Namespace mismatch bị từ chối; đổi registry authoritative khiến cache cũ không phục vụ (negative control khóa entities), reconnect đọc namespace mới và fill key mới. |
| `postgres_fidelity` (2) | Giữ thay đổi `-0.0 ↔ +0.0` cả token và nested node props qua commit/history/replay/reconnect, không rewrite entity khi JSON giống hệt; thứ tự project và mọi resource array theo Rust trên database ICU có thứ tự SQL khác bytewise. |
| `postgres_fidelity` (worker sửa độc lập) | Regression cho ordering không phụ thuộc collation PostgreSQL và bảo toàn thay đổi signed zero `-0.0`/`+0.0`; kết quả chờ lượt root cuối. |
| server `postgres_backend` (1) | Router/AppState thật dùng PG+Redis: HTTP create/list/snapshot lặp, MCP import/apply/dry-run/rollback/replay/get_changes/reset, HTTP asset/font/bundled downloads, restart router bằng backend mới cùng DB và exact old import replay. |

Không dùng mock PostgreSQL, không dùng service3847/3848, không khởi động binary
native và không ghi `~/.opencanvas`. Router test dùng port0 và không tạo frame nên
không khởi tạo browser measurement/render job. Bằng chứng này không thay thế
nghiệm thu renderer/viewer/PNG hoặc deployment.

## Lệnh tái chạy và hạ tầng cô lập

```sh
bash scripts/task19-check.sh
# Nếu hook RTK lọc mất output cần điều tra:
rtk bypass bash scripts/task19-check.sh
```

Runner tạo container tên ngẫu nhiên có label `opencanvas.task=task19`, PG16 và
Redis7 mới, host port loopback ngẫu nhiên, password PG ngẫu nhiên trong file tạm
mode600. Không đọc database hay credential của task khác. Docker startup có
deadline, port mapping có retry, readiness hữu hạn và kiểm cả hai service.
EXIT/signal cleanup xóa cả container/anonymous volume/file credential. Chỉ in
stage, không in DSN/password; không dùng `bash -x`.

`TASK19_POSTGRES_URL`, `TASK19_REDIS_URL` và container identifiers chỉ tồn tại
trong process runner/test. Test fault pause chỉ nhận tên container đúng prefix
do runner sở hữu, có Drop unpause; EXIT cleanup vẫn chạy khi test thất bại.
Redis CLIENT PAUSE test đợi PING trở lại trước khi kết thúc để không ảnh hưởng
test kế tiếp. Cargo build giới hạn2 jobs, test threads1. Fixture PostgreSQL dùng
ICU `en-US`; fidelity test kiểm ngay SQL collation khác thứ tự Rust, tránh bài
test thứ tự xanh rỗng trên database C locale.

Muốn chạy một suite trên services mới:

```sh
bash scripts/task19-check.sh cargo test --offline -p opencanvas-storage \
  --test postgres_cache -- --ignored --test-threads=1
```

Image đã dùng trong lượt đầu:

- `postgres:16-alpine`, digest `sha256:0cefaabd882e42bcb2a2f3dc56d3ce4770560ba62fd7f197f1a2e008fd5731ab`.
- `redis:7-alpine`, digest `sha256:8da19c7399dca900c6822a1cf30f9d2285585e2c4b457ff4095b0f4500054d3f`.

Generated logs/output ở tool/temp, không thêm vào source hoặc commit.

## Kết quả lệnh thực tế

| Lệnh | Kết quả thực tế |
| --- | --- |
| `scripts/task19-check.sh` lượt worker đầu | **16 storage + 1 server PASS**, sau đó runner exit127: worker đã sửa file runner trong lúc Bash còn chạy, làm lệch vị trí đọc script. Đây không phải lượt runner exit0; không dùng để tuyên bố final green. Cleanup đã chạy, inventory label rỗng. |
| `bash scripts/task19-check.sh` lượt root ban đầu | Exit1 trước ready, không có output; chưa xác định nguyên nhân. Sau đó thêm safe stage logs/deadlines và mapping/readiness retries để lỗi sau có thể điều tra. |
| `scripts/task19-check.sh /usr/bin/true` sau hardening | **PASS exit0**, hai service thật ready và cleanup xong; label inventory rỗng. Chỉ chứng minh startup/cleanup, không chạy tests. |
| `cargo test --offline -p opencanvas-storage --test postgres_contract --test postgres_domain --test postgres_resources --test postgres_transactions --test postgres_cache --test postgres_cache_failures --test postgres_namespace --no-run -j2` | **PASS** compile đủ18 storage test sau API `with_cache -> Result` và namespace coverage. |
| Targeted `rustfmt --check --edition 2021 --config skip_children=true` trên các test/support mới; `bash -n scripts/task19-check.sh` | **PASS** trước lượt root cuối. |
| `cargo clippy --offline -p opencanvas-storage --test postgres_contract --test postgres_domain --test postgres_resources --test postgres_transactions --test postgres_cache --test postgres_cache_failures --test postgres_namespace -j2 -- -D warnings` | **PASS** sau sửa một assertion tránh clone không cần thiết; lần đầu lint này fail, không có runtime failure. |
| `cargo clippy --offline -p opencanvas-server --test postgres_backend -j2 -- -D warnings` | **PASS**. |
| `rtk bypass bash scripts/task19-check.sh` — root độc lập sau review | **PASS exit0: 20 storage + 1 HTTP/MCP**, không ignored/failed trong các suite được chọn; bao gồm namespace và fidelity trên ICU. Docker inventory theo label Task19 rỗng sau cleanup. |
| `cargo test --offline -p opencanvas-storage --lib -- --test-threads=2` — root | **13 PASS**, gồm PostgreSQL config/handshake và 6 bitcode/cache unit tests. |
| `cargo test --offline -p opencanvas-storage --test backend_contract --test backend_contract_resources --test assets --test font_portability --test transactions --test batch_file -- --test-threads=2` — root | **19 PASS / 6 suites** hồi quy SQLite sau tách shared helpers. |
| `rtk bypass cargo clippy --offline -p opencanvas-storage -p opencanvas-server --lib --bin opencanvas-server -j2 -- -D warnings` — root | **PASS** trên source cuối. |
| `rtk bypass cargo fmt --all --check`; `rtk bypass git diff --check` — root | **PASS**. |

Review độc lập phát hiện hai điểm cần sửa trước nghiệm thu cuối: thứ tự TEXT theo
collation PostgreSQL không nhất thiết giống thứ tự Rust/SQLite; và `PartialEq`
của số JSON coi `-0.0` bằng `+0.0`, có thể bỏ sót entity delta khi chỉ đổi dấu zero.
Root đã giao worker mới sửa persistence/order và thêm `postgres_fidelity`.
Lượt root cuối đã kiểm cả hai sửa. Signed zero là giới hạn được sao chép từ
SQLite cũ, không phải regression parity; phạm vi này sửa adapter PG mới, không
đổi persistence SQLite. Root đọc độc lập delta sửa; review không còn finding
bắt buộc chưa xử lý trong phạm vi Task19.

## Giới hạn và bước tiếp theo

- Đây là runtime adapter/injection hoàn chỉnh trong phạm vi đã kiểm; chưa đổi
  composition root của CLI. Default SQLite và dữ liệu người dùng giữ nguyên.
- Chưa có công cụ migration SQLite→PG/cutover/rollback, cấu hình production,
  TLS deployment, HA/Kubernetes hoặc kiểm workload nhiều writer. Không suy ra
  throughput/latency production từ time bound trong fault tests.
- PG staging filesystem inbox không được giả lập: trả unsupported tường minh.
  Backup/bundle/WAL admin hiện vẫn là API cụ thể SQLite, không thành PG admin.
- Cache là disposable snapshot owned, không là durability source. Asset/font
  downloads vẫn xác minh bytes từ PostgreSQL; cache không thay resource storage.
- Fault SQL trong tests chỉ thực hiện trong DB/container mới của runner.
  Corruption/namespace controls không phải API quản trị sản phẩm.
- Toàn bộ source/test mới và shared helper được sửa không quá250 dòng; lớn nhất
  cache unit tests250, resources233 và fidelity229; runner72. Mỗi module có một
  vai trò, không cần ngoại lệ kích thước. Lockfile generated không áp ngưỡng này.

Tham chiếu: [`tasks/19-postgres-runtime-backend.md`](../tasks/19-postgres-runtime-backend.md),
[`STORAGE-BACKEND.md`](STORAGE-BACKEND.md), [`REDIS-BINARY-CACHE.md`](REDIS-BINARY-CACHE.md).
