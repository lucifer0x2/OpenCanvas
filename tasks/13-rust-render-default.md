# Task 13 — Đổi mặc định sang engine Rust khi chạy native trên mac

**Trạng thái:** CHƯA GIAO. Đóng gói 2026-10-29.
**Phụ thuộc:** Task 10, 11, 12 đã nghiệm thu và người dùng đồng ý đổi mặc định.
**Phạm vi sở hữu:** `crates/layout/src/lib.rs` (`RENDERER_VERSION`), `renderer/types.ts` (hằng số version tương ứng),
nơi đặt mặc định backend trong `crates/server`, `crates/server/src/fingerprint.rs`, cờ renderer mặc định trong `web/src/`,
`scripts/app.mjs`, `compose.yaml` (chỉ ghim backend cũ), test/fixture có hard-code version
(`web/tests/state.test.ts`, `web/tests/fixture-*.mjs`, `spikes/pipeline/README.md`), golden/E2E cần chụp lại,
`README.md`, `docs/ACCEPTANCE.md`, `docs/SERVER.md`, `docs/VIEWER.md`, `docs/PURE-RUST-EXPORT.md`.
Không xóa `renderer/`, `worker/`, đường `chromium` hay `canvas2d` (để một chu kỳ dùng ổn định rồi gỡ bằng task riêng).
Không sửa `PLAN.md`, `AGENTS.md`.
**Workspace:** worktree riêng, nhánh mới từ nhánh nền có Task 10–12. Không push khi chưa được yêu cầu.

## Bối cảnh

Sau Task 10–12: backend `rust` tự đo text, dựng scene và xuất PNG; viewer `renderer=wasm` vẽ bằng cùng crate. Cả hai
vẫn sau cờ, mặc định là đường Node + Chromium cũ. Phạm vi đã chốt là macOS chạy native (xem `docs/PURE-RUST-EXPORT.md`);
Linux/Docker để sau.

Điểm cần đổi cùng lúc (đọc lại trước khi sửa):

- `RENDERER_VERSION = "web-2"` ở `crates/layout/src/lib.rs` và `renderer/types.ts`; cũng hard-code trong
  `web/tests/state.test.ts`, `web/tests/fixture-data.mjs`, `web/tests/fixture-dense.mjs`, `spikes/pipeline/README.md`.
  Viewer TS kiểm version này (`renderer/drawing.ts:19`), nên bump phải đi cùng viewer.
- `crates/server/src/fingerprint.rs:30` chạy `node` + Playwright lúc khởi động để lấy identity; với backend `rust`
  bước này không cần và không được làm server lỗi khi thiếu Node/Chromium.
- `docs/ACCEPTANCE.md` có các khẳng định "0 RGBA differences" của pipeline cũ (ví dụ dòng 14): là bằng chứng lịch sử,
  không được sửa thành bằng chứng mới khi chưa đo lại.
- `compose.yaml` và `Dockerfile` (Linux) chưa thuộc phạm vi: phải ghim backend `chromium` để container giữ hành vi cũ.

## Mục tiêu

Chạy native trên mac thì mặc định dùng engine Rust cho cả export và viewer, không cần Chromium lúc chạy; đường cũ vẫn
chọn lại được để rollback.

## Việc phải làm

1. Đổi mặc định backend server sang `rust` khi chạy native; giữ cờ chọn `chromium`. Ghim `chromium` trong `compose.yaml`.
2. Bump `RENDERER_VERSION` (ví dụ `rs-1`) ở Rust và TS cùng lúc để mở cache namespace mới; cập nhật các chỗ hard-code.
3. `fingerprint.rs`: identity của backend `rust` lấy từ binary + `Cargo.lock` + crate render, không gọi Node; backend
   `chromium` giữ cách cũ.
4. Viewer: mặc định `renderer=wasm`; `canvas2d` còn chọn được.
5. `scripts/app.mjs`/README: chạy native không yêu cầu cài Chromium; Chromium chỉ cần cho test parity, E2E và backend cũ.
6. Chụp lại golden/E2E với engine mới; đo lại các mục pixel trong `docs/ACCEPTANCE.md` và ghi kết quả mới thành mục
   riêng có ngày, giữ bằng chứng cũ.
7. Ghi trong `docs/PURE-RUST-EXPORT.md`: ngày đổi, cách rollback, danh sách việc gỡ đường cũ cho task sau.

## Nghiệm thu

### Acceptance Criteria

- Máy không có `.runtime/browsers`: chạy native, `render_frame` và `export_nodes` ra PNG, viewer hiển thị bằng WASM.
- Rollback: chọn lại `chromium` + `canvas2d` rồi restart thì chạy như trước đổi (test hiện có pass ở chế độ này).
- Container compose vẫn dùng `chromium`, smoke Docker không đổi kết quả.
- Không còn chỗ hard-code `web-2` ngoài tài liệu lịch sử; cache cũ không bị dùng cho engine mới.
- Golden/E2E mới pass; `docs/ACCEPTANCE.md` có số đo mới, có ngày.
- File sửa ≤ 250 dòng hoặc nêu lý do; Clippy nghiêm ngặt, fmt, diff-check, `npm --prefix web test` sạch.

### Cách kiểm

```bash
cargo test --workspace
```

```bash
npm run test:e2e
```

Thử thêm với `PLAYWRIGHT_BROWSERS_PATH` trỏ thư mục rỗng để chắc đường mặc định không cần Chromium (test cần browser
bỏ qua có ghi rõ). Kiểm live bằng server tạm `--port 0` + data root tạm.

### Đầu ra

- Mã, golden mới, tài liệu cập nhật; báo cáo cuối gồm cách rollback và danh sách việc gỡ đường cũ.

## Ràng buộc

- Không restart hay đổi server cổng 3847 của người dùng; không chạm volume Docker, `~/.opencanvas`.
- Không xóa mã đường cũ trong task này. Không push.
