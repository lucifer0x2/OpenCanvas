# P24 — Component + instance tối thiểu

Ngày kiểm: 2026-10-29. Nhánh `codex/p24-components-instances`, nền `a0d7743`.
Triển khai storage, validation, compute boundary và MCP reads; không thêm tool,
không đổi layout engine/renderer. Chưa push. Mọi fixture dùng data root tạm và
server cổng tự chọn; không ghi dữ liệu demo vào project thật hoặc cổng 3847.

## Hành vi và ranh giới module

- Master là frame có `component:{name}`; instance là authored leaf với `componentId`,
  sizing/position/name/visibility và override phẳng theo ID con master.
- `storage::instances::expand` là một điểm mở rộng trước resolve/layout; ID ảo ổn định
  `<instanceId>/<masterChildId>`. Snapshot, commits và bundle giữ nguyên references.
- Validator tách graph/cycle/depth khỏi override/write/delete guards. Override chỉ nhận
  `text`, `fill`, `visible`, `styleId`, `assetId`, `name`, đúng kind và mọi theme.
  Không có layout/structure override; component subtree không có links/flow/animations.
- Context reference giữ một định nghĩa cho mỗi master phụ thuộc, gồm phụ thuộc lồng nhau.
  Compact resources đóng trên cả định nghĩa master lẫn overrides, kể cả base style đã
  bị thay thế ở mọi instance. Expand mode, inspect và export dùng cùng virtual tree.
- Schema DB tăng **3 → 4**: schema 3 đã dùng cho notes trên main. Không đổi cột SQL;
  migration version-only từ 3. Project/bundle schema 1–3 vẫn mở được; bundle chỉ được
  migrate trong private staging sau khi exact legacy schema đã qua kiểm tra.

## Nghiệm thu theo AC

| AC | Bằng chứng / kết quả |
| --- | --- |
| 1. 20 instance, byte ops ≤30% | `instances_mcp_ops_pixels_master_invalidation_and_virtual_reads`: 100 → 25 ops; JSON ops **19.951 → 4.964 B**, còn **24,88%**. Fixture dùng cú pháp API hiện hành và cùng dữ liệu ở hai cách author. |
| 2. PNG tương đương | Cùng test so sánh toàn bộ bytes PNG của frame 20 instance với frame 100 node thủ công: bằng nhau. Bounds 20 cell query cũng bằng nhau. |
| 3. Một lần sửa master | Một `update_node` đổi padding ngang 12 → 24: revision 1 → 2, sceneKey đổi, PNG đổi; cả 20 cell query tăng x đúng 12. Inspect virtual child trả bounds đúng scene. |
| 4. Override và lỗi whitelist | Storage tests giữ text/fill instance khác và master nguyên vẹn; MCP lỗi có `props.overrides.cmpRow-q.layout` và `allowedKeys`. Resource regression kiểm text style, fill token hai theme, visibility, image/name và missing asset rollback. |
| 5. Cycle / delete guard | Direct/indirect cycle trả `INSTANCE_CYCLE`; 8 levels hợp lệ, 9 bị `INSTANCE_DEPTH`. Delete master trả `COMPONENT_IN_USE`; fixture 98 instance trả count 98 nhưng chỉ 20 IDs. Failed batches giữ nguyên snapshot/revision. |
| 6. Context reference / expand | Mặc định 1 master trong `components`, 21 scoped nodes (frame + 20 instance), expand 101 nodes. **8.694 → 33.386 B**, reference còn **26,04%**, giảm 73,96%. Computed child bounds/lines theo virtual ID khớp expand; definitions chỉ trên trang đầu. |
| 7. Find | `kinds:["instance"]` trả 20; `textContains:"t3"` trả `row3/cmpRow-q`. Outline/subtree có componentId, childCount 4; đọc trực tiếp virtual child được. Export instance và virtual child đều ready. |
| 8. Migration / bundle | Project schema 2/3 mở lại giữ nodes/revision. Bundle schema 1/2/3 import + restore giữ authored nodes, revision, idempotency replay. Schema-4 component bundle round trip giữ componentId/overrides, không persist ID ảo. Malicious legacy schema objects/version mismatch bị reject trước activation; migration lỗi rollback. |
| 9. Catalog / kích thước | Live `{tools}` JSON **24.871 → 24.966 B**, **+95 B <300 B**; vẫn 16 tool. Tất cả file Rust mới ≤250 dòng; expander 120 dòng. |

## Lệnh và kết quả

Các lệnh Cargo dùng `CARGO_TARGET_DIR=/private/tmp/opencanvas-p24-target`, `-j 2`.
Server/render tests dùng `PLAYWRIGHT_BROWSERS_PATH` trỏ runtime Chromium đã cài,
không thay đổi browser/runtime đang phục vụ người dùng. `rtk bypass` giữ output thô.

```sh
cargo test -p opencanvas-storage --test instances --test instances_boundaries \
  --test instances_legacy --test migration --test bundle_database \
  --test bundle_roundtrip --test bundle_archive -j 2 -- --test-threads=2
cargo test -p opencanvas-storage --test instances_legacy -j 2 -- --test-threads=1
cargo test -p opencanvas-storage --test instances_boundaries -j 2 -- --test-threads=1
cargo test -p opencanvas-server --test instances_resources --test instances_mcp \
  -j 2 -- --test-threads=1 --nocapture
cargo test -p opencanvas-server --test context --test compact_context --test find_nodes \
  --test export_nodes --test export_svg --test skill_layer --test tool_schemas \
  -j 2 -- --test-threads=2
cargo test -p opencanvas-server --lib -j 2 -- --test-threads=2
cargo clippy -p opencanvas-storage -p opencanvas-server --lib -j 2 -- -D warnings
cargo build -p opencanvas-server --bin opencanvas-server -j 2
cargo fmt --all -- --check
node --check spikes/p24/context-benchmark.mjs
git diff --check
wc -l -c AGENTS.md
```

- Storage 7 suites ban đầu: 20/20; sau thêm malicious legacy và expansion limit,
  rerun `instances_legacy` 2/2 và `instances_boundaries` 3/3. Tổng hiện tại của 7
  suites là 22 tests; không ghi nhận một lần chạy lại chung cả 22.
- MCP instance 3/3 + resource closure 1/1. Phiên chính chạy độc lập lại 4/4 sau fix.
- Related context/find/export/skill/schema regressions: 19/19.
- Server unit tests/golden: 35/35. Fixture `Computed` cũ cần thêm map `lines` rỗng;
  golden output không thay đổi.
- Strict library Clippy, final server build, Rust fmt, JS syntax và diff check: đạt.
- AGENTS.md: 64 dòng / 7.861 B, dưới ngân sách 120 dòng / 10 KB.

## Đo latency và cách tái lập

Raw samples: [before](../artifacts/p24/perf-before.json),
[after](../artifacts/p24/perf-after.json). Phiên chính đo độc lập baseline main và
binary P24 trên cùng máy, server tạm, 5 warmup + 30 samples. Fixture 6.900 authored
nodes, **không có instance**; chỉ trang đầu `compact:true,limit:200,budget:65536`.

| Số đo | Trước | Sau | Thay đổi |
| --- | ---: | ---: | ---: |
| p95 | 354,685 ms | 293,630 ms | −17,2%; đạt budget ≤+10% |
| median | 254,379 ms | 262,041 ms | +3,01% |
| tools/list bytes | 24.871 | 24.966 | +95 |
| tool count | 16 | 16 | 0 |

Workload này kiểm regression khi không có instance; chênh lệch timing có nhiễu,
không dùng nó để suy ra throughput của expansion.

Đo thêm workload lặp thực tế: baseline `repeated` có 1.150 row × (frame + 5 rectangle),
6.902 authored nodes; P24 `instances` có 1 master 6 node + 1.150 instance, tổng
1.158 authored nodes. Hai frame đều có **6.901 expanded nodes** gồm root frame,
cùng geometry/props. Vẫn đo trang đầu compact reference, 5 warmup + 30 samples.
Raw samples: [manual before](../artifacts/p24/perf-repeated-before.json),
[instances after](../artifacts/p24/perf-repeated-after.json).

| Số đo với expansion | Manual baseline | P24 instances | Thay đổi |
| --- | ---: | ---: | ---: |
| p95 | 275,931 ms | 191,564 ms | −30,58%; đạt budget ≤+10% |
| median | 249,658 ms | 182,931 ms | −26,73% |

Số đo này bao gồm validation/expansion và tạo context reference, không đo render hoặc
toàn bộ các trang của expanded context. Phiên chính đã kiểm cùng binary cuối.

Script [context-benchmark.mjs](../spikes/p24/context-benchmark.mjs) nhận binary,
workspace, output path và mode tùy chọn (`rectangles` mặc định, `repeated`, `instances`),
tự tạo/dọn data root và process:

```sh
node spikes/p24/context-benchmark.mjs \
  /private/tmp/opencanvas-p24-target/debug/opencanvas-server \
  "$PWD" /private/tmp/p24-context-repeat.json
node spikes/p24/context-benchmark.mjs \
  /private/tmp/opencanvas-p24-target/debug/opencanvas-server \
  "$PWD" /private/tmp/p24-instances-repeat.json instances
```

Để so baseline, build binary ở revision `a0d7743`, cung cấp workspace/runtime tương
ứng rồi chạy cùng script: mode `rectangles` so với `rectangles`, hoặc `repeated` ở
baseline so với `instances` ở P24. Không chạy vào data root thật.

## Kích thước và giới hạn

File mới lớn nhất: `storage/tests/instances.rs` 213 dòng; validator instance 210;
MCP tests 190; boundary tests 176; expander 120. Mọi file Rust được sửa vẫn ≤250
dòng. `docs/SERVER.md` 556 dòng là reference tổng hợp đã có sẵn; thêm 21 dòng hợp
đồng liên quan, không trộn implementation responsibility vào module lớn.

Reference definitions nằm trên trang đầu, vẫn bị response budget kiểm chặt;
definition quá lớn trả `CONTEXT_ITEM_TOO_LARGE`, có thể chọn expand để phân trang.
Virtual visibility dùng opacity 0 và giữ layout space. Không có variant, structure
override, tự extract component, downgrade/hot replacement, hay propagation của
links/animations. Không chạy full workspace suite hoặc thay đổi layout engine.
