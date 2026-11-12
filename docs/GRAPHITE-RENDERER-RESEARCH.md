# Graphite làm renderer cho OpenCanvas — đánh giá tiềm năng

Ngày khảo sát: **2026-11-04**. Nhánh `codex/compose-postgres`. Chỉ nghiên cứu: không sửa mã sản phẩm,
không build/chạy Graphite. Upstream đọc tại [GraphiteEditor/Graphite](https://github.com/GraphiteEditor/Graphite)
master 2026-11-03 (clone 200 commit gần nhất, chỉ đọc), crates.io API và bản web `editor.graphite.art`.
Bổ sung cho [EXPORT-ENGINE-RESEARCH.md](EXPORT-ENGINE-RESEARCH.md) và [PURE-RUST-EXPORT.md](PURE-RUST-EXPORT.md).

## Kết luận

**Không tích hợp Graphite, cũng không phụ thuộc crate của nó.** Đáng giá thật nằm ở hai chỗ:

1. **Vello (thư viện Graphite dùng), không phải Graphite.** Graphite chứng minh mô hình "một scene, render GPU qua
   WebGL2 trên web và wgpu trên desktop" chạy được ở quy mô lớn. Nếu mở lại hướng Rust render (Task 09–13, đang gác
   từ 2026-10-29), thêm `vello_cpu` (server) + `vello_gpu` (viewer) vào bake-off cạnh tiny-skia, đúng như phương án E.
2. **Chép có chọn lọc thuật toán** (MIT OR Apache-2.0, tương thích MIT của OpenCanvas, giữ ghi công): text → outline
   cho SVG, stroke alignment/dash, cách nhúng canvas WebGL2 vào SVG overlay.

## OpenCanvas cần gì từ renderer

| Yêu cầu | Hiện tại |
| --- | --- |
| Hình học khớp Chrome | Taffy layout; width/line break/baseline text phải trùng Chrome. Spike HarfRust đạt 0 lệch nhờ mô phỏng làm tròn advance của Chrome. |
| Ngữ nghĩa CSS | radius 4 góc giảm theo CSS, stroke từng cạnh (inside/center/outside), `box-shadow` outer/inset, gradient linear/radial nội suy premultiplied như CSS. |
| Hai đích, một hình | `renderer/*.ts` (~1 480 dòng Canvas2D) dùng chung cho viewer và worker Chromium xuất PNG. |
| Server | CPU, macOS native hoặc Docker Linux không GPU. |
| Viewer | Chỉ xem (React/Konva), cần nhẹ; preview motion chạy DOM riêng. |

## Graphite thực sự là gì

| Khía cạnh | Thực tế upstream |
| --- | --- |
| Trưởng thành | Alpha. Không có semver: mọi crate `0.0.0`, `publish = false`; release duy nhất là tag `latest-stable` (2022). ~27,8k sao, 200 commit trong 4 tuần, chủ yếu 3 người. |
| License | `MIT OR Apache-2.0` toàn workspace. |
| Toolchain | Rust stable, MSRV 1.98, edition 2024, không cần nightly. rust-gpu đã gỡ (#4873, 2026-10-30). |
| Render | Trait `Render` (`node-graph/libraries/rendering/src/renderer.rs`, 3 542 dòng) có hai backend: `render_svg` (chuỗi SVG, CPU) và `render_to_vello` → `vello_gpu` 0.3 trên **fork** `Keavon/vello`, wgpu 30. Web chạy WebGL2 từ #4807 (2026-10-25). |
| CPU raster | **Không có.** Không dùng `vello_cpu`; không GPU thì chỉ còn SVG. |
| Headless | `graphene-cli export` ra svg/png/webp/gif…, nhưng báo "No GPU executor is available" khi không có adapter wgpu, **kể cả xuất SVG**. CI cài Mesa lavapipe để chạy. |
| Text | `parley` 0.9 + `skrifa`; text box chỉ có `max_width`/`max_height`; SVG xuất glyph dạng path. Line break theo parley, không theo Chrome. |
| Mô hình | Đồ họa thủ tục bằng node graph: `List<T>` struct-of-arrays, `Vector` là pool điểm, `Appearance` là danh sách lượt Fill/Stroke, ~30 blend mode kiểu Photoshop. File `.gdd` gồm registry node + cây delta CRDT. |
| Layout | Không có flexbox/auto-layout cho nội dung ("responsive layout solver" nằm ở mốc LTS). |
| Animation | Node `animation_time` + keyframe `AnimationCurve`; panel timeline thuộc Alpha 5. |
| Frontend | Svelte 5 + wasm-bindgen; artwork là `<svg>` chứa `<foreignObject>` nhét canvas WebGL2; overlay vẽ canvas 2D riêng. |
| Kích thước web | `graphite_wasm_wrapper_bg.wasm` ~24,3 MB thô, ~5,9 MB brotli (cả editor, không riêng renderer). |
| Ghép nối | Crate `rendering` phụ thuộc vô điều kiện `core-types`, `vector-types`, `graphic-types`, `brush-types`, `text-nodes`, `dyn-any`, `usvg`, `parley` và `vello_gpu` (nên kéo cả wgpu, kể cả chỉ cần SVG); không có feature flag bỏ Vello; cần `[patch.crates-io]` sang fork. API lõi đổi nhiều lần trong 4 tuần (#4902, #4906, #4873). |

## Đánh giá từng cách tích hợp

| Cách | Backend | Frontend | Kết luận |
| --- | --- | --- | --- |
| 1. Dịch scene OpenCanvas sang tài liệu Graphite, chạy cả graph | Cần `graph-craft` + `interpreted-executor` + `graphene-std` + GPU/lavapipe; không có box-shadow CSS, stroke từng cạnh hay layout Taffy tương đương. | Bundle cỡ editor (~6 MB brotli). | **Loại.** Lệch mô hình: Graphite là editor đồ họa thủ tục, OpenCanvas là box/CSS. |
| 2. Phụ thuộc riêng crate `rendering` (git dep ghim commit) | Kéo wgpu 30 + fork Vello; không raster CPU; vẫn phải map scene sang `Graphic`/`List<Vector>`. | WebGL2 dùng được, nhưng kích thước standalone chưa đo. | **Loại.** Không publish, không version, đổi hàng tuần; chi phí rebase lớn hơn tự viết painter 647 dòng như spike. |
| 3. Dùng Vello trực tiếp (crate Linebender, có publish) | `vello_cpu` cho Docker CPU. | `vello_gpu`/WebGL2 cho viewer, vẽ lại mượt khi zoom. | **Đáng thử nếu mở lại Task 09.** Rủi ro: hai rasterizer CPU/GPU khác nhau, phải đo parity theo dung sai; Graphite không cho bằng chứng vì họ không có CPU raster. |
| 4. Chép thuật toán có chọn lọc | Text → path cho SVG (sửa `SVG_TEXT_NOT_OUTLINED`), stroke alignment/dash. | Mẫu `<svg>` + `<foreignObject>` + canvas GPU; mipmap ảnh (#4908). | **Nên làm khi có task tương ứng**, ghi công MIT/Apache, ghim commit nguồn. |

## Vì sao Graphite không giải được bài toán chính của OpenCanvas

- **Chuẩn chất lượng là Chrome.** Phần khó (khớp width/line break/baseline của Chrome) đã được giải bằng HarfRust +
  mô hình làm tròn advance. Graphite dùng parley với line break riêng, nên đổi sang sẽ làm hỏng phần đã đạt.
- **Server không GPU.** Đường raster và CLI của Graphite đều đòi wgpu adapter. Với Docker CPU phải chạy lavapipe
  (Vulkan phần mềm): thêm Mesa vào image, chưa ai đo tốc độ cho workload này. Spike tiny-skia đã ra binary 2,1 MB,
  nhanh hơn Chromium ở cold mọi cấu hình.
- **Tính năng CSS không có sẵn.** `box-shadow` (Gaussian sigma = blur/2), stroke 4 cạnh khác màu với seam kiểu Chromium,
  radius giảm theo CSS đều phải tự viết, dù chọn Graphite hay không.
- **Viewer chỉ để xem.** 24 MB WASM của editor đổi lấy năng lực chỉnh sửa mà OpenCanvas không cần.

## Nếu muốn đi tiếp

1. Giữ quyết định 2026-10-29 (phương án A, Node + Chromium) trừ khi người dùng mở lại hướng Rust render.
2. Khi mở lại: mở rộng Task 09 thành bake-off ba ứng viên trên cùng scene cố định — tiny-skia (đã có),
   `vello_cpu`, `vello_gpu` WebGL2 — đo kích thước `.wasm`, thời gian khung lớn, native↔WASM/CPU↔GPU theo dung sai
   premultiplied, như cách PhotoCraft dùng CPU làm oracle.
3. Chỉ chép mã Graphite theo từng file nhỏ, ghim hash `GraphiteEditor/Graphite`, kèm license và test riêng.

## Chưa kiểm chứng

- Kích thước thật của một build chỉ gồm crate `rendering` sang WASM.
- `graphene-cli export` có chạy được trên lavapipe trong Docker không, và tốc độ bao nhiêu.
- Desktop 1.0/Beta đã phát hành giữa tháng 3 và tháng 11/2026 chưa.
- Thời điểm Graphite thêm MIT vào license (trước 2026-10-09).

## Nguồn

- [GraphiteEditor/Graphite](https://github.com/GraphiteEditor/Graphite): `Cargo.toml`,
  `node-graph/libraries/rendering/src/{renderer.rs,vello_scene_builder.rs}`, `node-graph/graphene-cli/src/lib.rs:147`,
  `editor/src/node_graph_executor/runtime.rs:353`, `frontend/src/components/panels/Document.svelte`,
  `node-graph/rfcs/document-format.md`; PR #4807, #4873, #4880, #4902, #4906, #4908.
- [graphite.art/features](https://graphite.art/features/), [graphite.art/blog](https://graphite.art/blog/),
  [It's FOSS](https://itsfoss.com/graphite-graphics-editor/), crates.io API, [Vello](https://github.com/linebender/vello).
