# Task16 — Interface storage chung

**Trạng thái:** Đã triển khai và root kiểm độc lập ngày 2026-11-02; xem [báo cáo](../docs/TASK16-REPORT.md) và [contract](../docs/STORAGE-BACKEND.md).
**Mục tiêu:** Server dùng một contract storage có thể thay implementation; SQLite
hiện tại là adapter mặc định. Không chỉ thêm trait không được runtime sử dụng.
**Workspace:** checkout hiện tại `/Users/luis/Workspace/OpenCanvas`, nhánh
`claude/design-properties`; giữ mọi thay đổi Task14/frontend/spike có sẵn.
Không commit/push, restart dịch vụ thật hoặc ghi dữ liệu người dùng.

## Thiết kế và ranh giới

- Một trait object-safe `StorageBackend: Send + Sync`, thao tác ở mức domain:
  list/create/project metadata, snapshot, apply/dry-run batch, commit replay,
  import/read asset, read font, read staged batch operations.
- Chỉ dùng model/request/result/error chung; không nhận/trả connection,
  transaction, rusqlite, tokio_postgres, DB path hay WAL state trong trait.
- `Storage` là handle clone được tới `Arc<dyn StorageBackend>`, hỗ trợ injection
  backend tùy biến. `ProjectHandle` là facade engine-neutral giữ storage + ID,
  bảo toàn cách gọi project trong server, không thêm trait DB thứ hai.
- `SqliteBackend` adapter gọi các implementation Store/Project đã có, giữ
  validation/atomicity/idempotency/path guards. Chuyển bounded project pins xuống
  adapter để server không giữ concrete SQLite Project hoặc mở/migrate lặp lại.
- Giữ API Store/Project cũ cho caller SQLite; chuyển `Store` sang `Storage` bằng
  conversion để constructor server cũ vẫn dùng được. Constructor server cũng
  nhận Storage được tạo từ backend khác; không hardcode SQLite trong request/jobs.
- Storage operations là blocking, chạy qua spawn_blocking hiện có. Adapter sau
  phải cung cấp thao tác blocking hữu hạn/driver phù hợp; không block_on reactor
  tùy tiện. Không giữ DB transaction/lock qua render, SSE hoặc response streaming.
- WAL checkpoint, SQLite backup/bundle `db.sql`, orphan inventory và diagnostics
  riêng engine giữ ở API SQLite, không ép thành contract chung giả tạo.
- Legacy `BatchFileRequest.path` vẫn thuộc API hiện có; backend chịu trách nhiệm
  staging/containment hoặc trả lỗi không hỗ trợ rõ, không giả định FS local chung.

## Lanes

1. Backend: `crates/storage/src/backend/**`, `backend.rs`, exports trong `lib.rs`;
   trait/facades/Sqlite adapter/pins. Giữ module nhỏ, công bố signature cho B/C.
2. Server: `crates/server/src/state*`, `jobs*` và caller cần thiết; constructor
   injection, runtime reads/writes/events/assets/fonts/export qua handle chung.
3. Tests/docs: contract tests qua trait/facade, backend thay thế độc lập và
   server injection tests; báo cáo contract và cách thêm adapter.

Root chỉ điều phối/kiểm chứng và cập nhật tiến độ; mỗi lane fresh worker.

## AC và kiểm tra

1. Object-safe: dùng được `Arc<dyn StorageBackend>`, không cần generic server
   theo engine; backend mới inject tại composition root.
2. Backend thay thế không dùng SQLite có thể phục vụ list/create/snapshot,
   mutation/replay và resource reads qua runtime test; không fallback âm thầm.
3. SQLite qua interface giữ canonical IDs/slug, bounded pins/path replacement,
   atomic rollback, revision conflict, lost-response replay/key collision,
   dry-run, resource bytes và batch file guards.
4. HTTP/MCP schemas/response/error và render/export behavior giữ nguyên; chạy
   compile/lint và regression phù hợp, E2E tạm đại diện nếu cần.
5. Không sửa spike thành production adapter, không migration hoặc đổi data root.
   Tài liệu nói rõ PostgreSQL production chưa được implement qua trait.
6. File mã viết tay mục tiêu100–200 dòng, >250 phải xem lại, tránh>350;
   ghi size/module review và kết quả lệnh thật trong report.

**Đầu ra:** contract/facades/SQLite adapter đã được server dùng thật, tests có
backend thay thế, `docs/STORAGE-BACKEND.md` và `docs/TASK16-REPORT.md`.

**Nghiệm thu:** root kiểm storage/cache8 bài, server15 bài, strict production
Clippy/fmt và native E2E16 nhóm + live workspace đạt. Backend RAM độc lập đã inject
qua router thật; defaultSQLite, dữ liệu/schema và spike PostgreSQL giữ nguyên.
Test locator Project được sửa riêng sau lần E2E đầu bị trùng combobox/listbox.
