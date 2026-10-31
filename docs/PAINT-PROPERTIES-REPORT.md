# Báo cáo nhóm5 — paint (P17/P04b/P09b/P09a) — 2026-10-30

Phạm vi: shorthand cạnh/góc, radius từng góc, shadow ngoài/inset, gradient linear/radial
trên pipeline Chromium hiện tại. Pipeline vẫn là bản tổng chưa phát hành `scene-3`/`web-4`
(không tăng version). Không blur, vector, `%`, rich text, variable axes.

## Ngữ nghĩa đã triển khai

- **radius**: số hoặc 1–4 giá trị CSS `[tl, tr, br, bl]`, mỗi giá trị nhận dimension token.
  Authored giữ nguyên dạng; scene: số vô hướng giữ dạng cũ, mảng luôn thành 4. Góc chồng
  nhau giảm theo CSS: một hệ số `f = min(cạnh / tổng hai bán kính kề)` trên border box,
  áp cho mọi góc (canvas `roundRect`, `renderer/corners.ts`, SVG `svg/geometry.rs`).
- **stroke**: width/color 1–4 giá trị như trước. Vòng stroke dùng bán kính đã giảm của
  node: inside = border CSS (bán kính trong = ngoài − width kề, ≥ 0, **không giảm lần
  hai**); center/outside nở `width·f` từ hình fill đã giảm.
- **fill gradient** (box, không cho text): `{type:"linear", angle?:180, stops}` hoặc
  `{type:"radial", center?:[.5,.5], radius?: số | [rx, ry], stops}`; stops 2..16 cặp
  `[position 0..1 không giảm, color]`, hard stop giữ thứ tự; màu stop nhận color token;
  fill style có thể là gradient. Nội suy premultiplied như CSS ở canvas, preview và SVG.
  Fill animation cần fill nghỉ solid: node có fill gradient (literal hoặc qua fill style)
  mà khai báo keyframe `fill` bị từ chối ở `props.animations[i].keyframes[j].fill`, vì
  preview HTML vẽ gradient bằng `background-image` che màu đang chạy. Gradient chỉ trong
  state vẫn hợp lệ (state luôn thắng animation fill). Preview cảnh báo nếu dữ liệu cũ lọt qua.
- **shadow**: 0..8 lớp `{x,y,blur,spread,color,inset}`, lớp đầu nằm trên; x/y/spread nhận
  number token, blur dimension token, color color token. Outer + inset; fill trong mờ
  khoét shadow ngoài như CSS.
- **states**: fill (kể cả gradient), opacity, radius (1–4), stroke. **Shadow trong state
  vẫn bị từ chối**, lỗi `props.states.<state>.shadow` nói rõ: chưa hỗ trợ, đặt shadow ở
  node nghỉ và đổi fill/opacity/radius/stroke theo state.
- **Instance**: override `fill` (kể cả gradient) được kiểm như prop gốc; radius/shadow/
  stroke đến từ master; override shadow bị từ chối ở `props.overrides.<id>.shadow`.
- **SVG production**: radii → `<path>` (cung, hệ số CSS), gradient → `<linearGradient>`/
  `<radialGradient>` với stop mở rộng (xem A). Shadow và stroke không vẽ, mỗi file báo
  `SVG_SHADOW_NOT_EXPORTED` / `SVG_STROKE_NOT_EXPORTED`; không bỏ im lặng.

## Lỗi thật đã sửa trong đợt này

1. Storage: `fill` gradient trong `states` **không được kiểm cấu trúc** (object bị bỏ qua
   như ref) → stops giảm dần lọt vào scene. Nay kiểm như prop gốc, lỗi
   `props.states.hover.fill.stops[1]`.
2. Storage: màu literal sai trong shadow báo field `color` thay vì `shadow[i].color`.
3. Canvas stroke: bán kính trong tính từ bán kính chưa giảm rồi `roundRect` giảm lần nữa;
   đường chia màu cạnh bị kẹp từng trục nên lệch góc với bán kính lớn; vòng nhiều màu
   tô màu đầu làm nền rồi phủ màu khác (viền đỏ lẫn ở cạnh AA). Xem B.
4. Preview: overlay stroke dùng bán kính chưa giảm cho center/outside.
5. SVG: gradient alpha nội suy straight-alpha. Xem A.
6. Clippy `never_loop` ở `validate/{paint,shadow}.rs`.

## Điểm review A — SVG alpha gradients

Thực nghiệm Chromium (SVG viết tay, stop-color + stop-opacity, so `linear-gradient`):
lệch tối đa **64** cho đỏ → `#0000` và đỏ → `#00f0`, **20** cho `#ff000080 → #0000ffcc`
(giữa dải: CSS `255,128,128`, SVG `191,128,128`). Cờ `color-interpolation` không đổi điều
đó. Sửa: `crates/server/src/svg/stops.rs` mở rộng stop giống `renderer/fill.ts` — stop
alpha 0 lấy màu hàng xóm ở cả hai phía; đoạn giữa hai alpha khác nhau (≠ 0) chia 64 bước
theo đường premultiplied; xuất hex 6 chữ số + `stop-opacity` (đọc được ở SVG 1.1).
Kiểm pixel production (`tests/paint_svg.rs` → `spikes/pipeline/paint-svg-offline.mjs`,
offline, không request mạng): SVG vs CSS **1 px / max 8** (scale 1), **6 px / max 8**
(scale 2), gồm red→`#0000`, red→`#00f0`, alpha khác nhau, radial alpha, ellipse mặc định.

## Điểm review B — radii lớn bất đối xứng + width không đều

Đã xác minh bằng pixel Chromium thật (border CSS) chứ không chỉ shape:
- Ngoài: hệ số CSS trên border box, đúng ở canvas (`roundRect`) và SVG `<path>`.
- Trong: Chromium dùng đúng `ngoài − width kề` (≥ 0) và **không giảm lần hai** khi bán kính
  trong tràn hộp trong (mẫu `[100,20,80,5]`, width `[1,1,20,1]`: cung tl cắt cạnh dưới).
  Giảm lần hai (cách cũ) lệch 952 px / max 255; theo từng trục 658 px; không giảm 143 px /
  max 64 (còn lại là AA). Canvas nay dựng vùng trong bằng clip các mảnh góc ellipse
  (`renderer/stroke-ring.ts`), không qua `roundRect`.
- Đường chia màu: từ góc ngoài qua góc trong tới dây cung của cung trong (như clip cạnh
  của Chromium), thứ tự vẽ: trong mờ trước, rồi top/bottom trước right/left; seam cùng màu
  được đẩy ~1 device px (Chromium clip không AA); góc vuông khác màu: cạnh vẽ trước phủ
  qua seam (`renderer/stroke-sides.ts`).
- Border một màu, width đều, mọi góc > w/2: Chromium vẽ như stroke đường giữa; canvas làm
  giống (1px r12: 148 px / max 88 → 0 px).
- Preview: overlay inside phát radius authored để CSS tự giảm; center/outside phát
  radius đã giảm. Preview vs CSS: tối đa 3 px / max 9 trên 60 phép so.

## Lệnh và kết quả (đã chạy)

| Lệnh | Kết quả |
| --- | --- |
| `cargo test -p opencanvas-storage --test paint_props --test paint_props_reuse --test stroke_props --test resolution --test bundle_roundtrip --test states --test instances` | 3+2+6+9+3+3+4 pass |
| `cargo test -p opencanvas-layout --lib` | 29 pass (6 paint/stroke) |
| `cargo test -p opencanvas-server --lib` | 42 pass (gồm `svg::stops`) |
| `cargo test -p opencanvas-server --test states_render --test stroke_context --test export_svg --test states_context --test paint_svg --test instances_states --test skill_layer --test tools_budget` | 1+2+2+3+1+2+2+1 pass |
| `npm run check:renderer`, `npm --prefix web run typecheck` | sạch |
| `npm run test:renderer` | 10/10 |
| `npm --prefix web test` | 24/24 |
| `npm run build:renderer` + `cargo build -p opencanvas-layout --bin layout-step`; `node --test --test-concurrency=1 spikes/pipeline/{paint-properties,stroke,stroke-opacity,opacity}.test.mjs` | 21/21 |
| `cargo clippy -p opencanvas-storage --all-targets`, `-p opencanvas-layout --all-targets`, `-p opencanvas-server --lib --test paint_svg` (`-D warnings`) | sạch |

`cargo clippy -p opencanvas-server --all-targets -D warnings` còn lỗi ngoài phạm vi paint:
`tests/states_render.rs:67` (`chunks_exact`), dead code `tests/support/mod.rs`.

## Số pixel (pixel có kênh lệch > 2 / lệch tối đa), canvas vs CSS Chromium

| Ca | scale 1 | scale 2 |
| --- | --- | --- |
| G1–G7, G9 gradient | 0 / ≤1 | ≤1 / ≤8 |
| G8 hard stop 60° | 0 / 1 | 20 / 42 (AA góc tròn, xem dưới) |
| S1–S7, S9, S10 shadow | 0 / ≤2 | 0 / ≤2 |
| S8 fill trong mờ + shadow | 11 / 8 | 13 / 7 |
| stroke 1px r12, width `[1,2,3,4]`, B-uniform | 0 / 0 | ≤13 / ≤7 |
| B-oversized, B-overflow, B-sides | 135–175 / ≤64 | 262–316 / ≤66 |
| seam nhiều màu (sides, keycap, translucent, square) | 9–76 / ≤72 | 14–153 / ≤109 |
| group opacity (clip/không clip), edge-crop | 0 / ≤1 | ≤2 / ≤3 |

Ngân sách test (`spikes/pipeline/paint-fixtures.mjs`, nhân theo scale): `edge` 12 px /
max 48, `seam` 80 px / 110, `arc` 190 px / 96. Chỉ cho AA: lỗi hình học ≥ 1 px hiện thành
lệch gần 255. Mốc `edge`: hộp solid 120×220 r12 không gradient đã lệch 19 px / max 42 ở
scale 2 gần góc dưới. G8 dùng 60°: ở 45° tâm pixel nằm đúng hard stop, CSS và canvas phá
hòa khác nhau (69 px / 255 ở scale 2). `arc`: cung lớn bất đối xứng, canvas và Chromium
chọn đường AA khác nhau (clip vs fill vs stroke); preview vs canvas center/outside ≤ 347 px
/ max 86 ở scale 2, ≤ 7 px / max 8 ở scale 1.

## Group opacity và crop ở mép export

- Opacity layer không cắt shadow: test kiểm pixel cách đáy group 6 px (trong blur, ngoài
  hộp group) vẫn có shadow khi `clip:false`, trắng khi `clip:true`; khớp CSS ≤ 1.
- Viewport export vẫn là layout bounds: node sát góc dưới phải với shadow `x6 y8 blur16`
  — canvas 480×300·scale, shadow bên trong vẫn vẽ, phần vượt mép bị cắt đúng như CSS nhìn
  qua cửa sổ 480×300 (≤ 2 px / max 3). Đây là **crop của viewport**, giới hạn đã công bố,
  khác lỗi clip của opacity layer.

## Giới hạn công bố và rủi ro còn mở

- SVG: shadow/stroke không vẽ, có diagnostic theo file; export PNG nếu cần.
- Không: blur, conic gradient, gradient/shadow trên text, shadow trong state.
- Preview: stroke center width lẻ đặt overlay ở nửa CSS px, Chromium snap (1140 px / max
  208 ở scale 1, width `[3,9,5,13]`) — giới hạn hình học DOM phân số có từ trước, test dùng
  width chẵn cho center.
- Stroke một màu dạng stroke đường giữa: một số pixel cạnh thẳng thiếu ≤ 3/255 alpha (Skia
  stroker); `stroke.test.mjs` so premultiplied trong ±3 thay vì bằng tuyệt đối.
- Inset shadow + inside stroke khi bán kính trong tràn hộp: clip vẫn qua `roundRect` (giảm
  lần hai); chưa có test pixel cho tổ hợp này.
- Canvas fill cung 76 px lệch AA với CSS ở scale 2 (49 px / max 38 trong ca SVG).
- `tsconfig.json` gốc thêm `allowImportingTsExtensions` để chuỗi `stroke.ts` import được
  khi web test chạy TS bằng Node.

## File

Mới: `renderer/stroke-ring.ts`, `renderer/stroke-sides.ts`, `crates/server/src/svg/stops.rs`,
`crates/server/tests/paint_svg.rs`, `crates/storage/tests/paint_props_reuse.rs`,
`spikes/pipeline/paint-svg-offline.mjs`, báo cáo này. Sửa: `renderer/{corners,stroke}.ts`,
`web/src/preview/{paintStyle,sceneDom,stateStyles}.ts`, `crates/storage/src/validate/
{paint,shadow,states,quad}.rs`, `crates/storage/tests/paint_props.rs`,
`crates/layout/src/{paint_tests.rs,model/fill.rs,model/shadow.rs}` (rustfmt),
`crates/server/src/svg/{paint,geometry,mod}.rs`, `crates/server/src/jobs/states.rs` (rustfmt),
`crates/server/src/props_doc.rs`, `crates/server/skills/{author,implement}-frame.md`,
`docs/SERVER.md`, `spikes/pipeline/paint-fixtures.mjs`, `spikes/pipeline/paint-properties.test.mjs`,
`spikes/pipeline/stroke.test.mjs`, `tsconfig.json`.
