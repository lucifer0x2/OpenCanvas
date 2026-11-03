# Task19 — PostgreSQL backend cho domain sản phẩm và Redis binary snapshot

**Trạng thái:** Đã triển khai và root kiểm độc lập ngày 2026-11-03;
xem [báo cáo](../docs/TASK19-REPORT.md).
Handoff bên dưới ghi phạm vi Task19 ban đầu. Cache runtime hiện tại dùng
[Moka](../docs/MOKA-SNAPSHOT-CACHE.md).
**Workspace/branch:** `/Users/luis/Workspace/OpenCanvas`, checkout dirty
`claude/design-properties`; giữ thay đổi Task14/16/canvas/spike, không git mutation,
commit/push, restart dịch vụ thật hoặc ghi dữ liệu người dùng.

## Phạm vi đã duyệt

- Implement đầy đủ `PostgresBackend: StorageBackend` trong source runtime, dùng
  validator/domain hiện có cho nodes/tokens/styles/notes/instances/themes.
- PostgreSQL authoritative; entity delta, revision, history và replay/result/hash
  commit cùng transaction, durable COMMIT trước ACK. Không dùng domain spike.
- Assets/fonts project-scoped, byte/hash/media checks và bundled font; staging
  không có filesystem inbox phải trả unsupported rõ theo contract Task16.
- Redis opt-in cho snapshot reads thật, một key snapshot mới nhất/project, binary
  thay JSON. Kiểm namespace/generation/revision authoritative, bounded timeout/
  object size/TTL; miss/corrupt/down fallback PostgreSQL, PG down không trả cache.
- Người dùng chốt tiếp **bitcode thay zero-copy** trong cùng phiên. Binary DTO
  phải giữ đủ cấu trúc và numeric fidelity; decode an toàn về `Snapshot` owned,
  không bọc JSON bytes trong binary hoặc hứa hiệu năng chưa đo. Không đổi
  HTTP/MCP JSON contract.
- Chưa làm migration SQLite→PG, cutover/rollback dữ liệu thật, Kubernetes hoặc
  Redis-primary. Chọn engine CLI thuộc bước2; bước1 chứng minh injection qua
  `Storage`/`AppState` và router runtime thật.

## Các lane và quyền sửa

1. Fresh backend worker: `crates/storage/src/postgres/**`, module entry,
   `crates/storage/Cargo.toml`, exports `lib.rs`, shared batch helpers tối thiểu,
   Cargo.lock và unit tests PG riêng. Dùng driver blocking có timeout hoặc worker
   được quản lý, không block_on reactor caller. Công bố API cho lane2/3.
2. Fresh Redis worker: `crates/storage/src/hot_cache{.rs,/**}` và bitcode/cache tests,
   `docs/REDIS-BINARY-CACHE.md`. Phối hợp dependencies/exports qua lane1, không sửa
   cùng file. Tích hợp snapshot với lane1 sau khi chốt API cache.
3. Fresh verification worker: PG domain/resource contract tests và server HTTP/MCP
   injection tests, infra test tạm và `docs/TASK19-REPORT.md`. PG/Redis containers
   mới, port loopback random, không đụng fixture/task trước; tự dọn owned resources.

Root chỉ điều phối/kiểm độc lập/tài liệu tiến độ. Worker không spawn tiếp; mỗi
agent một task. Mục tiêu file100–200dòng, đánh giá>250, tránh>350.

## AC / kiểm chứng

1. Mọi method trait được implement; canonical ID/slug/project scope; full product
   validation và error context/ordered semantics giữ tương đương SQLite.
2. Atomic late rollback/dry-run, conflict có currentRevision, replay sau commit
   sau/reconnect và exact original result, changed-body key collision; history
   ordered/whole commits/reset semantics, consistent snapshot.
3. Transaction serialization qua project row lock; replay query sau lock trước
   expectedRevision; only changed entities persisted; resources atomic với metadata.
4. Assets/fonts byte parity, hash/corruption/media/size checks, project isolation;
   bundled font; unsupported staging rõ. Internal SQL/DSN/paths không lộ lỗi.
5. Redis hit bỏ entity SELECT, cold/hot/new revision/generation/racing fill,
   malformed/oversize/unavailable fallback; writer không chờ Redis; PG failure
   không được cache che. One-key bounded replacement và binary numeric fidelity.
6. Real `AppState`/router HTTP+MCP+history+resource downloads dùng injected PG,
   không SQLite fallback; tests cùng DB sau reconnect. Không chạy service thật.
7. Chạy tests targeted với concurrency giới hạn, fmt/strict Clippy phù hợp,
   review size/modules và root review độc lập. Generated outputs không version.

**Đầu ra:** adapter/runtime cache source, tests, report/runbook injection, danh
sách giới hạn và bước2/3 còn lại. Không ghi hoàn tất trước khi kiểm thực tế.

**Nghiệm thu:** root20 PG/Redis storage +1 HTTP/MCP,13 unit và19 SQLite
regression đạt; strict production Clippy/fmt/diff sạch. Hai finding review đã
sửa qua fresh worker và root kiểm lại trên ICU locale; mọi module ≤250 dòng.
Chưa CLI engine selection, migration/cutover, TLS hoặc số đo runtime performance.
