# P06c — Viewer: chế độ Flow, sơ đồ frame và mũi tên điều hướng, bật/tắt
Ưu tiên: P1 · Kích cỡ: ~85 phút dev · Phụ thuộc: P06a (dữ liệu `props.links`/`flowStart` trong snapshot); không cần P06b · Nguồn: U6 ("hiện trên viewer dạng flow")

Phase viewer của P06. Người dùng mở page và bấm "Flow" để thấy mọi root frame thành hộp, với mũi tên giữa chúng theo link mà agent đã khai báo. Bấm "Canvas" để quay về màn xem frame như cũ.

## Vấn đề

- **Viewer chỉ xem từng frame một.**
  - `App.tsx:29-31` lấy root frame của page cho ô chọn "Frame".
  - `App.tsx:37-47` dựng một `SceneRequest` duy nhất cho frame đang chọn.
  - `CanvasView.tsx:51-119` vẽ đúng một bitmap frame.
  - Không có góc nhìn nào cho thấy quan hệ giữa các frame. Với 18 frame như QueryNest (`~/Documents/opencanvas.md:91-94`), người dùng không kiểm được agent đã nối màn nào tới màn nào.
- **Sau P06a, link chỉ hiện dạng JSON** trong Inspector của nút nguồn (`Inspector.tsx:45-47` in mọi prop authored bằng `JSON.stringify`). Muốn thấy link vào một màn thì không có cách nào.

## Bằng chứng

- **Snapshot.**
  - Viewer có toàn bộ node và props qua `GET /snapshot` (`web/src/types.ts:10-22`, `hooks/useSnapshot.ts`), nên tính được đồ thị phía client mà không cần API mới hay P06b.
  - SSE `commit` đã làm viewer tải lại snapshot (`docs/VIEWER.md`, mục Consistency), nên sơ đồ tự cập nhật khi agent thêm link.
- **Root frame và layout.**
  - Root frame là scene độc lập: input render đặt `parent_id = None` (`crates/server/src/jobs/input.rs:19-20`), x/y của root frame không được dùng.
  - Vì vậy flow map phải tự xếp chỗ. Không thể dựa vào vị trí authored trên page như Figma.
- **Hạ tầng tái dùng được.**
  - `useCanvasTransform(viewport, key)` (`hooks/useCanvasTransform.ts:46-90`) cho sẵn pan/zoom/fit theo một kích thước bất kỳ. Truyền kích thước sơ đồ và key `flow:<pageId>` là dùng lại được.
  - `useScene(request | null)` (`App.tsx:47`) nhận `null`, nên chế độ Flow tắt render bằng cách truyền `null`.
- **Render job.** B01 đang xử lý `QUEUE_FULL` do nhiều render (note #7). Vẽ thumbnail cho mọi frame sẽ enqueue N job mỗi lần mở Flow. Phase này **không render**, chỉ vẽ hộp và tên. Thumbnail để sau.
- **Test.**
  - Unit test viewer chạy `node --test` trên file `.ts` (`web/package.json:10`, `web/tests/state.test.ts`).
  - Smoke Playwright dùng fixture HTTP ghi lại mọi request (`web/tests/fixture.mjs:12-19`, `:105`) và có cơ chế commit/SSE (`fixture.mjs:30`, `:109-112`). Đủ để kiểm "không có request `/scene` ở chế độ Flow" và "commit cập nhật sơ đồ".
- **Kích thước file.** `App.tsx` 231 dòng, gần ngưỡng 250, nên chỉ được thêm vài dòng; phần còn lại phải nằm ở file mới.
- **Tham khảo.** Figma prototype tab vẽ "noodle" giữa frame và có flow starting point. Penpot vẽ mũi tên interaction khi ở chế độ Prototype (https://help.penpot.app/user-guide/prototyping/). Cả hai chỉ hiện khi bật chế độ prototype, nên ở đây cũng là chế độ bật/tắt.

## Giải pháp

### Bố cục và hành vi

**Thanh viewbar** thêm nhóm nút hai trạng thái **Canvas | Flow** (`aria-pressed`), cạnh ô chọn Page/Frame.
- Mặc định Canvas.
- Lựa chọn nhớ trong `localStorage["opencanvas.viewMode"]`, bọc try/catch, lỗi thì về Canvas.

**Chế độ Flow** thay `CanvasView` bằng `FlowView` cho page đang chọn. Panel Layers và Inspector giữ nguyên.

Hộp:
- Mỗi root frame của page là một hộp: rộng 200; cao `200 × height/width` của frame, kẹp 80–360; mặc định 800×600 khi không phải số.
- Hộp có tên frame và kích thước ("390 × 844").
- Frame có `flowStart` có nhãn "▶ Main" phía trên hộp.
- Đích nằm ở page khác thành hộp viền gạch, nhãn "Page / Frame".

Mũi tên (một mũi tên cho mỗi cặp `(frame nguồn, đích, action)`, có số đếm "×3" khi nhiều nút cùng dẫn tới):
- `navigate`: nét liền, màu accent `#1672e8`;
- `overlay`: nét gạch;
- `on:"hover"`: nét chấm, mảnh hơn.

Link `back`/`close` không có đích: badge "↩ n" / "✕ n" ở góc hộp nguồn.

Xếp chỗ: phân tầng theo BFS, xác định hoàn toàn từ dữ liệu.
1. Gốc là các frame có `flowStart`, theo `sortKey`. Không có thì gốc là frame đầu tiên.
2. Cột = khoảng cách BFS theo link navigate/overlay.
3. Frame chưa được thăm: lấy frame đầu tiên còn lại theo `sortKey` làm gốc mới ở cột 0, BFS tiếp, cho tới khi hết.
4. Trong một cột, xếp theo thứ tự thăm; khoảng cách cột 120, hàng 48.

Đường mũi tên:
- Cạnh tiến (cột đích > cột nguồn): từ giữa mép phải nguồn tới giữa mép trái đích.
- Cạnh lùi hoặc cùng cột: cong vòng dưới hộp (Konva `Arrow` với `tension`).
- Tự trỏ: vòng nhỏ trên hộp.

Tương tác:
- Kéo để pan, lăn để zoom, phím F để fit: dùng chung `useCanvasTransform`.
- Bấm hộp → `selectScope({pageId, frameId, nodeId: frameId})` rồi chuyển về Canvas. Hộp ở page khác chuyển cả page.

**Truy cập không cần chuột (đồng thời là bề mặt test):** dưới canvas có hai danh sách DOM thật.
- `<ul aria-label="Flow screens">`: mỗi màn một nút "Open Home".
- `<ul aria-label="Flow links">`: mỗi link một mục "Home → Search · click · navigate" (back/close ghi "Home · click · back"), theo đúng thứ tự cạnh của P06a/P06b.
- Có thể thu gọn. Mặc định hiển thị dạng danh sách nhỏ bên dưới, đủ để đọc bằng screen reader.

**Trạng thái rỗng:** page không có link và không có `flowStart` vẫn vẽ các hộp, kèm thông báo "No links on this page yet. Agents declare them with props.links."

**Không render:** ở chế độ Flow, `App` truyền `null` cho `useScene`. Không có request `/scene` hay `/jobs`, không thêm áp lực hàng đợi.

### Module (tách để `App.tsx` không phình)

| File | Trách nhiệm |
| --- | --- |
| `web/src/flow/graph.ts` (mới) | `buildFlow(nodes, pageId)` → `{screens, edges, flows}`. Chuẩn hoá mặc định (`on:"click"`, `action:"navigate"`) giống `opencanvas_storage::links`. Frame nguồn là root frame chứa nút. Bỏ qua phần tử link sai dạng thay vì ném lỗi. Hàm thuần, không React. |
| `web/src/flow/layout.ts` (mới) | `layoutFlow(flow)` → `{boxes, arrows, badges, width, height}`. Thuật toán BFS phân tầng ở trên, hàm thuần. |
| `web/src/components/FlowView.tsx` (mới) | Stage Konva: hộp, mũi tên, nhãn, badge; pan/zoom bằng `useCanvasTransform`; hai danh sách truy cập. |
| `web/src/components/ViewModeToggle.tsx` (mới) | Nút Canvas/Flow và lưu `localStorage`. |
| `web/src/flow.css` (mới) | style danh sách, nhãn rỗng, toggle. |
| `web/src/App.tsx` | state `mode`, đặt toggle trong viewbar, chọn `FlowView`/`CanvasView`, `useScene(mode === "canvas" ? request : null)`. |

## Phạm vi file

| File | Việc | Ước lượng dòng |
| --- | --- | --- |
| `web/src/flow/graph.ts` (mới) | dựng đồ thị | ~100 |
| `web/src/flow/layout.ts` (mới) | xếp chỗ, đường mũi tên | ~120 |
| `web/src/components/FlowView.tsx` (mới) | vẽ và danh sách | ~160 |
| `web/src/components/ViewModeToggle.tsx` (mới) | toggle | ~35 |
| `web/src/flow.css` (mới) | style | ~50 |
| `web/src/App.tsx` | nối | 231 → ≤ 245 |
| `web/tests/flow.test.ts` (mới) | unit `graph`/`layout` | ~140 |
| `web/tests/fixture-data.mjs` | thêm frame `pricing`, `menu` và link trên `frame`/`pricing`; link `hover` | 182 → ~215 |
| `web/tests/fixture.mjs` | commit thêm link (cờ `linked`, như cờ `deleted`) | 135 → ~150 |
| `web/tests/smoke.mjs` | bước Flow (AC 3–8), ảnh `artifacts/viewer/flow.png` | 156 → ~195 |
| `web/package.json` | script `test` chạy thêm `tests/flow.test.ts` | +0 (sửa 1 dòng) |
| `docs/VIEWER.md` | mục "Flow view" | +15 |

Không file nào vượt 250 dòng. `App.tsx` ≤ 245. Nếu không giữ được, tách khối viewbar ra `components/Viewbar.tsx`, nhưng tránh nếu có thể để giảm xung đột với P01/P02b/phiên UI redesign.

Nếu quá giờ, cắt theo thứ tự (ghi vào `.night/dev/P06c.md`):
1. badge back/close;
2. nhãn "×n";
3. đường cong cạnh lùi (thay bằng đường thẳng).

## Tương thích MCP

Không đổi server, MCP hay HTTP. Viewer chỉ đọc `props.links`/`flowStart` từ snapshot hiện có. Không thêm request mới; chế độ Flow còn ít request hơn Canvas.

## AC

1. **Đồ thị (unit).** `web/tests/flow.test.ts` với snapshot mẫu: 2 page; 5 root frame; link navigate, overlay, hover-overlay, back, close; 1 link khác page; 1 `flowStart`; 1 link sai dạng.
   - `buildFlow(nodes, "page-1")` trả đúng `screens` (kể cả stub khác page với `pageId`), `edges` (mặc định đã ghi rõ, thứ tự frame → preorder → mảng) và `flows`.
   - Link sai dạng bị bỏ, không ném lỗi.
2. **Xếp chỗ (unit).**
   - `layoutFlow` gọi hai lần cho kết quả giống hệt.
   - Không hai hộp nào giao nhau (kiểm từng cặp).
   - Frame có `flowStart` ở cột 0; đích navigate trực tiếp của nó ở cột 1.
   - Frame không được thăm vẫn có hộp.
   - Cạnh lùi có ≥ 3 điểm (đường cong); cạnh tiến có 2 điểm.
   - Ba nút cùng dẫn `home → search` thành một mũi tên `count:3`.
3. **Toggle.**
   - Smoke: nút "Canvas" `aria-pressed=true` lúc đầu.
   - Bấm "Flow" → `aria-pressed` đổi; reload trang vẫn ở Flow; bấm "Canvas" quay lại và scene xuất hiện như cũ ("Scene rN").
4. **Nội dung Flow.** Với fixture mới (frame `Welcome` → `Pricing` navigate, `Welcome` → `Menu` overlay, `Pricing` back):
   - danh sách "Flow screens" có đúng 3 nút;
   - "Flow links" có đúng 3 mục với văn bản "Welcome → Pricing · click · navigate", "Welcome → Menu · click · overlay", "Pricing · click · back".
5. **Không render.** Số request có đường dẫn kết thúc `/scene` trong `fixture.requests` không tăng trong suốt thời gian ở chế độ Flow (đếm trước khi bấm Flow và trước khi bấm Canvas).
6. **Mở frame.** Bấm "Open Pricing" → chế độ Canvas, ô chọn Frame có giá trị `pricing`, có một request `/scene` cho `pricing`.
7. **Live.** Fixture commit thêm link `Menu → Welcome` (cờ `linked`, SSE `commit`) khi đang ở Flow → "Flow links" có 4 mục mà không reload.
8. **Ảnh.** Smoke lưu `artifacts/viewer/flow.png`, thấy 3 hộp và mũi tên. Ảnh được kiểm bằng mắt và ghi trong `.night/dev/P06c.md`.
9. **Rỗng.** Page không link, không `flowStart` hiện thông báo trạng thái rỗng và vẫn có hộp cho mỗi root frame (unit hoặc smoke).
10. **Lệnh kiểm.** `npm --prefix web test`, `npm --prefix web run typecheck`, `npm --prefix web run build`, `npm --prefix web run test:browser` pass. Không có `pageerror` trong smoke (smoke đã thu `errors`).
11. **Modular.** File mới ≤ 200 dòng; `App.tsx` ≤ 245; `smoke.mjs` ≤ 230. Báo `wc -l`.

## Cách kiểm

```bash
cd web
npm test                    # state.test.ts + flow.test.ts
npm run typecheck
npm run build
npm run test:browser        # fixture HTTP + Vite 5179 + Playwright; ghi artifacts/viewer/flow.png
wc -l src/App.tsx src/flow/*.ts src/components/FlowView.tsx src/components/ViewModeToggle.tsx tests/smoke.mjs
```

Kiểm live tuỳ chọn trên server tạm sau P06a:
- `--port 0 --data-root $(mktemp -d)`;
- seed 3 frame có link qua MCP;
- chạy Vite với `OPENCANVAS_API_URL=<base>`, mở Flow, chụp ảnh;
- dọn process và thư mục.

## Rủi ro

- **Xung đột file viewer.**
  - `App.tsx` bị P02b (note pin) và có thể bị phiên "OpenCanvas UI redesign" (ngoài đội) sửa trên main.
  - `CanvasView.tsx` bị P01 viết lại.
  - Phase này không đụng `CanvasView.tsx`. Chỉ thêm ≤ 14 dòng vào `App.tsx`, còn lại là file mới, nên rebase rẻ.
  - Dev orchestrator xếp sau P01/P02b nếu chúng đang mở cùng lúc.
- **Sơ đồ lớn.** 18–50 frame vẫn đọc được nhờ pan/zoom. Mũi tên có thể chồng nhau nếu nhiều cạnh lùi. Chấp nhận ở phase này, vì danh sách "Flow links" luôn chính xác.
- **Xếp chỗ không giống bố cục authored.** Root frame không có vị trí canvas có nghĩa (`jobs/input.rs:19-20`), nên không có bố cục "thật" để giữ. Ghi trong VIEWER.md.
- **Fixture thêm frame** có thể làm lệch các bước smoke cũ phụ thuộc số frame. Frame mới có `sortKey` sau `Welcome` nên lựa chọn mặc định không đổi; dev chạy lại toàn bộ smoke.

## Ngoài phạm vi / phase sau

- **P06d (P2):**
  - hotspot link trên chế độ Canvas: viền gạch quanh nút có link và chip "→ Pricing", bấm để sang màn;
  - mục "Navigation" trong Inspector: link ra của node, link vào của frame.
- **Sau:**
  - thumbnail frame trong hộp (dùng scene đã cache, không enqueue mới; cần B01);
  - chế độ "play" prototype;
  - xếp chỗ thủ công hoặc dùng x/y authored của root frame nếu sau này có canvas page;
  - nhãn hiển thị tên nút nguồn trên mũi tên.
