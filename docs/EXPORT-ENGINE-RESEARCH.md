# Nghiên cứu engine xuất ảnh OpenCanvas

Ngày khảo sát: **2026-10-29**. Checkout: `codex/p24-components-instances`,
HEAD `535d7e1a7f2bb6017918ccbe1c0aa2b34f3745fa`.
Đây là nghiên cứu và handoff spike; **chưa thay engine, chưa benchmark ứng viên**.

## Khuyến nghị điều hành

**Chưa chọn Rust thuần hay C++ thắng.** Nếu mục tiêu chính là bỏ cả Node và
Chromium trên server, chạy bake-off CPU giữa **resvg/tiny-skia với text stack
HarfRust** và **Skia C++ qua skia-safe**. Nếu yêu cầu trước mắt là giảm dependency
mà giữ hình gần nhất hiện tại, **Rust điều khiển Chromium qua CDP** là bước chuyển
ít đổi semantics hơn; bước này bỏ Node runtime, vẫn cần Chromium và JavaScript.

Lý do không thay ngay bằng một crate PNG: text width quyết định Taffy layout,
còn glyph shaping và raster quyết định hình. Encoder chỉ nén pixel đã vẽ.
Thay encoder không loại browser; thay riêng raster cũng chưa loại browser đo text.
Viewer hiện vẽ bằng cùng Canvas2D code, nên native export tạo thêm một engine cần
chứng minh parity. Nguồn code và ranh giới tại [trace](export-engine-research/01-current-pipeline.md).

### Xếp hạng theo mục tiêu, không phải bảng tốc độ

| Mục tiêu | Thứ tự đề xuất | Điều kiện đổi thứ tự |
| --- | --- | --- |
| Bỏ cả Node/Chromium, CPU Docker | 1. resvg + HarfRust; 2. skia-safe CPU; 3. vello_cpu dự phòng | Skia lên đầu nếu resvg không qua font/stroke/image fidelity hoặc cần effects sớm. |
| Ít thay đổi hình và hành vi | 1. tối ưu worker đang có; 2. Rust CDP + Chromium; 3. native bake-off | Giữ Chromium nếu native không đạt ngưỡng fidelity đã thống nhất. |
| Native viewer dài hạn | So CPU backend chung trước, rồi xét WASM/CanvasKit | Không thay React/Konva chỉ để bỏ browser của server. |

Đây là **phán đoán kiến trúc**, chưa phải kết quả đo. resvg được xếp trước vì
hướng static scene hợp bài toán và có thể tái dùng mô hình SVG nội bộ; skia-safe
là đối chứng quan trọng về độ phủ drawing. Không có bằng chứng local rằng Rust
nhanh hơn C++, Python hay Chromium cho workload này.

## Những phát hiện thay đổi quyết định

- PNG hiện dùng Node → Chromium Canvas2D. SVG được **viết markup bằng Rust**,
  nhưng job SVG vẫn gọi browser compute trước. SVG public hiện bỏ stroke, live
  text không embed font; lấy file này đưa thẳng vào resvg không giữ fidelity PNG.
- `P10a` và `P11a` vẫn là kế hoạch: measurement hiện round 1/64, baseline heuristic,
  chưa có `textWrap`, chưa báo glyph fallback. Font file load thành công chưa chứng
  minh mọi glyph có trong file. Không xem lỗi hiện tại là chuẩn typography vĩnh viễn.
- **RustyBuzz đã archived**, README khuyên chuyển HarfRust; `ttf-parser` đang
  maintenance mode và khuyên fontations cho dự án mới. Cần pin cả dependency gián
  tiếp thay vì sao chép stack từ bài cũ. [RustyBuzz](https://github.com/harfbuzz/rustybuzz),
  [ttf-parser](https://github.com/harfbuzz/ttf-parser).
- resvg/usvg upstream hiện dùng HarfRust/skrifa/fontdb; cosmic-text cũng dùng
  HarfRust. Đây là các lựa chọn hiện tại đã tra nguồn, chưa được build trong repo.
  [usvg manifest](https://github.com/linebender/resvg/blob/main/crates/usvg/Cargo.toml),
  [COSMIC Text](https://github.com/pop-os/cosmic-text).
- Không đánh đồng toàn bộ Vello với GPU: upstream hiện phân `vello_cpu`,
  `vello_gpu` và compute renderer ở research. GPU/wgpu vẫn không phải ưu tiên cho
  Docker không GPU. [Vello](https://github.com/linebender/vello).
- Bridge C++/Python gọi implementation gốc. Nó không chuyển mã C++/Python thành
  Rust, không tự bỏ C++ runtime hoặc CPython, không tự giải quyết crash/timeout.

## Đường quyết định khả thi

1. Chốt corpus và font bytes/hash; đóng băng baseline hiện tại, tách các AC P10a
   thành nhánh cải thiện riêng để không trộn sửa text với đổi raster.
2. Spike A: render **computed scene cố định** qua resvg và skia-safe, gồm stroke,
   opacity và ảnh; đo riêng raster/encode. Chưa gọi đây là pipeline bỏ browser.
3. Spike B: đo text native → Taffy → scene → native PNG, so lại line breaks,
   baseline và viewer. Đây mới chứng minh khả năng bỏ Chromium.
4. Spike CDP nếu parity native không đạt hoặc cần bước bỏ Node sớm. Không mặc định
   làm cả ba production backend; dừng các ứng viên không qua gate.
5. Chuyển qua feature flag nội bộ, cache namespace mới, cùng HTTP/MCP/jobs;
   rollback bằng chọn lại backend và restart, không sửa authored DB.

Handoff có AC, corpus, cách đo, ngưỡng đề xuất và rollback ở
[spike/migration](export-engine-research/05-spike-and-migration.md).

## Bản đồ tài liệu

| Tài liệu | Trả lời |
| --- | --- |
| [01 — Pipeline hiện tại](export-engine-research/01-current-pipeline.md) | Code ở đâu, phụ thuộc lúc build/runtime và phần có thể tối ưu. |
| [02 — Ứng viên](export-engine-research/02-candidates.md) | Rust CPU/GPU, Skia, Cairo/Pango, Qt, Python, CDP và lý do sàng lọc. |
| [03 — Bridges và đóng gói](export-engine-research/03-bridges-packaging.md) | C ABI/cxx, PyO3/subprocess, ABI, GIL, cross build và failure isolation. |
| [04 — Fidelity/operations](export-engine-research/04-fidelity-and-operations.md) | Ma trận text, hình, fonts, formats, viewer/native/WASM và limits. |
| [05 — Spike/migration](export-engine-research/05-spike-and-migration.md) | Công việc tối thiểu, số đo cần có, gates và bảo toàn contract. |

## Mức độ bằng chứng và phần còn thiếu

- **Local**: đọc source tại HEAD trên, đối chiếu PLAN/README/ACCEPTANCE/VIEWER/SERVER,
  tasks và feature handoffs. Không chạy server, Docker/build/tests/benchmark;
  không chạm data thật hoặc cổng 3847.
- **Upstream**: các link nguồn chính thức trong phụ lục được mở ngày 2026-10-29.
  `main`/`latest` có thể đi trước release; spike phải pin tag/hash và lưu license
  của dependency graph thực. Không coi README performance hay tính tất định là
  số đo/guarantee OpenCanvas.
- **Giả thuyết**: native có thể giảm footprint và overhead; chưa đo latency, RSS,
  binary/image size, build time hay pixels. Chưa chứng minh Docker Linux arm64
  hoặc macOS native parity, WOFF2/emoji/fallback coverage, font redistribution,
  PDF vector/text hay lợi ích throughput. Đây là gate spike, không phải việc đã đạt.
