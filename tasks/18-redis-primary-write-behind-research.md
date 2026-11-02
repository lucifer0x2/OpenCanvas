# Task18 — Research Redis primary HOT, PostgreSQL COLD/write-behind

**Trạng thái:** Backlog nghiên cứu; người dùng yêu cầu lưu ngày 2026-11-02.
Không triển khai cùng Task17 và chưa chọn Redis làm database chính.

## Ý tưởng của người dùng

Project hoạt động nằm trong Redis; mutation ghi Redis và đánh dấu chưa sync.
TTL dự kiến1h; worker mỗi10phút chỉ đồng bộ project có cache và dirty xuống
PostgreSQL. Redis đồng thời phục vụ API/list nodes agent gọi liên tục.

## Mục tiêu nghiên cứu sau

Đánh giá Redis primary có lợi hơn PostgreSQL durable commit + Redis read cache
của [Task17](17-postgres-delta-redis-hot-reads.md) hay không, với workload thật,
chi phí vận hành và contract ACK/recovery được xác định trước.

## Câu hỏi và AC research

- Chốt RPO/RTO và ý nghĩa ACK. Redis primary phải có durability/persistence,
  replication/failover/backup phù hợp; không coi AOF everysec hoặc WAIT là tự
  động bảo đảm không mất acknowledged writes trong mọi failure.
- Dữ liệu dirty không TTL/eviction; chỉ expire bản đã durable sync. Mô tả memory
  pressure/backpressure và worker/PG hỏng lâu hơn TTL, không bỏ dữ liệu chưa lưu.
- Dùng currentRevision/persistedRevision thay synced boolean; snapshotR sync
  xong không đánh dấu sạch R+1. Atomic mutation/revision/idempotency/journal;
  queue có retry, không dựa duy nhất vào quét các key còn tồn tại.
- Giữ ordered batch validation, project scope, delete tombstones, replay response,
  commit history/SSE cursor; chỉ sync snapshot cuối không đủ cho các contract đó.
- Worker sync có monotonic revision/fencing, duplicate delivery, partial failure,
  crash sau PG COMMIT trước ACK queue; không overwrite revision mới bằng cũ.
- Cache miss khi PG còn trễ, Redis restart/failover, mất journal, recreate/restore
  identity và Kubernetes redeploy phải có hành vi cụ thể, không silently rollback.
- Đo cùng workload/build/resources với Task17: durable ACK latency, read latency,
  sync lag/backlog, RSS/WAL/disk, recovery và dữ liệu thực còn sau fault injection.
- Chỉ research/spike cô lập sau khi được giao; không đổi runtime/dữ liệu thật.

## Nguồn tham khảo ban đầu

- [Redis persistence](https://redis.io/docs/latest/operate/oss_and_stack/management/persistence/)
- [Redis eviction](https://redis.io/docs/latest/develop/reference/eviction/)
- [Redis replication](https://redis.io/docs/latest/operate/oss_and_stack/management/replication/)
- [WAITAOF](https://redis.io/docs/latest/commands/waitaof/)
- [Write-behind architecture](https://redis.io/docs/latest/integrate/write-behind/architecture/)

Write-behind tích hợp sẵn được mô tả trong tài liệu Redis Enterprise/RDI;
không mặc định coi là tính năng Redis OSS đã có sẵn trong OpenCanvas.
