# Nhật ký viewer đã đóng — lưu 2026-11-09

Chuyển nguyên nội dung các mục đã hoàn tất từ `../LOG.md`; các liên kết tương
đối tới báo cáo đã được điều chỉnh theo vị trí archive. Task stress đang mở
vẫn ở nhật ký hiện hành.

### Viewer All frames — 2026-11-06

- Người dùng yêu cầu option All trong Frame, xem đủ frame của page với tên/khoảng cách và bố cục theo kích thước. Mở worktree riêng `OpenCanvas-viewer-wasm-scene` trên `feat/viewer-wasm-scene`, giữ nguyên checkout đang bẩn ở nhánh khác.
- Giao hai worker mới: API batch scene theo page/revision có pagination giới hạn hàng đợi; frontend All dùng chung snapshot/WASM/font, bố cục cân đối, giữ tương tác và fallback server. Không commit/push/deploy hoặc dùng dữ liệu thật.
- Người dùng nhắc app thật chạy native và cấm deploy khi chưa đồng ý; đã chuyển chỉ đạo cho workers. API worker báo unit scene3, jobs24 và HTTP integration3 đạt trên data root tạm; root đang review độc lập, frontend chưa nghiệm thu.
- Root kiểm lại HTTP integration3 và strict server lib Clippy đạt. Chromium từ shell bị macOS MachPort error141/SIGTRAP chặn; native Chrome qua CUA đã xem All3/4/9 và Inspector frame-local, chưa thay thế smoke tự động. Review tìm ra xung đột sentinel, thứ tự ID và tiếp tục enqueue sau timeout; worker đã sửa, đang hoàn thiện nhãn đọc được khi Fit.
- Nhận frontend hoàn tất: root kiểm 78 web tests, typecheck/build đạt; worker kiểm scoped format, reviewer độc lập 18 focused tests và không còn Required. Nhãn giữ 12 CSS px khi Fit; kiểm thuần bao phủ mức 27%/5%, screenshot cuối chưa chụp được do CUA timeout. Detector không có finding, mã/test sửa tối đa236 dòng.
- Điểm dừng: [báo cáo và lệnh kiểm lại](../../ALL-FRAMES-REPORT.md). Fixture/Vite đã dừng, không commit/push/deploy hoặc restart native. Playwright smoke/prerender Chromium còn chờ môi trường cho phép browser khởi động; không ghi pass thay cho phần chưa chạy.

### Bản test riêng viewer WASM/All frames — 2026-11-06

- Theo yêu cầu người dùng, dựng snapshot source gồm All frames chưa commit và PostgreSQL cluster riêng; test viewer loopback3867, DB2704, không thay native3857/3858, launchd hay Funnel.
- Kiểm5snapshot và170resource hashes trùng nguồn, đủ5projects/10.432nodes/88assets/82fonts; cả8bảng DB gồm lịch sử/idempotency/binary trùng digest. Tài khoản test bị DB nguồn từ chối; batch3frames, PNG native, WASM compute và helper stop/start/restart đạt.
- Giữ bản test chạy để người dùng thử. Hướng dẫn vận hành và giới hạn nằm ở output `opencanvas-viewer-test.md` của chat; không commit/push hoặc sửa mã ứng dụng.

### Viewer progressive scenes và persistent cache — 2026-11-06

- All mặc định; board có skeleton theo authored geometry ngay, mỗi frame/resource hoàn tất được publish riêng; giữ camera và completed prefix khi fallback. IndexedDB dùng chung All/single, exact revision + source/WASM fingerprint, giới hạn64scene/32MiB; Retry bypass, lỗi storage quay về compute.
- Typecheck và production web build đạt, diff whitespace sạch; không chạy browser/runtime/DB/tests/benchmark theo yêu cầu người dùng tự thử. Source mới/sửa tối đa232dòng, không file nào vượt250; chi tiết tại [báo cáo](../../VIEWER-PROGRESSIVE-CACHE.md). Không deploy, đổi process/port, commit/push hoặc chạm dữ liệu.
