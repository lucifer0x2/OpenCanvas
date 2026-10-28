# Đề xuất tính năng OpenCanvas — đợt chạy đêm 2026-10-28

Gói này dành cho người dùng cân nhắc trước khi triển khai. Nguồn: `.night/proposals/` (bộ phận Nghiên cứu); nhánh nền `night/integration` @ `36b8b50`.

## Cách dùng

- Mỗi file trong `tasks/` là một task handoff tự đủ. File có vấn đề, bằng chứng `file:dòng`, giải pháp, phạm vi file, AC đo được, cách kiểm, rủi ro, ước lượng phút và phụ thuộc. Giao nguyên file cho một agent dev, mỗi task một phiên.
- `tasks/` chỉ chứa task **chưa merge**. Đã merge vào `night/integration`: P01, P02a, P03a, P03b, P04a, P05, P06a, P07a, P08a, cùng B01–B05, F-B01-1, F-TOOLS.
- Thứ tự merge và xung đột giữa các task nằm trong `merge-order-P21.md`, gồm 3 luồng S (schema MCP), P (paint/props), T (text).
- Bảng dưới là chỉ mục lúc 02:26. Cột trạng thái có thể cũ: P06a, P07a, P03b đã merge sau đó.

## Xếp theo giá trị cho agent

Thứ tự: sai lệch agent gặp nhiều nhất khi vẽ lại UI thật (ghi chú QueryNest) → chi phí token cố định và tự kiểm → tính năng mới → đường nhập liệu.

| # | ID | Việc | Ưu tiên | Phút | Trạng thái | Vì sao đáng làm |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | P10a | Text đo như Chrome: hết vỡ chữ khi hug ("⌘C"), baseline theo font, tắt hinting Linux, `textWrap` | P1 | 75 | chưa làm | Chạm mọi text; hết phải nới width +12 px và tự bù baseline (note #4, #5, #13). |
| 2 | P19 | Gọn `tools/list`: schema công bố gọn, mô tả không lặp; lỗi schema chỉ rõ field/rule | P1 | 90 | chưa làm | Mọi phiên agent trả chi phí này: 24 525 → ~18 500 B (−24 %); lỗi `INVALID_REQUEST` hết chung chung. |
| 3 | P08a | Stack: padding từng cạnh, `align`, `justify` | P1 | 85 | **merged** ec1fc01 | Layout thật không còn phải dùng `free`. |
| 4 | P04a | `stroke` đều hoặc từng cạnh, màu riêng, giữ bo góc | P1 | 90 | **merged** d08f243 | Hết giả viền bằng rect chồng. |
| 5 | P05 | Group opacity composite như CSS | P1 | 80 | **merged** 565b12b | Nút disabled, ⌘K không còn lộ lớp (note #10). |
| 6 | P09b | `shadow` nhiều lớp outer/inset như `box-shadow` | P1 | 90 | chưa làm | Ring, highlight, shadow hiện phải vẽ bằng rect. |
| 7 | P09a | `fill` gradient linear/radial | P1 | 80 | chưa làm | Nút, icon tile, glow đang phải dùng màu trung bình. |
| 8 | P10b | `letterSpacing`, `textAlign`, `textDecoration`, `textTransform` | P1 | 90 | chưa làm (sau P10a) | Hết tách từng ký tự thành node, hết gạch ngang bằng rect. |
| 9 | P17 | Shorthand CSS 1–4 cho `stroke.width`/`stroke.color`/`radius` như `layout.padding` | P2 | 50 | chưa làm (trước P04b) | Một quy ước mảng cho mọi prop cạnh/góc; Δ 0 B. |
| 10 | P04b | `radius` từng góc | P1 | 45 | chưa làm (sau P17) | Tab chỉ bo góc trên. |
| 11 | P14a | Kind `vector` (path SVG, màu theo token) | P1 | 85 | chưa làm | Icon không phải raster PNG 2x; path ~581 B so với PNG ~1 080 B. |
| 12 | P13a | `render_frame.compare`: so với PNG tham chiếu → % lệch, vùng, node, offset | P1 | 90 | chưa làm | Agent tự kiểm thay vì nhìn bằng mắt; ≤ +800 B (tool riêng ~1 410 B). |
| 13 | P20 | `codeSyntax` trên token (CSS var, Android, iOS) | P1 | 50 | chưa làm (sau P19) | Như Figma `Variable.codeSyntax`: coder dùng đúng tên biến của repo; ≤ +160 B. |
| 14 | P02a | Note trên node: 4 op `apply_batch`, đọc qua `get_design_context` | P1 | 90 | **merged** 0db047a | Designer và coder trao đổi ngay trên phần tử (U4). |
| 15 | P01 | Viewer zoom luôn nét (vẽ vector) | P1 | 85 | **merged** f2b0376 | U1; sửa cả lỗi `ceil` PIXEL_BUDGET làm mờ vĩnh viễn. |
| 16 | P03a | Export PNG + `exportSettings` (`export_nodes`, HTTP) | P1 | 90 | **merged** 0847b4b | U5. |
| 17 | P06a | Link điều hướng `links`/`flowStart` + `get_design_context.navigation` | P1 | 85 | QA (r3, 02:18) | U6; coder đọc được flow giữa màn hình. |
| 18 | P07a | Animation khai báo (`animations` keyframe kiểu CSS) | P1 | 85 | QA (rebase, 02:20) | U3. |
| 19 | P11a | Diagnostic `MISSING_GLYPH` theo node lúc đo text | P1 | 70 | chưa làm (sau P10a) | Biết ký tự rơi sang font fallback, width sai (note #12); ±0 B. |
| 20 | P12a | `delete_font`, thay font theo id, `delete_asset`, dọn asset mồ côi, `unused:true` | P1 | 85 | chưa làm | Dữ liệu thật đang kẹt 18 font cũ (note #11); ≤ +600 B. |
| 21 | P15 | Bulk apply: POST file NDJSON, server tự chia batch, chạy tiếp theo dòng | P2 | 85 | chưa làm | 450 op qua tool call ~96 KB tham số (~30k token); qua file ~170 B lệnh curl. |
| 22 | P07b | `get_design_context animationCss` → CSS @keyframes cho coder | P1 | 70 | chưa làm (sau P07a) | Coder dùng thẳng, không phải dịch. |
| 23 | P10c | Chọn mặt font theo `fontFamily` + `fontWeight` + `fontStyle` | P2 | 60 | chưa làm | Italic; không phải nhớ font ID cho từng weight. |
| 24 | P22 | `get_outline bounds:true` → `[x,y,w,h]` | P2 | 60 | chưa làm (sau P19) | Tìm "card bên phải" trong một lượt đọc, như Figma `get_metadata`; ≤ +260 B. |
| 25 | P02c-r2 | Rà note toàn project bằng `find_nodes notes` | P2 | 45 | chưa làm | +196 B so với +1 065 B nếu thêm tool `list_notes`. |
| 26 | P06b | Tool `get_navigation`: bản đồ điều hướng một lần gọi | P1 | 55 | chưa làm (sau P06a) | ~1,5 KB so với 29–43 KB nếu quét; có lý do thêm tool. |
| 27 | P11b | `sortKey` luôn 0..n−1 sau delete/reparent; mô tả thứ tự vẽ | P2 | 40 | chưa làm | Đọc đúng thứ tự vẽ (note #14); ≤ +250 B. |
| 28 | P18a | Importer DOM → NDJSON ops (capture trong trang + converter Node) | P2 | 85 | chưa làm (nên sau P15) | Ý tưởng người dùng: chính thức hoá pipeline ~300 dòng tự viết; ±0 B. |
| 29 | P08b | `alignSelf`, `justify` space-around/space-evenly | P2 | 40 | chưa làm | |
| 30 | P09c | `blur` và `backdropBlur` | P2 | 85 | chưa làm | Modal làm mờ nền. |
| 31 | P14b | Op `import_svg` trong `apply_batch` | P2 | 90 | chưa làm (sau P14a) | Dán SVG icon trực tiếp; chặn script/href. |
| 32 | P14c | Stroke cho `vector`, cap/join/miterLimit, `fill:"none"` | P2 | 75 | chưa làm (sau P14a, P14b) | Icon nét kiểu Lucide khớp SVG 0/255; ≤ +40 B. |
| 33 | P13b | Ảnh tham chiếu qua HTTP raw, `compare.referenceRect`, cảnh báo ICC | P2 | 70 | chưa làm (sau P13a) | Tránh base64 PNG hàng chục nghìn token. |
| 34 | P13c | `compare.diffImage`, `compare.ignore`, so theo `nodeId` | P2 | 85 | chưa làm (sau P13a) | Thấy lệch ở đâu, bỏ qua vùng động. |
| 35 | P12b | Variable font: một file, `fontWeight` chọn trục `wght` | P2 | 75 | chưa làm (sau P10c) | Hết tự instance 18 file tĩnh; ≤ +120 B. |
| 36 | P18b | Importer: mapper border/radius/shadow/gradient theo catalog server | P2 | 85 | chưa làm (sau P18a) | Lưu ý: dò cờ bằng chữ trong `props.md` dễ gãy. |
| 37 | P18e | Importer: mapper text props (P10a/b/c) và stack P08b | P2 | 80 | chưa làm (sau P18b) | |
| 38 | P03b | Export SVG | P1 | 80 | QA (02:21) | |
| 39 | P03d | Export JPG/WebP + mọi node được đánh dấu (`withinId`) | P2 | 80 | chưa làm | |
| 40 | P07c | `render_frame {timeMs}`: render tại thời điểm t | P1 | 90 | chưa làm (sau P07a/b) | Agent kiểm animation bằng ảnh. |
| 41 | P23 | Lọc `list_tokens`/`list_styles` theo `query` và `type`/`kind` | P2 | 40 | chưa làm (sau P19, P20) | Design system 400 token: ~52 KB → ~1,4 KB khi lọc; ≤ +320 B. |

### Viewer (cho người xem, ít tác động tới agent)

| ID | Việc | Ưu tiên | Phút | Trạng thái |
| --- | --- | --- | --- | --- |
| P02b | Note trên viewer (chỉ đọc) + SSE `notesOnly` | P1 | 85 | Dev tạm dừng chờ P02a; P02a đã merge, có thể làm tiếp |
| P06c | Chế độ Flow: sơ đồ frame và mũi tên | P1 | 85 | chưa làm (sau P06a) |
| P03c | Export trên viewer + HTTP enqueue | P1 | 80 | chưa làm |
| P07d | Play/scrub animation, vẽ lại theo vùng | P1 | 90 | chưa làm (sau P07c) |
| P06d | Hotspot link + Inspector Navigation | P2 | 60 | chưa làm (sau P06c) |

### Hướng dẫn và đã thay thế

| ID | Nội dung |
| --- | --- |
| P21 | Rà chéo các đề xuất: 7 xung đột quy ước, 7 giả định cũ, thứ tự merge 3 luồng (bên dưới), tổng byte `tools/list`. Không có code. Dev và QA kiểm 7 AC khi rebase. |
| P02c (`P02c-list-notes.md`) | Đã thay bằng P02c-r2 (bỏ tool mới `list_notes`, +1 065 B). |

## Thứ tự merge gợi ý (từ P21)

Ba luồng có thể làm song song. Các việc trong cùng một luồng sửa chung file nóng nên làm tuần tự.

- **S, schema MCP** (`schemas.rs`): P19 → P11b → P02c-r2 → P12a → P20 → P23 → P22 → P15 → P13a–c → P06b → P07a–c → P03b/d.
- **P, paint và props** (`renderer/paint.ts`, `props_catalog`): P17 → P04b → P08b → P09b → P09a → P09c → P14a–c.
- **T, text** (`renderer/measurement.ts`): P10a → P10b → P10c → P11a → P12b.
- **`tools/list`:**
  - hiện ~24,5 KB;
  - sau P19 còn ~18,5 KB;
  - sau cả hàng chờ ~23,6 KB.

  P21 đề nghị nâng trần của P19 từ 24 KiB lên 26 KiB.

## Còn lại

- **Nghiên cứu đêm nay đã xong:** không còn phiên con nào chạy, không đề xuất nào chờ duyệt.
- **Phase sau đã ghi trong từng đề xuất:**
  - Code Connect (P20 #2, cần khái niệm component);
  - P12c `fontVariationSettings`;
  - P18c (svg/img), P18d (compare cho importer).

## Nghiên cứu sâu thêm (02:44–02:55, theo yêu cầu người dùng)

Các task này viết nhanh trong khoảng 10 phút, nên cần duyệt kỹ hơn các task ở bảng trên trước khi giao.

| ID | Việc | Ưu tiên | Phút | File |
| --- | --- | --- | --- | --- |
| P26 | `update_matching`: sửa đồng loạt theo bộ chọn trong `apply_batch` (300 lần `update_node` 25 200 B → 1 op 148 B; tools/list ~+500 B) | P2 | 110 (P26a 70 + P26b 40) | `tasks/P26-bulk-update-by-selector.md` |
| P24 | Component + instance tối thiểu: master frame `component`, kind `instance` + overrides phẳng, expand ở storage; `get_design_context` trả component một lần (20 hàng bảng 22 191 B → 6 018 B, −73 %; tools/list < +300 B) | P1 | 165 (P24a 90 + P24b 75) | `tasks/P24-components-instances.md` |
| P25 | Workflow designer → coder đo theo byte: một vòng ~302 KB → ~108 KB sau các task trên; bước đắt nhất còn lại là `get_design_context` compact (39 %) và handshake (40 %); harness `workflow_budget.rs` + skill workflow + compact mặc định | P1 | 130 (P25a 60 + P25b 70) | `tasks/P25-agent-workflow-e2e.md` |
