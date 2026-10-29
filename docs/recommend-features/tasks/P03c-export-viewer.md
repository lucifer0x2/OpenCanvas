# P03c — Export trên viewer + HTTP enqueue
Ưu tiên: P1 · Kích cỡ: ~80 phút dev · Phụ thuộc: P03a (bắt buộc), P03b (nên: thêm nút SVG) · Nguồn: U5

## Vấn đề

U5 yêu cầu export dùng được qua MCP, HTTP **và viewer**. Sau P03a mới có MCP `export_nodes` và route tải file `GET /api/v1/jobs/{job}/files/{i}`. Người dùng xem design trên viewer chưa có nút export, chưa thấy `exportSettings` của node dưới dạng dễ đọc, và client HTTP thuần (curl, script, viewer) chưa có cách tạo job export.

## Bằng chứng

- Viewer cố ý chỉ dùng GET: comment ở `web/src/api.ts:13` ("Design changes belong to the agent's MCP client"). E2E kiểm mọi request là GET (`spikes/e2e/browser.mjs:143-149`). Endpoint export phải là GET, giống tiền lệ `GET /api/v1/projects/{project}/scene` vốn cũng enqueue job (`crates/server/src/http.rs:42,87-102`).
- Inspector hiện chỉ in authored props dạng JSON (`web/src/components/Inspector.tsx:41-49`). `exportSettings` sẽ hiện thành một khối JSON thô. Inspector nhận `node` và `scene` từ `App.tsx:181`, chưa nhận `projectId` (có ở `selection.projectId`, `App.tsx:72`).
- `http.rs` đang có 179 dòng và thành khoảng 205 sau P03a. Thêm route nữa thì nên tách module (theo Modular rules).
- `X-Content-Type-Options: nosniff` áp cho mọi response (`http.rs:162-165`). Route file của P03a có `Content-Disposition: attachment`, nên trình duyệt tải về ngay cả khi URL tuyệt đối khác origin (`http://127.0.0.1` so với `http://localhost`).
- Test viewer dùng server mock (`web/tests/fixture.mjs`, `web/tests/smoke.mjs`) và Playwright, nên thêm mock route để test download được.
- Figma: Export panel nằm trong thanh phải, mỗi export setting là một dòng (scale · suffix · format) có nút "Export <name>". Penpot làm tương tự (https://help.penpot.app/user-guide/exporting/).

## Giải pháp

### HTTP enqueue (server)

```
GET /api/v1/projects/{project}/nodes/{node}/export?format=png&scale=2&suffix=@2x&theme=default&waitMs=10000
```

- Query là **một** setting tuỳ chọn: `format`, `scale` | `width` | `height`, `suffix`, `svgIdAttribute`. Không có query setting thì dùng `exportSettings` của node, nếu không có thì PNG 1x (giống MCP khi không truyền `settings`).
- `theme` tuỳ chọn. `waitMs` 0–10000, **mặc định 0**: route GET không chặn, giống `/api/v1/jobs/:id` (`docs/SERVER.md:50`). Viewer truyền 10000.
- Response:
  ```json
  {"projectId","revision","export":{"nodeId","jobId","status","files"|"error"}}
  ```
  `export` cùng shape một item của `export_nodes`. Xử lý dùng lại `export::enqueue_node` của P03a, không sinh logic thứ hai.
- Lỗi theo bảng mã của P03a: `NODE_NOT_FOUND` 404; `EXPORT_UNSUPPORTED` 400; `QUEUE_FULL` 503 (`error.rs:54-63`).
- Tách route export và file sang `crates/server/src/http/export.rs`, để `http.rs` quay về khoảng 180 dòng.

### Viewer

- `web/src/components/ExportSection.tsx` (mới): đặt trong Inspector khi node đang chọn không phải page.
  - Node có `exportSettings`: mỗi setting một dòng, ví dụ `PNG · 2x · Box@2x.png` hoặc `SVG · Box.svg`, kèm nút **Export**; thêm nút **Export all** khi có ≥ 2 dòng.
  - Không có: các nút preset `PNG 1x`, `PNG 2x`, `PNG 3x`, và `SVG` nếu P03b đã merge. Tooltip "Add exportSettings via MCP to save presets", vì viewer chỉ đọc.
  - Trạng thái: `Exporting…` (disable nút), lỗi in `code: message`, xong thì tải.
- `web/src/hooks/useNodeExport.ts` (mới):
  1. Gọi `read(projectPath(p) + "/nodes/{id}/export?waitMs=10000&…")` (`web/src/api.ts:14`).
  2. Nếu vẫn `pending`, gọi lại cùng URL (job đã được cache), tối đa 3 lần.
  3. Với mỗi file, tạo `<a href={relative(url)} download={fileName}>` rồi `click()`. `relative()` bỏ origin để giữ same-origin.
- `Inspector.tsx`: thêm prop `projectId` và một dòng `<ExportSection …/>`. Ẩn `exportSettings` khỏi danh sách JSON thô vì đã có section riêng (+3 dòng). `App.tsx:181` truyền `selection.projectId`.
- CSS: thêm vào `web/src/workspace.css` (229 dòng → ≤ 250) hoặc file mới `web/src/export.css` nếu vượt.

## Phạm vi file

| File | Tạo/sửa | Dòng ước tính |
| --- | --- | --- |
| `crates/server/src/http/export.rs` | tạo: route enqueue + chuyển route `files/{i}` của P03a sang đây | ~90 |
| `crates/server/src/http.rs` | sửa: `mod export;` + `.merge(export::routes())` | −20/+3 |
| `crates/server/src/export/mod.rs` | sửa: tách `enqueue_node(state, project, node, settings, theme)` công khai trong crate, nếu P03a chưa có | +10 |
| `crates/server/tests/export_http.rs` | tạo | ~90 |
| `web/src/components/ExportSection.tsx` | tạo | ~120 |
| `web/src/hooks/useNodeExport.ts` | tạo | ~70 |
| `web/src/types.ts` | sửa: `ExportSetting`, `ExportItem` | +15 |
| `web/src/components/Inspector.tsx`, `web/src/App.tsx` | sửa | +5 |
| `web/src/workspace.css` hoặc `web/src/export.css` | style section | ~30 |
| `web/tests/fixture.mjs` / `fixture-data.mjs`, `web/tests/smoke.mjs` | mock route export/files + kịch bản download | +60 |
| `docs/VIEWER.md`, `docs/SERVER.md`, `docs/CONTRACT.md` | mục Export viewer + endpoint | ~20 |

## Tương thích MCP

- Không đổi tool nào. HTTP chỉ thêm route GET. Route `files/{i}` giữ nguyên URL, chỉ chuyển module.
- Viewer vẫn chỉ GET; không sửa được `exportSettings` (sửa qua MCP `update_node`). Giữ nguyên nguyên tắc read-only của `docs/VIEWER.md`.

## AC

1. `GET …/nodes/box/export?scale=2&waitMs=10000` (server tạm, Chromium thật) → 200, `export.status:"ready"`, `files[0].width = 2×bounds`, `files[0].url` tải được.
2. Không query setting, node có 2 `exportSettings` → `export.files.length == 2`.
3. Mặc định `waitMs=0` → response trả trong < 200 ms với `status:"pending"` hoặc `"ready"`. Gọi lại cùng URL → cùng `jobId`.
4. Page id → 400 `EXPORT_UNSUPPORTED`. Id không tồn tại → 404 `NODE_NOT_FOUND`. `scale=9` → 400 `INVALID_REQUEST` với message nêu miền 0.01–4.
5. Viewer smoke (`npm --prefix web run test:browser`):
   - chọn node có 2 setting → thấy 2 dòng có đúng `fileName`;
   - bấm Export dòng 2 → Playwright bắt `download` với `suggestedFilename()` = `fileName` và kích thước file = `bytes`.
6. Node không có `exportSettings` → có đủ preset `PNG 1x/2x/3x` (+ `SVG` nếu P03b đã merge). Bấm `PNG 2x` → file `<name>@2x.png`.
7. Lỗi mock `QUEUE_FULL` → section hiện text chứa `QUEUE_FULL`, nút bấm lại được.
8. Mọi request viewer trong kịch bản là GET: dùng `assertReadOnly` của e2e hoặc kiểm tương đương trong smoke.
9. `npm --prefix web run typecheck` pass. `ExportSection.tsx` ≤ 150 dòng, `useNodeExport.ts` ≤ 100, `Inspector.tsx` ≤ 110, `http.rs` ≤ 200. Báo `wc -l`.

## Cách kiểm

- `CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-server --test export_http` (AC1–4).
- `npm --prefix web run typecheck && npm --prefix web run test:browser` (AC5–8).
- Live:
  1. server tạm `--port 0 --data-root $(mktemp -d)`, `npm --prefix web run build`;
  2. mở viewer bằng trình duyệt có sẵn (Claude Browser pane) tại cổng tạm, chọn node, bấm Export;
  3. kiểm file tải;
  4. dọn.

  Không dùng 3847.

## Rủi ro

- **Xung đột UI:** phiên "OpenCanvas UI redesign" (ngoài đội) có thể đang sửa `web/src`. Giữ thay đổi chủ yếu trong file mới; Inspector và App chỉ thêm vài dòng. Dev orchestrator kiểm trạng thái phiên đó trước khi merge (không nhắn tin cho nó).
- **Trình duyệt chặn nhiều download liên tiếp** khi bấm "Export all" (Chromium hỏi quyền). Test chỉ dùng download đơn. "Export all" tải tuần tự, cách nhau 200 ms.
- **GET có tác dụng phụ** (tạo job): theo tiền lệ `/scene`, không đổi dữ liệu design. Host/Origin guard giữ nguyên.

## Ngoài phạm vi / phase sau

- Sửa `exportSettings` ngay trên viewer: cần route ghi, trái nguyên tắc viewer read-only.
- Zip nhiều file (Figma "Export all" ra zip).
- Export đa chọn trên canvas.
- Preview ảnh export trong panel.
