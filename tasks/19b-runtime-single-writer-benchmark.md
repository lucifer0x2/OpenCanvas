# Task19b — Số đo runtime PostgreSQL/Redis bitcode, một writer

Handoff bên dưới ghi phạm vi của hai lượt
[baseline Redis/bitcode](../docs/RUNTIME-STORAGE-BENCHMARK.md). Runner hiện tại
đo SQLite, PostgreSQL và PostgreSQL với [Moka](../docs/MOKA-SNAPSHOT-CACHE.md).

**Yêu cầu:** Người dùng hỏi số liệu sau Task19 (2026-11-03). Đo source runtime
thật; không dùng kết quả spike Task17 thay số mới. Không mở tối ưu engine hoặc
nhiều writer trong task này.
**Workspace:** checkout dirty `/Users/luis/Workspace/OpenCanvas`, nhánh
`claude/design-properties`; giữ mọi thay đổi, không git mutation/commit/push,
dịch vụ thật, dữ liệu thật hoặc `~/.opencanvas`.

## Phạm vi / AC

1. Harness nhỏ gọi backend thật qua `Storage`, và real AppState/router cho HTTP
   snapshot. Không sửa production code/Cargo dependencies nếu không cần thiết.
2. Build **release**, fixture100 và5.000 authored nodes đúng cùng shape/bytes giữa
   SQLite, PG, PG+Redis; xác nhận snapshot/revision/history/replay sau đo. Một
   writer tuần tự, batch cập nhật3 node; warmup20 và100 samples/cell tối thiểu.
3. Đo riêng commit storage, snapshot storage, HTTP GET snapshot end-to-end có
   nhận body và parse JSON. Phân biệt server/network body và client parse nếu có.
   Redis HOT cell cùng revision, có bằng chứng cache hit đầy đủ; cold/miss ghi
   riêng. Không trộn storage time với HTTP hoặc cộng p50 các pha thành total.
4. Báo p50/p95/p99, số mẫu/failures; không gọi p99 từ100samples là SLA. Nếu cần
   chạy lại để root kiểm, ghi từng run chứ không gộp/loại outlier tùy tiện.
5. Đo byte snapshot JSON, Redis STRLEN bitcode envelope và MEMORY USAGE; đây là
   kích thước representation hiện tại, không mặc định bitcode nhỏ/nhanh hơn JSON.
6. Dùng Docker PG/Redis mới qua runner Task19 hoặc wrapper riêng, loopback random
   ports, resources cố định và report images/digests/build/toolchain; SQLite
   tempfile. Raw samples/output chỉ ở temp hoặc path ignored. Tự dọn owned
   containers/volumes/server/process/credentials; không chạm fixture khác.
7. Report thời gian backend call có validation/driver/pool/COMMIT; không suy ra
   physical I/O hoặc so trực tiếp release product với debug scalar spike. Nêu
   full candidate validation, bitcode intermediate DOM và cache fresh connection
   như tradeoff, chỉ kết luận bottleneck khi có đo pha thực.

## Handoff

Fresh benchmark worker owns new `crates/server/tests/runtime_storage_benchmark.rs`
and focused modules, optional new `scripts/task19-bench.sh`,
`docs/RUNTIME-STORAGE-BENCHMARK.md`; dùng lại runner source frozen của Task19.
Public config APIs ở `docs/POSTGRES-BACKEND.md` và `docs/REDIS-BINARY-CACHE.md`.
Root chỉ kiểm/chỉ đạo/progress docs; fresh read-only reviewer kiểm phương pháp,
timer boundaries, hit proof và report/raw consistency. Không spawn đệ quy.

Mỗi module mục tiêu100–200 dòng; đánh giá>250, tránh>350. Chạy targeted
compile/fmt/Clippy và benchmark, không chạy lại toàn bộ correctness suites/build.
**Đầu ra:** số đo thật cùng runnable harness và kết luận thẳng về regression/
tradeoff; không version generated samples/logs hoặc tuyên bố cải thiện trước đo.
