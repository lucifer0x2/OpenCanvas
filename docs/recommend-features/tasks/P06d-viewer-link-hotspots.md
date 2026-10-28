# P06d — Viewer: hotspot link trên frame và mục Navigation trong Inspector
Ưu tiên: P2 · Kích cỡ: ~60 phút dev · Phụ thuộc: P06a (dữ liệu), P06c (`web/src/flow/graph.ts`); nên làm sau P01 và P02b vì cùng chạm `CanvasView.tsx`/`Inspector.tsx` · Nguồn: U6

Phase cuối, tuỳ chọn, của P06. P06c cho thấy quan hệ giữa các màn. P06d cho thấy **nút nào** trên màn đang xem dẫn đi đâu, và cho phép bấm để đi theo link như xem prototype. Mọi thứ vẫn chỉ đọc.

## Vấn đề

Ở chế độ Canvas, sau P06a/P06c, link của một nút chỉ hiện dạng JSON trong Inspector (`web/src/components/Inspector.tsx:45-47`). Người dùng phải chọn từng layer mới biết nút nào có link. Không có cách nào thấy "màn này được mở từ đâu" khi đang xem nó.

## Bằng chứng

- **Bounds của node.**
  - Scene của frame đang xem có bounds frame-local cho từng node (`renderer/types.ts:47-51`).
  - `CanvasView.tsx:110-117` đã vẽ viền chọn bằng `Rect {...selected.bounds}` với nét `1.5 / transform.scale`.
  - Hotspot dùng đúng cơ chế này, không cần dữ liệu mới.
- **Chọn node.** Bấm canvas chọn node qua `hitTest` trong `onPointerUp` của Stage (`CanvasView.tsx:71-87`). Chip bấm được phải đặt `cancelBubble` để không đồng thời chọn node bên dưới.
- **Toggle.** Thanh nổi trên canvas (`components/CanvasChrome.tsx`, 64 dòng) đã chứa zoom/fit, là chỗ hợp lý cho nút "Show links". `useFollowAgent.ts:12-24` là mẫu lưu bật/tắt trong `localStorage`.
- **Đồ thị.** P06c đã có `buildFlow` (`web/src/flow/graph.ts`), cho cả link ra theo node và link vào theo frame. Inspector dùng lại.

## Giải pháp

**Nút "Show links"** trên `CanvasChrome` (`aria-pressed`):
- Mặc định tắt; nhớ trong `localStorage["opencanvas.showLinks"]`, bọc try/catch.
- Disabled khi frame đang xem không có node nào có link.

**Khi bật**, `LinkOverlay` (Konva `Group`, `listening` chỉ ở chip) vẽ cho mỗi node của scene có `props.links`:
- Viền gạch màu accent quanh bounds.
- Một chip mỗi link ở góc phải trên bounds. Nhãn theo action:
  - `→ Pricing` (navigate);
  - `⧉ Menu` (overlay);
  - `↩ Back`;
  - `✕ Close`.
  - Link `hover` thêm hậu tố "· hover".
  - Tên lấy từ `name` của frame đích, không có thì dùng ID.
- Chip có đích, bấm vào → `selectScope({pageId của đích, frameId: to, nodeId: to})` (đi theo link như prototype). Chip back/close chỉ để xem.
- `canvas-host` mang `data-link-hotspots="<số node có link>"` để test đếm được mà không cần đọc canvas.

**Inspector**, mục mới "Navigation" (component riêng):
- Node chọn có `links`: mỗi link một dòng nút "click → Pricing · navigate", bấm để mở đích.
- Node chọn là root frame:
  - "Flow start: Main" nếu có;
  - "Opened from": danh sách link vào (frame nguồn / tên nút nguồn · action), bấm để mở frame nguồn và chọn nút nguồn.
- Không có gì thì không hiện mục.

**Không render thêm:** bật/tắt chỉ vẽ đè lên bitmap có sẵn, không gửi request `/scene`.

## Phạm vi file

| File | Việc | Ước lượng dòng |
| --- | --- | --- |
| `web/src/components/LinkOverlay.tsx` (mới) | viền và chip Konva, xử lý bấm chip | ~110 |
| `web/src/components/NavigationSection.tsx` (mới) | mục Inspector | ~90 |
| `web/src/flow/labels.ts` (mới) | `linkLabel(edge, screens)` dùng chung cho chip, Inspector và danh sách "Flow links" của P06c | ~30 |
| `web/src/hooks/useShowLinks.ts` (mới) | state và `localStorage` | ~25 |
| `web/src/components/CanvasView.tsx` | nhận `nodes`/`onOpenFrame`, chèn `<LinkOverlay>`, `data-link-hotspots` | 134 → ≤ 142 |
| `web/src/components/CanvasChrome.tsx` | nút "Show links" | 64 → ~76 |
| `web/src/components/Inspector.tsx` | chèn `<NavigationSection>` | 100 → ≤ 106 |
| `web/src/App.tsx` | truyền `nodes`/`openFrame` | +≤ 4 (vẫn ≤ 250 sau P06c) |
| `web/tests/flow.test.ts` | unit `linkLabel` | +25 |
| `web/tests/smoke.mjs` | bước hotspot (AC 2–6) | +30 |
| `docs/VIEWER.md` | đoạn "Link hotspots" | +8 |

Nếu P01 đã thay `CanvasView` bằng renderer vector, `LinkOverlay` vẫn là một `Group` trong Layer selection, vì cùng hệ toạ độ frame-local. Dev kiểm lại điểm chèn khi rebase.

## Tương thích MCP

Không đổi server, MCP hay HTTP.

## AC

1. **Nhãn (unit).** `linkLabel` trả đúng 5 dạng ("→ Pricing", "⧉ Menu", "↩ Back", "✕ Close", "⧉ Tip · hover"); thiếu `name` thì dùng ID.
2. **Toggle.**
   - Smoke trên frame `Welcome` của fixture P06c: nút "Show links" `aria-pressed=false`, `data-link-hotspots="0"`.
   - Bấm → `aria-pressed=true`, `data-link-hotspots="2"` (nút tới Pricing và nút tới Menu).
   - Reload vẫn bật.
   - Frame không có link → nút disabled.
3. **Không render.** Số request `/scene` trong `fixture.requests` không đổi qua hai lần bật/tắt.
4. **Đi theo link.**
   - Bấm chip "→ Pricing": toạ độ = góc phải trên bounds của nút trong scene × transform, lấy từ `scene.nodes` qua `page.evaluate`, hoặc qua nút tương ứng của Inspector ở AC 5.
   - Kết quả: ô chọn Frame = `pricing`; node `Welcome` bên dưới không bị chọn.
5. **Inspector.**
   - Chọn nút nguồn → mục "Navigation" có nút "click → Pricing · navigate"; bấm → mở `pricing`.
   - Chọn root frame `pricing` → "Opened from" có "Welcome / <tên nút> · navigate"; bấm → frame `frame` được mở và nút nguồn được chọn (heading Inspector là tên nút).
6. **Ảnh.** Smoke lưu `artifacts/viewer/link-hotspots.png`, thấy viền gạch và chip; ghi trong `.night/dev/P06d.md`.
7. **Lệnh kiểm.** `npm --prefix web test`, `typecheck`, `build`, `test:browser` pass, không `pageerror`.
8. **Modular.** File mới ≤ 150 dòng; `CanvasView.tsx` ≤ 142; `Inspector.tsx` ≤ 106; `CanvasChrome.tsx` ≤ 80; `App.tsx` ≤ 250. Báo `wc -l`.

## Cách kiểm

```bash
cd web
npm test && npm run typecheck && npm run build && npm run test:browser
wc -l src/components/{LinkOverlay,NavigationSection,CanvasView,CanvasChrome,Inspector}.tsx src/flow/labels.ts src/hooks/useShowLinks.ts src/App.tsx
```

## Rủi ro

- **File nóng.**
  - `CanvasView.tsx` bị P01 viết lại và P02b thêm pin note.
  - `Inspector.tsx` bị P02b thêm mục Notes.
  - Đây là lý do P06d xếp P2 và đi sau. Thay đổi trong hai file này giữ ≤ 8 dòng; logic nằm ở component mới.
- **Chip che nội dung** khi nhiều nút nhỏ sát nhau. Toggle mặc định tắt. Chip có kích thước cố định trên màn hình (chia cho `transform.scale`) để không phình khi zoom.
- **Bấm chip lẫn với chọn node.** Dùng `cancelBubble`; AC 4 kiểm node bên dưới không bị chọn.

## Ngoài phạm vi / phase sau

- Chế độ "play" (ẩn chrome, chỉ bấm theo link, hiện overlay chồng lên màn);
- mô phỏng hover;
- lịch sử back;
- tô màn đích trong flow map khi rê chuột lên chip.
