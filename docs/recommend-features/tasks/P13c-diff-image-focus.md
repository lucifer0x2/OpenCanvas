# P13c — `compare.diffImage` (URL ảnh diff), `compare.ignore`, `compare.nodeId`: xem lệch ở đâu và chỉ so phần cần so
Ưu tiên: P2 · Kích cỡ: ~85 phút dev · Phụ thuộc: P13a (`render_frame.compare`); độc lập với P13b · Nguồn: note "Ý tưởng — Tool so sánh ảnh" (`~/Documents/opencanvas.md:84`), brief P13 "tuỳ chọn ảnh diff (URL file, không inline base64)"

Không thêm tool. Ba field con optional của `render_frame.compare` (P13a), một route HTTP GET, và một store diff giới hạn trong bộ nhớ.

## Vấn đề

P13a trả số liệu và bbox. Ba nhu cầu còn lại:

1. **Nhìn thấy chỗ lệch.**
   - Người dùng, và agent có vision khi bbox chưa đủ rõ (chữ lệch glyph, icon sai hình), cần một ảnh tô đỏ pixel lệch.
   - Ảnh phải là **URL**, không phải base64 inline: một PNG 1440×900 base64 tốn hàng chục nghìn token mỗi lần.
   - Agent chỉ tải ảnh khi cần, giống `imageUrl` của `render_frame`.
2. **Nội dung động làm nhiễu.** Đồng hồ, timestamp, con trỏ nhấp nháy, avatar và số liệu thật trong screenshot app khác với text mẫu trong design. Chúng tạo vùng lệch vĩnh viễn và chiếm chỗ trong `maxRegions`.
3. **Tập trung vào một component.** Khi đang sửa một card trong frame 18 màn hình, agent chỉ muốn % lệch và vùng lệch của card đó. Không muốn 27 vùng chữ lệch baseline ở chỗ khác (P13a đo trên DOM vs canvas) đẩy vùng của card ra `moreRegions`.

## Bằng chứng

- **Prototype P13a** (scratchpad phiên P13):
  - ảnh diff = luma tham chiếu nhạt + pixel lệch đỏ, encode `png` `Compression::Fast`;
  - thời gian encode @1x / @2x: debug thuần 267 / 998 ms, `opt-level 3` 7,9 / 28,5 ms, release 3,6 / 13 ms;
  - đã xem crop ảnh diff ca DOM vs canvas: viền glyph đỏ trên nền nhạt, đọc được ngay chữ nào lệch.
- **Ca DOM vs canvas** (P13a): 29 vùng, 27 là chữ `offset [0,2]`. Hai vùng của nút lệch 1 px **không** lọt top 8 theo số pixel; phải tăng `maxRegions`. Lọc theo `nodeId` của card chứa nút sẽ đưa nó lên đầu.
- **Code:**
  - Job PNG được phục vụ ở `/api/v1/jobs/{job}/image` (`crates/server/src/http.rs:46,106-108`), lưu trong record tối đa 32 job (`docs/SERVER.md:388`).
  - Router và guard ở `http.rs:36-53,131-158`. Đường dẫn không chứa `/assets/` thì chỉ cho same-origin hoặc không có Origin.
  - `AppState` ở `crates/server/src/state.rs` (94 dòng) là nơi đặt store dùng chung.
  - Scene node có `bounds`, `parentId`, `clip` (`renderer/types.ts:47-63`), đủ để lấy vùng của một node và cây con của nó.
- **Tham khảo:**
  - pixelmatch xuất ảnh diff với `alpha` 0.1 (độ đậm ảnh gốc), `diffColor` đỏ, `aaColor` vàng. https://github.com/mapbox/pixelmatch
  - Playwright `toHaveScreenshot` có `mask: Locator[]` (phủ hộp màu `maskColor` lên phần tử động) và `clip`. https://playwright.dev/docs/api/class-pageassertions
  - `mcp-design-comparison` có rect loại trừ theo toạ độ design và ghi file diff image. https://glama.ai/mcp/servers/w01fgang/mcp-design-comparison/tools/compare_design

## Giải pháp

Ba field mới trong `compare` (P13a), đều optional, bỏ qua = hành vi P13a:

```json
"compare":{"referenceAssetId":"asset_…","diffImage":true,
  "ignore":[{"nodeId":"clock"},{"x":1200,"y":16,"width":200,"height":24}],
  "nodeId":"card-2"}
```

### 1. `diffImage: true` → `comparison.diffImageUrl`

Ảnh PNG cùng kích thước pixel với tham chiếu (sau `referenceRect` của P13b nếu có):

| Pixel | Màu |
| --- | --- |
| lệch | `#ff0000` |
| bị `ignore` hoặc ngoài vùng `nodeId` | `#c7d2fe` mờ (blend 50 % trên luma nhạt) |
| viền 1 px ảnh của mỗi vùng trả về | `#ff00ff`, chỉ vẽ lên pixel **không** lệch, để số pixel đỏ vẫn đúng bằng số pixel lệch |
| còn lại | luma tham chiếu nhạt: `255 − (255 − Y)/4` |

Store:
- `DiffStore` trong `AppState`, `Mutex<VecDeque<(id, Vec<u8>)>>`, tối đa **8** ảnh và **64 MiB**; quá giới hạn thì bỏ cũ nhất.
- `id = "diff_" + 16 hex đầu của sha256(jobId, sha256 tham chiếu, các tham số compare đã chuẩn hoá)`. Cùng phép so → cùng URL, ghi đè không nhân bản.

Route `GET /api/v1/diffs/{id}`:
- trả `image/png`;
- không còn thì 404 `DIFF_NOT_FOUND` "Diff image expired; repeat the comparison with diffImage:true".

Không trả đường dẫn file; không base64. Encode trong `spawn_blocking` cùng phép so.

### 2. `ignore: [...]` (tối đa 16 phần tử)

Mỗi phần tử là **đúng một** trong hai dạng; dạng khác → `INVALID_REQUEST` "ignore item needs nodeId or x,y,width,height":
- `{x,y,width,height}`: CSS px frame-local;
- `{nodeId}`: bounds hiển thị của node trong scene của job, nới 2 px, giống Playwright `mask`.

Pixel trong vùng ignore (đổi sang px ảnh, cắt theo khung):
- không bao giờ là pixel lệch;
- **không** tính vào mẫu số của `mismatchedPercent`;
- trong tìm offset được coi là khớp.

Mặt nạ ignore là một `Vec<bool>` đưa vào `imagediff::compare`. `nodeId` không có trong scene → `INVALID_REQUEST` có tên id.

### 3. `nodeId`: chỉ so một node

- Vùng so = bounds hiển thị của node trong scene của job, nới **8 CSS px** (đủ chỗ cho offset ±4 của P13a), cắt theo frame.
- Cả hai ảnh được cắt đúng vùng đó trước khi so.
- `mismatchedPercent` tính trên vùng đó; toạ độ vùng lệch vẫn là CSS px của frame.
- `nodeIds` chỉ lấy từ node đó và cây con của nó (cùng thứ tự ưu tiên P13a).
- Node không thuộc frame → `INVALID_REQUEST` "nodeId is not in frame <frameId> at this revision".
- `comparison.area: {x,y,width,height}` báo vùng đã so, chỉ khi có `nodeId`.

### Vì sao chọn cách này
- **Field con của `compare`, không thêm tool:** cùng vòng gọi, cùng job.
- **URL thay vì base64:** chi phí token chỉ phát sinh khi agent chủ động tải ảnh. Đồng bộ với `imageUrl`.
- **Store trong bộ nhớ, giới hạn cứng, thay vì file trên đĩa:** không lộ đường dẫn (`docs/SERVER.md:50`), không phải dọn file, không đụng data root. Mất khi restart là chấp nhận được, vì ảnh luôn tạo lại được.
- **`ignore` nhận cả `nodeId`:** agent biết id node động (đồng hồ) nhưng không biết toạ độ của nó trong screenshot.
- **Không dùng `ignoreRegions` theo toạ độ ảnh:** toạ độ frame CSS px khớp `inspect_layout` và vùng lệch đã trả, nên agent chép thẳng bbox của một vùng nhiễu vào `ignore`.
- **`nodeId` là cắt vùng, không phải render riêng node:** giữ đúng nền và tổ tiên như trong frame; export riêng node là việc của P03a.

## Phạm vi file

| File | Tạo/sửa | Ước lượng |
| --- | --- | --- |
| `crates/imagediff/src/diff_png.rs` | tạo | ~80: vẽ ảnh diff + viền vùng + encode PNG Fast |
| `crates/imagediff/src/lib.rs` | sửa | +15: `Options.ignore: Option<Vec<bool>>`, `crop(rect)`, `diff_png()` |
| `crates/imagediff/src/pixels.rs`, `offset.rs` | sửa | +10 mỗi file: bỏ qua pixel ignore |
| `crates/imagediff/tests/synthetic.rs` | sửa | +60 |
| `crates/server/src/compare/focus.rs` | tạo | ~110: ignore → mask px ảnh, `nodeId` → vùng cắt + lọc cây con, kiểm id |
| `crates/server/src/compare/diff_store.rs` | tạo | ~80: `DiffStore` (8 ảnh / 64 MiB), id, get |
| `crates/server/src/compare.rs` | sửa | +20: gọi focus, store, thêm `diffImageUrl`/`area` |
| `crates/server/src/state.rs` | sửa | +3: field `diffs` |
| `crates/server/src/http.rs` | sửa | +3: route `GET /api/v1/diffs/{id}` (handler trong `diff_store.rs`) |
| `crates/server/src/schemas/render.rs` | sửa | +8: 3 field trong `compare` |
| `docs/SERVER.md`, 2 skill | sửa | +10 / +3 dòng |
| `crates/server/tests/render_compare_focus.rs` | tạo | ~170, Chromium thật |

Mọi file mới ≤ 200 dòng; `http.rs` (179) tăng ≤ 3 dòng; `state.rs` ≤ 100.

## Tương thích MCP

- Không thêm tool; `render_frame.compare` thêm 3 field optional.
- Ngân sách: `tools/list` tăng ≤ 700 B (ước lượng: ignore ~330 B, nodeId ~100 B, diffImage ~25 B, mệnh đề description ~200 B).
- Không có 3 field này thì response giữ nguyên byte so với P13a.
- HTTP thêm một route GET; không sửa route cũ.
- Cập nhật `docs/SERVER.md` (render_frame compare + HTTP API), skill `author-frame.md`/`implement-frame.md` (ignore nội dung động; `nodeId` khi sửa một component; chỉ tải `diffImageUrl` khi bbox chưa đủ).

## AC

Fixture: như P13a AC3 (nút dịch 4 px), thêm text `clock` (x=1300, y=20, "12:00" trong design, "12:01" trong ảnh tham chiếu) và card `card-2` chứa `card-2-button` lệch 1 px như ca DOM-nudge.

1. **Ngân sách `tools/list`.** `serde_json::to_vec(&schemas::tools()).len()` tăng ≤ 700 B so với sau P13a; số tool không đổi.
2. **diffImage tắt.** Không truyền `diffImage` → không có `diffImageUrl`; store không thêm phần tử (test qua `AppState`).
3. **diffImage bật.** `diffImage:true` → `comparison.diffImageUrl` dạng `…/api/v1/diffs/diff_<16 hex>`. `GET` trả:
   - `200 image/png`, kích thước = ảnh tham chiếu;
   - số pixel đúng `#ff0000` chia tổng pixel × 100, làm tròn 2 chữ số, bằng `mismatchedPercent`;
   - có pixel `#ff00ff` trên viền mỗi vùng trả về.
4. **Cùng phép so cùng URL.** Gọi lại cùng tham số → cùng URL, store vẫn 1 phần tử.
5. **Giới hạn store.**
   - 9 phép so khác nhau → URL đầu trả 404 `DIFF_NOT_FOUND`, 8 URL sau trả 200.
   - Tổng bytes trong store ≤ 64 MiB (unit test với ảnh lớn giả).
6. **Không inline.** Không có field nào trong response chứa chuỗi base64 > 200 ký tự (quét chuỗi trong JSON).
7. **ignore theo node.** Không ignore → có vùng `nodeIds[0]=="clock"`. Với `ignore:[{nodeId:"clock"}]`:
   - không vùng nào giao bounds `clock`;
   - `mismatchedPercent` giảm đúng bằng phần của clock;
   - mẫu số loại bỏ diện tích ignore: unit test so với công thức `lệch / (tổng − ignore)`.
8. **ignore theo rect.** `ignore` bằng đúng bbox vùng của nút dịch 4 px (copy từ response trước) → vùng đó biến mất; vùng còn lại không đổi toạ độ.
9. **ignore sai.**
   - `{nodeId:"x", x:1}` hoặc `{}` → `INVALID_REQUEST`;
   - `nodeId` không tồn tại → `INVALID_REQUEST` chứa id;
   - 17 phần tử → `INVALID_REQUEST`.
10. **nodeId.**
    - Ca DOM-nudge (P13a, 29 vùng) với `nodeId:"card-2"` → các vùng đều trong bounds `card-2` nới 8 px;
    - vùng của `card-2-button` có mặt với `offset [-1,0]` ngay ở `maxRegions` mặc định 6;
    - `comparison.area` = bounds `card-2` nới 8 px, cắt theo frame;
    - mọi `nodeIds` thuộc cây con `card-2`.
11. **nodeId ngoài frame.** Node của frame khác → `INVALID_REQUEST` "nodeId is not in frame …".
12. **Hiệu năng.** `perf` test (ignored) của imagediff: thêm `diff_png` @1x ≤ 40 ms, @2880×1800 ≤ 120 ms (prototype opt-level 3: 7,9 / 28,5 ms). `nodeId` vùng 400×400 trên frame 1440×900 nhanh hơn so cả frame (in số đo).
13. **Tương thích.** Toàn bộ test P13a (`render_compare`, `tool_schemas`, `render_response`) pass không sửa kỳ vọng, trừ con số gốc của test ngân sách.
14. **Modular.** File mới ≤ 200 dòng; `cargo clippy -p opencanvas-imagediff -- -D warnings` sạch.

## Cách kiểm

```bash
W=/Volumes/MySpace/Workspace/OpenCanvas/.night/wt/P13c
CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-imagediff --manifest-path $W/Cargo.toml
CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-imagediff --test perf --manifest-path $W/Cargo.toml -- --ignored --nocapture
CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-server --test render_compare_focus --test render_compare --test tool_schemas --manifest-path $W/Cargo.toml
```

Kiểm live trên server tạm (`--port 0 --data-root $(mktemp -d)`):
1. `render_frame` + `compare {diffImage:true}`, tải `diffImageUrl` bằng curl, mở xem.
2. Lặp lại với `ignore` và `nodeId`; ghi byte response.
3. Dọn process và thư mục tạm.

## Rủi ro

- **Bộ nhớ.** 8 ảnh diff 2880×1800 nén Fast, ước 0,5–2 MB mỗi ảnh. Trần 64 MiB chặn trường hợp xấu.
- **Ảnh mất sau restart hoặc bị đẩy ra.** Đã có lỗi `DIFF_NOT_FOUND` hướng dẫn so lại.
- **`nodeId` dùng bounds trong render.** Node dịch xa hơn 8 px thì nội dung tham chiếu nằm ngoài vùng cắt, phần đó thành "thiếu" chứ không thành offset. Ghi trong description: nếu vùng lệch chạm mép `area`, bỏ `nodeId` và so lại.
- **`ignore` có thể giấu lỗi thật.** Agent không được ignore vùng chỉ vì khó sửa. Skill nói rõ: chỉ dùng cho nội dung động.
- **Route GET mới không xác thực**, cùng mức với `/api/v1/jobs/{job}/image`. id 64 bit ngẫu nhiên-tất định (hash), không lộ dữ liệu ngoài ảnh của chính project.

## Ngoài phạm vi / phase sau

- Viewer hiển thị vùng lệch hoặc overlay diff.
- Lưu ảnh diff bền vững.
- Diff nhiều frame một lần.
- Mask theo `kind` (mọi text).
- Ảnh diff dạng heat-map hoặc side-by-side.
