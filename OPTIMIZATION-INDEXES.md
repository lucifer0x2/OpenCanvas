# PostgreSQL spike — indexes và query-plan evidence

Số đo và kết quả kiểm dưới đây là lịch sử của ngày ghi báo cáo. Raw logs,
JSON/JSONL, manifests và archives được sinh tự động không còn được version trong
cây hiện tại; xem [chính sách output và cách chạy lại](spikes/realtime-db/evidence/README.md).

Ngày kiểm: 2026-11-02. Phạm vi chỉ là spike `spikes/realtime-db`; runtime SQLite,
dữ liệu người dùng và dịch vụ thật không thay đổi.

## Kết quả đã kiểm

Run pg-1793597486325-94df21f9
đạt `rustContract` 12/12 và `indexEvidence`; nguồn giữ nguyên trong run, gateway,
database fixture và container do run sở hữu đã dọn. Fingerprint nguồn:
`a792088d464857c9c551608463a7df857014a84f25503d8f85c45efcd3e19aed`.
Đây là kiểm riêng contract/query-plan, không phải run toàn bộ matrix/recovery.

Chỉ thêm hai index có truy vấn thực hoặc kiểm FK làm căn cứ:

| Index | Căn cứ | Kích thước trên fixture |
| --- | --- | ---: |
| `nodes_parent(project_id,parent_id)` | Phía tham chiếu của self-FK composite; khi xóa parent, PostgreSQL cần tìm children theo cả hai cột. | 745.472 bytes |
| `render_jobs_artifact(artifact_id) WHERE artifact_id IS NOT NULL` | `GET` artifact thực chạy `SELECT 1 FROM render_jobs WHERE artifact_id=$1 LIMIT 1`; jobs chưa có artifact không cần entry. Cùng cột cũng là FK tới shared blobs. | 16.384 bytes |

Định nghĩa additive ở [indexes.sql](spikes/realtime-db/postgres/indexes.sql);
gateway startup nạp sau hai schema hiện có. Đây là `CREATE INDEX IF NOT EXISTS`
trong spike, chưa phải migration online/concurrent cho database sản phẩm đang chạy.

PK hiện có đã đáp ứng project revision/row lock, node point lookup, commits cursor
và idempotency. Không thêm index trùng PK, index JSONB GIN khi không có predicate
JSONB, hoặc index sort children `(sort_key,node_id)` khi gateway không truy vấn
thứ tự đó. [PostgreSQL 16 giải thích](https://www.postgresql.org/docs/16/ddl-constraints.html)
FK không tự tạo index ở phía tham chiếu;
[multicolumn B-tree](https://www.postgresql.org/docs/16/indexes-multicolumn.html)
phù hợp equality trên leading columns.

## Phương pháp và số đo

PG 16.9 được ghim image digest, container 2 CPU/2 GiB.
Một database dùng một lần được tạo riêng **sau** các phase workload; không thêm
fixture vào database đang đo matrix. Fixture có 10.000 projects; 20 projects có
mỗi project 5.000 nodes (100.000 tổng); 20.000 commits, 20.000 idempotency rows,
10.000 jobs, một artifact chọn lọc. Harness kiểm count thực trước khi đọc plan.

Mỗi stage chạy `ANALYZE`; before chỉ có indexes gốc, after thêm đúng hai index.
Mỗi probe có một lần warmup và một lần ghi
`EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)`. Không tắt sequential scan hoặc ép
planner. Settings thực (`enable_seqscan/indexscan/bitmapscan=on`, cost/cache/
statistics), SQL có placeholders, params và SQL literal đã bind đều được ghi trong
lượt đo cùng 20 before/after plans.
Các literal được tạo trong fixture, không lấy SQL từ client bên ngoài.

| Probe | Before → after | Buffer hits tại root plan | Execution ms |
| --- | --- | ---: | ---: |
| Parent có 101 children | Bitmap `nodes_pkey`: lấy 5.000 entries, loại 4.899 rows → Bitmap `nodes_parent`: lấy 101 entries, không loại row | 229 → 193 | 0,335 → 0,104 |
| Parent leaf không children | Bitmap `nodes_pkey`: loại 5.000 rows → `nodes_parent` Index Scan, 0 rows | 128 → 2 | 0,302 → 0,018 |
| Artifact chọn lọc | Seq Scan: loại 9.999 jobs → Index Only Scan `render_jobs_artifact`, 1 row | 114 → 2 | 0,385 → 0,012 |
| Snapshot 5.000 nodes | Bitmap `nodes_pkey` → Bitmap `nodes_parent`; vẫn đọc/trả đủ 5.000 nodes và sort theo node_id | 131 → 102 | 2,957 → 3,035 |
| Full-table control 100.000 nodes | Seq Scan cả hai stage, đủ 100.000 rows | 1.818 → 1.818 | 5,601 → 5,652 |

Revision và locked revision tự dùng `projects_pkey`; node point dùng `nodes_pkey`;
cursor trả 10 commits dùng `commits_pkey`; idempotency trả 1 row dùng
`idempotency_pkey` ở cả before/after. Harness buộc các probe chọn lọc này dùng
index tương ứng, kiểm actual cardinality, buffers và execution timing thực;
không buộc snapshot/full-table dùng index. Inventory `pg_indexes` lưu toàn bộ
định nghĩa/kích thước và kiểm không trùng index trên cùng bảng.

Chênh lệch rows/buffers là bằng chứng chính. Timing chỉ là quan sát cache ấm
một lần, không phải p95/p99 hoặc bảo đảm độ nhanh; thứ tự chạy/cache/statistics
có thể ảnh hưởng. Root buffer counts đã bao gồm cây con, không cộng chúng lần nữa.
Parent probes mô phỏng predicate và row lock FK; EXPLAIN này không hiển thị
trực tiếp plan SELECT nội bộ của deferred trigger.
[Tài liệu EXPLAIN](https://www.postgresql.org/docs/16/using-explain.html) giải thích
các scan có thể khác nhau theo selectivity và sequential scan hợp lệ khi đọc rộng.

## Đúng hành vi và cách chạy lại

Trên database riêng đã có index, harness chạy gateway thật để tạo cây, reparent
child sang sibling, xóa subtree cũ, kiểm child còn; reparent tới parent thiếu phải
thất bại; xóa subtree mới phải xóa cả child/leaf. Snapshot cuối chỉ còn page,
revision đúng 4. UPDATE SQL cố tạo dangling parent phải bị deferred FK từ chối
với lỗi `violates foreign key constraint`; dữ liệu sau lỗi vẫn nguyên.

Unit tests 2/2 kiểm harness không thể xanh với evidence rỗng, sai actual rows,
thiếu buffers hoặc sai index, và kiểm binding params. Lệnh đã chạy:

```sh
node --test spikes/realtime-db/harness/indexes.test.mjs
PHASES=rustContract,indexEvidence node spikes/realtime-db/harness/run-all.mjs
```

Run thử trước `pg-1793597367324-ef89a496` có plans đạt,
mutation fixture dùng kind `rectangle` không được spike hỗ trợ nên run thất bại;
đã sửa fixture thành `group` trước run đạt nêu trên. Không dùng run thất bại làm
bằng chứng nghiệm thu. Sandbox không truy cập Docker được; run thực dùng quyền
truy cập Docker socket ngoài sandbox, chỉ tạo/dọn tài nguyên run sở hữu.

Các module mới: `indexes.mjs` 102 dòng (điều phối/evidence),
`indexes-fixture.mjs` 81 dòng (SQL/probes/mutations), `indexes.test.mjs` 25 dòng,
SQL 6 dòng. Không file mới nào vượt 250 dòng. Parent index giúp FK/lookup chọn lọc;
đường apply batch/snapshot vẫn tải payload toàn project, cần đánh giá thay đổi
fast path và client retry bằng matrix riêng.
