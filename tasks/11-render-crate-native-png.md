# Task 11 — Crate render Rust dùng chung (native + WASM) và đường xuất PNG không cần Chromium

**Trạng thái:** CHƯA GIAO. Đóng gói 2026-10-29.
**Phụ thuộc:** Task 10 (dùng chung `crates/text`). Task 09 phải ra **go** (WASM đủ nhỏ/nhanh, native↔WASM khớp);
nếu no-go, bỏ yêu cầu WASM ở task này và hỏi lại người dùng trước khi làm.
**Quyết định cần người dùng xác nhận khi giao:** ngưỡng pixel so với Chromium (AC thứ hai).
**Phạm vi sở hữu:** crate mới `crates/render/`, `crates/text/` (thêm outline glyph nếu cần), `Cargo.toml` gốc,
`crates/server/src/jobs/runner.rs`, `crates/server/src/jobs/export.rs` (nhánh PNG theo backend), `crates/server/src/jobs/png.rs`
nếu cần, test mới `crates/render/tests/`, `crates/server/tests/` (test so backend), script so pixel trong `spikes/rust-export/`
hoặc `crates/render/`, `docs/SERVER.md`, `docs/CONTRACT.md` (nếu đổi trường provenance), `docs/PURE-RUST-EXPORT.md`.
Không sửa `renderer/`, `worker/`, `web/`, `PLAN.md`, `AGENTS.md`; không đổi `SCENE_VERSION`/`RENDERER_VERSION`.
**Workspace:** worktree riêng, nhánh mới từ nhánh nền có Task 10. Không push khi chưa được yêu cầu.

## Bối cảnh

Hướng đã chọn: một engine render Rust cho hai đích, export native và viewer WASM (Task 12), phạm vi macOS native
(xem `docs/PURE-RUST-EXPORT.md`). Sau Task 10, backend `rust` đã tự đo và dựng scene nhưng vẫn gửi scene cho worker
Chromium vẽ PNG (`crates/server/src/jobs/runner.rs`, `export.rs:115`–`:128` gọi op `render`).

Ngữ nghĩa vẽ phải giữ, đọc từ `renderer/` (~1 070 dòng):

- `drawing.ts`: kiểm version scene/renderer và hash manifest; scale trong (0, 4]; canvas = ceil(viewport × scale),
  tổng ≤ `MAX_PIXELS` 16 000 000; strict thì lỗi `RESOURCE_NOT_READY` khi thiếu font/ảnh; nền trong suốt.
- `paint.ts`: fill `roundRect` (bán kính đơn), text từng dòng `fillText` baseline alphabetic, ảnh crop + fit
  (`image-fit.ts`) trong clip `roundRect`; placeholder "[Missing font]"/"[Missing image]" khi không strict.
- `stroke.ts`: viền kiểu CSS border: ring evenodd từ `strokeEdges`, một màu thì fill một lần, màu khác nhau thì wedge
  theo cạnh, logic `opaque` riêng cho màu trong mờ; viền vẽ sau subtree, ngoài clip của chính node.
- `layers.ts` + `scene-tree.ts`: group opacity qua layer tạm khi subtree có ≥ 2 lần vẽ, offset pixel nguyên, budget
  layer 16M pixel, vượt thì gập alpha và đếm `opacityLayerFallbacks`; clip con theo `roundRect` có AA.
- Định dạng ảnh vào: PNG, JPEG, WebP (`crates/server/src/schemas.rs:145`). Trình duyệt áp ICC khi decode.
- Kết quả op `render` của worker có `width`, `height`, `draw` (số dòng, `opacityLayers`, `opacityLayerFallbacks`) và
  provenance gồm `browserVersion`, `runtimeVersion`, `readiness` (`worker/pipeline.mjs:181`–`:224`).

Spike đã làm phần lớn trong 647 dòng (`spikes/rust-export/src/paint.rs`, `text.rs`): hình và opacity mean Δ 0,16/255,
Δ>8 chỉ ở mép AA; ảnh mean Δ ~2; text lệch mép glyph vì Chrome mac raster glyph bằng CoreText (không thể về 0).
Spike chưa có viền màu từng cạnh, decode JPEG/WebP, ICC, budget layer.

## Mục tiêu

`crates/render` là **nguồn sự thật duy nhất về pixel**: thuần (không I/O, không thread, không tokio), build cả native
và `wasm32-unknown-unknown`. Với backend `rust`, server xuất PNG (frame và `export_nodes`) không cần Node/Chromium.

## Việc phải làm

1. **API nhỏ:** nhận scene + bytes font/ảnh + scale (+ vùng tùy chọn cho Task 12), trả RGBA premultiplied/unpremultiplied
   rõ ràng và thống kê `draw`; encode PNG tách riêng. Mọi kiểm ở `drawing.ts` có mã lỗi giống hệt.
2. **Port đủ ngữ nghĩa** ở Bối cảnh, mỗi phần một module (fill/path, stroke, layers, image, text). Text dùng chung
   shaping/advance với `crates/text` để vị trí glyph khớp line list đã đo.
3. **Ảnh:** decode PNG/JPEG/WebP thuần Rust, áp ICC sang sRGB (`moxcms` hoặc `qcms`), lấy mẫu bilinear như
   `imageSmoothingEnabled` mặc định. Ghi version crate và license.
4. **Server:** backend `rust` vẽ bằng crate này trong `runner.rs` và `export.rs`; giữ shape kết quả và mã lỗi; thay
   `browserVersion`/`runtimeVersion` bằng định danh engine (cập nhật `docs/CONTRACT.md` nếu client đọc chúng).
   Backend `chromium` không đổi.
5. **So với Chromium (shadow):** script hoặc test chạy cả hai backend trên fixture có viền, clip, opacity, ảnh, text,
   lưu số liệu và ảnh ghép Chrome | Rust | diff. Thử thêm biến thể raster glyph diện tích chính xác và đường cong
   coverage (mục PhotoCraft trong `docs/PURE-RUST-EXPORT.md`); chỉ giữ nếu số đo text tốt hơn.
6. **Native ↔ WASM:** build crate cho WASM, chạy cùng fixture trong Node (WebAssembly), so RGBA với native.

## Nghiệm thu

### Acceptance Criteria

- Backend `rust` xuất PNG frame và `export_nodes` (đủ setting scale/kích thước) không spawn Node hay Chromium.
- So Chromium trên fixture: ngoài vùng text, pixel Δ>8 dưới 0,5% (đề xuất, chờ xác nhận); vùng text có ảnh ghép để duyệt.
- Native ↔ WASM: max Δ ≤ 1/255 trên mọi fixture (hoặc bằng 0 nếu Task 09 cho thấy làm được).
- Mã lỗi `PIXEL_BUDGET`, `RESOURCE_NOT_READY`, `PIPELINE_MISMATCH` và placeholder khi không strict giống renderer TS.
- `opacityLayerFallbacks` khớp TS trên fixture vượt budget.
- Backend `chromium` mặc định không đổi; test hiện có pass.
- File mới ≤ 250 dòng hoặc nêu lý do; Clippy nghiêm ngặt, fmt, diff-check sạch.

### Cách kiểm

```bash
cargo test -p opencanvas-render
```

```bash
cargo build -p opencanvas-render --target wasm32-unknown-unknown --release
```

Kiểm live: server tạm `--port 0` + data root tạm, backend `rust`, gọi `render_frame` và `export_nodes`, mở PNG;
so với backend `chromium` cùng frame.

### Đầu ra

- Crate + test + script so pixel; ảnh ghép đại diện lưu cạnh script (không commit ảnh thô lớn).
- Mục kết quả trong `docs/PURE-RUST-EXPORT.md`: bảng Δ theo vùng, kết quả biến thể glyph, native↔WASM, thời gian.
- Báo cáo cuối: phần nào chưa khớp và vì sao, số dòng file sửa.

## Ràng buộc

- Không đổi ngữ nghĩa scene hay contract HTTP/MCP; không thêm tính năng mới (gradient/shadow thuộc P09, làm sau).
- Không chạm server cổng 3847, Docker, `~/.opencanvas`; tests dùng data root tạm. Không push.
