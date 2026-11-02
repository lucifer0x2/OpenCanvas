# Thiết kế ghi dữ liệu PostgreSQL theo phần thay đổi

Số đo và kết quả kiểm dưới đây là lịch sử của ngày ghi báo cáo. Raw logs,
JSON/JSONL, manifests và archives được sinh tự động không còn được version trong
cây hiện tại; xem [chính sách output và cách chạy lại](../spikes/realtime-db/evidence/README.md).

Ngày nghiên cứu: 2026-11-02. Phạm vi: một writer, spike PostgreSQL; chỉ đọc mã và tài liệu, chưa triển khai hoặc đo phương án đề xuất. Runtime sản phẩm vẫn SQLite qua `StorageBackend`; không migration hoặc đụng dữ liệu/dịch vụ thật.

## Quyết định đề xuất

**Giữ PostgreSQL và bảng entity với JSONB từng node; đổi đơn vị xử lý một lần ghi từ toàn project sang delta có phạm vi validation được chứng minh.** Transaction lấy khóa project, đọc replay, tải những entity thực sự cần kiểm, áp operations vào overlay, kiểm các invariant bị ảnh hưởng rồi ghi delta cùng revision/history/idempotency/event. Những operation chưa chứng minh được phạm vi vẫn chạy validator đầy đủ.

Đây là một thiết kế storage/validation chung, không phải thay engine hoặc chỉ thêm index. Schema hiện tại đã có khóa node và cây parent phù hợp đọc có chọn lọc. Phần cần đổi chính là hợp đồng giữa command core, validator và persistence: chúng đang đòi một project đầy đủ ngay cả khi chỉ sửa ba tên.

Lát cắt đầu nên chỉ mở đường nhanh cho `update_node` sửa literal `name` trên authored text thông thường; điều kiện chi tiết ở dưới. **Lát cắt này chưa tăng tốc được mọi lần sửa text, paint hoặc layout.** Các nhóm đó có bài kiểm riêng và chỉ mở đường gia tăng sau khi chứng minh tương đương validator đầy đủ. Không dùng kết quả workload đổi tên để tuyên bố CRUD tổng quát đã nhanh.

## Bằng chứng hiện có và điều chưa biết

| Bằng chứng trong repo | Kết luận được phép rút ra |
| --- | --- |
| [`batch.rs`](../spikes/realtime-db/postgres/src/batch.rs): sau project lock/replay/revision check, gọi `db::state` cho mọi request hợp lệ | Đường ghi thành công tải mọi node dưới khóa project |
| [`db.rs`](../spikes/realtime-db/postgres/src/db.rs): `WHERE project_id=$1 ORDER BY node_id`, không có tập touched IDs | Index không thể làm kết quả 5.000 rows thành ba rows khi câu hỏi vẫn yêu cầu cả project |
| [`core/lib.rs`](../spikes/realtime-db/core/src/lib.rs), [`validation.rs`](../spikes/realtime-db/core/src/validation.rs): clone `State`, map toàn nodes, kiểm mọi node/ancestor, sort toàn nodes | Có công việc O(N), hoặc O(N × depth), ngoài SQL; tỷ trọng profile baseline mới ở dưới |
| [`batch_write.rs`](../spikes/realtime-db/postgres/src/batch_write.rs): ghi tập thay đổi nhưng vẫn lọc toàn `next.nodes` | Bulk SQL đã giảm lượt trao đổi; chưa đổi đơn vị xử lý candidate |
| [`load.mjs`](../spikes/realtime-db/harness/load.mjs) | Mỗi batch đo sửa **ba `props.name`**, không phải ba lần sửa layout/text content; có continuous full-snapshot reader và SSE |
| [`TASK15-BULK-SQL-REPORT.md`](TASK15-BULK-SQL-REPORT.md) | Sau bulk, 5.000 nodes/1 writer: median POST 34,64 → 41,31 ms; p99 POST 68,93 → 90,04 ms. Mỗi cell chỉ 12 mẫu; không kết luận nhân quả hoặc SLA |

Probe giao thức cũ đo batch 1/3/500 changed IDs đều dùng sáu SQL executions sau bulk, gồm BEGIN/COMMIT. Giảm tiếp số SQL không tự loại được việc truyền/deserialize/clone/validate cả project.

EXPLAIN đã lưu cho fixture index riêng, **không phải profile của batch hoặc tải sản phẩm**: `snapshot` trả 5.000 rows, execution 3,25 ms; `nodePoint` trả một row, 0,033 ms. Hai con số không được trừ khỏi latency HTTP hoặc nhân ba để dự báo đường ghi mới.

Profile một writer mới đã có trên **baseline chưa tối ưu**, hai lượt độc lập: r1, r2. Mỗi cell100/5.000nodes có20warmup+100measured samples, continuous reader+SSE, PG16.9/2CPU/2GiB, gateway dev build trên host. Cùng source fingerprint `73dc2a1d588f452a212c5d7a1d0dbdd9d55c2c008f6563cdf53beb90735a97cc`; cleanup được lưu trong mỗi report.

| 5.000nodes, một writer | Lượt1 | Lượt2 |
| --- | ---: | ---: |
| Toàn thao tác p50/p95/p99 (ms) | 36,87 / 58,46 / 62,70 | 36,63 / 60,27 / 89,38 |
| Median server / nodeQuery / nodeDecode / apply (ms) | 35,323 / 9,834 / 8,578 / 10,253 | 35,007 / 9,307 / 8,811 / 10,354 |
| Median validate bên trong apply (ms) | 8,966 | 9,065 |
| Median persist / commit (ms) | 0,876 / 1,421 | 0,846 / 1,477 |
| Tổng elapsed của query+decode+apply / server elapsed | 82,87% | 81,38% |

Các tỷ trọng tính từ tổng thời gian100samples, không cộng median; core phases đã nằm trong apply. Query gồm PostgreSQL xử lý **và** wire/Tokio scheduling; không là SQL CPU riêng. HTTP/revision GET/client/reader/SSE nằm ngoài serverTotal. Với100nodes, commit chiếm37,7–38,6%, query+decode+apply chỉ26,7–27,2%: không khái quát kết luận project lớn sang mọi project.

Giả thuyết của người dùng được củng cố ở mức **thiết kế đường truy cập dữ liệu đang khuếch đại một edit nhỏ thành xử lý toàn project**. Profile không chỉ ra engine hoặc disk là nguyên nhân chính ở5.000nodes; phần read/materialize/apply mới là nơi đáng thử trước. Chưa đo phương án delta hoặc cache, nên các số trên không là mức cải thiện dự báo.

## Đối chiếu proposal người dùng gửi từ phiên khác

Nguồn: [proposal nguyên văn người dùng cung cấp](../tasks/sources/postgresql-schema-proposal.md), tiêu đề “Cấu trúc Database PostgreSQL Tối ưu cho OpenCanvas”, từ tệp đính kèm `Pasted text.txt` trong cùng phiên ngày2026-11-02. Đề xuất có hướng per-node hợp lý nhưng nhiều phần chính **đã có trong baseline spike**, chưa xử lý full-project validation.

| Phần proposal | Quyết định và chỉnh lại để áp dụng đúng |
| --- | --- |
| Một row/node, structural columns, JSONB props, adjacency | **Giữ**; hiện đã có trong `schema.sql`, không phải đổi từ một document canvas |
| `jsonb_to_recordset` ghi theo tập | **Giữ**; bulk hiện đã thực hiện, tổng batch thành công sáu SQL executions. Recordset chỉ là một statement ghi, không toàn transaction một round-trip |
| Global revision + row-level locks cho các node khác nhau ghi song song | **Sửa diễn giải**: project revision/lock vẫn tuần tự hóa writers cùng project. Node revision mới không thay contract `expectedRevision` toàn project; ưu tiên hiện tại là một writer |
| `parent_id REFERENCES nodes(id)` và UUID mặc định | **Không bê DDL**: FK đơn chỉ đảm bảo ID tồn tại, không ngăn parent thuộc project khác; giữ composite project/node FK deferred. API hiện dùng IDs ASCII do caller chọn, có thể tái dùng giữa projects; UUID-only sẽ cần mapping/migration contract, chưa có lợi ích đo |
| `ltree` + GiST | **Hoãn**: chưa có profile traversal chứng minh cần; không giải quyết full read/deserialize/validator. Phải duy trì parent/path nhất quán và chống cycle, rewrite path của descendants khi move, kiểm project scope của path query |
| GIN `props jsonb_path_ops` tạo mặc định | **Hoãn**: chưa có containment/jsonpath query cần nó trên hot path; proposal vừa khuyên chọn lọc vừa tạo index toàn props vô điều kiện. Opclass này không hỗ trợ mọi JSON operator; không mặc định tốt nhất |
| Unique key + `INSERT ON CONFLICT DO NOTHING` là idempotency hoàn chỉnh | **Sửa**: phải so canonical request hash, lưu exact original response, lookup replay trước expectedRevision và commit cùng mutation; unique key một mình không cho các semantics đó |
| `VALUES` làm PostgreSQL xử lý từng row/round-trip | **Sửa**: `UPDATE ... FROM (VALUES...)` cũng là một set-based statement; recordset có cách truyền tham số khác. Chỉ benchmark cùng workload mới so được parse/plan/encoding chi phí |
| Chia batch >5.000nodes | **Sửa theo contract**: tối đa500operations/2MiB, changed IDs có thể nhiều hơn do cascade. Không tách một request atomic thành nhiều commits để lấy latency tốt |
| Snapshot async sau mỗi batch | **Chỉ dùng làm derived read cache**, key đúng revision/provenance; không validation từ snapshot chậm hơn head, không bắt rebuild full snapshot đồng bộ trên write path |

Trong PG16, ltree có ancestor/descendant operators và GiST, **không có cam kết truy vấn “tức thời”**. Tài liệu16 hiện cho labels chứa alphanumeric/underscore/hyphen; IDs authored hiện tại phù hợp trong C locale và≤128bytes, nên không tuyên bố hyphen luôn cần encode. UUID-only không bắt buộc để dùng ltree; virtual ID có `/` không thể dùng nguyên làm label. Cần test grammar theo image16.9 thực, encoding nếu domain IDs mở rộng và invariant path/parent. GiST `path` của proposal cũng không tự có cột project chỉ vì index tên `project_path`. [PostgreSQL16: ltree](https://www.postgresql.org/docs/16/ltree.html).

Ordinary MVCC SELECT có thể đọc version nhất quán trong khi row đang UPDATE; row write lock không mặc định chặn reader thông thường. Không có bằng chứng chi phí JSONB tăng “theo cấp số nhân”: phải đo document bytes, WAL/index work và số lần update. `VALUES` có ví dụ UPDATE nhiều rows chính thức; không đồng nghĩa nhiều chuyến mạng. [PostgreSQL16: MVCC](https://www.postgresql.org/docs/16/mvcc-intro.html), [VALUES](https://www.postgresql.org/docs/16/sql-values.html). `jsonb_path_ops` hỗ trợ `@>`, `@?`, `@@`, khác `jsonb_ops` về key-existence; lựa chọn phải theo truy vấn cụ thể. [PostgreSQL16: JSONB indexing](https://www.postgresql.org/docs/16/datatype-json.html#JSON-INDEXING).

## Schema và dữ liệu được giữ/thêm

| Thành phần | Thiết kế đề xuất |
| --- | --- |
| Source of truth | Giữ `projects`, `nodes(project_id,node_id,parent_id,kind,sort_key,props JSONB)`, commits và idempotency. Khi port sản phẩm, giữ entity riêng cho tokens/styles/assets/fonts/notes, cùng khóa scope project |
| Tree | Giữ composite PK và deferred parent FK; adjacency là nguồn thật. Dùng truy vấn children/subtree khi operation cần, không materialize toàn cây ở mỗi update |
| Chứng nhận validation | Thêm metadata project: `validated_revision`, `validator_version`, `dependency_revision`, `dependency_version` (nullable, mặc định chưa valid). Chỉ stamp cùng transaction đã kiểm đúng candidate/inputs; không là cache nội dung |
| Phụ thuộc validation | Thêm bảng dẫn xuất `validation_inputs`, chỉ cho rule đã port và kiểm parity. Nội dung là input/predicate → check owner, gồm aspect và theme; không phải chỉ ID ref xuất hiện trong JSON |
| Snapshot | Vẫn đọc entities trong transaction snapshot nhất quán. Đường snapshot/scene độc lập đường mutation; không duy trì một JSONB toàn project đồng bộ theo mỗi write |

Hình dạng bảng phụ thuộc, để chốt contract trước khi viết DDL:

```text
validation_inputs(
  project_id,
  input_kind, input_id, input_aspect,
  check_kind, owner_kind, owner_id, theme
)
PK(project_id,input_kind,input_id,input_aspect,
   check_kind,owner_kind,owner_id,theme)
INDEX(project_id,check_kind,owner_kind,owner_id,theme)
```

PK hỗ trợ tìm checks bị ảnh hưởng; index ngược hỗ trợ thay toàn bộ inputs của check vừa chạy lại. `theme` có giá trị tường minh, kể cả `default`/`all`; không dựa vào NULL trong khóa. Bảng này có thể bỏ và xây lại; không thay authored model hoặc history. Không giả định FK đa hình giữa mọi entity loại khác nhau: extractor/validator kiểm target, bảng entity giữ FK thích hợp nơi biểu diễn được.

`input_aspect` tránh coi đổi `name` như đổi `width`. `input_kind` còn có predicate như `font_family`, `theme_catalog`, `flow_name`, `children_membership`, không chỉ `node`/`token`. Ví dụ thêm một font trùng face có thể làm lookup trước đó hợp lệ thành ambiguous dù font ID cũ không đổi. Chỉ lưu successful ID lookups sẽ bỏ sót trường hợp này.

Không cần GIN `props` cho workload ghi theo IDs; không tạo index trùng PK. Nếu port reorder/children thực tế cần thứ tự, đo rồi thay index parent bằng `(project_id,parent_id,sort_key,node_id)`, không tự giữ cả hai. B-tree nhiều cột thu hẹp theo điều kiện cột đầu; phải kiểm EXPLAIN sau ANALYZE với dữ liệu phù hợp. [PostgreSQL16: multicolumn indexes](https://www.postgresql.org/docs/16/indexes-multicolumn.html), [examining index usage](https://www.postgresql.org/docs/16/indexes-examine.html).

JSONB một node vẫn là đơn vị hợp lý cho props linh hoạt. Một JSONB toàn project buộc thay đổi nhỏ tác động cùng một row và giữ payload lớn; EAV từng property thêm join/index mà không loại logic full validation. PostgreSQL cũng khuyến nghị JSON document có cấu trúc ổn định và lưu ý update khóa cả row. Đây là lý do giữ per-node, chưa phải benchmark chứng minh mọi props JSONB tối ưu. [PostgreSQL16: JSON document design](https://www.postgresql.org/docs/16/datatype-json.html#JSON-DOC-DESIGN).

## Tính đúng của validation gia tăng

Gọi S là revision đã kiểm bằng đúng phiên bản validator, D là delta, A là tập predicates có input có thể đổi do D. Candidate S+D hợp lệ nếu: mọi predicate trong A được kiểm lại trên candidate cuối; mọi predicate ngoài A có toàn bộ input không đổi. Đây là yêu cầu chứng minh của từng rule, không phải suy luận rằng chỉ cần validate changed IDs.

Overlay giữ rows gốc đã tải và bản thay đổi/tombstone; lookup ưu tiên overlay rồi repository trong cùng transaction. Operations vẫn chạy **theo thứ tự**, giữ đúng kiểm tra ở từng thời điểm mà implementation gốc thực hiện, rồi kiểm candidate cuối. Cần phân biệt hai domain: spike chỉ kiểm properties/parent graph ở candidate cuối nên có trường hợp forward-parent hoặc property sai tạm rồi sửa lại được chấp nhận; production `Candidate::apply` còn gọi `node()` ngay sau một số operation. Không thêm full validation sau mỗi op cho spike, cũng không bỏ per-op guards của production bằng cách chỉ merge kết quả cuối. `changedIds` xuất phát từ delta kể cả reorder/cascade, không quét lại mọi node.

Chứng nhận candidate chỉ dùng được khi `validated_revision == projects.revision` và `validator_version` khớp binary hiện tại. Rule dùng dependency table còn bắt buộc `dependency_revision == projects.revision` và extractor version khớp; name-only đã chứng minh độc lập chỉ cần chứng nhận candidate. Import, restore, migration hoặc writer cũ chưa stamp ⇒ dùng full validator trước. Phải revalidate/backfill dưới project lock, không đặt mặc định “valid” cho dữ liệu cũ. Thay validator/rule/extractor ⇒ đổi version, vô hiệu đường nhanh tương ứng cho đến khi kiểm lại.

Full fallback chỉ chứng minh candidate valid, **không tự chứng minh bảng inputs còn đúng**. Transaction đó phải rebuild inputs đầy đủ cho registry rules đã bật, hoặc đặt `dependency_revision=NULL`; rows phụ cũ có thể giữ để rebuild nhưng không được dùng. Local edit không đổi tập dependencies đã chứng minh có thể advance cả hai certificates cùng revision. Không stamp một extractor version chỉ vì full validator đã pass.

Soundness đòi **mọi mutation được hỗ trợ đi qua cùng write boundary**, kể cả import asset/font, note, restore và gateway khác. Old gateway tăng revision mà không stamp khiến certificate lệch, đường nhanh tự tắt. SQL bên ngoài sửa entity nhưng không tăng revision/invalidate certificate có thể phá giả định: admin write phải vô hiệu certificate trước trong cùng transaction rồi revalidate, hoặc bị chặn bằng DB role/API quản trị. Certificate không phát hiện được superuser cố tình bypass; không được công bố hỗ trợ arbitrary SQL writes. Nếu cần hỗ trợ maintenance DML trực tiếp, thêm trigger invalidation cho mọi bảng mutation và kiểm lock ordering trước khi bật fast path.

### Lát cắt đầu có thể kiểm độc lập

- Request chỉ có `update_node`; patch có đúng key literal `name`; target là authored `text`, không virtual ID chứa `/`, không instance/component subtree. Nếu chưa biết target có thuộc component thì đọc ancestor chain tối đa depth128; không đoán từ props của target.
- Tất cả IDs được tải bằng một set query, ví dụ `WHERE project_id=$1 AND node_id=ANY($2::text[])`; map overlay theo ID. Ancestors được lấy theo tập bằng recursive query có guard cycle/depth; candidate đã certified phải nhất quán.
- Kiểm tồn tại/operation order, node kind/write guard và string bound đúng validator hiện hành: spike tối đa100.000 bytes ở final candidate, production `name` tối đa256 bytes với per-op guards hiện có. Không chia sẻ nhầm hai rule/thời điểm kiểm. Tái sử dụng rule nhỏ từ validator; không copy regex/limits sang SQL và Rust thành hai nguồn khác nhau.
- Name không đổi tree, refs, token/theme, font selection, style, layout hoặc expansion size. Scope hạn chế này giúp chứng minh predicates khác giữ nguyên; chưa cần `validation_inputs` để chạy nó.
- Bất kỳ operation/key/target/certificate không nằm trong phạm vi ⇒ chạy đường đầy đủ. Không âm thầm bỏ kiểm hoặc chấp nhận subset candidate như project đầy đủ. PERSIST nhận trực tiếp ba rows delta, không tạo/lọc/sort `State`5.000nodes.

Với ba text độc lập ở fixture shallow, số node tải/clone/kiểm để ghi tỷ lệ với ba targets và ancestor dùng chung, không N. Đây là mục tiêu cấu trúc phải đo/kiểm, **chưa là kết quả tốc độ**.

### Mở rộng cùng thiết kế cho domain thật

| Thay đổi | Phạm vi phải chứng minh trước khi mở đường gia tăng |
| --- | --- |
| Literal text content của text thường | Kiểu/giới hạn64KiB và required props không đổi. Hiện authored validator không đo glyph để kiểm hug; scene/layout sau commit vẫn phải invalidation text measurement và ancestors hug bị ảnh hưởng. Text trong master/override chưa mặc định local |
| Literal paint/opacity | Kiểm cả node có tương tác `animatedFill`, states, stroke/paint type; token/style refs thay đổi cần extractor đúng path. Không suy “màu” luôn độc lập, nhất là instance paint merging |
| Width/height/min/max/layout | Ancestor definite-width chain, parent hug/fill và wrap constraints của children/descendants, mọi theme. Cha chuyển fixed→hug có thể làm child fill sai; đổi width ancestor có thể làm row wrap descendant sai |
| Token/style | Alias graph gồm **hợp tất cả nhánh theme**, wrong type/missing/depth32, reverse users qua styles/tokens/paint/states/animations; theme catalogue mới/mất làm scope thay đổi. Tập ảnh hưởng có thể là toàn project |
| Scoped theme/reparent | Descendants kế thừa nearest theme, cùng layout ancestry; phải tính old và candidate dependency set. Reparent còn ảnh hưởng sibling sortKeys, depth/cycle, root frame và component membership |
| Component/master/override | Master→instance users, nested component DAG/depth8, expansion budget, override descendants, placed ancestry/theme. Cần kiểm virtual placements; authored changed IDs riêng không đủ |
| Font/resource, navigation, notes/delete | Font-family membership/ambiguity; asset media type; root-frame links/flow uniqueness; cascade/prune links và anchored notes. Cần predicate/set dependencies và changedIds gián tiếp |

Tập bị ảnh hưởng là closure của **old dependencies + candidate dependencies + structural/predicate rules**, chạy đến fixed point; thay inputs của checks đã chạy trong cùng commit. Không materialize transitive closure mọi cặp ancestor/descendant ngay từ đầu: adjacency và reverse direct inputs đủ để khảo sát closure khi cần, giảm fan-out maintenance khi reparent. Nếu rule chưa có extractor đầy đủ, closure vượt budget, thiếu version hoặc cần predicate toàn cục: full fallback là đường đúng bắt buộc. Không cắt closure rồi báo valid.

Đặc biệt, production [`validate/candidate.rs`](../crates/storage/src/validate/candidate.rs) còn kiểm notes/navigation/tokens, clone candidate theo từng theme rồi kiểm mọi style/node và expanded instances. Spike chỉ có page/frame/group/text và vài scalar props: không tokens/styles/fonts/instances/layout semantic tương đương sản phẩm. Hoàn tất delta trong spike chưa đủ để cắm adapter PostgreSQL vào `StorageBackend` và tuyên bố đạt production contract.

## Đường đọc/ghi và ranh giới transaction

1. Ngoài transaction: parse envelope, giới hạn bytes/ops và canonical hash; giữ fingerprint/replay semantics hiện có. Không dựa vào thứ tự key của JSONB.
2. `BEGIN READ COMMITTED`; `SELECT revision, validation metadata FROM projects ... FOR UPDATE`. Mọi sanctioned writer của project đều lấy khóa này **trước đọc/mutation entity**. Một writer vẫn giữ khóa để đúng khi retry qua process khác.
3. **Statement riêng sau lock** đọc idempotency: cùng hash trả đúng response cũ trước expectedRevision, khác hash trả collision. Không gộp vào snapshot SQL lúc bắt đầu chờ khóa; sau lock wait phải thấy replay writer trước đã commit. READ COMMITTED dùng snapshot theo statement; `FOR UPDATE` có thể đợi và trả row version mới. [PostgreSQL16: isolation](https://www.postgresql.org/docs/16/transaction-iso.html#XACT-READ-COMMITTED), [row locks](https://www.postgresql.org/docs/16/explicit-locking.html#LOCKING-ROWS).
4. Reject expectedRevision cũ trước node query. Với request mới hợp lệ, chọn delta scope hoặc full fallback theo certificate/rules. Tải entities/check inputs theo tập trong transaction; không publish hoặc ghi candidate vào cache dùng chung.
5. Áp tuần tự trong overlay; kiểm theo operation và candidate cuối. Dry run rollback, không certificate/history/key/event/resource mutation. Chứng nhận thiếu thì full validate candidate; dry run không backfill chứng nhận.
6. Ghi delta bằng set-based DML, update dependency rows đã thay, advance revision `R+1`, stamp certificate, thêm commit và idempotency response **trong cùng transaction**; transactional NOTIFY chỉ đánh thức. Có thể giữ writable CTE bulk hiện có nếu các tập upsert/delete rời nhau. Không dựa vào thứ tự sibling CTE hoặc sửa cùng row hai lần. [PostgreSQL16: data-modifying WITH](https://www.postgresql.org/docs/16/queries-with.html#QUERIES-WITH-MODIFYING).
7. `COMMIT` thành công mới trả success. Giữ `fsync=on`, `synchronous_commit=on`; không lấy asynchronous ACK để cải thiện latency. Local WAL durability không tự có failover/HA. [PostgreSQL16: WAL settings](https://www.postgresql.org/docs/16/runtime-config-wal.html).

Snapshot read giữ `REPEATABLE READ READ ONLY`: đọc revision + entities + retention trong cùng snapshot, không ghép revision cache với rows mới. Có thể vẫn tốn O(N) cho yêu cầu full snapshot; mục tiêu là không bắt writer làm việc này. Đường mutation không gọi render/measurement trong transaction.

Commit table vẫn là durable event log. NOTIFY chỉ giao sau commit và không phải replay store; SSE process khác dùng listener riêng + catch-up theo revision, reconnect/polling sửa gap như spike hiện có. Listener trễ không ảnh hưởng validation vì đường ghi đọc authoritative revision dưới khóa. Không cần cache RAM để phương án này đúng. [PostgreSQL16: NOTIFY](https://www.postgresql.org/docs/16/sql-notify.html).

## GET nên có cache nào trong cùng thiết kế?

**Đề xuất thử cache in-memory có giới hạn theo revision cho GET snapshot; chưa thêm Redis.** `db::state` hiện được gọi cả từ batch write và snapshot GET; hai đường có yêu cầu khác nhau. Cache giảm query/decode lặp của GET cùng revision, còn delta-first giảm dữ liệu phải xử lý khi ghi. Các phase profile ở trên thuộc **POST**, không phải breakdown CPU của GET; lợi ích read cache và giảm tranh CPU từ reader vẫn là giả thuyết cần đo.

| Lựa chọn | Đánh giá với một writer hiện tại |
| --- | --- |
| Local LRU, immutable `Arc<State>` và/hoặc `Arc<encoded response bytes>` | Bước đầu nhỏ nhất, không thêm network service; `Arc` tránh deep clone khi đọc. Có thể giữ encoded response của đúng endpoint để tránh serde lặp, không trộn với domain State |
| Redis shared revision cache | Có thể chia sẻ giữa gateways và giảm duplicate misses, nhưng thêm truyền payload mạng, encoding/decoding hoặc copy encoded bytes, memory/service vận hành và failure mode. Chưa có số hit rate/DB pressure để biện minh chi phí đó; chưa pin phiên bản Redis hoặc triển khai |

Đây là đánh giá kiến trúc, không phải benchmark Redis so với RAM/PostgreSQL. Redis có `maxmemory`/eviction và client tracking, nhưng tracking chỉ quản lý keys Redis, không tự biến PostgreSQL commit thành invalidation tin cậy. Dù cache nào, nguồn revision vẫn là PostgreSQL. [Redis: eviction](https://redis.io/docs/latest/develop/reference/eviction/), [client-side caching](https://redis.io/docs/latest/develop/reference/client-side-caching/).

Contract cache đề xuất:

1. Key domain snapshot gồm `(database namespace, immutable project generation, canonical projectId, revision, authored schema version)`. Encoded response thêm response format/version; resolved/scene/render cache thêm theme/state/viewport/pipeline/font/resource provenance như contract hiện có. Validation cache/check result thêm validator/dependency versions. Không dùng slug hoặc `projectId:latest` làm bằng chứng đúng revision.
2. GET “current”: đọc authoritative identity/revision/schema nhẹ từ DB primary trước. Nếu cache exact key hit, trả immutable snapshotR; điểm đọc revision là thời điểm tuyến tính của response, commit sau đó không làm snapshotR pha trộn. Đây là consistent current-at-read, không hứa không ai commit trong lúc HTTP đang gửi.
3. Miss: đọc snapshot bằng `REPEATABLE READ READ ONLY` như hiện tại; snapshot có thể là revisionS mới hơn revisionR vừa probe. Chỉ populate/return dưới key **S thực đọc cùng identity/schema**, không gắn payloadS vào keyR. Không gồm full read snapshot và HTTP serialize dưới project writer lock. Gộp concurrent misses theo key để tránh stampede, không giữ cache mutex khi chờ SQL.
4. Candidate dùng cho write, nếu cache được dùng như tối ưu bổ sung: vẫn lấy project lock và replay trước, rồi chỉ đọc State cache đúng revision+identity+validator version. Miss quay về DB trong transaction; stale entry không được dùng để validate. Không mutate shared Arc; không deep clone cả State để làm “delta” rồi gọi đó là O(touched). Thiết kế được chọn không cần write cache để đạt tính đúng hoặc tránh full read của local edit.
5. Chỉ populate candidateState/check result cho revision mới sau COMMIT đã xác nhận. Commit lỗi/không rõ outcome: không publish speculative candidate. Có thể để miss tự fill lại từ DB; **không rebuild toàn snapshot đồng bộ sau mỗi batch** chỉ để làm nóng cache. NOTIFY/TTL dùng dọn/prefetch, không là correctness boundary; mất/reorder notification vẫn an toàn do exact authoritative revision probe.
6. Đặt cả byte budget, entry cap và giới hạn retained revisions/project; skip object lớn hơn budget, không cache unbounded job/asset bytes. Các cap là cấu hình được đo RSS, không giả định entry count đủ giới hạn RAM. Nếu vừa giữ State vừa encoded bytes phải hạch toán cả hai; chú ý Arc đang được response/job giữ sau eviction. Restart/eviction/miss/cache lỗi quay DB; không serve stale khi DB lỗi theo TTL mặc định.
7. Hai gateways có thể có local entries khác nhau nhưng cùng key revision không đổi. Delete/recreate/restore cùng project ID cần generation mới trong DB (hoặc cam kết ID không tái dùng trong namespace); source data root/DB đổi phải đổi namespace. Khi identity chưa chứng minh ổn định, tắt cache. Key này không thay authorize/scope check của từng request; không chia private/public endpoint response khác quyền. Assets/fonts tách riêng theo immutable content hash và access checks, không nhét binary vào node cache.

GET trả full JSON vẫn phải truyền O(N) bytes; cache State riêng còn serialize O(N), encoded bytes chỉ bỏ phần encode lặp. Mỗi write đổi revision nên read/write ratio thấp hoặc readers hay đến revision chưa cache sẽ có hit rate thấp; reader vòng lặp liên tục của harness không đại diện mọi usage. Mục tiêu đọc “snapshot revision cũ bất kỳ” vẫn chịu retention contract hiện có; giữ cache entry không tự mở API history mới. Jobs phải pin immutable input/provenance đúng revision và lifetime, không giữ transaction chỉ để đợi render.

AC cache cần tách hiệu quả đọc và tác động gián tiếp lên writer: cold/hot/evicted/miss, same-revision multi-GET, một GET/mỗi commit và burst10GET/commit, reader thực tải scene; đo hit/miss/stale-rejected/eviction/load-coalescing, rows/bytes DB, serialize/send time, RSS và latency. Kiểm commit xen giữa revision probe và fill, commit failure, lost notification, gateway restart, DB outage, project identity replacement và scope/public response separation. Nếu local cache không có hit rate hữu ích, không triển khai Redis để che nguyên nhân đó.

### Lỗi và rollback

| Tình huống | Hành vi bắt buộc |
| --- | --- |
| Operation cuối sai, ref sai hoặc predicate ngoài target bị ảnh hưởng | Không entities/revision/history/key/certificate/dependencies/event được commit; error/op attribution giữ contract |
| FK deferred hoặc lỗi SQL tại COMMIT | Rollback cả transaction; không để chứng nhận “valid” hoặc notification của batch lỗi. FK không tự chặn tree cycle/layout/theme; không thay validator bằng CHECK gọi query bảng khác. [PG16 constraints](https://www.postgresql.org/docs/16/ddl-constraints.html) |
| Commit đã durable nhưng mất response | Client gửi cùng key/body qua gateway khác nhận response cũ; không áp delta lần hai |
| Connection mất lúc commit, chưa biết kết quả | Không báo success hoặc khẳng định rollback chắc chắn; giữ error envelope theo contract. Retry cùng key/body là cách phân giải; không tạo key mới rồi ghi lại |
| Process khác sửa project trước request | Project lock + đọc replay mới + expectedRevision quyết định; không dựa vào cached candidate/revision |
| Crash/restart | Authoritative entities/log/certificate cùng commit; RAM overlay bỏ được. Check metadata version sau restart, fallback nếu không khớp |

## Triển khai module, migration và rollback

Public contract `StorageBackend::apply_batch`/dry run/snapshot không đổi. Chia phần nội bộ: `mutation_plan` phân loại footprint; `candidate_overlay` giữ delta/ordered semantics; `validation_scope` tìm checks/inputs; validator rule nhỏ dùng lại; `delta_repository` set reads/writes; `snapshot_cache` chứa revision-keyed reads; `batch` chỉ orchestration/transaction. Không nhét PostgreSQL vào validator thuần domain; không đưa driver/transaction qua trait storage công khai. Mỗi module mục tiêu100–200 dòng; đánh giá tách khi >250, tránh >350.

Lộ trình là các lát cắt kiểm độc lập của **cùng thiết kế**:

1. Baseline một writer đã đóng băng/profile. Tách validator scalar không đổi semantics; thử local revision cache GET riêng để tách lợi ích reader, rồi overlay/name-only delta trong spike sau khi được duyệt. Certificate chưa có/không khớp luôn full fallback; cache miss không là lỗi dữ liệu.
2. Migration additive metadata nullable và `validation_inputs`; cấp version riêng cho spike/production. Backfill một project/transaction dưới khóa bằng full validator, đối chiếu inputs và revision rồi stamp. Không đổi ID/props/history/hash hoặc cắt dữ liệu; không chạy trên project thật trong bước research.
3. Port một rule class/lần (text thường, paint, layout, refs, instances), chạy shadow full validation/differential trên dữ liệu test trước khi cho nó quyết định commit. Chưa có dependency table parity thì không bật incremental cho class đó.
4. Khi port adapter production, mọi entry point/resource mutations phải tham gia certificate/dependency invalidation; giữ schema backup/import/restore và full domain contract. Benchmark spike không là cutover approval.

Rollback bằng feature flag về full reader/validator/bulk persistence, vẫn cùng tables/history/idempotency. Old binary bỏ qua bảng/cột phụ; revision nó tăng làm certificate cũ lệch. Khi bật lại binary mới, thiếu/mismatch version bắt full revalidation. Không xóa metadata/index khi còn code đọc; schema destructive cleanup là việc sau. Nếu phát hiện extractor sai, đổi dependency version ngay và tắt class tương ứng trước khi rebuild.

## AC và phép đo để quyết định triển khai

Không cần matrix nhiều writer ở vòng này. Một writer thật, hai gateway cho replay/cross-process correctness; DB mới riêng cho mỗi biến thể, cùng PostgreSQL16.9 pinned image/resources2CPU/2GiB và cùng build profile/client. Báo version16 vì spike đang pin16.9, không gọi đây là phiên bản PostgreSQL mới nhất.

- Baseline và proposed chạy tuần tự, tối thiểu warmup20 + measured100 samples cho100/5.000nodes; lặp lượt để xem độ ổn định. Reader full snapshot + SSE giữ như workload cũ và thêm writer-only control để nhận diện chi phí reader. Báo p50/p95/p99 POST và end-to-end, số mẫu/retries/read throughput, không chỉ best run.
- Profile hash/pool/begin/lock/replay/query/decode/clone/execute/validate/sort/persist/commit riêng; ghi queried rows/bytes, touched/validated counts, SQL executions, build/source fingerprint và cleanup. Không đồng nhất thời gian await query với DB execution: nó có network/scheduling; EXPLAIN là bằng chứng riêng.
- AC cấu trúc name-only: không full-project SELECT/sort/clone/validation trên certified path; ba targets shallow cho row counts bị chặn bởi targets+ancestors, độc lập số siblings/project. Truy vấn theo tập không tăng SQL theo changed IDs. Fallback, replay và conflict có counters để kiểm thực sự đi nhánh nào.
- AC semantics: old/new candidate snapshots, changedIds, errors/op order và dry run tương đương cho1/3/500ops, repeated IDs, late-invalid rồi sửa lại, missing target, bulk mixed CRUD/reparent/delete, replay sau later revisions/key collision, deferred COMMIT failure, dropped response, cross-process replay, consistent reader/SSE và crash/restart. Giữ contract19ca cũ, thêm targeted regression cho certificate/version/fallback.
- AC domain sản phẩm: token alias cycle ở unused theme, removed scoped theme, parent fixed→hug với child fill, ancestor wrap width, nested master/override/theme, font-family ambiguity, delete link/note cascade. Mỗi class phải bị từ chối/full fallback đúng trước khi bật gia tăng; không chỉ kiểm ba changed nodes vẫn “valid”.
- Workload thực đại diện ngoài đổi tên: sửa text tiếng Việt trong fixed và hug containers; literal fill/gradient+states/animation; width/padding/wrap; token/theme fan-out; master text/instance override; create/delete/reparent một subtree. Báo **class nào nhanh hơn, class nào còn full fallback**, số entities ảnh hưởng và thời gian scene readiness tách khỏi durable batch ACK.
- AC hiệu năng đề xuất để chốt trước đo: median và p95 POST name-only5.000nodes cải thiện so baseline qua các lượt đồng điều kiện, không regression correctness; báo nguyên p99 vì100samples còn ít cho tail. Nếu không đạt, giữ raw evidence và profile nguyên nhân tiếp; không đặt mục tiêu ms hoặc phần trăm từ suy đoán. Global fan-out vẫn có thể O(N); không hứa mọi edit O(3).

Các ngưỡng latency cụ thể, mức fan-out dự kiến của project thật, cache hit rate, chi phí duy trì dependency rows/WAL, độ phủ classifier và full production validator parity **chưa đo/chưa chốt**. Kết quả research đủ chọn một hướng triển khai thử: giảm dữ liệu đi qua transaction bằng delta có chứng minh, giữ source tables/fallback và local immutable cache cho GET khi có reuse; chưa đủ để hứa tốc độ hoặc thay runtime.

## Nguồn và giới hạn kiểm chứng

Tài liệu kỹ thuật chính thức PostgreSQL16/Redis đã mở ngày2026-11-02, gắn trực tiếp tại claim tương ứng. Nghiên cứu khi đó dùng working tree dirty và raw Task15 đã đối chiếu; researcher chỉ đọc và đối chiếu reports, không tự chạy database/benchmark. Hai baseline profiles được worker benchmark thực hiện trong cùng phiên, không phải kết quả phương án đề xuất. Nghiên cứu này không sửa schema/code và không có số đo hiệu năng delta/cache.
