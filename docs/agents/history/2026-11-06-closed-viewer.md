# Nhật ký đã đóng — viewer và Compose 2026-11-05

### Đồng bộ PR #7 với main — 2026-11-05

- Người dùng yêu cầu merge main; giao fresh worker hòa conflict trong worktree tạm, giữ nguyên checkout/cache dev ở thư mục gốc. Giữ capabilities hover/responsive của main, bỏ khai báo animations trùng và bảo toàn nhật ký hai nhánh.
- Nhận hai file đã hòa, không còn unmerged; root kiểm web68/68 và typecheck đạt. Kiểm Rust capabilities dùng target tạm riêng; chỉ push sau gate rồi dọn đúng worktree/cache tạm.
- Điểm dừng trước merge commit: Rust capabilities1/1, scoped fmt/diff đạt; capabilities khớp main và toàn bộ dòng nhật ký hai parent còn đủ. Duyệt commit merge main04f6a05 vào nhánh PR, push rồi xác nhận GitHub hết conflict và dọn worktree/target tạm; target dev gốc vẫn1,5GiB.

### Compose Rust native mặc định và dọn worktree — 2026-11-05

- Người dùng yêu cầu bật backend Rust native trong Compose, push rồi xóa worktree viewer riêng. Giao fresh worker sửa hai cấu hình SQLite/PostgreSQL, giữ override Chromium và kiểm merge public-viewer bằng env tạm. Không restart/deploy app thật, không sửa checkout chính; chỉ dọn worktree sau khi commit đã xác nhận trên remote.
- Nhận cấu hình, worker kiểm16 tổ hợp và đối chiếu toàn bộ config còn lại với HEAD; root kiểm thêm bốn trường hợp public-overlay mặc định/override, đều đạt. Hai Compose mặc định Rust, giữ env override Chromium; docs đã cập nhật, diff sạch, email đúng yêu cầu, AGENTS trong ngân sách.
- Điểm dừng trước commit: duyệt commit năm file cấu hình/tài liệu và push lên đúng nhánh; sau khi remote khớp HEAD và worktree sạch sẽ xóa worktree cùng artifact generated. Không có build/process đang dùng worktree; `.runtime` là symlink nên chỉ bỏ liên kết, giữ thư mục đích ở checkout chính. App thật và volume Docker không được restart/xóa.

### WASM scene mặc định — 2026-11-05

- Người dùng chốt WASM làm mặc định, server chỉ fallback; giao fresh worker sửa cờ chọn engine và regression unit/browser, giữ override server tường minh. Chỉ worktree viewer riêng; không sửa checkout chính hoặc runtime thật. Nhánh đã push theo yêu cầu trước đó, cập nhật sau khi kiểm đủ thay đổi này.
- Nhận thay đổi; root kiểm độc lập web68/68, typecheck, production build và WASM browser smoke đều đạt. URL thường không gọi scene API; module lỗi fallback một lần, server override không tạo Worker/tải WASM; storage thiếu/lỗi vẫn chọn WASM.
- Điểm dừng trước commit: source/test tối đa239 dòng, reference VIEWER.md cũ254 dòng chỉ sửa mô tả, AGENTS trong ngân sách; diff/format sạch. Email Git đúng yêu cầu; duyệt commit nhỏ và cập nhật nhánh remote đã được người dùng cho push, không artifact/trailer hoặc deploy.

### Docker browser scene WASM — 2026-11-05

- Người dùng yêu cầu build WASM trong image, loại artifact host khỏi context và phục vụ precompressed static; giao fresh workers cho Docker/build scripts, static HTTP và backstop Worker. Chỉ worktree viewer riêng, không push hoặc sửa main checkout; kiểm Compose dùng project/env/data tạm.
- Image baseline `opencanvas:kubernetes-local` hiện có 354.279.699 byte; sẽ đo thêm một build baseline cùng nguồn trước khi so kích thước. Email Git đã xác nhận đúng yêu cầu.
- Nhận ba lát triển khai: shared script giữ flags release, Docker build tạo sidecars, static service negotiate br/gzip riêng với MIME gốc, và Worker có post-time backstop giữ deadline qua replay. Root kiểm web66/66, renderer11/11, binding2/2 và WASM browser smoke đạt; image mới build được, chuyển sang Compose tạm để kiểm end-to-end.
- Compose/browser probe đạt: WASM Brotli đúng MIME, Worker vẽ71 node, không gọi scene API; static regression4/4 và parity20/20 không mismatch đạt. Lượt Rust cuối gặp đầy disk; chỉ dọn cache incremental generated của worktree rồi chạy lại không incremental. OrbStack đã dừng/socket mất sau đó; hỏi người dùng trước khi restart dịch vụ dùng chung để dọn canary, không tự restart.
- Người dùng báo OrbStack đã chạy lại; root xác nhận và dọn đúng canary/container/network tạm. Rust cuối460 pass/0 fail/36 ignored, gồm server249; đủ gate yêu cầu. [Báo cáo Docker](../../VIEWER-WASM-DOCKER.md) ghi size/time, artifact transfer và lệnh kiểm lại; chờ cache check/format cuối trước commit.
- Điểm dừng trước commit: cache build10,49s và final image/browser probe đạt; canary đã dọn, format/syntax/diff sạch. Source/test tối đa224 dòng, AGENTS69 dòng/9476 byte; LOG append-only hiện275 dòng theo ngân sách riêng. Duyệt một commit cục bộ từ đúng source đã kiểm, email đúng yêu cầu, không artifact/attribution, không push hoặc sửa main checkout.
