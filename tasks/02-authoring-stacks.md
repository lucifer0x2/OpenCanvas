# Task 02 — Author design bằng stacks thay vì free

**Trạng thái:** hoàn tất và kiểm độc lập 2026-10-28; project thật đã tạo qua MCP sau khi người dùng đồng ý.
**Phụ thuộc:** nên làm sau Task 01 (skill `author-frame.md` là nơi quy tắc được ghi), nhưng có
thể chạy song song nếu người làm Task 02 chỉ sửa `samples/` và không chạm `crates/server`.
**Phạm vi sở hữu:** `samples/welcome/`, `artifacts/sample/`, thư mục mới cho design mẫu thứ hai.
Không sửa `crates/`, không sửa `web/`.
**Workspace:** `/Users/luis/Workspace/OpenCanvas`, branch `main`. Không reset, không checkout.

## Bối cảnh

Đây là gốc rễ của vấn đề, không phải tầng MCP hay tầng export.

Frame mẫu `welcome-dashboard` (project `p_dfafcf1451e543a5b1fcedb6ad441a81`, revision 1, 1040×720)
được author như sau — đếm trên toàn bộ 71 node:

| Thuộc tính | Thực tế |
| --- | --- |
| `layout.mode` | `"free"` ×8, 63 node còn lại không khai báo (mặc định `free`) |
| `width` | 100% là số px cố định |
| `height` | `"hug"` cho 41 node text, số px cho phần còn lại |
| `row` / `column` / `fill` / `minWidth` / `maxWidth` | **không node nào dùng** |

`PLAN.md` đã công bố layout v1 gồm `row`/`column`, padding, gap, fixed/fill/hug và min/max, và
định nghĩa "Responsive v1 chỉ là agent chọn viewport hữu hạn và bố cục trong phạm vi row/column
đã công bố". Năng lực đó đã nằm sẵn trong `crates/layout` và được validate
(`crates/layout/src/validation.rs:30` cho sizing, `:84` cho mode). Design mẫu chỉ là chưa dùng tới.

## Bằng chứng đã có

Chạy trực tiếp trên layout engine, cùng một bức hình, hai cách author, hai viewport:

```
A-free @ 1040×720            A-free @ 390×844
  side  x=0    w=176           side  x=0    w=176     ← không đổi 1 pixel
  c1    x=208  w=248           c1    x=208  w=248
  c3    x=744  w=248           c3    x=744  w=248     ← nằm ngoài khung, bị clip

B-stacks @ 1040×720          B-stacks @ 390×844
  side  w=176                  side  w=176            ← fixed, giữ nguyên
  body  w=864                  body  w=464            ← fill, co lại
  c1    w=253.3                c1    w=120            ← chạm minWidth
```

Cùng engine, không sửa một dòng Rust nào. Lý do A không reflow: `crates/layout/src/tree.rs:60-64`
ép root frame bằng viewport, còn node con dưới parent `free` nhận `Position::Absolute` với
`inset` là `x`/`y` authored.

Script tái tạo (cần `cargo build -p opencanvas-layout --bin layout-step` trước; chỉ dùng
rectangle nên không cần measure cache):

```js
import { spawn } from 'node:child_process';
const base = { schemaVersion: 1, projectId: 'probe', revision: 1, frameId: 'f', theme: 'default', state: 'default', fonts: [], assets: [] };
const free = [
  { id: 'f', parentId: null, kind: 'frame', props: { name: 'f', width: 1040, height: 720, clip: true, layout: { mode: 'free' }, fill: '#F5F6FA' } },
  { id: 'side', parentId: 'f', kind: 'group', props: { name: 'side', x: 0, y: 0, width: 176, height: 720, fill: '#fff' } },
  { id: 'c1', parentId: 'f', kind: 'rectangle', props: { name: 'c1', x: 208, y: 148, width: 248, height: 112, fill: '#fff' } },
  { id: 'c2', parentId: 'f', kind: 'rectangle', props: { name: 'c2', x: 476, y: 148, width: 248, height: 112, fill: '#fff' } },
  { id: 'c3', parentId: 'f', kind: 'rectangle', props: { name: 'c3', x: 744, y: 148, width: 248, height: 112, fill: '#fff' } },
];
const stacks = [
  { id: 'f', parentId: null, kind: 'frame', props: { name: 'f', width: 1040, height: 720, clip: true, layout: { mode: 'row', padding: 0, gap: 0 }, fill: '#F5F6FA' } },
  { id: 'side', parentId: 'f', kind: 'group', props: { name: 'side', width: 176, height: 'fill', minWidth: 72, fill: '#fff' } },
  { id: 'body', parentId: 'f', kind: 'group', props: { name: 'body', width: 'fill', height: 'fill', layout: { mode: 'column', padding: 32, gap: 28 }, fill: '#F5F6FA' } },
  { id: 'cards', parentId: 'body', kind: 'group', props: { name: 'cards', width: 'fill', height: 112, layout: { mode: 'row', padding: 0, gap: 20 }, fill: 'transparent' } },
  { id: 'c1', parentId: 'cards', kind: 'rectangle', props: { name: 'c1', width: 'fill', height: 'fill', minWidth: 120, fill: '#fff' } },
  { id: 'c2', parentId: 'cards', kind: 'rectangle', props: { name: 'c2', width: 'fill', height: 'fill', minWidth: 120, fill: '#fff' } },
  { id: 'c3', parentId: 'cards', kind: 'rectangle', props: { name: 'c3', width: 'fill', height: 'fill', minWidth: 120, fill: '#fff' } },
];
const viewports = [{ width: 1040, height: 720 }, { width: 390, height: 844 }];
const reqs = [];
for (const [label, nodes] of [['A-free', free], ['B-stacks', stacks]])
  for (const vp of viewports) reqs.push({ label, vp, line: JSON.stringify({ input: { ...base, viewport: vp, nodes } }) });
const proc = spawn('target/debug/layout-step');
let buf = ''; const out = [];
proc.stdout.on('data', d => {
  buf += d; let i;
  while ((i = buf.indexOf('\n')) >= 0) { out.push(buf.slice(0, i)); buf = buf.slice(i + 1); }
  if (out.length === reqs.length) {
    proc.stdin.end();
    out.forEach((line, k) => {
      const r = JSON.parse(line); const { label, vp } = reqs[k];
      console.log(`\n-- ${label} @ ${vp.width}x${vp.height} -- ${r.status}`);
      if (r.status === 'ready') for (const n of r.scene.nodes)
        console.log(`   ${n.id.padEnd(6)} x=${n.bounds.x} y=${n.bounds.y} w=${n.bounds.width} h=${n.bounds.height}`);
    });
  }
});
for (const r of reqs) proc.stdin.write(r.line + '\n');
```

## Mục tiêu

Có một design mẫu **thật** chứng minh rằng pipeline hiện tại đã đủ cho responsive v1, để
Task 01 và Task 05 có dữ liệu để kiểm, và để agent đọc thấy cấu trúc chứ không thấy toạ độ.

## Việc phải làm

1. Viết script author một frame mẫu mới qua MCP thật (như `samples/welcome/` đã làm), dùng
   `row`/`column` + `fill`/`hug` + `minWidth`/`maxWidth` cho toàn bộ bố cục. `free` chỉ dùng ở
   chỗ thật sự là overlay hoặc trang trí, và mỗi lần dùng phải ghi lý do trong comment.
2. Giữ nguyên bức hình: ở 1040×720 frame mới phải trông như frame cũ. Đây là ràng buộc chính —
   mục tiêu là cùng thiết kế, khác cách khai báo.
3. Render và so sánh ở ít nhất hai viewport, lưu PNG làm bằng chứng.
4. Ghi lại những chỗ **không** diễn đạt được bằng schema hiện tại. Đây là đầu vào cho Task 04,
   quan trọng không kém phần làm được.

## Acceptance Criteria

- Frame mẫu mới tồn tại, author qua MCP thật, không ghi thẳng DB hay filesystem.
- Ở 1040×720, chênh lệch so với frame cũ nằm trong ngưỡng đã ghi rõ trong báo cáo; mọi chênh
  lệch còn lại được giải thích.
- Ở viewport hẹp hơn, bounds **có thay đổi** và thay đổi đó hợp lý (sidebar giữ, nội dung co,
  chạm min thì dừng).
- Script tái tạo được, ≤250 dòng, nằm trong `samples/`.
- Danh sách giới hạn gặp phải khi author bằng stacks, kèm ví dụ cụ thể.

## Cách kiểm

- Chạy script trên data root tạm trước, chỉ tạo project thật khi người dùng đồng ý.
- `render_frame` + `inspect_layout` ở cả hai viewport, lưu `artifacts/`.
- Xem ảnh thật, không kết luận từ số liệu.

## Đầu ra

- Script author + README ngắn trong `samples/`.
- PNG hai viewport trong `artifacts/`.
- Báo cáo: bounds hai viewport, chênh lệch so với frame cũ, và danh sách giới hạn schema.

## Ngoài phạm vi

- Không sửa `crates/layout` để lách giới hạn. Gặp giới hạn thì ghi lại, chuyển Task 04.
- Không xoá frame mẫu cũ — người dùng đang giữ để xem.

## Kết quả triển khai — 2026-10-28

- Script mới tại `samples/welcome-stacks/`, chia authoring, section builders và
  verification thành 8 module (24–86 dòng/file); giữ nguyên `samples/welcome/`.
- Root chạy độc lập `node samples/welcome-stacks/verify.mjs` ngoài sandbox: pass,
  MCP thật trên backend/data root tạm, kiểm 3 viewport, render/inspect/context,
  `success=true`, `cleanup=true`. Không ghi DB trực tiếp; không restart3847.
- Ngưỡng chốt trước đo: ≤0,5% pixel RGBA khác và mean absolute channel error
  ≤0,25/255. Desktop1040×720 thực tế **0/748800 pixel khác, MAE0**; baseline
  tái tạo cùng runtime cũng0 channel sai khác so với artifact mẫu cũ.
- Bounds1040/800/390: sidebar176/176/176, body864/624/480, mỗi card248/168/120.
  Root đã xem PNG thật: nội dung co ở800;390 chạm min, clip và có text overlap.
  Tổng min656px, không gọi kết quả390 là mobile layout hoàn chỉnh.
- Có182 node toàn project,181 trong frame; chỉ2 nhóm free có comment lý do:
  logo-overlay và chart-plot. Header/legend của chart dùng stacks; context đọc đủ.
- Giới hạn/schema examples, bảng bounds, cách tái tạo ở
  `samples/welcome-stacks/README.md`; bằng chứng ở `artifacts/sample-stacks/`.
  Còn thiếu wrap/breakpoints, per-edge padding/alignment, weighted/percentage fill;
  chart plot giữ tọa độ cố định, progress cap không biểu diễn80% tại mọi width,
  text wrap có thể vượt chiều cao desktop cố định. Không sửa engine để lách.
- Worker `node --check` cả8 module pass; root `git diff --check` pass. Mọi source
  trong scope dưới250 dòng; không có file lớn cần ngoại lệ.
- Người dùng đã xác nhận **Đồng ý tạo project thật** sau khi xem bằng chứng.
  Worker mới `publish_stacks_sample` thực hiện thao tác này; không tái sử dụng
  worker đã hoàn tất, không sửa/xóa project cũ.
- Preflight3847 từ chối kết nối vì không có server; worker đầu lỗi capacity trước
  mutation. Fresh `publish_stacks_live` start mới server khi port/process trống,
  rồi tạo project `p_ae68646852a94fddbb9837277b7c88ef`, slug
  `opencanvas-welcome-stacks`, revision1, frame `welcome-dashboard-stacks`.
  Không restart/kill server đang chạy. Instance mới giữ chạy để người dùng xem.
- Live render/inspect1040 và800 pass; desktop so với QA tạm0 pixel khác. Viewer
  so PNG0/2995200 channels khác, chỉ GET API, không page error. Evidence tại
  `artifacts/sample-stacks/live/verification.json`; root đọc lại project/context
  qua connected MCP và xem viewer screenshot thật. Context stacks diagnostic[];
  project cũ vẫn revision1. Root yêu cầu mở viewer `http://127.0.0.1:3847` trong
  Codex panel (tool trả queued). Không commit/push.
