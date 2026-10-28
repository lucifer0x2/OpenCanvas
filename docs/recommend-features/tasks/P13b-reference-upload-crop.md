# P13b — Đưa ảnh tham chiếu vào gần 0 token: HTTP upload raw, `compare.referenceRect` crop, cảnh báo ICC
Ưu tiên: P2 · Kích cỡ: ~70 phút dev · Phụ thuộc: P13a (`render_frame.compare`) · Nguồn: note "Ý tưởng — upload PNG tham chiếu" (`~/Documents/opencanvas.md:84`)

Bản 01:58: theo ngân sách `tools/list`, không thêm tool. Upload là route HTTP (không vào `tools/list`). `referenceRect` là field con của `render_frame.compare` (P13a).

## Vấn đề
- `import_asset` chỉ nhận base64 trong tool call (`crates/server/src/domain.rs:32-56`, `schemas.rs:136`). Một PNG 1440×900 khoảng 100 KB thành ~133 KB base64, tức hàng chục nghìn token mỗi lần agent đưa screenshot vào.
- Screenshot thường chứa cả khung cửa sổ, nên kích thước không khớp frame và gây `REFERENCE_SIZE_MISMATCH` (P13a). Hiện agent phải tự cắt ảnh bằng công cụ ngoài rồi import lại.
- Screenshot macOS thường gắn ICC Display P3 (đã biết, chưa đo trên máy này), làm lệch màu hệ thống. Playwright/CDP xuất sRGB: worker chạy `--force-color-profile=srgb`.

## Bằng chứng
- Router chỉ có GET asset (`http.rs:43`). Body limit 15 MiB (`http.rs:51`). Guard Host/Origin ở `http.rs:131-158`: đường dẫn chứa `/assets/` được coi là resource chỉ đọc và cho phép origin loopback bất kỳ.
- Đường commit hiện có: `domain::mutate` → `import_asset` → phần đuôi chung (event `commit`, prerender) ở `domain.rs:61-89`, và `with_commit_fields` ở `domain.rs:94-102`. Lỗi idempotency là `IDEMPOTENCY_KEY_REUSED` (`crates/storage/src/db.rs:109`).

## Giải pháp
1. **`POST /api/v1/projects/{project}/assets?expectedRevision=N&idempotencyKey=K`**
   - Body là bytes thô; `Content-Type` ∈ `image/png|image/jpeg|image/webp`. Không có font: font cần metadata, vẫn đi qua MCP.
   - Tách phần commit của arm `import_asset` thành `domain::import(state, ImportAssetRequest)`. MCP và HTTP dùng chung hàm này, gồm cả event và prerender. Response JSON giống hệt MCP `import_asset`.
   - Giới hạn 10 MiB (`MAX_ASSET_BYTES`).
   - Ví dụ:
     ```bash
     curl -s --data-binary @shot.png -H 'content-type: image/png' "$BASE/api/v1/projects/$P/assets?expectedRevision=12&idempotencyKey=ref-welcome-1"
     ```
   - Đường dẫn không có `/` sau `assets`, nên guard coi đây là mutation: chỉ same-origin hoặc không có header Origin (curl).
   - Handler đặt trong file mới `http_upload.rs`; `http.rs` chỉ thêm `.post(upload)`.
2. **`render_frame.compare.referenceRect {x,y,width,height}`** tính theo px **ảnh tham chiếu** (số nguyên ≥ 0, width/height ≥ 1).
   - Crop trước khi suy scale (enqueue) hoặc kiểm kích thước (poll), nên mọi quy tắc của P13a áp dụng cho phần đã cắt.
   - Rect phải nằm trong ảnh; nếu không thì `INVALID_REQUEST` "referenceRect must lie inside the 1640×1000 reference".
   - Toạ độ vùng trong `comparison` vẫn là CSS px của frame, không phải của ảnh gốc.
   - Thêm `Image::crop` trong `crates/imagediff`. Lúc suy scale, `compare/reference.rs` dùng kích thước rect thay cho IHDR.
3. **Chẩn đoán màu.**
   - `decode_png` đọc `Info` của crate png (`icc_profile`, `srgb`).
   - Có iCCP mà bytes profile không chứa chuỗi ASCII "sRGB", và không có chunk sRGB, thì thêm `comparison.diagnostics` (mảng chuỗi, chỉ có khi khác rỗng) phần tử `"REFERENCE_COLOR_PROFILE: reference PNG has a non-sRGB ICC profile; pixels are compared as stored values. Capture with Playwright/CDP or convert to sRGB."`
4. **Tài liệu, ít byte cố định.**
   - Lệnh curl đặt trong `docs/SERVER.md` và 2 skill (đọc khi cần, không tốn token cố định).
   - `import_asset` description chỉ thêm một mệnh đề: " Large files: POST raw bytes to /api/v1/projects/{projectId}/assets?expectedRevision=&idempotencyKey= with Content-Type." (~110 B).
   - `render_frame` schema thêm `referenceRect` trong `compare` (~170 B).

## Phạm vi file
- `crates/server/src/http_upload.rs` (tạo, ~90)
- `domain.rs` (tách hàm, ±20, vẫn < 150 dòng)
- `http.rs` (+2)
- `compare/reference.rs` (+25)
- `crates/imagediff/src/decode.rs` (+30)
- `schemas/render.rs` (+1 field `referenceRect` trong `compare`), `schemas.rs` (+1 mệnh đề description `import_asset`)
- `docs/SERVER.md`, 2 skill
- tests: `crates/server/tests/upload_asset.rs` (~140), +3 case trong `render_compare.rs`, unit test ICC trong imagediff

## Tương thích MCP
- Không đổi hành vi hay schema `import_asset`; chỉ thêm một mệnh đề vào description.
- `render_frame.compare` thêm một field optional (`referenceRect`).
- Số tool không đổi; `tools/list` tăng ≤ 300 B (AC10).
- Route HTTP mới chỉ thêm, không sửa route cũ.
- Cập nhật `docs/SERVER.md` (mục HTTP API và render_frame) và 2 skill.

## AC
1. POST PNG hợp lệ:
   - HTTP 200, `asset.id` có, `committedRevision == N+1`;
   - tập key giống response MCP `import_asset` cho cùng bytes (project khác);
   - event `commit` được phát (SSE test như `tests/changes.rs`).
2. Replay cùng key và cùng body → `replayed:true`, cùng `asset.id`, revision không đổi. Cùng key nhưng body khác → `IDEMPOTENCY_KEY_REUSED`.
3. `expectedRevision` cũ → cùng mã lỗi và HTTP status như MCP (`REVISION_CONFLICT`, 409).
4. Validate đầu vào:
   - `Content-Type: font/ttf` hoặc thiếu → 400 `INVALID_REQUEST`, message liệt kê 3 media type hợp lệ;
   - chữ ký PNG gửi với type `image/jpeg` → cùng lỗi validation như MCP;
   - body > 10 MiB → lỗi "Asset exceeds 10 MiB".
5. Guard:
   - `Origin: http://127.0.0.1:1` → 403 `ORIGIN_REJECTED`;
   - `Host: example.com` → 403 `HOST_REJECTED`;
   - GET asset với origin loopback của worker vẫn 200 (`tests/multi_resources.rs` pass).
6. `referenceRect`:
   - ảnh 1640×1000 chứa PNG render frame 1440×900 dán tại (100,60), với rect `{100,60,1440,900}` → `mismatchedPercent:0`;
   - không truyền rect → `REFERENCE_SIZE_MISMATCH`;
   - `x+width > 1640` → `INVALID_REQUEST` có "1640×1000".
7. `referenceRect` ở 2x: ảnh 3280×2000, frame tại (200,120) cỡ 2880×1800 → `scale:2`, 0 vùng.
8. ICC:
   - PNG có iCCP với bytes không chứa "sRGB" → `diagnostics` có phần tử bắt đầu bằng `REFERENCE_COLOR_PROFILE`;
   - có chunk sRGB, hoặc iCCP chứa "sRGB", hoặc không có chunk màu → không có diagnostic đó;
   - số vùng/% lệch không đổi so với khi không có chunk.
9. Test P13a và `tests/render.rs`, `multi_resources.rs` pass. File mới ≤ 200 dòng. `http.rs` tăng ≤ 3 dòng.
10. **Ngân sách `tools/list`.** `serde_json::to_vec(&schemas::tools()).len()` tăng ≤ 300 B so với sau P13a; số tool không đổi. Test nằm trong `tests/tool_schemas.rs`, cập nhật giá trị gốc.
11. **Token khi đưa ảnh vào.** Test ghi lại hai số: độ dài body JSON của lời gọi MCP `import_asset` cho PNG 1440×900 của fixture (≈ 4/3 × kích thước file), và độ dài dòng lệnh curl tương ứng (≤ 300 B). Số liệu chép vào `.night/dev/P13b.md`.

## Cách kiểm
```bash
CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-imagediff
CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-server --test upload_asset --test render_compare --test multi_resources --test render
```
Kiểm live: server tạm `--port 0 --data-root $(mktemp -d)`, upload bằng curl, gọi `render_frame` với `compare.referenceRect` (rồi poll cùng `compare`), rồi dọn.

## Rủi ro
- Mở thêm một đường ghi qua HTTP không xác thực. Rủi ro ngang `/mcp` (cùng guard loopback/Origin). Nếu đặt đường dẫn có `/assets/…` thì guard sẽ cho origin loopback lạ ghi: AC5 khoá điều này.
- Phát hiện ICC bằng chuỗi "sRGB" chỉ là heuristic. Nó chỉ cảnh báo, không đổi màu.

## Ngoài phạm vi
- Chuyển đổi màu ICC → sRGB.
- Upload font qua HTTP.
- Tự tìm vị trí frame trong screenshot (auto-crop).
- P13c (`P13c-diff-image-focus.md`): `compare.diffImage` URL, `compare.ignore`, `compare.nodeId`.
