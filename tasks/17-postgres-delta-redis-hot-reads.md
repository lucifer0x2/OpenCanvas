# Task17 — PostgreSQL commit delta trước ACK, Redis HOT reads

**Trạng thái:** Đã triển khai và root kiểm độc lập trong spike ngày 2026-11-02.
Xem [báo cáo](../docs/TASK17-REPORT.md) và [review](../docs/TASK17-REVIEW.md).
Generated outputs đã bỏ khỏi cây versioned; giữ kết luận/lệnh tái chạy trong báo
cáo, output mới được ignore theo [chính sách](../spikes/realtime-db/evidence/README.md).
**Workspace/branch:** `/Users/luis/Workspace/OpenCanvas`, checkout dirty
`claude/design-properties`; giữ mọi thay đổi có sẵn, không commit/push/reset/stash.
**Phạm vi:** Tiếp tục spike Task15 trong `spikes/realtime-db/`. Không chuyển
runtime SQLite, migration dữ liệu thật hoặc khởi động lại dịch vụ thật.

## Quyết định

- PostgreSQL giữ authoritative entities/revision/history/idempotency. COMMIT
  bền vững thành công rồi mới ACK; Redis không tham gia quyết định commit.
- Redis cache HOT snapshot/list-node HTTP response đúng identity/revision.
  Cache miss/lỗi/eviction/restart trở về PostgreSQL, không serve stale.
- Đường ghi update scalar trong domain spike chỉ tải các rows cần sửa khi
  revision đã valid bằng đúng validator version. Unsupported/structural batch
  hoặc thiếu certificate dùng full validation hiện có; giữ ordered semantics.
- Đây không phải port toàn bộ validator/domain sản phẩm. Không suy fast path
  scalar spike thành text/layout/theme/instance parity của production.
- Redis làm database chính/write-behind để research sau tại [Task18](18-redis-primary-write-behind-research.md).

## Handoff và ranh giới module

Fresh worker A: core delta/validation, PostgreSQL batch/delta persistence, schema
metadata (generation/certificate), targeted Rust contracts. Worker B: Redis client,
HOT read modules, HTTP/compatibility/transport integration, cache contracts và
dependencies. Worker C: isolated Redis infra, baseline capture, lifecycle/cache
integration tests, one-writer benchmark/evidence/docs. Root phối hợp declarations
ở `main.rs`/test registry, kiểm độc lập và cập nhật tiến độ; worker không tạo agent.

Schema chung để phối hợp: `projects.generation TEXT NOT NULL DEFAULT
gen_random_uuid()::text`, `validated_revision BIGINT NULL`, `validator_version
TEXT NULL`. Migration additive/idempotent cho fixture cũ; version mismatch luôn
full fallback. Generation mới phân biệt create/recreate cùng project ID. Restore
vào cùng DB URL giữ generation/revision phải rotate generation/namespace hoặc
xóa cache liên quan trước bật lại; Task17 không thêm API restore cùng identity.

Mỗi module một trách nhiệm, mục tiêu100–200dòng; đánh giá tách >250, tránh >350.
Không thay public HTTP batch/snapshot payload hoặc revision/idempotency semantics.

## AC và kiểm chứng

1. Update scalar đường delta có counters/evidence chứng minh không full-project
   SELECT/clone/sort/validation; dùng cùng property rules và đúng thời điểm kiểm
   của core cũ. Repeated IDs, transient-invalid-then-fixed, errors và final state
   tương đương đường đầy đủ. Structural/mixed batch full fallback.
2. Replay/collision trước revision; stale reject trước nodes; dry run không
   ghi certificate/log/key/cache candidate; SQL/COMMIT lỗi rollback toàn bộ;
   lost response retry cùng key/body qua gateway khác không thêm commit.
3. Redis HOT reads lấy identity/revision authoritative từ DB trước; key có DB
   namespace/generation/revision/response version. Miss đọc một consistent DB
   snapshot và cache dưới revision thực đọc. Không ghép revision cũ với rows mới.
4. GET cache hit không SELECT nodes; Redis unavailable/timeout/restart/eviction,
   malformed cache/miss đều fallback có giới hạn thời gian. DB lỗi không trả
   cached success như dữ liệu head hiện tại. Cache không cản ACK ghi thành công.
5. Hai gateway nhìn commit mới, không cần tin NOTIFY invalidation; recreate ID,
   rollback, race probe→fill và old revision entries không trả stale. Cache TTL
   1h và giới hạn object/cache memory có cấu hình; credentials không vào evidence.
6. Chạy contract PostgreSQL cũ và cache contracts trên PG/Redis tạm cô lập;
   test Redis shutdown/restart và mất cache. Không dùng DB spike cũ người dùng xem.
7. Đo baseline trước sửa và bản mới tuần tự, cùng driver/build/resources; chỉ
   writer1/100–5.000nodes, warmup20 +100samples, reader/SSE. Báo riêng lợi ích delta
   và cache/hit/miss khi làm được, không lấy số nhiều writer cũ làm nghiệm thu.
8. Root kiểm targeted tests/lint/fmt/size, raw/source freeze và cleanup. Đầu ra
   `docs/TASK17-REPORT.md`, hướng chạy Redis opt-in và giới hạn được công bố.
