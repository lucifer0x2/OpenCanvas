# Task 14 — Instance colour schemes và Structured Operations

**Trạng thái:** Đã lưu trước triển khai, 2026-11-02; người dùng cho phép triển khai.
**Kết quả:** P1–P5 đã triển khai và kiểm độc lập; xem `docs/TASK14-REPORT.md`.
U07 rebuild/golden/payload còn chờ design worker; P6 chưa triển khai.
**Nguồn:** bản đề xuất U07 ở `tasks/sources/u07-instance-proposal.md`, cùng yêu cầu
ưu tiên Structured Operations của người dùng ngày 2026-11-02.
**Workspace/branch:** `/Users/luis/Workspace/OpenCanvas`, `claude/design-properties`.
Giữ mọi thay đổi frontend/AGENTS/LOG có sẵn; không checkout/reset/stash, commit/push,
restart server thật hoặc ghi vào `~/.opencanvas`. Tests dùng temp root/cổng ngẫu nhiên.

## Mục tiêu và quyết định phạm vi

Thực hiện P1–P5 của proposal, P6 contrast check để backlog (optional trong nguồn).
Agent author/edit design ưu tiên tool MCP với JSON operations; không viết/chạy JS,
shell client hay HTTP client để thay thế các mutation tools đã có. Đây là hướng dẫn
client, không phải khả năng backend kiểm soát mọi chương trình trên máy agent.
Không cấm việc triển khai app/code hay test fixtures bằng JS; không suy đoán nguồn
gốc batch từ số lượng/lặp operations và không chặn batch hợp lệ theo heuristic đó.

Số liệu 652.8 KB/3,140 ops → 224.4 KB/792 ops do tái sử dụng master/instances;
không quy giảm này cho việc đổi JS transport sang MCP. Mục tiêu ~4 KB/instance
là dự đoán cần đo lại, chưa phải kết quả. Không thay đổi design U07 thật ở task này:
proposal giao việc rebuild/re-measure cho U07 design worker sau khi API có sẵn.

## Lane A — Themes và instance paint (P1–P3)

Phạm vi: storage instances/validation/resolution/catalog; layout scene theme;
server reads/jobs/inspect, props guide/capabilities và tests liên quan.

- `theme` string trên frame/instance; phải tồn tại trong ít nhất một token,
  sai tên trả `UNKNOWN_THEME`. Nearest scoped ancestor wins; render/export theme
  là global fallback; default giữ tương thích. Scope của instance phải dùng nơi
  instance được đặt; explicit theme của instance thắng master root theme.
- Resolve fill/stroke/shadow/states/styles/token aliases đúng theme mỗi node;
  context expanded/resolvedStates, render PNG/SVG/preview và inspect thống nhất.
  Scene và inspect cho biết effective theme. Cache dựa vào resolved input như cũ.
- Overrides thêm stroke (merge object), shadow (replace array), states (merge theo
  state và merge props bên trong state để giữ thuộc tính không được ghi đè),
  opacity/radius. Validate đúng target kind và full merged value; lỗi có đường dẫn
  `overrides.<id>...` rõ (giữ prefix `props.` theo convention repo nếu phù hợp).
  Giữ layout/sizing/children/component không override. Không mở interaction shadow.
- Instance nhận fill/stroke/shadow/radius/opacity áp lên expanded master root;
  stroke merge để giữ width/align khi chỉ đổi color. Authored source không đổi.
- Test nested scopes, nested instances, named/default themes, invalid atomic rollback,
  master inheritance, stroke width2, giữ focus/pressed khi patch hover, shadow replace,
  style/font refs, context/inspect/render đúng. Props/capabilities fingerprint cập nhật.

## Lane B — Bulk JSON và dry run (P4–P5)

Phạm vi: storage batch/transaction/path helper và tests; server schemas/domain/MCP,
tool docs và tests. Không sửa instance/resolution hoặc authored operation enum.

- `apply_batch_file {projectId, expectedRevision, idempotencyKey, path, dryRun?}`:
  đọc pure JSON operation array chỉ trong `<data-root>/<project-slug>/inbox/`.
  Reject traversal/symlink escape/ngoài inbox bằng lỗi rõ; giới hạn bytes trước đọc,
  schema nghiêm ngặt, tối đa500 operations. Không eval/JS hoặc auto-chunk commits:
  caller chia nhiều batch, mỗi batch atomic và có revision/key riêng.
- Chia sẻ pipeline với apply_batch: validation, atomicity, conflict, idempotency;
  hash dựa trên nội dung canonical operations (không chỉ path). Retry cùng key/body
  replay, file thay đổi không được replay nhầm. Không công bố arbitrary local paths.
- `apply_batch dryRun:true` và file dryRun chạy cùng full candidate validation,
  trả dryRun/baseRevision/would-be changedIds/diagnostics; không revision/commit/log/
  idempotency/SSE/prerender/asset side effect. Dry run không tiêu thụ commit key.
  Giữ DTO/public Rust BatchRequest cũ nếu có thể bằng request wrapper riêng.
- Tests success/conflict/idempotency/body mismatch, negative path/symlinks/size/JSON/
  op count, late invalid rollback, dryRun equivalence và no writes/events/restart.

## Lane C — MCP authoring workflow

Phạm vi: `crates/server/src/instructions.rs`, `crates/server/skills/author-frame.md`,
resource workflow docs/tests mới và tài liệu hướng dẫn riêng. Phối hợp A/B khi
thêm props/tool capabilities; không sửa schema/domain/capability cùng lúc với họ.

- Connect instructions và author skill nói rõ MCP-first structured JSON; đọc
  tools/list/schema, props/capabilities theo fingerprint, reuse tokens/styles/master/
  instances trước khi nhân cây; đọc revision mới nhất nếu chưa có read/commit hiện tại.
- Few-shot đúng schema thực: `update_node {id,props:{text,fontSize}}`, không bịa
  `update_text`, `nodeId` hay `props.content`. Ví dụ create master/instance/theme,
  token update, batching ≤500, dry run, bulk file fallback thuần JSON khi có quyền FS.
- Lost response: retry y hệt key/body. Conflict: đọc changedSince/re-read phần liên
  quan, dựng lại với revision mới/key mới; không blind retry hay read lại vô ích sau
  mỗi commit thành công. File tool vẫn là MCP có cùng structured ops, không cần JS.
- Kiểm các few-shot qua schema/runtime MCP thật trên data tạm; xác nhận lặp operations
  hợp lệ không bị từ chối. Hướng dẫn không được hứa cưỡng chế system prompt của client.

## Cách kiểm và đầu ra

Mỗi lane là một fresh subagent, không tạo agent con, không tái sử dụng sau hoàn tất.
Mỗi worker khảo sát module boundaries trước sửa; dùng apply_patch, rg, explicit RTK;
mục tiêu100–200 dòng/file, đánh giá tách >250, tránh >350; báo size exceptions.
Chạy targeted storage/server/layout tests, fmt, Clippy/typecheck liên quan; không
full suite/build vô cớ. Lane A cần real render smoke với worker/data tạm.
Root review diff thô, kiểm độc lập seams chính và tích hợp docs/fingerprint.

Đầu ra: code/tests/docs; reports riêng `docs/INSTANCE-SCHEMES-REPORT.md`,
`docs/STRUCTURED-BATCH-REPORT.md`, `docs/STRUCTURED-AUTHORING-REPORT.md` với lệnh thật,
kết quả, giới hạn và file sizes. Root cập nhật LOG khi giao/nhận/dừng.
U07 golden AC (fuzz≤3%, only text/styleId overrides) ghi rõ chờ design worker
rebuild trên API mới; không tuyên bố đạt từ fixture tổng hợp.
