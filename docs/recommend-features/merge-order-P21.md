# P21 — Xung đột giữa các đề xuất, thứ tự merge, tổng byte `tools/list`
Ưu tiên: P1 · Kích cỡ: 0 phút code (chỉ chỉnh đề xuất/hướng dẫn rebase) · Phụ thuộc: không · Nguồn: rà chéo `.night/proposals/` @ `night/integration` `d08f243`

## Vấn đề

Đề xuất viết song song, nhiều cái trước khi P04a/P08a/P03a merge. Dev ngày mai sẽ gặp: (1) cùng loại prop có quy ước khác nhau, (2) 10–26 đề xuất cùng sửa vài file nóng, (3) ngân sách `tools/list` của P19 không tính đủ hàng chờ, (4) đề xuất dựa trên trạng thái code đã cũ.

## Bằng chứng

Code thật: `night/integration` `d08f243` đã có P05, P02a, P08a (`ec1fc01`), P03a (`0847b4b`), P04a (`d08f243`). INDEX còn ghi P08a "QA", P04a "chờ rebase" (`INDEX.md:28-29`), đã cũ.

### Xung đột quy ước/tên

| # | Xung đột | Dẫn chứng | Hệ quả nếu không xử lý |
| --- | --- | --- | --- |
| C1 | Mảng 4 cạnh/góc có 3 quy ước. `layout.padding` nhận 1–4 kiểu CSS. `stroke.width/color` (P04a, đã merge) nhận **đúng 4**. P04b đề xuất `radius` "số hoặc đúng 4" và file riêng `validate/corners.rs`. P17 đổi tất cả sang 1–4, tạo `model/sides.rs` + `validate/quad.rs`. | `props_catalog.rs:94` (padding), `:103` (stroke); `P04b-corner-radii.md:88`; `P17-four-side-shorthand.md:11,34,119-122,170` | Nếu P04b vào trước P17 thì có 2 bộ kiểm (corners.rs, quad.rs) và 1 đợt đổi nghĩa thứ hai cho `radius`. |
| C2 | P18b mapper có hai nhánh cho `radius`: `radiusQuad` bật/tắt, vì chưa biết P04b/P17 merge chưa. | `P18b-dom-importer-mappers.md:23-25,153` | Nhánh chết sau khi P17 vào. |
| C3 | `blur` cùng tên, khác thang đo: `shadow[].blur` là bán kính CSS (σ = blur/2); `blur` cấp node là σ như CSS `filter: blur()`. | `P09b-box-shadow.md:71-75`; `P09c-blur-backdrop.md:41`; `P18b…:28` | Cả hai đúng CSS, giữ tên. Nhưng props.md phải nói rõ "như CSS box-shadow blur" và "như CSS filter blur()". Mapper P18b không được dùng chung hàm đổi đơn vị. |
| C4 | `PropScope::Box` = mọi kind trừ page/text, nên **tự nhận `vector`** khi P14a thêm kind. Nhãn `Rendered` viết cứng "frame, group, rectangle, text, image". P14a mô tả `PropScope` chỉ có Any/Container/Text/Image (cũ). P14c muốn `stroke` lại có trên vector, kèm cap/join. | `props_catalog.rs` dòng 8–40 (enum, `Box => !matches!(kind, Page \| Text)`); `P14a-vector-path-node.md:24`; `P14c-vector-stroke.md:11,21` | `stroke`/`shadow`/`radius`/`blur` được chấp nhận trên vector trước khi renderer hỗ trợ: dữ liệu lọt validate nhưng không vẽ. |
| C5 | Alias gợi ý: P10b cho `align` → `textAlign`. Trên frame, agent viết `align` thường là ý `layout.align`. | `P10b-text-style-props.md:71`; `props_catalog.rs:94` | Gợi ý sai trên container. |
| C6 | Từ vựng căn lề: `layout.align/justify` dùng `start/center/end`; `textAlign` dùng `left/center/right` (không start/end). | `props_catalog.rs:94`; `P10b…:71,87` | Đúng CSS, chấp nhận. Lỗi validate nêu đúng tập giá trị của từng prop. |
| C7 | Enum `kinds`: P19 AC7 cố định "6 kind". P14a thêm `vector`, `maxItems` 6 → 7. | `P19-tools-list-diet.md:125`; `P14a…:234` | Test P19 vỡ nếu P14a merge trước, hoặc P14a phải sửa test của P19. |

### Giả định đã cũ

| # | Đề xuất giả định | Thực tế @ `d08f243` | Dẫn chứng |
| --- | --- | --- | --- |
| S1 | P07b, P07c: "vẫn 15 tool" | 16 tool (`export_nodes`), 17 sau P06b; `tool_schemas` so theo catalog từ F-TOOLS `817906c` | `P07b-animation-css-export.md:154`; `P07c-render-at-time.md:201` |
| S2 | P09b: "tạo `PropScope::Box` nếu chưa có" | Đã có (P04a) | `P09b-box-shadow.md:2,183` |
| S3 | P17: P04a "chờ merge" | Đã merge; `Sides<T>{All, Each([T;4])}` ở `crates/layout/src/model/stroke.rs:8`. P17 phải đổi luôn test P04a đang từ chối mảng 2 phần tử. | `P17…:11,34` |
| S4 | P14c import `Sides`/stroke từ `model/stroke.rs` | Sau P17, `Sides` chuyển sang `model/sides.rs` | `P14c…:23`; `P17…:121` |
| S5 | P12a ước byte theo `object()`/`id()` ở `schemas.rs:38-49` (có `minLength`/`pattern`) | P19 đổi các helper này sang bản "lean", nên số byte và dòng đều lệch | `P12a-font-delete-replace.md:164`; `P19…:122` |
| S6 | P07c dựa vào test `schemas/render.rs:49-62` ("published = hợp hai mode") | P19 đổi `published()` của render; P13a/P13b/P13c cũng thêm property vào đó | `P07c…:201`; `P19…` phạm vi file |
| S7 | P19: hàng chờ "+3 046 B", `TOTAL_MAX` hạ còn 24 KiB | Thiếu P06b (≤ +1 200), P20 (+151), P12b (≤ +120), P14a/P14b, P07b/P07c, P03d. Tổng thật ~+5,1 KB (bảng dưới) | `P19…:6,104` |

### File nóng (số đề xuất chưa merge chạm vào)

| File | Dòng hiện tại | Đề xuất |
| --- | --- | --- |
| `crates/server/src/schemas.rs` | 163 | 23: P03d P06b P07b P07c P08b P09a–c P10a–c P11b P12a P12b P13a P13b P14a–c P15 P17 P19 P20 |
| `crates/storage/src/props_catalog.rs` | 138 | 15: P04b P06a P07a P08b P09a–c P10a–c P14a P14c P17 P18a/b |
| `renderer/types.ts` | — | 14 |
| `crates/layout/src/model.rs` | 216 | 13: P04b P06a P07a P07c P08b P09a–c P10a P10b P11a P14a P17 |
| `validate/refs.rs` / `validate/properties.rs` | 151 / 127 | 11 / 11 |
| `reads/tree.rs` (server) | 143 | 10: P06a P06b P07a P07b P10c P11b P12a P14a P19 P20 |
| `renderer/paint.ts` | 103 | 8: P07c P09a P09b P09c P12b P14a P14c (+P07d viewer) |
| `RENDERER_VERSION` | — | 8: P07c P09a P09b P10a P10b P11a P14a P14c |
| `renderer/measurement.ts` | 104 | 5: P10a P10b P10c P11a P12b |
| `schemas/render.rs` | 107 | 6: P07c P11a P13a P13b P13c P19 |
| `schemas/operations.rs` | 67 | 11: P06a P07a P09a P09b P11b P12a P14a P14b P18a P19 P20 |
| `crates/layout/src/stack.rs` / `layout/src/tree.rs` | 67 / 93 | P08b P17 / P10a P14a |

`model.rs` đã 216 dòng; 13 đề xuất cộng ~5–20 dòng mỗi cái sẽ vượt 350. Mỗi đề xuất nên đặt struct mới trong `model/<prop>.rs` (như `model/stroke.rs`), chỉ thêm 1–3 dòng `pub mod`/field vào `model.rs`.

## Giải pháp

### Thứ tự merge đề nghị

Ba luồng ít đụng nhau. Trong mỗi luồng merge tuần tự; giữa các luồng chỉ đụng nhau ở `props_catalog.rs`, `model.rs`, `types.ts`, `RENDERER_VERSION`, toàn chỗ nối thêm nên rebase máy móc.

1. **Luồng S — schema/tool** (`schemas*.rs`, `instructions.rs`, `reads/`):
   P19 → P11b → P02c-r2 → P12a → P20 → P15 → P13a → P13b → P13c → P06a (đang rebase) → P06b → P07a (đang rebase) → P07b → P07c → P03b (WIP) → P03d.
   P19 đi đầu để mọi schema về sau viết thẳng bằng helper lean và đo byte trên nền mới. P13a–c và P07c cùng `render.rs`, nên P07c đi sau để chỉ rebase một lần.
2. **Luồng P — paint/props** (`model.rs`, `props_catalog.rs`, `refs.rs`, `paint.ts`, `stroke.ts`):
   P17 → P04b → P08b → P09b → P09a → P09c → P14a → P14b → P14c.
   P17 trước P04b (C1) và trước P08b (cùng `stack.rs` và `model/layout.rs`). P14a sau P09x để loại vector khỏi mọi prop Box một lần (C4).
3. **Luồng T — text/font** (`measurement.ts`, `resources.ts`, `worker/`):
   P10a → P10b → P10c → P11a → P12b. P12a (luồng S) phải vào trước P12b vì cùng `resources.ts` và gỡ trùng mặt font.
4. **Cuối cùng**: P18a → P18b, sau P15 và sau mọi prop mà mapper xuất ra (P04b, P09a, P09b, P10b, P17).

### Chỉnh sửa nhỏ ở từng đề xuất (Dev làm lúc thẩm định, không cần viết lại đề xuất)

| Đề xuất | Chỉnh |
| --- | --- |
| P19 | AC7: kiểm `allowed` bằng enum kind lấy từ code, không cố định "6 kind" (C7). Đặt `TOTAL_MAX` 26 KiB thay vì 24 KiB, hoặc ghi rõ P06b/P13c sẽ nâng (S7). Ghi số đo nền mới ở `d08f243`, không dùng 24 525 @ `0847b4b`. |
| P17 | Viết lại tham chiếu P04a thành "đã merge". Đổi test P04a đang từ chối 2 phần tử. Chuyển `Sides` sang `model/sides.rs`, export lại từ `model/stroke.rs` để P14c không vỡ import (S3, S4). |
| P04b | Bỏ `validate/corners.rs`, dùng `quad.rs` của P17. Mô tả: "1–4 like CSS border-radius [tl, tr, br, bl]". Thêm test mở rộng 2/3 giá trị theo thứ tự góc CSS (C1). |
| P09b | Không tạo `PropScope::Box` (S2). Mô tả `shadow.blur`: "CSS box-shadow blur radius" (C3). |
| P09c | Mô tả `blur`/`backdropBlur`: "like CSS filter blur(): standard deviation" (C3). |
| P10b | Alias `align` chỉ gợi ý `textAlign` khi kind là text; trên container gợi ý `layout.align` (C5). |
| P14a | Cập nhật bằng chứng `PropScope` (6 biến thể). Sửa `Box` thành `!matches!(kind, Page \| Text \| Vector)`, sửa nhãn `Rendered`. Test B02 `prop_catalog.rs` chạy kind × key phải có vector (C4). |
| P14c | Thêm scope `Stroke` = Box ∪ Vector chỉ cho `stroke`; import `Sides` từ `model/sides.rs` (S4). |
| P12a, P11b, P20, P13a–c, P06b, P07b, P07c | Đo byte lại sau P19, dùng `serde_json::to_vec`. P07b/P07c sửa "15 tool" thành "theo catalog" (S1, S5). P07c viết lại phần test `render.rs` theo `published()` sau P19 và sau P13c (S6). |
| P18b | Bỏ nhánh `radiusQuad` tắt nếu P17 đã merge (C2). Mapper `filter: blur()` và `box-shadow` tách đơn vị (C3). |
| Mọi đề xuất luồng P/T | `RENDERER_VERSION`: khi rebase lấy giá trị trên integration + 1, không giữ số của nhánh. |

### Tổng byte `tools/list` theo thứ tự luồng S

Số là trần do từng đề xuất tự ghi, đo trước khi có P19. Sau P19 có thể thấp hơn ~20 %. "≈" là ước của P21 khi đề xuất không ghi số.

| Bước | Δ (B) | Tích luỹ (B) |
| --- | --- | --- |
| Nền `0847b4b` (16 tool) | — | 24 525 |
| P04a, P08a, P03a đã có | đo lại ở `d08f243` | ~24 525 |
| P19 | −6 000 | ~18 500 |
| P11b | ≤ +250 | 18 750 |
| P02c-r2 | +196 | 18 946 |
| P12a | ≤ +600 | 19 546 |
| P20 | ≤ +160 | 19 706 |
| P15 | ≤ +160 | 19 866 |
| P13a / P13b / P13c | ≤ +800 / +300 / +700 | 21 666 |
| P06b (tool 17) | ≤ +1 200 | 22 866 |
| P07b (`animationCss` boolean) | ≈ +60 | 22 926 |
| P07c (`timeMs`, `play`) | ≈ +150 | 23 076 |
| P03b / P03d (enum format, `withinId`) | ≈ +80 | 23 156 |
| P14a (`vector` trong 2 enum) | ≈ +20 | 23 176 |
| P14b (biến thể `import_svg`) | ≈ +300 | 23 476 |
| P14c / P12b | ≤ +40 / +120 | **~23 640** |
| Các đề xuất còn lại: P04b P08b P09a–c P10a–c P11a P17 P18a/b | 0 (`props` tự do) | ~23 640 |

Kết luận: sau cả hàng chờ, tổng vẫn thấp hơn hôm nay khoảng 900 B. Nhưng nó chỉ cách trần 24 KiB (24 576 B) mà P19 đề xuất ~900 B, nên cần trần 26 KiB (S7).

## Phạm vi file

- Chỉ file này. Không sửa code, không sửa file đề xuất khác. Dev áp các chỉnh sửa ở bảng trên trong nhánh task tương ứng.

## Tương thích MCP

- P21 không đổi gì. Các chỉnh sửa đề nghị chỉ đổi mô tả props.md và trần test. Riêng P17 nới `stroke` từ "đúng 4" sang 1–4, chỉ nhận thêm input, không đổi nghĩa input cũ.

## AC (Dev kiểm khi rebase mỗi đề xuất)

1. `cargo test -p opencanvas-server --test tools_budget -- --nocapture` trước và sau mỗi merge luồng S: Δ ≤ trần ở bảng byte. Ghi số vào `.night/dev/<ID>.md`.
2. Sau P17 + P04b: `stroke.width:[1,2]`, `radius:[8,0]` và `layout.padding:[8,16]` cùng được nhận và mở rộng như CSS. Chỉ có **một** hàm mở rộng (`rg -n 'fn quad|fn expand' crates/` ra 1 kết quả).
3. Sau P14a: `create_node kind:"vector"` kèm `stroke`/`shadow`/`radius` bị từ chối `field: props.<key>`, cho tới khi P14c bật riêng `stroke`.
4. `cargo test -p opencanvas-server --test tool_schemas` pass sau P06b và P07b/P07c: số tool lấy từ catalog (17 sau P06b), không hardcode 15 hay 16.
5. `wc -l crates/layout/src/model.rs crates/storage/src/props_catalog.rs crates/server/src/schemas.rs renderer/paint.ts` sau mỗi merge ≤ 250. Vượt thì tách `model/<prop>.rs` ngay trong đề xuất đó.
6. `RENDERER_VERSION` trên integration tăng đúng 1 mỗi merge luồng P/T. `rg -n RENDERER_VERSION` không còn giá trị trùng giữa Rust và TS.
7. Mô tả props.md của `shadow.blur` và `blur` có chữ "box-shadow" và "filter" tương ứng.

## Cách kiểm

- Mỗi lần rebase: `git diff night/integration...HEAD --stat` chỉ được chạm đúng file nóng của luồng mình. Chạy AC1, AC5, AC6. Chạy AC2 sau P04b, AC3 sau P14a.

## Rủi ro

- Số byte trong bảng là trần tự khai trước P19, có thể lệch ±20 %.
- Thứ tự có thể đổi theo slot dev. Ràng buộc cứng chỉ có: P19 trước mọi schema mới, P17 trước P04b/P08b/P14c, P13a trước P13b/c và P07c, P10a trước P10b/P11a, P12a trước P12b, P14a trước P14b/c.

## Ngoài phạm vi / phase sau

- Viewer (P02b, P03c, P06c, P06d, P07d): không đụng file nóng server. Chỉ `web/tests/fixture-data.mjs` và `CanvasView.tsx` cần xếp tuần tự với P06c/P06d/P07d.
- P20 (`token-code-syntax`) chưa có trong INDEX. Thứ tự ở trên đặt sau P12a.
