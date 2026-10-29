# Task 12 — Viewer vẽ bằng engine Rust build WASM: cùng pixel với ảnh export

**Trạng thái:** CHƯA GIAO. Đóng gói 2026-10-29.
**Phụ thuộc:** Task 11 (crate `crates/render` build được WASM, native↔WASM đã khớp). Task 09 cho số đo nền
(kích thước, thời gian vẽ khung lớn) để chọn chiến lược vẽ lại.
**Phạm vi sở hữu:** `web/src/components/CanvasView.tsx`, `web/src/hooks/useScene.ts`, module mới trong `web/src/`
(worker WASM, cache tile, cờ renderer), `web/vite.config.ts` và `web/package.json` (build/nhập `.wasm`), `scripts/app.mjs`
(bước build WASM), bước build WASM của `crates/render` (feature/binding nếu cần), test mới trong `web/tests/` và một kịch bản
trong `spikes/e2e/`, `docs/VIEWER.md`, `docs/PURE-RUST-EXPORT.md`.
Không sửa `web/src/preview/` (preview motion giữ DOM), `renderer/`, `worker/`, `crates/server` (trừ khi phải sửa MIME
`.wasm`, ghi rõ), `PLAN.md`, `AGENTS.md`.
**Workspace:** worktree riêng, nhánh mới từ nhánh nền có Task 11. Không push khi chưa được yêu cầu.

## Bối cảnh

Hướng đã chọn: viewer chỉ để xem, agent tự code lại UI theo frontend dự án đích, nên viewer phải **giống ảnh export**,
không cần giống Chrome từng pixel. Dùng chính `crates/render` build WASM thì viewer = export theo cấu tạo.

Viewer hiện tại (đọc lại trước khi sửa):

- `web/src/components/CanvasView.tsx:83`–`:164`: Konva `Stage` có `scaleX/scaleY = transform.scale`; `Shape.sceneFunc`
  gọi `paintScene` mỗi lần vẽ nên nét ở mọi mức zoom (P01, comment `:152`); khung chọn là `Rect` có
  `strokeWidth = 1.5 / scale` (`:161`–`:164`); click/hover dùng `hitTest` (`:113`, `:125`).
- `web/src/canvasHitTest.ts`: hit-test chỉ dùng bounds/clip của scene, **không phụ thuộc painter**.
- `web/src/hooks/useScene.ts:45`–`:53`: lấy scene job, `loadResources` (fetch font/ảnh, kiểm SHA, tạo FontFace/Image),
  vẽ một canvas parity bằng `drawScene`.
- `/preview` (`web/src/preview/`, `sceneDom.ts`) là renderer DOM riêng cho animation, hover state và link: **ngoài phạm vi**.

Ngoại suy từ spike native (chưa đo WASM): khung 1440×900 DPR 2 cỡ 50 ms native, có thể ~100 ms WASM chưa tối ưu.
Đủ cho "vẽ lại khi zoom dừng", không đủ để vẽ lại mỗi frame khi kéo. Dùng số thật của Task 09.

## Mục tiêu

Sau cờ (mặc định tắt), viewer vẽ khung bằng WASM trong Web Worker, nét ở mọi mức zoom, thao tác không giật,
tương tác giữ nguyên.

## Việc phải làm

1. **Build:** thêm bước build `crates/render` sang WASM vào `scripts/app.mjs` và build web; `.wasm` nằm trong `web/dist`,
   không tải từ CDN. Kiểm server phục vụ `.wasm` với MIME `application/wasm` (cần cho `instantiateStreaming`).
2. **Worker:** Web Worker giữ instance WASM, nhận scene + bytes font/ảnh (tái dùng phần fetch + kiểm SHA của
   `renderer/resources.ts`, không tạo FontFace/Image cho đường WASM), trả `ImageBitmap` cho một vùng ở một tỉ lệ.
   Không chặn main thread.
3. **Chiến lược vẽ:** khi kéo/zoom thì biến đổi bitmap đang có; khi dừng (debounce ~100–150 ms) vẽ lại vùng nhìn thấy ở
   `scale × devicePixelRatio`, giới hạn theo `MAX_PIXELS`; cache tile theo (scene key, mức tỉ lệ, tile), xóa khi scene đổi.
   Hủy job cũ khi có job mới.
4. **Tương tác:** giữ `hitTest`, khung chọn, hover, element actions như cũ (lớp overlay Konva hoặc canvas mỏng phía trên).
5. **Cờ:** `renderer=wasm|canvas2d` (query hoặc localStorage, mặc định `canvas2d`); thiếu WASM hoặc lỗi thì rơi về
   `canvas2d` và hiện diagnostic, không trắng màn hình.
6. Cập nhật `docs/VIEWER.md`: kiến trúc, cờ, giới hạn.

## Nghiệm thu

### Acceptance Criteria

- Với `renderer=wasm`, ở zoom 100% DPR 1, bitmap viewer trùng PNG export native cùng frame: max Δ ≤ 1/255.
- Zoom 400%: sau khi dừng, vùng nhìn thấy được vẽ lại ở tỉ lệ đó (so với PNG native scale 4 cùng vùng: max Δ ≤ 1/255);
  không còn ảnh phóng to mờ.
- Kéo/zoom liên tục: không có long task > 50 ms trên main thread do vẽ (đo bằng PerformanceObserver trong test).
- Click chọn, hover, element actions, layers panel hoạt động như `canvas2d` (test hiện có pass với cả hai cờ).
- `canvas2d` mặc định không đổi; `/preview` không đổi.
- File mới ≤ 250 dòng hoặc nêu lý do; `npm --prefix web test` và typecheck sạch.

### Cách kiểm

```bash
npm --prefix web test
```

```bash
npm run test:e2e
```

Thêm kịch bản E2E bật `renderer=wasm` trên fixture có text, viền, clip, opacity, ảnh: so bitmap với PNG export, đo long
task khi zoom. Kiểm tay bằng Vite dev trỏ server tạm `--port 0` + data root tạm.

### Đầu ra

- Mã + test; số liệu (kích thước `.wasm`, thời gian vẽ lại, long task) trong `docs/PURE-RUST-EXPORT.md`.
- Báo cáo cuối: khác biệt còn lại giữa hai cờ, số dòng file sửa.

## Ràng buộc

- Không xóa đường `canvas2d` hay `renderer/` trong task này (Task 13 quyết).
- Không chạm server cổng 3847, Docker, `~/.opencanvas`. Không push.
