# Task14 — Instance schemes và Structured Operations

Ngày 2026-11-02. Task được lưu trước triển khai; ba fresh workers làm ba lane,
root điều phối, review diff thô và kiểm độc lập. Checkout giữ nguyên
`claude/design-properties`, không commit/push hoặc bỏ thay đổi frontend có sẵn.

## Kết quả

P1–P5 đã triển khai trong source và bản build debug đồng bộ:

- Scoped frame/instance themes resolve styles/tokens/aliases/paint/states/fonts
  theo nearest placed scope; context/scene/inspect cho biết effective theme.
- Descendant stroke object merge, shadow array replace, states merge theo state
  và thuộc tính; instance root nhận fill/stroke/shadow/radius/opacity. Structure
  overrides và state shadow vẫn bị từ chối. Props/capabilities fingerprint đổi.
- `apply_batch_file` nhận pure JSON trong project inbox, tối đa500 operations/
  2 MiB; chặn traversal/symlink/special files và dùng cùng atomic transaction,
  revision và canonical content idempotency với inline calls.
- `dryRun:true` chạy full candidate validation và kiểm key collision, không ghi
  revision/commit/idempotency/assets, phát events hoặc gọi render worker.
- Connect/author guide ưu tiên MCP structured JSON, có fewshots đúng API,
  revision/conflict/retry và reuse master/token/theme. Đây là client guidance;
  không kiểm soát system prompt/external programs của agent. Backend không đoán
  payload có nguồn từ JS và không từ chối operations hợp lệ chỉ vì lặp.

Chi tiết implementation/worker evidence:
[schemes](INSTANCE-SCHEMES-REPORT.md), [batch](STRUCTURED-BATCH-REPORT.md),
[authoring](STRUCTURED-AUTHORING-REPORT.md), [bulk API](BULK-OPERATIONS.md).

## Root kiểm độc lập

Mọi runtime tests dùng temporary data roots/loopback random ports. Các test server
được chạy bằng escalation cho phép bind loopback; không đụng listener/dữ liệu thật.

| Kiểm | Kết quả cuối |
| --- | --- |
| Storage lib +13 integration suites bên dưới | 48/48 pass |
| Layout lib | 29/29 pass |
| Server11 integration suites bên dưới | 25/25 pass |
| `npm --prefix web test` (runner có preview-schemes) | 26/26 pass |
| `npm run check:renderer` | Pass |
| `npm run build -- --offline` | Debug server/layout/renderer/web đồng bộ pass |
| `node web/tests/instance-schemes-smoke.mjs` | Real browser preview pass |
| `cargo fmt --all --check`, `rtk bypass git diff --check` | Pass |

Các lệnh Rust (mọi lệnh dùng `rtk bypass`, `--offline`):

```sh
cargo test -p opencanvas-storage --offline --lib --test resolution --test instances --test instances_boundaries --test instances_legacy --test paint_props --test paint_props_reuse --test states --test font_face --test transactions --test batch_file --test instance_schemes --test instance_scheme_fonts --test bundle_roundtrip -- --test-threads=2
cargo test -p opencanvas-layout --offline --lib -- --test-threads=2
cargo test -p opencanvas-server --offline --test structured_batch --test structured_authoring --test instance_schemes --test instances_mcp --test instances_states --test instances_resources --test compact_context --test inspect_paging --test multi_viewport --test props_reference --test tools_budget -- --test-threads=1
```

Root đã xem shared paint merge, borrowed scoped resolver, compact resource closure,
request pipeline và descriptor-based inbox guards. Lần storage đầu bắt một test
cũ còn reject shadow override; worker đổi thành positive array-replacement và giữ
invalid paint/rollback checks, sau đó root chạy lại toàn bộ nhóm48 tests thành công.
Root cũng yêu cầu dry run kiểm read-only key collision và bỏ effectiveTheme lặp
trong context resolvedProps; cả hai đã bổ sung trước nghiệm thu cuối.

Real browser smoke dùng production code/debug build: hai instance cùng master
paint night/day, hover chỉ có trong override hoạt động, scoped motion dùng màu
night và duration750ms. Server tests kiểm PNG samples, SVG fill, context/states,
inspect, paging/compact/multi-viewport, cache và hồi quy instance cũ. Batch tests
kiểm unchanged DB/WAL, events/prerender có đối chứng thật, restart, atomicity,
file/inline replay và malformed inputs. Chỉ kiểm các vùng liên quan, không chạy
full workspace/pipeline suites. Worker targeted Clippy xanh; broad all-tests
Clippy còn unused shared-support warnings cũ ở integration targets không đổi.

Root kiểm sizes: runtime source lớn nhất220 dòng. Ngoại lệ đã đánh giá:
`crates/server/tests/inspect_paging.rs`251 dòng giữ chung oracle và hai kịch bản
pagination; author skill Markdown272 dòng giữ một resource/checklist/fewshots.
Không mã viết tay sửa/tạo vượt350 dòng. Build còn cảnh báo chunk JS >500kB cũ.
Catalog đo bởi lane B:17 tools/26,183B, trong budget32KiB; không claim giảm
transport bytes từ một baseline chưa đo được.

## Còn mở và cách dùng

U07 worker cần rebuild thật bằng master tokens + themed instances rồi so A1–A4
golden fuzz≤3% và đo payload. Chưa thực hiện việc này hoặc chứng minh dự đoán
~4KB/scheme. Số liệu653KB/3,140ops →224KB/792ops trong proposal là lợi ích reuse
master/instances; không quy thành lợi ích đổi JS transport sang MCP. P6 optional
contrast check chưa triển khai. SVG giữ diagnostics thiếu stroke/shadow như trước.

Debug artifacts sẵn, không build release hoặc tự restart dịch vụ thật. Server đang
chạy phải được người dùng nâng/restart và client tải lại catalog/instructions để
nhận API/hướng dẫn mới. Không thêm dữ liệu demo hoặc sửa U07 project thật.
