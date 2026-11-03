# Task16 — Storage backend chung, nghiệm thu 2026-11-02

Đã có **một trait `StorageBackend` dùng thật trong runtime**. AppState, HTTP/MCP,
history, jobs và SVG/font/asset exports dùng handle chung; SQLite là adapter mặc
định. Backend khác có thể được inject ở composition root, không cần đổi từng route.
Chưa triển khai PostgreSQL production hoặc chuyển dữ liệu người dùng.

## Cấu trúc

| Module | Trách nhiệm |
| --- | --- |
| [`backend.rs`](../crates/storage/src/backend.rs) | `StorageBackend: Send + Sync`, object-safe, 11 domain operations bắt buộc; chỉ dùng model/request/result/error chung. |
| [`backend/handle.rs`](../crates/storage/src/backend/handle.rs) | `Storage` giữ `Arc<dyn StorageBackend>`; `ProjectHandle` giữ canonical ID và handle, không giữ connection. |
| [`backend/sqlite.rs`](../crates/storage/src/backend/sqlite.rs) | `SqliteBackend` delegate về Store/Project hiện có; giữ validators, transaction, revision/idempotency và resource guards. |
| [`backend/pins.rs`](../crates/storage/src/backend/pins.rs) | Bounded strong pins ≤128 keys, tái sử dụng writer, kiểm identity và loại alias cũ khi thư mục bị thay. |
| [`state.rs`](../crates/server/src/state.rs) và jobs/export | Chỉ phụ thuộc Storage/ProjectHandle; bỏ cache giữ concrete SQLite Project trong server. |

`Store`/`Project` cũ vẫn là API SQLite. `From<Store> for Storage` giữ cách gọi
`AppState::new(store, config)`; constructor cũng nhận handle của backend khác:

```rust,ignore
let storage = opencanvas_storage::Storage::new(my_backend);
let state = opencanvas_server::AppState::new(storage, config);
```

Có `Storage::from_shared(Arc<dyn StorageBackend>)` cho backend đã được chia sẻ.
Không thêm dependency. Model, schema DB, validators và các implementation ghi
SQLite cũ không bị viết lại. Tài liệu triển khai adapter và required semantics ở
[`STORAGE-BACKEND.md`](STORAGE-BACKEND.md).

## Contract và giới hạn thiết kế

- Operations là **blocking**, chạy qua spawn_blocking của server hoặc boundary
  hiện có trong export worker; driver phải có thời gian chờ hữu hạn. Trait không
  đưa connection/transaction/SQL ra caller và không cung cấp block_on tùy tiện.
- Backend chịu trách nhiệm atomicity, snapshot consistency, project scope,
  durable ACK, replay trước revision check, collision, dry run và exact resource bytes.
  Không có default implementation trả thành công giả.
- WAL/checkpoint, SQLite backup/bundle chứa `db.sql` và diagnostics riêng engine
  vẫn ở API SQLite. Không ép chúng thành abstraction chung sai ý nghĩa.
- Legacy batch-file path được backend xử lý theo staging/containment của nó;
  không giả định backend khác dùng chung filesystem. Không hỗ trợ phải trả lỗi rõ.
- Đây là boundary thay implementation; dữ liệu/schema/migration, deployment và
  realtime nhiều Pod không tự được chuyển chỉ nhờ implement trait.

## Bằng chứng thay backend

[`storage_backend.rs`](../crates/server/tests/storage_backend.rs) inject một
backend RAM độc lập, không gọi Store/Project/driver SQLite và không có data root.
Qua router thật, test kiểm HTTP create/list/snapshot, MCP apply/replay/get_changes,
HTTP asset/font bytes và lỗi unsupported. Call tracing cùng IDs `memory_*` xác
minh dispatch vào backend được inject; thư mục fixture trống, không fallback SQLite.
Fixture chỉ thực hiện phạm vi hẹp đã công bố, không được gọi là engine production.

SQLite contract tests đi qua trait object/facade, kiểm lookup bằng ID/slug,
scope giữa projects, race cùng revision, replay nguyên kết quả sau commit mới,
key collision, rollback batch lỗi cuối, dry run, fonts/assets, inbox guards,
directory replacement và giải phóng root lock khi handle cuối bị drop.
Pin unit kiểm warm ID/slug cùng writer, bounded eviction và canonical reopen.

## Kiểm độc lập bởi root

| Lệnh/nhóm | Kết quả |
| --- | --- |
| storage `--lib backend::pins::tests`, 1 thread | 1/1 đạt; 65 project tạm để kiểm bounded eviction. |
| storage `--test backend_contract --test backend_contract_resources`, 2 threads | 7/7 đạt. |
| server `storage_backend`, `project_pins`, `transport`, `structured_batch`, `instances_states`, `font_portability`, `export_svg`, 2 threads | 15/15 đạt; router/MCP/SSE và Chromium font/SVG thật. |
| strict Clippy storage/server libraries + server binary | Đạt `-D warnings`. |
| `cargo fmt --all --check`, scoped diff/whitespace, JS syntax | Đạt. |
| build debug server/layout + `node spikes/e2e/run.mjs` | Build đạt; final E2E16 nhóm và live-workspace đạt. |

Server lane cũng kiểm riêng font replacement/face/hash export unit: 1/1 đạt.
Một số test binaries cũ còn dead_code warnings trong shared support do không dùng
hết helper; production strict lint không có warning. Không sửa helper ngoài phạm vi.

E2E đầu tiên dừng ở locator `getByLabel("Project")` khớp cả combobox và listbox
khi menu mở, chưa được tính là pass. Fresh worker sửa duy nhất selector trigger
thành role combobox/exact name trong
[`live-workspace.mjs`](../spikes/e2e/live-workspace.mjs); giữ toàn bộ assertions,
không đổi UI. Sau đó root chạy lại toàn bộ trên binary vừa build và đạt.

## E2E cuối và trạng thái bàn giao

Lượt cuối hoàn tất **14:00:46 ngày 2026-11-02, giờ Việt Nam**, Chromium156.0.8078.4,
Node24.21.0, MCP2025-11-25; scene3/rendererweb4.
[Evidence đã chụp riêng](../artifacts/task16/e2e-20261102T070046Z/evidence.json)
ghi16 checks, PNG/viewer pixel differences0 tại desktop/mobile/restart, durable
replay/rollback, SSE update, persistence qua restart, row wrap và GET-only viewer.
[Responsive](../artifacts/task16/e2e-20261102T070046Z/responsive.json) và
[wrap](../artifacts/task16/e2e-20261102T070046Z/wrap.json) giữ chi tiết;
các giới hạn mobile của fixture cũ vẫn nguyên, không biến pixel equality thành fit.
`temporaryDataRemoved:true`; các process test được harness dừng, không restart dịch vụ thật.

[Provenance](../artifacts/task16/e2e-20261102T070046Z/task16-provenance.json)
lưu hash19 file mã mới/sửa của Task16 và binary debug đã kiểm. Đây không phải
manifest của toàn bộ dirty checkout hoặc dependency tree. Nhánh vẫn
`claude/design-properties`; giữ các thay đổi Task14/frontend/spike có sẵn.

19 file mã được rà kích thước, lớn nhất230 dòng (`jobs.rs`), tất cả≤250.
File mới backend31–116 dòng, tests/helpers47–158 dòng. Không có ngoại lệ kích thước.
Native debug đã build; dịch vụ thật không được khởi động lại. Không migration,
ghi `~/.opencanvas`, commit hoặc push. PostgreSQL vẫn ở spike Task15; adapter
production sau này phải implement và vượt contract tests trước khi chuyển runtime.
