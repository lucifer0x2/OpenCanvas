# P03d — Export JPG/WebP và export mọi node được đánh dấu trong một frame (`withinId`)
Ưu tiên: P2 · Kích cỡ: ~80 phút dev · Phụ thuộc: P03a (bắt buộc), P03b (dùng lại `export/image_meta.rs` để đọc kích thước JPEG/WebP) · Nguồn: U5

## Vấn đề

1. U5 nêu "có thể thêm PDF/JPG/WebP". Sau P03a/b chỉ có PNG và SVG. Ảnh minh hoạ hay ảnh chụp xuất PNG thì nặng; web thường cần WebP, còn email hay tài liệu cần JPG.
2. AI coder muốn "tải mọi asset được đánh dấu trong frame này", nhưng hiện phải tự tìm node có `exportSettings`. `find_nodes` chỉ lọc theo tên, text, kind hoặc id, không lọc được "có prop X" (`crates/server/src/schemas.rs:104-106`). Vì vậy agent phải đọc `get_subtree`/`get_design_context` cả frame (tốn token) rồi tự lọc, sau đó mới gọi `export_nodes`.

## Bằng chứng

- Worker encode PNG cứng: `canvas.toDataURL("image/png")` (`worker/pipeline.mjs:193`). Server chỉ chấp nhận chữ ký PNG (`crates/server/src/jobs/png.rs:4-27`).
- Probe trên Chromium headless của worker (`.runtime/browsers/chromium_headless_shell-1248`): `toDataURL` trả đúng tiền tố `data:image/jpeg` và `data:image/webp`. Encoder có sẵn, không cần chuyển đổi.
  - Penpot thì phải tạo PNG rồi convert sang WebP, vì screenshot của Playwright chỉ hỗ trợ JPEG/PNG (commit thêm WebP 03/2025, xem https://doc.plugins.penpot.app/interfaces/Export và lịch sử repo Penpot).
- JPG không có alpha. Vùng trong suốt của canvas (`renderer/drawing.ts:139`, `clearRect`) khi encode JPEG thành đen, nên phải phủ nền trước.
- Duyệt subtree theo preorder có sẵn: `TreeIndex::preorder(Some(root))` (`crates/server/src/reads/index.rs:40-57`).
- Figma có format JPG (REST `format=jpg`, https://developers.figma.com/docs/rest-api/file-endpoints/), không có WebP. Penpot có cả `jpeg` và `webp`.

## Giải pháp

### Định dạng JPG/WebP

- `format` thêm `"jpg"` và `"webp"`. `"jpeg"` bị từ chối kèm gợi ý "use jpg".
- Thêm `quality`: integer 1–100, mặc định 90, chỉ hợp lệ với jpg/webp; đặt trên png hoặc svg thì `VALIDATION_FAILED`.
- `scale`/`width`/`height`/`suffix` giữ như PNG. Đuôi file `.jpg` và `.webp`.
- Worker op `render` nhận thêm `mimeType` (mặc định `image/png`, nên request cũ không đổi) và `quality`:
  - `jpeg`: sau `drawScene`, đặt `globalCompositeOperation = "destination-over"` và `fillRect` trắng toàn canvas. JPG luôn có nền trắng (cố định ở v1).
  - Encode bằng `canvas.toDataURL(mimeType, quality/100)`.
  - Không sửa `renderer/` (dùng chung với viewer); chỉ sửa `worker/pipeline.mjs` + `worker/cli.mjs`.
- Server: `jobs/png.rs` → `jobs/raster.rs::decode(value, mime)`. Hàm này kiểm chữ ký PNG, JPEG (`FF D8 FF`) hoặc WebP (`RIFF…WEBP`) và kích thước khớp `width/height` worker báo. Kích thước JPEG/WebP đọc bằng `export/image_meta.rs` của P03b.

### `withinId`: export node được đánh dấu

```json
{"projectId":"p_x","withinId":"frame_home"}
```

- Dùng **thay** `nodeIds`. Server enforce chỉ một trong hai; `settings` không được dùng cùng `withinId` (node đánh dấu dùng setting của chính nó).
- Published schema vẫn là object phẳng. `required` nới từ `["projectId","nodeIds"]` thành `["projectId"]` (nới, không phá).
- Lấy các node trong subtree của `withinId` (kể cả chính nó) có `exportSettings` khác rỗng, theo preorder, tối đa 8 (cùng giới hạn `nodeIds`).
- Response thêm `marked` (tổng số node đánh dấu trong scope). Khi `marked > 8` thì thêm `remainingIds` (≤ 200 id còn lại), agent gọi tiếp với `nodeIds: remainingIds`. Không có node nào thì `exports: []`, `marked: 0`, không phải lỗi.

## Phạm vi file

| File | Tạo/sửa | Dòng ước tính |
| --- | --- | --- |
| `worker/pipeline.mjs` | sửa `render`: `mimeType`, `quality`, nền trắng cho JPEG | +10 (226 → ~236) |
| `worker/cli.mjs` | truyền `mimeType`/`quality` | +2 |
| `crates/server/src/jobs/png.rs` → `jobs/raster.rs` | đổi tên + decode theo MIME | ~55 |
| `crates/server/src/export/png.rs` → `export/raster.rs` | gửi `mimeType`/`quality`, đuôi file | +15 |
| `crates/server/src/export/marked.rs` | tạo: thu node đánh dấu, `marked`, `remainingIds` | ~60 |
| `crates/server/src/export/mod.rs`, `schemas/export.rs` | `withinId`, `quality`, kiểm loại trừ | +20 |
| `crates/storage/src/validate/export_settings.rs` + test | `jpg`, `webp`, `quality` | +25 / +30 |
| `crates/server/tests/export_formats.rs` | tạo | ~160 |
| `crates/server/Cargo.toml` | dev-dep `image` bật thêm `jpeg`, `webp` (resolve và fetch offline đều được) | ±1 |
| `docs/SERVER.md`, `crates/server/skills/implement-frame.md` | format mới, `withinId` ("tải mọi asset của frame bằng `withinId`") | ~12 |

## Tương thích MCP

- `export_nodes`: thêm enum format, `quality`, `withinId` và các field response `marked`/`remainingIds`. `nodeIds` không còn bắt buộc trong published schema nhưng server bắt buộc có đúng một trong `nodeIds`/`withinId`. Mọi call hợp lệ ở P03a vẫn hợp lệ và cho cùng kết quả.
- Worker protocol: op `render` không có `mimeType` → PNG như cũ. `render_frame` không đổi.

## AC

1. `settings:[{"format":"jpg","scale":2}]` trên rect radius 6 fill `#ef4444` 40×40:
   - file `image/jpeg`, đuôi `.jpg`, bytes bắt đầu `FF D8 FF`;
   - decode (dev-dep `image`) ra 80×80;
   - pixel (0,0) mỗi kênh ≥ 250 (nền trắng);
   - pixel tâm lệch ≤ 8/kênh so với (239,68,68).
2. `{"format":"webp","scale":2}` cùng node → `RIFF…WEBP`, decode ra 80×80, pixel (0,0) alpha = 0 (giữ trong suốt).
3. Image node chứa ảnh nhiễu (fixture tạo PNG ngẫu nhiên 256×256): JPG `quality:40` < 0.6 × kích thước JPG `quality:95`.
4. Storage từ chối, chỉ đúng field:
   - `{"format":"png","quality":80}`;
   - `{"format":"jpg","quality":0}`;
   - `{"format":"jpg","quality":101}`;
   - `{"format":"jpeg"}` (message chứa "jpg");
   - `{"format":"svg","quality":80}`.
5. `withinId` = frame có 3 node đánh dấu (1 node lồng 2 cấp) + 2 node không đánh dấu → `exports` gồm đúng 3 item theo preorder, `marked:3`, không có `remainingIds`.
6. Frame có 10 node đánh dấu → 8 item, `marked:10`, `remainingIds` có 2 id. Gọi tiếp `nodeIds: remainingIds` → 2 item ready.
7. Những call sau → `INVALID_REQUEST` có message nêu cách sửa:
   - `withinId` + `nodeIds`;
   - `withinId` + `settings`;
   - không có cả hai.

   `withinId` không có node đánh dấu → `exports:[]`, `marked:0`.
8. Không hồi quy:
   - op `render` không `mimeType` cho fixture `spikes/pipeline` trả base64 **giống hệt** trước khi sửa (golden);
   - test PNG/SVG của P03a/b và `cargo test -p opencanvas-server` pass;
   - `npm run test:pipeline` pass.
9. `worker/pipeline.mjs` ≤ 250 dòng. File mới ≤ 200. Báo `wc -l`.

## Cách kiểm

- `npm run build:renderer && npm run test:pipeline` (AC8 phần worker; thêm test golden mimeType mặc định vào `spikes/pipeline/lifecycle.test.mjs` hoặc file mới).
- `CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-storage --test export_settings` (AC4).
- `CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-server --test export_formats` (AC1–3, 5–7) và `cargo test --offline -p opencanvas-server` (AC8).
- Live trên server tạm `--port 0 --data-root $(mktemp -d)`: `export_nodes {withinId}`, `curl -o` từng `url`, `file *.jpg *.webp`, dọn.

## Rủi ro

- Chất lượng và kích thước JPG/WebP phụ thuộc encoder Chromium. Không so byte, chỉ so ngưỡng (AC1–3).
- Nền trắng JPG cố định. Nếu design cần nền màu khác thì phase sau thêm `background`.
- `withinId` trên frame rất lớn quét toàn subtree (≤ 2 048 node render, tối đa toàn project). Chi phí nhỏ so với compute, không cần index thêm.

## Ngoài phạm vi / phase sau

- PDF: Chromium `page.pdf()` của SVG (P03b) có nhúng font. Cần phase riêng: nhúng font, kích thước trang = bounds, kiểm text còn là vector.
- `background` cho JPG.
- WebP lossless.
- Lọc `find_nodes` theo "có prop".
- Zip nhiều file.
