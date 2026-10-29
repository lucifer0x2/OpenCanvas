# Task 09 — Spike WASM: painter Rust chạy trong trình duyệt có đủ nhỏ, đủ nhanh và ra cùng pixel không

**Trạng thái:** CHƯA GIAO. Đóng gói 2026-10-29. Là gate cho Task 11 (yêu cầu build được WASM) và Task 12 (viewer WASM).
**Phụ thuộc:** `spikes/rust-export/` và `docs/PURE-RUST-EXPORT.md` phải có trên nhánh nền (lúc đóng gói chúng chưa commit,
nằm trên `codex/p24-components-instances`). Cần target `wasm32-unknown-unknown`: máy lúc đóng gói **chưa cài**;
`rustup target add wasm32-unknown-unknown` sửa toolchain chung, phải hỏi người dùng trước.
**Phạm vi sở hữu:** chỉ `spikes/rust-export/` (thêm thư mục `wasm/` hoặc tách `src/lib.rs`, script đo trong `js/`),
cập nhật `spikes/rust-export/README.md` và mục kết quả trong `docs/PURE-RUST-EXPORT.md`.
Không sửa `crates/`, `renderer/`, `web/`, `worker/`, `PLAN.md`, `AGENTS.md`.
**Workspace:** worktree riêng, nhánh mới từ nhánh nền mới nhất lúc nhận việc. Không push khi chưa được yêu cầu.

## Bối cảnh

Hướng đã chọn (xem `docs/PURE-RUST-EXPORT.md`, mục "Phương án"): **một engine render Rust cho hai đích**. Server build
native để xuất PNG; viewer build WASM để hiển thị. Viewer chỉ để xem, agent tự code lại UI theo frontend của dự án đích,
nên viewer cần giống ảnh export chứ không cần giống Chrome từng pixel. Phạm vi hiện tại: macOS, chạy native.

Spike trước đã có painter tiny-skia + text HarfRust/skrifa (647 dòng, `spikes/rust-export/src/`), nhưng chỉ build native:
release 2 107 408 byte, render + PNG 2,40 ms (1×) và 7,10 ms (2×) cho scene 400×300. Chưa biết ba điều quyết định
tính khả thi của viewer WASM:

1. Kích thước `.wasm` (painter + shaper + font parser + PNG decode).
2. Thời gian vẽ một khung lớn trong trình duyệt. Viewer hiện vẽ vector mỗi frame qua Konva
   (`web/src/components/CanvasView.tsx:152`–`:158`); bản WASM sẽ vẽ lại khi zoom dừng, nên cần biết một lần vẽ tốn bao lâu.
3. Native và WASM có ra cùng pixel không. tiny-skia dùng đường SIMD khác nhau theo target (NEON trên arm64, scalar hoặc
   `simd128` trên WASM).

## Mục tiêu

Ra số đo và kết luận go/no-go cho viewer WASM, không viết mã sản phẩm.

## Việc phải làm

1. Tách phần render của spike thành thư viện build được cho cả native và `wasm32-unknown-unknown`. Không đọc file, không
   thread trong phần thư viện; font và ảnh truyền vào dạng bytes. Ưu tiên export C-ABI thủ công (`alloc`, `render`) để
   khỏi cài thêm `wasm-bindgen-cli`; nếu dùng wasm-bindgen thì ghi version và lý do.
2. Build release WASM hai biến thể: không SIMD và `-C target-feature=+simd128`. Bật `opt-level` hợp lý, LTO, strip.
   Đo kích thước raw, gzip -9, brotli. Tách số đóng góp chính nếu dễ (ví dụ `twiggy` nếu đã có; không bắt buộc cài).
3. Scene đo: lát scene spike (`data/scene.json`) thành lưới phủ khung 1440×900 (khoảng 12 card có text, viền, ảnh,
   clip, nhóm opacity). Render ở DPR 1 và DPR 2, cộng một vùng 1/4 khung (mô phỏng vẽ lại một tile).
4. Chạy trong Chromium pin của repo (`.runtime/browsers`, Playwright từ `node_modules`), trong **Web Worker**: đo
   instantiate, lần render đầu (cold) và p50/p95 của 20 lần warm. Đo cùng scene ở native release để so.
5. So RGBA native với WASM (cả hai biến thể) trên scene spike 1×/2× và khung lớn: số pixel Δ>0, max Δ. Nếu lệch, tìm
   nguyên nhân (đường SIMD, FMA, thứ tự cộng) và ghi lại; không sửa tiny-skia.
6. Ghi bảng số vào `spikes/rust-export/README.md` và một đoạn kết luận vào `docs/PURE-RUST-EXPORT.md`.

## Nghiệm thu

### Acceptance Criteria

- Có số đo cho đủ: kích thước 2 biến thể (raw/gzip/brotli); instantiate; cold và p50/p95 warm ở 1440×900 DPR 1, DPR 2
  và tile 1/4; native cùng scene; native↔WASM Δ.
- Ghi rõ go/no-go theo ngưỡng đề xuất, hoặc đề xuất ngưỡng khác có lý do:
  - `.wasm` gzip ≤ 2 MB (chưa gồm font).
  - Khung 1440×900 DPR 2, p50 warm trong Worker ≤ 200 ms; tile 1/4 ≤ 60 ms.
  - Native↔WASM: max Δ ≤ 1/255; nếu không bằng 0 thì nêu nguyên nhân.
- Không đổi file ngoài phạm vi; không cài công cụ toàn cục ngoài target rustup đã được người dùng cho phép.

### Cách kiểm

```bash
cargo build --release --manifest-path spikes/rust-export/Cargo.toml
```

```bash
cargo build --release --target wasm32-unknown-unknown --manifest-path spikes/rust-export/Cargo.toml
```

Script đo Chromium chạy bằng Node trong `spikes/rust-export/js/`, dùng server HTTP tạm cổng 0, đóng browser sau khi đo.
Root kiểm lại bằng cách chạy lại script đo và đối chiếu số trong README.

### Đầu ra

- Mã spike WASM trong `spikes/rust-export/`, bảng số trong README, kết luận go/no-go trong `docs/PURE-RUST-EXPORT.md`.
- Báo cáo cuối: số chính, go/no-go, việc chưa làm được.

## Ràng buộc

- Không chạm server cổng 3847, Docker, `~/.opencanvas`. Không ghi dữ liệu thật.
- Không tối ưu sớm painter quá mức cần để đo; nếu số gần ngưỡng, ghi các tối ưu khả dĩ (cache glyph, layer theo bounds)
  thay vì làm hết.
- Đây là spike: mã không cần đạt chuẩn production, nhưng file mới vẫn ≤ 250 dòng.
