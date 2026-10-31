# Memory khi export QueryNest Table Data — 2026-10-29

Đo trực tiếp instance native macOS release đang chạy, theo yêu cầu người dùng.
Không restart, thay runtime hoặc sửa authored design. Revision trước/sau vẫn 61.

## Workload và phương pháp

- Project QueryNest — Faithful UI & Motion, page `page_1` / “02 · Table Data”.
- Page có một root frame `frame_table_data`, 1440×900; API không export page trực tiếp.
- MCP `export_nodes`, PNG scale1, revision61, suffix riêng để có export cache key mới.
- Worker Node/Chromium đã tồn tại trước đo; không phải cold-start benchmark.
- Lấy mẫu `/bin/ps -axo pid=,ppid=,rss=,comm=` và tìm toàn bộ descendants của server.
- 931 mẫu trong 93,93 giây; median khoảng cách101ms, lớn nhất132ms.
- Baseline: median5 giây cuối trước gọi tool. Sau30/60 giây: median cửa sổ±1 giây
  quanh thời điểm tool trả kết quả cộng30/60 giây. Peak: tổng cao nhất quan sát
  trong toàn lượt đo, kể cả đoạn ngay sau tool trả kết quả.
- Nhóm Rust chỉ server; `layout-step` được lấy nếu kịp thấy, nhưng không hiện trong
  sample peak/baseline. Process ngắn hơn nhịp lấy mẫu có thể bị bỏ lỡ.
- Đơn vị MiB = RSS KiB /1024. Không gồm browser viewer của người dùng hoặc process đo.

## Kết quả

| Nhóm RSS | Trước export | Tại đỉnh chung | Sau30 giây | Sau60 giây |
| --- | ---: | ---: | ---: | ---: |
| Rust server | 13,58 MiB | 33,22 MiB | 16,98 MiB | 16,69 MiB |
| Node worker | 9,88 MiB | 125,50 MiB | 105,72 MiB | 105,72 MiB |
| Chromium (4 processes) | 104,03 MiB | 387,17 MiB | 244,39 MiB | 246,66 MiB |
| **Tổng cây process** | **127,48 MiB** | **545,89 MiB** | **367,09 MiB** | **369,06 MiB** |

Mỗi cột idle dùng median riêng nên tổng các số làm tròn có thể lệch nhẹ.
Peak chung cách lúc bắt đầu gọi tool4,18 giây, sau khi tool trả ready khoảng1,35 giây;
không phải peak được chứng minh nằm trong hàm raster duy nhất. Phần tăng so baseline
là khoảng418,41 MiB ở peak và241,58 MiB sau60 giây. Node+Chromium chiếm khoảng94%
RSS tại peak; đây là tỷ trọng quan sát, không phải dự báo lượng RAM tiết kiệm khi đổi engine.

Export thành công: PNG1440×900 RGBA, 157.410 byte. Tool round-trip2,826 giây gồm
connector overhead, không dùng làm thời gian render thuần. Đã tải file và xem ảnh.
Quan sát thêm78,58 giây sau ready; median5 giây cuối368,83 MiB. Đã dừng sampler.

## Giới hạn diễn giải

- RSS cộng có thể đếm trùng shared pages, không bằng physical footprint hoặc cột
  Memory trong Activity Monitor. Resident pages có thể được reclaim/compress và
  fault lại; baseline Node9,88 MiB không chứng minh toàn footprint interpreter chỉ có vậy.
- Một lần export không kết luận được memory leak, GC behavior hoặc steady-state dài hạn.
- Không cô lập hoạt động của người dùng/agent khác trên instance; không đo FE viewer.
- Peak lấy mẫu là cận dưới của peak RSS thật; không có kernel high-water toàn cây.
- PNG job/cache và worker còn tồn tại đúng lifecycle server; không ép GC, xóa cache hay kill worker.

## Bằng chứng local

Thư mục kết quả: `/private/tmp/opencanvas-memory-uaPBFh/`:
`samples.jsonl` (từng sample/group/process), `metadata.json` (scope/timestamps/job),
`summary.json` (cách chia giai đoạn/số liệu) và `table-data.png`.
Đây là artifact tạm, không nằm trong authored project data và có thể được OS dọn.
