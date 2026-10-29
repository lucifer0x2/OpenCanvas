# P13a — `render_frame.compare`: so render của frame với PNG tham chiếu → % lệch, vùng lệch, node, offset
Ưu tiên: P1 · Kích cỡ: ~90 phút dev · Phụ thuộc: B01, B04 (đã merge); không chạm P03a · Nguồn: note "Ý tưởng — Tool so sánh ảnh" (`~/Documents/opencanvas.md:84`), BACKLOG "khả năng tự kiểm bằng ảnh"

Bản 01:55 (29/10) thay bản 00:10: theo ngân sách `tools/list`, **không thêm tool `compare_render`**; mở rộng `render_frame` bằng object optional `compare` (xem mục "Vì sao tool mới").

Chuỗi P13:
- **a** — so ảnh lõi (file này);
- **b** — đưa tham chiếu vào rẻ token: HTTP upload, `compare.referenceRect`, cảnh báo ICC (`P13b-reference-upload-crop.md`);
- **c** — `compare.diffImage` URL, `compare.ignore`, `compare.nodeId` (`P13c-diff-image-focus.md`).

Mỗi phase dùng được riêng; b và c chỉ cần a, không cần nhau.

## Vấn đề

Phiên vẽ lại QueryNest (18 frame, ~6 900 node) làm vòng lặp: `render_frame` → tải PNG → **so bằng mắt** với screenshot app (`opencanvas.md:5-6`, `:84`: "Hiện phải nhìn bằng mắt").
- Nhìn bằng mắt tốn token ảnh: mỗi PNG 1440×900 đưa vào context là hàng nghìn token, và mỗi vòng sửa lại tốn lại.
- Nó không chỉ ra **node nào** sai, sai **bao nhiêu px**, sai **màu gì**. Lỗi 1–4 px, màu gần đúng hay baseline chữ lệch 2 px gần như không thấy được.

Hiện không có cách nào so ảnh: `inspect_layout` ghi rõ "No layout explanation or diff in v1" (`crates/server/src/schemas.rs:153`). Agent cần một lời gọi trả về:
- % lệch tổng;
- ≤ N vùng lệch (bbox theo toạ độ frame);
- node nằm ở mỗi vùng;
- khi vùng lệch chỉ là nội dung bị dịch: dịch bao nhiêu.

Tất cả gọn ≤ ~2 KB.

## Bằng chứng

### Code (worktree `night/integration` @ `dcfb7ad`)

Render job:
- Lưu PNG trong bộ nhớ: `Entry.png` (`crates/server/src/jobs.rs:37-41`), ghi ở `jobs/record.rs:34-37`.
- Đọc bằng `Jobs::image(id)` (`jobs.rs:179-193`) và `Jobs::get` (`jobs.rs:169-177`). Scene đầy đủ vẫn nằm trong record.
- Chờ bằng `Jobs::wait` ≤ 10 000 ms (`jobs/wait.rs:17-23`).
- Canvas có kích thước `ceil(viewport × scale)` (`renderer/drawing.ts:63-64`). Ngân sách: scale ≤ 4, ≤ 16 M pixel (`jobs/input.rs:59-80`).
- Enqueue `Direct` dùng lại job cùng `sceneKey` nếu chưa lỗi (`jobs.rs:113-125`).

`render_frame`:
- Hai mode loại trừ nhau, enqueue và poll (`schemas/render.rs:9-37`). `tools/list` công bố hợp phẳng của hai mode; field dùng chung phải có ràng buộc giống nhau (test `schemas/render.rs:47-63`).
- Response được định hình ở `render_response.rs:13-35`: lấy `includeScene` ra khỏi args, gọi `mcp::render`, rồi `shape`. Đây là đúng chỗ móc `compare`.
- Poll có `waitMs` (`mcp.rs:140-153`).

Scene, asset và lỗi:
- Scene node có `id`, `parentId`, `kind`, `bounds` (frame-local, CSS px), `clip`, `opacity` (`renderer/types.ts:47-63`).
- `import_asset` nhận `image/png|jpeg|webp` (`schemas.rs:136`); chỉ PNG có width/height trong IHDR (`crates/storage/src/assets.rs:202-207`). Đọc bytes bằng `Store::read_asset` (`assets.rs:17-26`).
- Lỗi có chi tiết: `ApiError::with_detail` (`error.rs:28`).

Decode PNG:
- `Cargo.lock` **không có** crate ảnh hay deflate nào; `jobs/png.rs:4-28` chỉ đọc IHDR.
- Cache cargo cục bộ có `png-0.18.1` và đủ dependency (fdeflate, flate2, miniz_oxide, crc32fast, adler2, simd-adler32), nên `cargo build --offline` chạy được (đã build prototype).

### Prototype và số đo

Prototype nằm ở scratchpad, không commit; thuật toán lõi chép ở phụ lục.
- Server tạm `--port 0 --data-root <tmp>`. Frame 1440×900 dạng dashboard: header, sidebar 8 text, 6 card mỗi card có title, body 3 dòng, chart rect và nút; tổng 43 node. Các biến thể render bằng `render_frame`.
- "App thật" giả lập bằng **screenshot DOM** (Playwright Chromium 1248, cùng font NotoSans, div absolute theo bounds scene) ở DPR 1 và 2.

| Ca | Kết quả prototype (threshold 0.063, AA-ignore) |
| --- | --- |
| Ảnh giống hệt | 0 pixel, 0 vùng |
| Nút (radius 6) dịch +4 px | 2 vùng 36 px cao tại x=676 và x≈792; `nodeIds[0]="card-4-button"`; `offset [4,0]` cả hai. Bề rộng 7–8 px do góc bo; rect không bo cho đúng 4 px. |
| Title đổi màu #111827→#dc2626 | 1 vùng 146×18 đúng bbox chữ; `nodeIds=["card-1-title"]`; màu ref/render = `#111827`/`#dc2626` chính xác |
| Text nav dài thêm "(mới)" | 1 vùng 33×12; `nodeIds=["nav-3"]` (sidebar nền bị loại vì chứa node đã chọn) |
| Chart #dbeafe dịch 1 px | threshold 0.1 (mặc định pixelmatch): **0 vùng** (ΔYIQ≈300 < 352). Threshold 0.063: 2 vùng 1 px cao, `offset [0,1]`, đúng node |
| **DOM screenshot vs render canvas** | 27 vùng, mỗi vùng đúng **một** text node, tất cả `offset [0,2]`: chữ canvas thấp hơn DOM đúng 2 px. Dò offset nguyên trên crop: dịch (0,2) cho sai số **0** tuyệt đối. Đây chính là lỗi baseline note #5, tool tự phát hiện. |
| DOM có nút lệch +1 px | ngoài 27 vùng chữ, có thêm 2 vùng 1 px của `card-2-button`, `offset [-1,0]` |
| Không gom ô, không dò offset (bản đầu) | DOM case gộp thành vùng 1335×304 trỏ `frame` → vô dụng. Vì vậy cần ô cố định 8 CSS px, top-N và offset theo vùng. |

Thời gian (máy dev 11 core):

| Build | @1x (1440×900) | @2x (2880×1800) |
| --- | --- | --- |
| debug thuần | decode 2 PNG 323 ms, diff 30 ms | decode 1 266 ms, diff 118 ms |
| `opt-level=3` cho crate so ảnh và png (dev profile) | tổng **≈ 40 ms** (decode 9–13, diff 2–6, vùng 0,7, offset + map 24) | tổng **≈ 93 ms** (decode 33, diff 15, vùng 2,5, offset + map 43) |
| release | decode 6 ms, diff 1,1 ms | decode 22 ms, diff 4,3 ms |

Kích thước response prototype: 111 B khi 0 vùng; 394 B khi 2 vùng; 1 334 B khi 8 vùng (đã gồm màu cho mọi vùng).

### Tham khảo
- pixelmatch (ISC): `threshold` mặc định 0.1, `includeAA:false`; bộ dò anti-alias theo Vyšniauskas (2009). Bản mới chuyển metric sang OKLab/HyAB. https://github.com/mapbox/pixelmatch
- Playwright `toHaveScreenshot`: `threshold` mặc định 0.2, "perceived color difference in the YIQ color space"; thêm `maxDiffPixels`, `maxDiffPixelRatio`. https://playwright.dev/docs/api/class-pageassertions
- Chromatic: `diffThreshold` mặc định **0.063**, mặc định bỏ qua pixel anti-alias (`diffIncludeAntiAliasing`). https://docs.chromaticqa.com/threshold
- MCP cùng loại:
  - `mcp-design-comparison` (w01fgang): pixelmatch + SSIM, bbox diff + heat grid, rect loại trừ, scale-to-fit. https://glama.ai/mcp/servers/w01fgang/mcp-design-comparison/tools/compare_design
  - `mcp-perfectpixel`: vùng lệch gom nhóm, gắn với phần tử DOM (thấy qua tìm kiếm web, chưa kiểm repo).
  - Ta làm như perfectpixel: gắn vùng lệch với **node** của scene.

## Giải pháp

### 1. `render_frame` nhận thêm object optional `compare` (cả hai mode)

```json
{"projectId":"p_…","revision":5,"frameId":"frame","viewport":{"width":1440,"height":900},
 "compare":{"referenceAssetId":"asset_…"}}
```
rồi poll:
```json
{"projectId":"p_…","jobId":"job_…","waitMs":10000,"compare":{"referenceAssetId":"asset_…"}}
```

| `compare.` | Kiểu | Mặc định | Ý nghĩa |
| --- | --- | --- | --- |
| `referenceAssetId` **hoặc** `referenceJobId` | id | đúng một trong hai | Asset PNG đã import, hoặc PNG của một render job cùng project. Job dùng để so hai revision, ví dụ kiểm refactor `free`→stack có đổi pixel không. |
| `threshold` | 0–1 | **0.063** (Chromatic) | Khác biệt màu YIQ tối thiểu để tính là lệch. |
| `maxRegions` | 1–32 | 6 | |

`additionalProperties:false`. Không có field nào khác của `render_frame` đổi nghĩa.

Xử lý (module mới `compare/`, gọi từ `render_response::render`, làm như với `includeScene`):

1. **Tách và kiểm `compare`.** Lấy `compare` ra khỏi args trước khi xử lý mode. Có cả hai field reference, hoặc không có field nào → `INVALID_REQUEST` "compare needs exactly one of referenceAssetId or referenceJobId".
2. **Lấy tham chiếu.**
   - Asset: `read_asset`. `media_type` khác `image/png` → `REFERENCE_NOT_PNG` ("Import the screenshot as PNG").
   - Job: phải cùng project và có ảnh; nếu không thì giữ lỗi của `Jobs::image`.
3. **Enqueue mode: suy scale, không resample.**
   - Đọc IHDR → `(rw, rh)`, đặt `s = rw / viewport.width`.
   - Hợp lệ khi `ceil(vw·s − 1e-9) == rw`, `ceil(vh·s − 1e-9) == rh`, `0 < s ≤ 4` và `rw·rh ≤ 16 M`.
   - Có `scale` trong args: phải bằng `s`. Không có `scale`: đặt `scale = s` rồi enqueue như cũ.
   - Sai thì `REFERENCE_SIZE_MISMATCH`, trả **trước** enqueue:
     - message gồm hai kích thước, kích thước mong đợi ở scale 1 và 2, và hướng sửa: "crop the reference to the frame, or pass viewport = reference size ÷ device pixel ratio";
     - `details: {reference:[rw,rh], viewport:[vw,vh]}`.
   - Lý do: renderer vẽ được ở mọi scale ≤ 4, nên render đúng mật độ của screenshot (2x Retina → scale 2) khớp pixel thật; resample thì sinh nhiễu anti-alias ở mọi cạnh.
4. **Poll mode.** Kích thước PNG của job phải bằng kích thước tham chiếu, nếu không thì `REFERENCE_SIZE_MISMATCH` với hint "enqueue with compare so the scale is inferred". Scale lấy từ `image.width / provenance.viewport.width`.
5. **Job chưa ready** (pending, failed, hoặc ready mà không có PNG): trả job record như hiện nay, **không** có `comparison`. Description nói: "poll with jobId, waitMs and the same compare".
6. **Job ready có PNG: so ảnh.** Chạy trong `spawn_blocking`, tối đa 2 phép so đồng thời (semaphore tĩnh trong module), bằng crate mới `opencanvas-imagediff`:
   1. **Decode** PNG → RGB 8-bit, trộn alpha trên nền trắng (quy ước pixelmatch), cho cả hai ảnh.
   2. **Mask lệch.** Pixel lệch khi ΔYIQ > `35215·threshold²` (Kotsarenko–Ramos, như pixelmatch ≤ v6 và Playwright) **và** không phải anti-alias ở cả hai ảnh (bộ dò pixelmatch/Vyšniauskas).
   3. **Vùng.**
      - Lưới ô 8 CSS px (= `8·round(s)` px ảnh); ô có ≥ 1 pixel lệch là ô bẩn.
      - Liên thông 8 hướng → bbox **chặt** theo pixel lệch thật.
      - Sắp theo số pixel lệch giảm dần, giữ `maxRegions`.
      - **Không** nới ô: bản prototype đầu nới ô và gộp mọi chữ thành một vùng vô dụng.
   4. **Offset theo vùng** (chỉ trên ≤ 32 vùng lớn nhất).
      - Thử dịch nguyên `(dx,dy) ∈ [−4,4]²` CSS px (bước `round(s)` px ảnh).
      - Mẫu tối đa ~20 k pixel/lần thử; dừng sớm khi vượt kết quả tốt nhất.
      - Có `d ≠ 0` khử ≥ 90 % pixel lệch → `offset:[dx,dy]`, nghĩa là **nội dung render nằm lệch `[dx,dy]` CSS px so với tham chiếu** (dương = phải/xuống).
   5. **Màu.** Vùng không có offset có thêm `referenceColor`/`renderColor`: màu xuất hiện nhiều nhất trong các pixel lệch của mỗi ảnh.
7. **Gắn node.** Hàm thuần trong crate, nhận box từ `scene.nodes` của job.
   - Bounds hiển thị = bounds ∩ tổ tiên `clip:true`. Bỏ node diện tích 0 và node có `opacity:0` (kể cả do tổ tiên).
   - Ứng viên là node thoả **một** trong hai:
     - phủ ≥ 50 % vùng, tính với bounds nới 2 px;
     - có một cạnh nằm trong vùng ±2 px và phủ ≥ 50 % cạnh tương ứng của vùng (vùng "bỏ trống" khi node dịch).
   - Sắp: nhóm phủ ≥ 50 % trước, trong nhóm diện tích nhỏ trước.
   - Bỏ ứng viên là tổ tiên **hoặc chứa trọn** node đã chọn (nền, card cha).
   - Lấy ≤ 3 → `nodeIds`.
8. **Định hình response khi có `compare`.**
   - Giữ `jobId`, `status`, `projectId`, `revision`, `frameId`, `sceneKey`, `imageUrl`, `error` (nếu có), `diagnostics` (chỉ khi không rỗng) và `comparison`.
   - Bỏ `provenance`, `readiness`, `image`. Bỏ `scene`, trừ khi `includeScene:true`.
   - Lý do: vòng so lặp nhiều lần, mỗi lần tiết kiệm ~1–3 KB metadata job (B04 đo ~3,6 KB/response ở frame 300 node).
   - Không có `compare` thì response giữ nguyên byte.
9. **Không side-effect.** Không đổi revision, không tạo asset. Job là job render thường; đọc bounds bằng `inspect_layout {jobId, sceneKey}`.

### 2. Response (ready + compare)

```json
{"jobId":"job_…","status":"ready","projectId":"p_…","revision":5,"frameId":"frame","sceneKey":"…",
 "imageUrl":"http://127.0.0.1:3847/api/v1/jobs/job_…/image",
 "comparison":{"scale":2,"mismatchedPercent":1.84,
  "regions":[
   {"x":293,"y":148,"width":299,"height":53,"mismatch":0.16,"offset":[0,2],"nodeIds":["card-0-body"]},
   {"x":677,"y":113,"width":146,"height":18,"mismatch":0.12,"nodeIds":["card-1-title"],"referenceColor":"#111827","renderColor":"#dc2626"}],
  "moreRegions":21,"offsets":[{"offset":[0,2],"regions":27}]}}
```

- Toạ độ vùng là **CSS px frame-local, số nguyên** (floor x/y, ceil cạnh phải/dưới), cùng hệ với `inspect_layout`, kể cả khi `scale:2`.
- `mismatchedPercent` là % pixel lệch, 2 chữ số thập phân. **Một** số thay cho cặp `score` + `mismatchedPercent` trong brief, vì hai số đo cùng một thứ dễ làm agent hiểu sai. Không lệch = `0` và `regions:[]`.
- `mismatch` của vùng: tỉ lệ pixel lệch trong bbox (0–1). Thấp là chữ hoặc rải rác; ≈ 1 là khối đặc.
- Field chỉ xuất hiện khi có dữ liệu:
  - `offset`;
  - `referenceColor`/`renderColor` (chỉ khi không có offset);
  - `moreRegions` (> 0);
  - `offsets`: offset mà ≥ 2 vùng cùng có, tối đa 3, đếm trong 32 vùng lớn nhất. Một dòng nói "lệch hệ thống", ví dụ baseline chữ 2 px.

### 3. Vì sao chọn cách so này
- **Rust trong server, không trong worker Chromium.**
  - Worker chỉ chạy 1 việc một lúc và do runner độc chiếm. So ảnh trong worker cần thêm loại job vào `jobs/queue.rs`/`runner.rs`/`record.rs` (vừa refactor ở B01/F-B01-1) và gửi 2 ảnh base64 qua NDJSON.
  - Rust thuần cho unit test xác định, không cần Chromium, chi phí 40/93 ms.
  - Giá phải trả: chỉ nhận PNG, thêm 7 crate thuần Rust nhỏ.
- **Crate riêng `crates/imagediff`** để đặt `[profile.dev.package.*] opt-level = 3` chỉ cho nó và cho png. Server test và README chạy debug (`README.md:17`); không có override thì 1 lần so @2x mất ~1,4 s.
- **Không dùng crate `image`**: kéo theo nhiều codec.
- **YIQ + bỏ AA + 0.063.** Cùng ngữ nghĩa ngưỡng với Playwright và Chromatic. Ngưỡng 0.1 bỏ sót màu pastel dịch 1 px (đo ở trên).
- **SSIM để sau.** SSIM theo ô khó giải thích, không ra bbox/offset trực tiếp.
- **Offset theo vùng biến nhiễu thành thông tin hành động được.** DOM so với canvas trở thành "27 text cùng lệch (0,+2)", và nút lệch 1 px vẫn hiện riêng.

## Vì sao tool mới

**Kết luận: không thêm tool.** Chọn phương án (a), mở rộng `render_frame` bằng object `compare`. Đây là phương án ít byte nhất mà agent vẫn hiểu.

Đo baseline trên server tạm từ binary `night/integration` (build 00:08, HEAD lúc đo `ec1fc01`), JSON compact của mảng `result.tools` trong `tools/list`:
- 15 tool, **22 509 B**;
- trong đó `render_frame` 1 917 B (description 1 008 ký tự), `inspect_layout` 1 406 B.

Byte thêm của từng phương án ước lượng bằng `JSON.stringify(...).length` trên schema dựng thử theo đúng helper `id()`/`object()` của `schemas.rs`:

| Phương án | Byte thêm vào `tools/list` | Agent có hiểu không | Đánh giá |
| --- | --- | --- | --- |
| **(a) `render_frame` + `compare {referenceAssetId\|referenceJobId, threshold, maxRegions}`, response thêm khối `comparison`** (chọn) | **~710** (+3,2 %): object `compare` ~390 + 1 câu description ~320 | Rõ: agent vốn gọi `render_frame` để kiểm frame; thêm một object là có so sánh | Dùng lại viewport, theme, revision, poll `waitMs`, pending, `QUEUE_FULL`/`retryAfterMs`, dedupe job. Hai mode sẵn có xử lý pending tự nhiên. |
| (b) Mode của `inspect_layout`: `reference {assetId\|jobId}` so PNG của job đã có | ~520 (+2,3 %) | Kém: agent phải tự tính scale (2x screenshot → render scale 2) và render trước ở đúng scale, thêm một vòng gọi, dễ sai | Trộn ngữ nghĩa "đọc bounds phân trang theo node" với kết quả so ảnh. Description hiện ghi "No layout explanation or diff in v1", phải viết lại. |
| (c) Tool mới `compare_render` (bản 00:10) | ~1 410 (+6,3 %) | Rõ, tự đủ | Lặp lại viewport/theme/revision/waitMs, tự chế pending ("gọi lại cùng tham số"). Gấp đôi (a) mỗi phiên, response chỉ gọn hơn ~200 B. |

(a) rẻ hơn (c) ~700 B mỗi phiên agent và ít hơn (b) một bước gọi lẫn một phép tính dễ sai. P13b/P13c chỉ thêm field con vào `compare`, mỗi phase có AC byte riêng (≤ 300 B và ≤ 700 B).

## Phạm vi file

| File | Tạo/sửa | Ước lượng |
| --- | --- | --- |
| `Cargo.toml` (workspace) | sửa | +1 member; `[profile.dev.package.X] opt-level = 3` cho `opencanvas-imagediff`, `png`, `fdeflate`, `miniz_oxide`, `crc32fast`, `adler2`, `simd-adler32` (chỉ crate có trong lock) |
| `Cargo.lock` | sửa | sinh offline |
| `crates/imagediff/Cargo.toml` | tạo | `png = "0.18"` (dependency duy nhất) |
| `crates/imagediff/src/lib.rs` | tạo | ~80: `Image`, `Options`, `Region`, `Comparison`, `compare()` |
| `crates/imagediff/src/decode.rs` | tạo | ~60: `png_size` (IHDR), `decode_png` |
| `crates/imagediff/src/pixels.rs` | tạo | ~110: ΔYIQ, AA, mask |
| `crates/imagediff/src/regions.rs` | tạo | ~110: lưới, liên thông, top-N, màu, `offsets` |
| `crates/imagediff/src/offset.rs` | tạo | ~60 |
| `crates/imagediff/src/nodes.rs` | tạo | ~100: `NodeBox`, `nodes_for` |
| `crates/imagediff/tests/synthetic.rs` | tạo | ~180 |
| `crates/imagediff/tests/perf.rs` | tạo | ~50, `#[ignore]` |
| `crates/server/Cargo.toml` | sửa | +1 dep path |
| `crates/server/src/compare.rs` + `compare/reference.rs` + `compare/shape.rs` | tạo | ~120 + ~80 + ~80: parse spec, suy scale/prepare (enqueue), attach (ready), định hình response, scene → `NodeBox` |
| `crates/server/src/render_response.rs` | sửa | 64 → ~85: tách `compare`, gọi prepare/attach |
| `crates/server/src/schemas/render.rs` | sửa | 107 → ~120: `compare` vào cả `enqueue()` và `poll()` (cùng ràng buộc), +1 câu `DESCRIPTION` |
| `crates/server/src/lib.rs` | sửa | +1 (`mod compare`) |
| `crates/server/src/instructions.rs` | sửa | +1 câu: "To check a frame against a screenshot, render_frame with compare …" |
| `crates/server/skills/author-frame.md`, `implement-frame.md` | sửa | +4 dòng mỗi file |
| `docs/SERVER.md` | sửa | +1 mục ~25 dòng |
| `crates/server/tests/render_compare.rs` | tạo | ~190, Chromium thật |
| `crates/server/tests/tool_schemas.rs` | sửa | +1 test ngân sách byte |

Mọi file mới ≤ 200 dòng; không file sửa nào vượt 250.

## Tương thích MCP

- `tools/list`: số tool không đổi. Schema `render_frame` thêm đúng một property `compare`; mọi tool khác giống hệt.
- Field cũ giữ nghĩa. Không có `compare` thì request, response và job như trước, byte-identical. Không đổi HTTP, job table, `inspect_layout`. Không có migration.
- Cập nhật: `docs/SERVER.md` (mục render_frame), `instructions.rs`, 2 skill.

## AC

Fixture chung (test Rust, Chromium thật): frame `frame` 1440×900 fill `#ffffff`, các node con:
- `button`: rectangle **radius 0**, x=676, y=808, 120×36, `#2563eb`;
- `title`: text `#111827`, x=96, y=96, width 400, size 16/22;
- `card`: frame 360×376 radius 8 chứa `chart` rect `#dbeafe`.

Mỗi lần so dùng hàm test `compare(s, args)`: enqueue có `compare`, rồi poll `{jobId, waitMs:10000, compare}`.

1. **Ngân sách `tools/list`.**
   - Test mới trong `crates/server/tests/tool_schemas.rs`: `serde_json::to_vec(&schemas::tools()).len()` (toàn mảng tools) tăng **≤ 800 B** so với baseline của HEAD `night/integration` mà nhánh P13a tách ra.
   - Dev đo baseline trước khi sửa, ghi hằng số vào test kèm comment HEAD. Tham chiếu lúc viết đề xuất: 22 509 B / 15 tool, nên trần ≈ **23 309 B** nếu baseline không đổi.
   - Ghi cả hai số (trước/sau) vào `.night/dev/P13a.md`.
   - Số tool không đổi; mọi tool khác `render_frame` giống hệt.
   - Published `render_frame` vẫn là hợp phẳng hợp lệ (test `schemas/render.rs` pass).
2. **Giống hệt.**
   - Ref = PNG của job `render_frame` cùng revision, viewport, scale 1 (tải qua `imageUrl`, `import_asset`); so ở revision mới nhất → `comparison.mismatchedPercent == 0`, `regions == []`, không có `moreRegions`/`offsets`.
   - `referenceJobId` của chính job đó → như trên.
3. **Dịch 4 px.** `update_node button x=680` → đúng **2** vùng:
   - vùng 1: `x∈[674,678]`, `width∈[2,6]`, `y∈[806,810]`, `height∈[34,38]`;
   - vùng 2: `x∈[794,798]`, cùng ràng buộc kích thước;
   - cả hai: `nodeIds[0]=="button"`, `offset==[4,0]`, không có `referenceColor`.
4. **Đổi màu.** `title fill #dc2626` → 1 vùng trong bounds `title` ±2 px; `nodeIds[0]=="title"`; `referenceColor=="#111827"`, `renderColor=="#dc2626"`; không có `offset`.
5. **Pastel 1 px.** `chart y+1` → ≥ 1 vùng, `nodeIds` chứa `chart`, `offset==[0,1]`. Với `compare.threshold:0.1` → `regions:[]` (khoá mặc định 0.063).
6. **Scale 2.** Ref = PNG `render_frame scale:2` (2880×1800); enqueue **không** truyền `scale` → job có `image` 2880×1800 (HTTP `/api/v1/jobs/:id`), `comparison.scale==2`, vùng theo CSS px như AC3 ±2.
7. **Khác kích thước.**
   - Ref 1440×901 → `REFERENCE_SIZE_MISMATCH`; message chứa `1440×901` và `1440×900`; `details.reference==[1440,901]`; lỗi không kèm `jobId`. Unit test `compare/reference.rs` chứng minh lỗi trả trước enqueue.
   - Ref 2x + `scale:1` → `REFERENCE_SIZE_MISMATCH`.
   - Poll job scale 1 với ref 2x → `REFERENCE_SIZE_MISMATCH`, message chứa "enqueue with compare".
   - Ref 1280×800 + `viewport {1280,800}` → ready, `scale 1`.
8. **Lỗi đầu vào.**
   - `compare` không có reference, hoặc có cả hai → `INVALID_REQUEST`.
   - Key lạ trong `compare` → `INVALID_REQUEST`.
   - Asset JPEG → `REFERENCE_NOT_PNG`.
   - `referenceJobId` của project khác → `INVALID_REQUEST`.
9. **Pending và tương thích.**
   - Enqueue + `compare` khi job mới → `status:"pending"`, không có `comparison`.
   - Poll cùng `compare` → `ready` + `comparison`, cùng `jobId`.
   - Poll **không** `compare` → không có `comparison`, response byte-identical với trước thay đổi (so chuỗi JSON với `tests/render_response.rs`).
10. **Offset hệ thống** (unit test imagediff): 10 "khối chữ" (mẫu đen/trắng giả ngẫu nhiên cố định bằng LCG, 40×12; không dùng mẫu tuần hoàn) dịch (0,2) và 1 rect đặc dịch (−1,0), `max_regions:16` →
    - `offsets` có `{offset:[0,2], regions:10}`;
    - vùng của rect có `offset [-1,0]`;
    - không vùng nào gộp hai khối cách nhau ≥ 16 px.
11. **Anti-alias** (unit test): cạnh trắng|đen có cột trung gian 128 so với 168 → 0 pixel lệch; `include_aa:true` (tham số nội bộ) → > 0.
12. **Gắn node** (unit test `nodes_for`):
    - vùng bỏ trống bên trái rect dịch 4 px → rect trước;
    - text trên rect nền anh em chứa nó → chỉ `[text]`;
    - vùng chia đôi hai anh em → cả hai, nhỏ trước;
    - con bị `clip` của cha không nhận vùng ngoài cha;
    - `opacity:0` không bao giờ được chọn.
13. **Gọn.** Đo `serde_json::to_string(structuredContent).len()`:
    - ≤ 650 B khi ready, 0 vùng (AC2, có `imageUrl`);
    - ≤ 1 900 B khi 6 vùng: unit test `compare/shape.rs` với 27 vùng chữ, `jobId`/`sceneKey`/`imageUrl` dài thật;
    - mọi toạ độ là số nguyên.
14. **Hiệu năng.** `cargo test -p opencanvas-imagediff --test perf -- --ignored --nocapture`, ảnh tổng hợp 1440×900 có 30 vùng lệch + 300 `NodeBox`, decode 2 PNG + compare + nodes:
    - ≤ **60 ms** @1x và ≤ **200 ms** @2880×1800 (dev profile có override; prototype 40/93 ms);
    - in số đo cho `.night/dev/P13a.md`.
15. **Không side-effect.** Revision, `list_assets` và `get_changes` trước/sau AC2 bằng nhau.
16. **Modular/check.**
    - File mới ≤ 200 dòng; `render_response.rs` ≤ 100, `schemas/render.rs` ≤ 130.
    - `cargo test -p opencanvas-imagediff` và `cargo test -p opencanvas-server --test render_compare --test tool_schemas --test render_response --test render_wait` pass.
    - `cargo clippy -p opencanvas-imagediff -- -D warnings` sạch.
    - `cargo build --offline --locked` pass sau khi cập nhật lock.

## Cách kiểm

```bash
W=/Volumes/MySpace/Workspace/OpenCanvas/.night/wt/P13a
CARGO_BUILD_JOBS=4 cargo build --offline -p opencanvas-layout -p opencanvas-server --manifest-path $W/Cargo.toml
CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-imagediff --manifest-path $W/Cargo.toml
CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-imagediff --test perf --manifest-path $W/Cargo.toml -- --ignored --nocapture
CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-server --test render_compare --test tool_schemas --test render_response --test render_wait --manifest-path $W/Cargo.toml
```

Kiểm live trên server tạm (`--port 0 --data-root $(mktemp -d)`):
1. Tạo frame mẫu, `render_frame scale:2`, tải PNG, `import_asset`, dịch một nút 4 px.
2. `render_frame {…, compare}` → poll `{jobId, waitMs, compare}` → ghi byte response, vùng, `nodeIds`, `offset`.
3. Tuỳ chọn: screenshot DOM bằng Playwright như prototype → kỳ vọng `offsets` có `[0,2]` cho text tới khi P10 sửa baseline.
4. Dọn process và thư mục tạm.

## Rủi ro

- **Nhiễu khác nền tảng.** Worker Docker (Linux/FreeType) so với screenshot macOS: AA và glyph khác nhau, sinh nhiều vùng chữ `mismatch` thấp, không có offset. Mới đo cùng engine (Chromium mac). Giảm nhẹ: bỏ AA, `threshold`, sắp theo pixel; P13c thêm `ignore`/`nodeId`.
- **Lệch hệ thống do renderer.** Ví dụ baseline #5: mọi vùng chữ báo `offset [0,2]` cho tới khi P10 sửa; `offsets` gói nó trong 1 dòng. Skill phải nói: "offset giống nhau ở mọi text thường là renderer, đừng dịch từng node".
- **ICC.** Screenshot macOS bằng phím tắt thường gắn Display P3; Playwright/CDP là sRGB (worker chạy `--force-color-profile=srgb`). P13a so giá trị thô; P13b cảnh báo.
- **Heuristic gắn node.** UI dày đặc có thể cho node anh em lân cận. Giữ ≤ 3 `nodeIds`; agent xác minh bằng `inspect_layout nodeIds`.
- **Offset chỉ ±4 CSS px.** Xa hơn thì thành 2 vùng có màu.
- **Bộ nhớ.** 2880×1800: 2 buffer RGB 15,5 MB + mask 5 MB + 2 PNG. Semaphore 2 giới hạn ~80 MB.
- **`render_frame` phình.** Thêm một field vào tool đã có hai mode; description dài thêm ~320 ký tự. Bù lại: không thêm tool, AC1 khoá ngân sách.
- **Mỗi lần poll có `compare` đều so lại** (stateless, 40–90 ms). Chấp nhận được; không cache.
- **Token khi import.** `import_asset` base64 cho PNG 1440×900 ~100 KB tốn hàng chục nghìn token. Agent nên import bằng script như người dùng (`opencanvas.md:5-6`); P13b thêm HTTP upload.
- **Asset tham chiếu là asset mồ côi.** Nếu P12 dọn mồ côi tự động thì sẽ xoá chúng; P12 nên chỉ dọn khi được gọi rõ.
- **Lock và Docker.** `Dockerfile` dùng `cargo build --locked`, nên phải commit `Cargo.lock`.
- **DPR không nguyên** (1,25): trình duyệt có thể làm tròn khác `ceil`. Message chỉ cách truyền `viewport`.

## Ngoài phạm vi / phase sau

- P13b: HTTP upload PNG raw, `compare.referenceRect`, chẩn đoán ICC.
- P13c: `compare.diffImage` → `comparison.diffImageUrl`, `compare.ignore`, `compare.nodeId`.
- Sau nữa: JPEG/WebP; SSIM theo ô; global auto-align; so nhiều frame một lần; viewer hiển thị vùng lệch.

## Phụ lục — lõi thuật toán đã đo (prototype, rút gọn)

```rust
// ΔYIQ bình phương (Kotsarenko & Ramos 2010, như pixelmatch ≤ v6). Ngưỡng: 35215 * t * t.
fn delta(a: [u8;3], b: [u8;3]) -> f32 {
    let (dr, dg, db) = (a[0] as f32 - b[0] as f32, a[1] as f32 - b[1] as f32, a[2] as f32 - b[2] as f32);
    let y = dr*0.29889531 + dg*0.58662247 + db*0.11448223;
    let i = dr*0.59597799 - dg*0.27417610 - db*0.32180189;
    let q = dr*0.21147017 - dg*0.52261711 + db*0.31114694;
    0.5053*y*y + 0.299*i*i + 0.1957*q*q
}
// AA (pixelmatch, ISC): trong 8 ô lân cận, >2 ô cùng màu với tâm → không phải AA. Phải có cả ô sáng hơn
// lẫn tối hơn (theo luma). Pixel là AA nếu ô tối nhất hoặc sáng nhất có >2 ô kề giống hệt nó trong CẢ HAI ảnh.
// Pixel lệch khi: a != b && delta > max && !(aa(ref, render, x, y) || aa(render, ref, x, y)).
// Trộn alpha trên nền trắng: c' = 255 - (255 - c) * a / 255.

// Vùng: ô = 8*round(scale) px; ô bẩn nếu ≥ 1 pixel lệch; liên thông 8 hướng; bbox = min/max pixel lệch thật;
// sort theo số pixel giảm dần; cắt maxRegions; KHÔNG nới ô.

// Offset: stride = ceil(sqrt(area / 20000)); với mỗi d trong [-4s, 4s]² bước s (s = round(scale)), d != 0:
// n(d) = #{p trong bbox lấy mẫu : ref(p) vs render(p + d) lệch (ngoài ảnh = lệch)}, dừng sớm khi n > best.
// best = argmin n (hoà thì |dx|+|dy| nhỏ hơn). Báo offset = best/s nếu n(best) * 10 <= n(0).

// Node: wb = bounds ± 2; coverage = area(region ∩ wb) / area(region);
// edge = (cạnh trái/phải của node ∈ [r.x0-2, r.x1+2] và phủ ≥ 50% chiều cao vùng) hoặc tương tự theo chiều ngang.
// ứng viên nếu coverage ≥ 0.5 || edge; sort (coverage ≥ 0.5 trước, rồi diện tích tăng dần);
// bỏ ứng viên là tổ tiên hoặc chứa trọn node đã chọn; lấy ≤ 3.
```

Prototype đầy đủ (≈ 330 dòng, gồm script dựng frame và screenshot DOM) chỉ nằm ở scratchpad phiên P13. Dev viết lại theo các module ở trên.
