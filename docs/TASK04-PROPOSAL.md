# Task04 — đề xuất row wrap đã được người dùng duyệt

Ngày 2026-10-28. **Tài liệu thiết kế và probe giai đoạn đề xuất.**
Row wrap nay đã triển khai và root nghiệm thu; xem `TASK04-REPORT.md` cho kết quả
production. Bằng chứng prototype bên dưới giữ bối cảnh trước khi sửa ứng dụng.
Người dùng đã duyệt qua form: **“Duyệt, triển khai row wrap”**. Nội dung bên dưới
là thiết kế được duyệt; thực thi cập nhật PLAN trước code ở worker mới.
Root trình duyệt phạm vi dưới đây, rồi cập nhật `PLAN.md` **trước** delta production.
Pha này không sửa PLAN, production, mẫu welcome, AGENTS/task status hoặc server3847.

## Quyết định đề nghị duyệt

Thêm **`layout.wrap: boolean`, mặc định `false`; `true` chỉ dùng với `mode:"row"`**.
Giữ sizing fixed/fill/hug, scalar padding/gap hiện có. Không thêm `%`, breakpoint,
media query, column wrap, wrap-reverse, align API hoặc variant/resolver mới.
Lý do: Task02/05 đã chứng minh card chạm minimum nhưng không xuống hàng; row wrap
giải quyết trực tiếp ca đó, không cần nhiều bộ props theo viewport.

Ví dụ các `props` trong một frame fixed1040×844, layout column/padding24:

```json
{
  "cards": {
    "width": "fill", "height": "hug",
    "layout": {"mode": "row", "gap": 16, "wrap": true}
  },
  "eachCard": {
    "width": "fill", "minWidth": 280, "height": "hug",
    "layout": {"mode": "column", "padding": 16, "gap": 8}
  },
  "cardText": {
    "width": "fill", "height": "hug", "text": "Nội dung tiếng Việt…",
    "fontId": "font_bundled", "fontSize": 16, "lineHeight": 24
  }
}
```

Đây là tên minh họa cho props, không phải operation/schema mới. Fixture hoàn chỉnh
ở [fixture.mjs](../spikes/responsive-proposal/fixture.mjs). Chiều rộng trong frame
là viewport−48: 992px chứa3 card;342px chỉ chứa1 card tối thiểu280px.
Min120px của mẫu cũ không thể hiện yêu cầu một cột:342px vẫn chứa2 card120px + gap16.

## Quy tắc schema và validate chính xác

1. `wrap` chỉ nhận JSON boolean; thiếu hoặc `false` giữ no-wrap. Cho phép explicit
   `false` trong row/column/free để không cần xóa field khi đổi mode. `true` đòi
   container page/frame/group và mode row; leaf, column/free bị từ chối.
   Không nhận null, string, số hoặc token/style ref cho wrap; unknown layout key
   vẫn bị storage từ chối. Direct typed input phải reject sai kiểu khi deserialize.
2. Row có `wrap:true` phải có width xác định: số **>0**, hoặc `"fill"` với chuỗi
   width của ancestor đều `"fill"` cho tới width số>0. Thiếu width/`"hug"` ở bất
   kỳ mắt xích nào bị từ chối; chỉ min/maxWidth không đủ để biến hug thành definite.
   Storage kiểm trên props đã resolve ở **mọi theme**, chốt ở root frame fixed.
   Direct layout dùng selected frame/viewport làm mốc definite (không đi ra page).
   Các ancestor khác vẫn chịu guard fill trong hug hiện có, cả width và height.
3. Mỗi **child trực tiếp có width fill của wrapping row** phải có `minWidth>0`
   tường minh (literal hoặc dimension ref đã resolve). Đây là guard mới, chỉ bật
   cho wrapping row; ngăn basis0 vô tình gom tất cả child vào một hàng.
   Child fixed/hug được phép, dùng kích thước hypothetical/intrinsic của Taffy.
   Không áp minimum này cho text fill bên trong card column. Row rỗng hợp lệ.
4. Min/max tiếp tục hữu hạn trong0…100000; **minWidth≤maxWidth và minHeight≤maxHeight**
   khi cả hai có mặt, ở storage và direct layout, kể cả no-wrap. Storage đã có
   kiểm này; direct hiện thiếu và phải bổ sung. Equality hợp lệ. Fixed size nằm
   ngoài min/max vẫn clamp như hiện tại (fixture fixed60/min80 phải giữ), không
   thêm luật reject fixed ngoài range. Root frame authored vẫn fixed cả hai trục.
5. Không reject thiết kế chỉ vì tổng minima vượt viewport. Một card lớn hơn inner
   width vẫn ở một hàng riêng và có thể overflow; wrap không có phép thu nhỏ dưới
   minimum. Padding lớn hơn available width cũng không hứa content fit. Các numeric
   bounds, text/node/pixel/depth budgets hiện có giữ nguyên.

Storage lỗi trả `VALIDATION_FAILED` kèm field, không commit một phần. Direct lỗi
ngữ nghĩa trả `LAYOUT_INVALID`; boolean sai kiểu bị CLI deserialize thành
`INVALID_JSON` như contract typed input hiện tại. Kiểm child/ancestor phải chạy
trên candidate cuối và cả khi update/reparent hoặc đổi token/theme làm rule mất
hợp lệ; giữ validation theo thứ tự operation hiện có, không tự thêm forward refs.

## Hành vi bố cục và text

Taffy giữ thứ tự sibling; collect tối đa child vừa **inner width**, tính min/max
vào hypothetical width rồi mới chia phần fill trên từng hàng. Fill vẫn
grow1/shrink1/basis0, không thêm weight. Hàng cuối một card fill có thể rộng hết
hàng; đây không phải Grid giữ cột trống. `gap` dùng cho cả giữa child và giữa hàng.

Mapping mới là `FlexWrap::Wrap` và **`AlignContent::START` chỉ khi wrap=true**;
`align_items:START` giữ nguyên. Không stretch chiều cao các card bằng nhau. Với
row height hug: height = padding trên/dưới + tổng max height từng hàng + gap giữa
hàng. Height fixed/min/max vẫn clamp theo quy tắc cũ, nên có thể overflow dọc;
không dùng clip để giấu bounds. Child height fill bên trong row height hug vẫn
bị guard cũ chặn; dùng card/text height hug hoặc fixed khi muốn content-driven.

`engine.rs` tiếp tục dùng callback đồng bộ cache-only; browser đo theo width thật
của card, rồi Rust dựng lại. Measurement key vẫn gồm text/font/hash/style/width
làm tròn1/64px; wrap không phải thành phần key đo text vì text ở cùng width có cùng
metrics. Một batch RPC mỗi pass, tối đa6 passes; không tăng budget để che cycle.
Không publish scene khi còn miss, kể cả line list ở final width. Text vẫn plain
text, LF/whitespace/grapheme wrapping do renderer hiện có; không thêm CSS shaping.

## Bằng chứng đã chạy, giới hạn của bằng chứng

[run.mjs](../spikes/responsive-proposal/run.mjs) copy nguyên crate layout vào thư
mục tạm, thêm bool DTO + mapping Taffy ở **bản sao**, cargo build offline vào target
tạm; reuse worker/renderer thật với binary tạm. Không sửa hoặc rebuild binary thật.
**Prototype chưa có các guard mới nêu trên; chưa đi qua storage/MCP/viewer.**

Lượt thành công08:44:17.533Z–08:44:23.372Z ngày2026-10-28: Node24.21.0,
Chromium156.0.8078.4, Taffy0.15.1; **15 ca pass**, cleanup thư mục tạm=true.
Xem [evidence.json](../spikes/responsive-proposal/evidence.json) cho bounds, lines,
font manifests, scene provenance và metrics đầy đủ.

| Viewport width | Số hàng / số card mỗi hàng | Width card | Row hug height, hình học | Row hug height, text thật |
| --- | --- | --- | ---: | ---: |
| 1040 | 1 / 3 | 320,320,320 | 80 | 128 |
| 640 | 2 / 2+1 | 288,288,592 | 156 | 224 |
| 390 | 3 / 1+1+1 | 342,342,342 | 212 | 320 |

Hình học dùng card heights40/60/80, gap16. Boundary chính xác: viewport920→3card
một hàng;919→2+1;624→2+1;623→1+1+1. Bounds card nằm trong vùng padding; text mỗi
line≤text width, card height=text height+32, text height=lineCount×24, hàng tiếp
cách max bottom hàng trước16px. Desktop text2/4/3 lines; mobile2/3/3 lines. Không
node fixture text nào overflow frame ngang/dọc; đã mở PNG1040 và390 để xem thật.
[PNG1040](../spikes/responsive-proposal/text-1040.png),
[PNG640](../spikes/responsive-proposal/text-640.png),
[PNG390](../spikes/responsive-proposal/text-390.png).

Text cold mỗi viewport:2 passes/1 batch/3 measurements; lặp lại:0 batch, scene
giữ nguyên. Geometry:1 pass/0 batch. Controls: no-wrap390 tràn; fill không min ở
390 vẫn một hàng; min344 tràn vùng inner342 dù đã wrap. Omitted wrap tại1040/390
cho `scene.nodes` giống binary production hiện có (không khẳng định hash/provenance
giống). Authored input không bị thay đổi trong mọi lượt.

Lệnh thật: `node spikes/responsive-proposal/run.mjs`; build copy offline thành
công sau khi sửa type inference của align-content trong prototype. Lượt sandbox
kế tiếp bị `listen EPERM` trước browser. Chạy lại cùng lệnh với escalation được
duyệt mới có15 ca pass; không có auto-review rejection. Không gọi server3847.
Prettier đã format3 module; syntax/format/diff và size checks ghi trong README
probe. Chưa chạy pipeline11/storage/server/E2E cho feature; chưa kiểm negative
guard trên app vì production chưa hỗ trợ wrap. Probe không thay thế các AC sau.

## Chi phí lan toả, file và tương thích

Không dependency mới; giữ pin Taffy`=0.15.1` (std/taffy_tree/flexbox). Đã đọc cached
source: `src/style/flex.rs` (FlexWrap), `src/style/mod.rs` (Style defaults),
`src/compute/flexbox.rs` (hypothetical min clamp, collect_flex_lines, align-content).
NoWrap là default; content-derived indefinite main size không chia hàng. Vì vậy
guard definite width và positive minimum cho fill là phần thiết kế, không chỉ bật
API. Không cần field “số hàng” trong scene: bounds/parentId đã thể hiện mọi hàng.

| File dự kiến sau duyệt | Trách nhiệm / giới hạn |
| --- | --- |
| `PLAN.md` | Root cập nhật layout scope + AC trước production delta |
| `crates/layout/src/model.rs` + mới `model/layout.rs` | Hiện225 dòng; tách Sizing/LayoutRule, re-export giữ public API, bool defaultfalse (skip serialize false để giữ dạng no-wrap cũ) |
| `crates/layout/src/tree.rs` | Hiện90 dòng; mapping wrap + align-content, engine183 dòng dự kiến không đổi |
| `crates/layout/src/validation.rs` + mới `validation/layout.rs` | Hiện132; tách geometry/layout guards, bổ sung direct min/max parity, giữ mỗi module≤250 |
| `crates/storage/src/validate/properties.rs` + mới `validate/layout.rs` | Hiện242, **phải tách** layout/sizing validation trước khi thêm, không dồn vào file này |
| `crates/storage/src/validate.rs` | Khai báo module mới; không sửa storage model128 dòng, DB/store/migration |
| Tests layout/storage + `spikes/pipeline/responsive-{fixture,test}.mjs` | Direct rejects, transaction/theme safety, Chromium fixture và no-wrap compatibility; mỗi module≤250 |
| `crates/server/tests/` + `spikes/e2e/` fixture nhỏ | MCP round-trip/context/render/restart + GET-only viewer của fixture mới, giữ ca cũ; không đổi server behavior hoặc web source |
| `docs/RESOLUTION.md`, `docs/SERVER.md`, `docs/ACCEPTANCE.md`, `crates/server/skills/{author,implement}-frame.md` | Công bố schema, minimum guard, HTML flex-wrap mapping, giới hạn và evidence; bỏ hướng dẫn “không có wrap” sau khi code được kiểm |

Mức lan toả vừa:2 seam validation +1 mapping layout, fixture/browser/regression là
phần chính; không mở rộng renderer/worker/viewer hoặc resolver architecture.
Nếu cần đổi web/schema scene để hoàn thành, dừng và trình scope mới.

`props_json` additive nên **không migration**; DB schema2, snapshot/API và scene
schema1, `scene-1`/`web-1` giữ nguyên. Apply/update wrap là batch bình thường tăng
revision đúng1, retry giữ idempotency. Token-dependent minima được validate mọi
theme; không thêm theme/variant vào authored model. Bundle bảo toàn JSON và dùng
validator mới khi import/restore; không đổi format bundle.

Scene/job key hiện hash input+viewport/theme/state/revision+scale+startup pipeline
fingerprint; bool và sizing refs đi theo input đã resolve. Fingerprint hiện đọc
`target/debug/layout-step`, renderer bundle, Cargo/package locks và worker/runtime;
**build layout binary mới rồi restart backend** sẽ đổi identity, không tái dùng
cache cũ. Không hứa hot replacement; pha này không restart3847. Text cache không
cần clear ngoài lifecycle worker hiện có. Direct source key có DTO serialization
riêng; skip false tránh thêm field vào serialization no-wrap không cần thiết.

Viewer vẫn đọc authored props và scene computed; root kích thước viewport hữu hạn,
flat frame-local bounds + line lists giữ nguyên, không layout/text wrap lần hai.
MCP `apply_batch` props đang là JSON object mở nên không cần endpoint hay envelope
mới. `get_subtree`/context trả authored wrap, inspect trả bounds, multi-viewport
context dùng jobs hiện có. Cần nâng đồng bộ backend + layout: backend cũ reject
wrap, layout binary cũ có thể bỏ qua unknown field; không hứa downgrade/hỗn hợp.

## AC triển khai sau duyệt và kiểm bắt buộc

1. Fixture mới qua MCP/temp root:1040 có3card một hàng,640 có2+1,390 có1+1+1;
   IDs/revision chung, đúng bounds và row hug heights theo bảng. Không descendant
   overflow frame hoặc card; không miễn node vì clip. Long Vietnamese/unbroken
   text có line list do browser thật, height đúng, không chồng lên hàng kế tiếp.
2. Có row hug2/3 hàng, boundary920/919/624/623, row fixed-height không tự stretch
   hàng, nested wrapping row có finite ancestor; control oversized min giữ overflow
   thật. Missing/false wrap giữ toàn bộ no-wrap fixtures cũ, authored bất biến.
3. Storage **và direct layout** reject wrap sai mode/type, hug/indefinite width,
   fill child thiếu/nonpositive minWidth, min>max cả2 trục; equal min/max hợp lệ.
   Test token/theme update, reparent và batch cuối chứa op lỗi: snapshot/revision/
   log/idempotency không commit phần trước; direct fail trước measurement miss.
4. MCP path: create→apply→subtree/context/inspect/render cả3 viewport→restart→read
   lại JSON wrap/revision/bounds/PNG; context reflow=true khi ready, pending/failed
   giữ contract. Chỉ temporary projects; viewer GET-only, vẽ cùng scene/PNG; không
   tạo sample mới ở data root người dùng.
5. Chạy `cargo test -p opencanvas-layout --offline`; `npm run test:pipeline`
   **≥11 tests, giữ đủ11 cũ**; full storage `cargo test -p opencanvas-storage
   --offline --features fault-injection -- --test-threads=2`; targeted server
   integration cho fixture, existing multi_viewport/skill_layer compatibility.
   `npm run test:e2e` giữ15 nhóm cũ và thêm kiểm wrap (builds theo script hiện có).
   Production Clippy layout/storage/server `--offline --lib -- -D warnings`;
   `cargo fmt --all --check`, JS format/syntax, `git diff --check`, size/module
   review≤250 dòng. Không ghi pass trước lệnh thật.

## Giới hạn vẫn còn sau Task04

Wrap **không tự sửa mẫu cũ thành mobile**: sidebar176 + body min480 vẫn cần656px;
không tự collapse navigation hoặc hạ body minimum. Fixed chart plot không scale,
progress không giữ tỷ lệ80%, fixed text/card heights có thể tràn dọc; cần author
lại riêng nếu muốn. Không thêm percentage sizing, breakpoint variants, tương tác,
arbitrary interaction state hoặc responsive export tự động. Bản đề xuất này chỉ
xin duyệt row wrap cùng guards và AC trên; chưa thay `PLAN.md` hay code production.
