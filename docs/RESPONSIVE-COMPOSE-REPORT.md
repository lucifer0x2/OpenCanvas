# Responsive trên nền Compose — 2026-11-04

## Tích hợp mã

Tích hợp responsive opt-in từ worktree đã kiểm sang working tree
`codex/compose-postgres` (base `f8bfc0f`). Hợp đồng tại
[CONTRACT.md](CONTRACT.md#responsive-layout), yêu cầu tại
[PLAN.md](../PLAN.md#responsive-opt-in--kích-thước-tỷ-lệ-và-breakpoint).

- Child `width`/`height` nhận literal decimal `0%`–`100%` trên trục cha definite;
  min/max vẫn tính theo pixel. Root được render theo viewport, authored size giữ nguyên.
- Container có tối đa tám full layout alternatives, `maxWidth` là integer tăng
  nghiêm ngặt trong 1–100000. Chọn ngưỡng inclusive đầu tiên theo viewport frame;
  ngoài ngưỡng dùng base, không lồng breakpoint. Mọi alternate được validate trong
  mọi theme, kể cả alternate không active.
- Storage dùng parser chung từ layout crate; refs padding/gap trong alternate
  resolve qua resolver hiện có. MCP catalog/capabilities/skills công bố cùng quy tắc.
- Compute, HTTP scene, inspect và PNG dùng chung revision/viewport/theme/scene key.
  Không thêm migration hay sửa authored revision khi chỉ đổi viewport.

Hai thiếu sót tìm thấy và sửa trong lượt hoàn thiện:

1. Nền Compose thiếu `Candidate::ancestors` mà validator responsive gọi. Thêm
   helper bounded 128 ancestor trong `crates/storage/src/validate/node.rs`, cùng
   trách nhiệm kiểm graph/node hiện có. Trước sửa storage compile lỗi E0599;
   sau sửa compile/test/Clippy đạt.
2. Test negative cũ trong `crates/storage/tests/references.rs` coi `"50%"` là
   unsupported. Đổi thành `"100.001%"` để tiếp tục kiểm giá trị ngoài miền được
   phép; không đổi production behavior để giữ kỳ vọng cũ.

Source responsive nằm trong `crates/layout/src/{responsive,model/layout,tree,
validation/layout,util}.rs`, storage `validate/{layout,layout_rule,breakpoints}.rs`,
catalog và server `reads/{tree,diagnostics}.rs`, capabilities/instructions/skills.
Test mới: layout `responsive_tests.rs`, storage/server `tests/responsive_layout.rs`.

## Bằng chứng kiểm cục bộ

Mọi lệnh dưới đây chạy thật trên working tree tích hợp. Dữ liệu test dùng root
tạm; không ghi dữ liệu người dùng hoặc `~/.opencanvas`.

| Lệnh (bỏ tiền tố `rtk bypass` cho dễ đọc) | Kết quả |
| --- | --- |
| `cargo test --offline -p opencanvas-layout --lib -- --test-threads=2` | 34/34 đạt; responsive 5, cùng fixed/fill/hug/stroke/wrap/alignment regressions |
| `cargo test --offline --locked -p opencanvas-storage --test responsive_layout --test stack_layout --test wrap -- --test-threads=2` | 10/10 đạt; all-theme alternates, placement instances, rollback/retry và wrap guards |
| `cargo test --offline --locked -p opencanvas-storage --test resolution --test instances --test references --test prop_catalog -- --test-threads=2` | 19/19 đạt sau sửa stale negative fixture; refs/themes/instances/catalog không hồi quy |
| `cargo test --offline --locked -p opencanvas-server --test responsive_layout --test wrap --test multi_viewport --test skill_layer -- --test-threads=1` | 7/7 đạt; MCP resources/batch, context, inclusive boundary 600/601, inspect, HTTP scene, PNG 390×400 và giữ revision |
| `cargo fmt --all -- --check` | Đạt; chạy lại sau sửa test cuối |
| `cargo clippy --offline --locked -p opencanvas-layout -p opencanvas-storage -p opencanvas-server --lib --bins -- -D warnings` | Đạt production libraries/binaries |
| `git diff --check` | Đạt; chạy lại sau sửa test cuối |

Integration test server cần bind loopback. Lần chạy trong sandbox dừng với
`PermissionDenied` tại listener; chạy lại với escalation đã được duyệt đạt toàn
bộ 7 test. Các warning dead-code ở shared test support là baseline; production
Clippy strict không có warning.

`--locked` đã chấp nhận Cargo.lock khi đưa `opencanvas-layout` từ dev-dependency
sang production dependency của storage. Backend và layout-step phải build từ
cùng source trong image; mixed-version/hot replacement không thuộc hợp đồng.

## Kiểm độc lập của root

Root báo kết quả thực trên cùng working tree:

- `cargo build --offline --locked -p opencanvas-layout --bin layout-step`: đạt.
- `node --test --test-concurrency=1 spikes/pipeline/responsive.test.mjs spikes/pipeline/geometry.test.mjs spikes/pipeline/constraints.test.mjs spikes/pipeline/stack-align.test.mjs`:
  10/10 đạt (1,607 giây), sau escalation cho loopback. Lần sandbox ban đầu EPERM
  ở before hook là giới hạn môi trường.

Golden traversal thiếu `effectiveTheme` đã được ghi nhận fail cả trên `main`
sạch trong lượt responsive trước. Lượt tích hợp này không chạy lại golden đó,
không dùng baseline lịch sử làm bằng chứng test hiện tại đạt.

## Module và giới hạn

Mọi Rust source/test được thay đổi ≤223 dòng (lớn nhất `server/src/reads/tree.rs`).
Module mới: parser responsive 36, layout test 155, breakpoint validator 50,
storage test 191, server test 214 dòng. Helper node 138, references test 177 dòng.
`server/skills/author-frame.md` có 300 dòng: tài liệu hướng dẫn, giữ một authoring
flow liền mạch; không phải module mã viết tay. Không có Rust file chạm >250 dòng.

Không mở CSS/Grid đầy đủ, column-wrap, nested breakpoints, cycle phần trăm hoặc
mixed-version runtime. Không chạy full suite/build lại không liên quan. Worker
tích hợp không commit/push, không thay runtime/Docker/dữ liệu thật. Phần rollout
và parity dữ liệu thực do worker vận hành bổ sung sau khi image được build.

## Canary PostgreSQL và trạng thái rollout

**Canary đạt; không triển khai app thật.** Người dùng chốt: “App thật tôi sẽ tự
cài, không tự deploy vào app thật”. Bàn giao image đã kiểm để người dùng tự cài;
agent không retry rollout, không sửa env hoặc đụng runtime thật/Funnel.

Trước chỉ đạo này, automatic approval review đã từ chối thao tác thay live app
vì đây là thay đổi dịch vụ dùng chung và ngữ cảnh duyệt chưa có lời cho phép
rollout production đủ rõ của người dùng. Lệnh bị chặn trước thực thi. Không thử
đường vòng; việc chốt bàn giao tuân theo yêu cầu người dùng sau đó.

Tiếp quản đúng stack tạm `opencanvas-responsive-7f7b5d96` tại loopback
3957/3958, PostgreSQL riêng phục hồi từ dump nguồn. Image cũ
`opencanvas:project-privacy-local` tái hiện HTTP 400 `VALIDATION_FAILED` cho
Training revision 137 (`fd-components`, width node
`cmp-course-card-done-p-f`) và Animation revision 10 (`responsive-lab-frame`,
width node `responsive-lab-cards`). Không đổi authored data để né lỗi.

Build image riêng bằng:

```sh
docker --context orbstack build -t opencanvas:responsive-compose-20261104-7f7b5d96 .
```

Build đạt, Dockerfile biên dịch và bundle cùng source cho server release và
`layout-step` dev. Tag mới có digest
`sha256:21186b1b084e2ec18f4a46685c954038a4110ca4071adb4ccea21da6129a1ad0`.
Chỉ thay app canary bằng `compose up -d --no-deps --wait ... opencanvas`, giữ
PostgreSQL canary. Image rollback privacy cũ vẫn còn.

Canary dùng HTTP và MCP thật trên PostgreSQL:

- `get_design_context` với hai viewport 1200/390 trả computed ready, revision
  10/137 và reflow; `render_frame`, `inspect_layout`, HTTP scene dùng cùng
  sceneKey. PNG có đúng signature, MIME và kích thước (lab 1200×740/390×740;
  Training 1200×900/390×900).
- Lab desktop có hai card 544 px cạnh nhau; mobile có hai card 350 px xếp dọc.
  Progress track desktop 502 px và mobile 308 px, hai fill giữ tỷ lệ 62%/38%
  trong sai số subpixel. Root kiểm độc lập thêm 721/720/390, xác nhận ngưỡng
  inclusive 720, sceneKey riêng và progress 62% trong sai số 0,001.
- Root đi Chromium viewer, đổi Frame size sang Mobile 390×844, xem screenshot
  mobile và PNG: không pageerror, HTTP ≥400 hoặc request mutation.
- Public Host đúng: viewer 200/readOnly, list 200 rỗng do cả bốn project private,
  private snapshot 404; MCP và tạo project 403, Host sai 403. Auth status vẫn
  `required:false`, ba tool quản trị auth vẫn được công bố; không đổi password,
  privacy hoặc gọi công cụ thu hồi phiên. Node 24 fetch không giữ Host tùy chỉnh
  trong probe này; kiểm guard dùng `node:http.request` với Host tường minh.

Fingerprint snapshot toàn bộ, so byte-for-byte nguồn/canary và kiểm nguồn lại
ngay trước lệnh rollout bị chặn:

| Project | Revision | SHA-256 snapshot |
| --- | --- | --- |
| Animation Creator | 10 | `446d91bee0e05b6e3f1085e9f7d3cdf9516b6b8c16729efef9e707cf199b92d3` |
| AXIS Courses | 60 | `73cc272f28406ce53f7cd592b0681314cdc2ce60149c96256c57241b215b4cc9` |
| Axis Training | 137 | `2986dd4703c51af5e53634f309eaf509362da50da1a68ce421a928cf0ef2eaa9` |
| AXIS Workspace — Login | 18 | `ac41029210130a2178baae1194b6c2adcc66fdb2c02bf7d95b07a1b6ee3e9a45` |

84 binary resource thuộc ba project có asset đều khớp contentHash/byteSize
nguồn và canary. Snapshot đã bao gồm font metadata. Counts và one-way hashes
của auth rows được ghi riêng trước rollout; không in giá trị credential. Capture
Funnel config giữ private; chưa có thay đổi Funnel/proxy/port.

Bằng chứng tạm nằm ở `/private/tmp/opencanvas-responsive.7f7b5d96/`:
`baseline-error.json`, `build.log`, `source-fingerprint.before.json`,
`source-resources.before.json`, `immediate-pre-rollout.json`,
`canary-verification.json`, `canary-*-context.json`, `canary-*-scene.json`,
`canary-*.png`; root có `root-canary.json`, `root-viewer.json`,
`root-lab-{721,720,390}.png` và `root-viewer-*.png`. Không version các capture,
dump, env, auth fingerprint hoặc Funnel config.

Điểm dừng: app thật vẫn image `opencanvas:project-privacy-local`, cả app và
PostgreSQL healthy, identity app/PG và volume khớp trước lệnh bị chặn. Hai
container canary và volume riêng vẫn giữ healthy để tiếp tục; dump và tag cũ
giữ cho rollback. Không tự dọn canary; chỉ dọn khi người dùng yêu cầu. Không có
kiểm hậu rollout thật vì app thật chưa được thay image. Không commit/push.
