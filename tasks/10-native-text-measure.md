# Task 10 — Đo text native trong Rust: Taffy tính layout một lượt, không cần Chromium

**Trạng thái:** CHƯA GIAO. Đóng gói 2026-10-29.
**Phụ thuộc:** không phụ thuộc Task 09. `spikes/rust-export/` và `docs/PURE-RUST-EXPORT.md` phải có trên nhánh nền.
**Quyết định cần người dùng xác nhận khi giao:** glyph thiếu trong font (mục 5 của Việc phải làm).
**Phạm vi sở hữu:** crate mới `crates/text/` (font từ bytes, shaping, wrap), `crates/layout/src/engine.rs` và module mới
cạnh nó cho đường đo đồng bộ, `crates/layout/Cargo.toml`, `Cargo.toml` gốc (members), `crates/server/src/jobs/runner.rs`
(chỉ nhánh compute theo backend), `crates/server/src/state.rs` hoặc nơi giữ `Config` (cờ backend), `crates/server/src/fingerprint.rs`,
test mới trong `crates/text/` và `crates/server/tests/`, `docs/SERVER.md`, `docs/PURE-RUST-EXPORT.md` (mục kết quả).
Không sửa `renderer/`, `worker/`, `web/`, `PLAN.md`, `AGENTS.md`; không đổi `SCENE_VERSION`/`RENDERER_VERSION`.
**Workspace:** worktree riêng, nhánh mới từ nhánh nền mới nhất. Không push khi chưa được yêu cầu.

## Bối cảnh

Hướng đã chọn: một engine render Rust, phạm vi macOS native (xem `docs/PURE-RUST-EXPORT.md`). Task này là bước 1:
thay phần **đo text** trước, vì nó quyết định layout. Chromium vẫn vẽ PNG trong task này.

Đường hiện tại (đọc lại trước khi sửa, số dòng có thể lệch):

- `crates/layout/src/engine.rs:10`–`:85`: closure measure của Taffy chỉ đọc `MeasureCache`; thiếu key thì gom vào
  `misses` và trả `LayoutStep::Pending`. `:104`–`:111`: lượt dựng scene đo lại text ở width cuối.
- `crates/layout/src/util.rs:33`–`:60`: `request()` tạo `MeasureRequest`, width lượng tử 1/64, key = hash(`RENDERER_VERSION`, request).
- `worker/pipeline.mjs:99`–`:176`: Node `spawnSync(layout-step)` **mỗi lượt**, gọi `renderer.measureBatch` trong Chromium,
  tối đa `MAX_PASSES = 6` (`crates/layout/src/lib.rs`).
- `renderer/measurement.ts`: chuẩn ngữ nghĩa phải giữ: CRLF → LF, mỗi LF là một đoạn, `trim`, gộp khoảng trắng ngang,
  wrap theo từ với `fits = round64(measureText(s)) <= maxWidth`, từ dài tách theo grapheme (`Intl.Segmenter("vi")`),
  width dòng `round64`, baseline `y = n·lh + fontSize + (lh − fontSize)/2`, height `round64(lines·lh)`.
- Font: `crates/server/src/jobs/input.rs:24`–`:35` chọn font dùng tới; font upload là asset của project
  (`crates/storage/src/assets.rs:17` `read_asset`), font mặc định `font_bundled` là `renderer/fonts/NotoSans.ttf`
  (`crates/server/src/http.rs:125`).
- Cache job: key gồm `pipeline_identity` (`crates/server/src/jobs.rs:125`–`:130`); identity tính ở
  `crates/server/src/fingerprint.rs`.

Spike đã chứng minh trên mac (`spikes/rust-export/README.md`, Chromium 156.0.8078.4, NotoSans, 1 232 cặp chuỗi×cỡ, 120 ca wrap):
HarfRust khớp `measureText` tới 0,000061 px khi **mỗi advance glyph được cắt về lưới 1/65536 px như Blink**
(`spikes/rust-export/src/font.rs:70`–`:100`, `width_fixed` biến thể 2); khi đó 0 width lệch sau round 1/64 và wrap trùng
120/120 cả text, width, baseline. Cộng advance thô thì 11 width lệch 1/64 px, đủ lật một lần xuống dòng.

## Mục tiêu

Với cờ backend `rust`, server tính scene **trong một lượt** bằng đo text native, ra scene giống hệt đường Chromium
(cùng line list, width, height, baseline, bounds), rồi vẫn gửi scene đó cho worker vẽ PNG như cũ.
Mặc định vẫn là `chromium`; không đổi output của đường mặc định.

## Việc phải làm

1. **`crates/text/`** (thuần, không I/O): nạp face từ bytes + kiểm hash; shaping HarfRust (LTR, kerning bật, như
   `fontKerning="normal"`); width theo mô hình advance của Blink; port nguyên thuật toán wrap ở trên (grapheme bằng
   `unicode-segmentation`; khoảng trắng phải khớp `\s` của JS, có test riêng cho NBSP và các khoảng trắng Unicode).
   Pin version crate (`harfrust`, `skrifa`/`read-fonts` dùng chung một bản); ghi lý do nếu khác spike.
2. **Layout:** thêm đường `compute` nhận một bộ đo đồng bộ (trait nhỏ) thay cho cache; Taffy gọi đo ngay trong closure,
   lượt dựng scene dùng lại kết quả. Giữ nguyên `compute(input, cache)` và binary `layout-step` cho đường Chromium.
   Tách module nếu `engine.rs` vượt 250 dòng.
3. **Server:** cờ chọn backend (CLI hoặc env, mặc định `chromium`). Với `rust`: nạp bytes font từ store/bundled, compute
   native, `states::apply` như cũ, rồi gọi worker `render` với scene đó (`runner.rs:65`–`:90`). Backend phải đi vào
   `pipeline_identity` để cache hai đường không lẫn nhau.
4. **Kiểm parity với Chromium** (test cần Chromium, chạy như các test render hiện có): corpus của spike
   (`spikes/rust-export/data/corpus.json`), fixture layout của repo có text, và ít nhất một font upload thật kiểu Inter.
   Trên mac, kiểm thêm một font hệ thống có bảng `trak` (ví dụ SF Pro) và ghi kết quả; Chrome mac lấy advance qua
   CoreText nên font này có thể lệch, không được giả định khớp.
5. **Glyph thiếu trong font** (Chrome âm thầm mượn font hệ thống, ví dụ "⌘" với NotoSans). Đề xuất mặc định, cần xác nhận:
   phát diagnostic `MISSING_GLYPH` kèm node và ký tự, đo bằng advance của `.notdef`, không quét font hệ thống. Các chuỗi
   này được loại khỏi AC parity và liệt kê riêng.
6. Đo thời gian compute hai đường trên cùng fixture (tham khảo, không phải AC) và cập nhật `docs/SERVER.md`.

## Nghiệm thu

### Acceptance Criteria

- Backend `rust`: line list, width, height, baseline và bounds mọi node **trùng 100%** với backend `chromium` trên corpus
  spike, fixture repo và font upload thử; ngoại lệ duy nhất là chuỗi có glyph thiếu (mục 5), được liệt kê.
- Backend `rust` không spawn `layout-step` và không gọi `measureBatch`; compute xong trong một lượt.
- Backend `chromium` (mặc định) không đổi: test render/golden hiện có pass nguyên trạng.
- Cache hai backend tách nhau qua `pipeline_identity`.
- `crates/text` và phần mới của `crates/layout` không có I/O, không thread; build được `wasm32-unknown-unknown` nếu
  target đã cài (nếu chưa, ghi rõ chưa kiểm).
- File sửa/mới ≤ 250 dòng, hoặc nêu lý do; Clippy nghiêm ngặt, fmt, diff-check sạch.

### Cách kiểm

```bash
cargo test -p opencanvas-text -p opencanvas-layout
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Test parity cần browser: `PLAYWRIGHT_BROWSERS_PATH=$PWD/.runtime/browsers` như các test render hiện có. Kiểm live bằng
server tạm `--port 0` + data root tạm, cả hai backend, gọi `render_frame` cùng frame và so scene.

### Đầu ra

- Mã + test như trên; mục kết quả trong `docs/PURE-RUST-EXPORT.md`; `docs/SERVER.md` mô tả cờ backend.
- Báo cáo cuối: bảng parity (số ca, số trùng, danh sách glyph thiếu), kết quả font `trak`, số dòng các file sửa.

## Ràng buộc

- Không đổi ngữ nghĩa đo (P10a làm sau, như một lần bump version riêng); không đổi contract HTTP/MCP.
- Không chạm server cổng 3847, Docker, `~/.opencanvas`; tests dùng data root tạm.
- Không reset/checkout/stash thay đổi của người khác; không push.
