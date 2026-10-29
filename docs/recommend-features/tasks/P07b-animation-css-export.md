# P07b — Xuất animation cho AI coder: `get_design_context animationCss:true` (CSS @keyframes/transition, JS cho click) + diagnostic `ANIMATION_DOES_NOT_SETTLE`
Ưu tiên: P1 · Kích cỡ: ~70 phút dev · Phụ thuộc: **P07a** (prop `animations`, validate R1–R11, `resolvedProps.animations`). File nóng `reads/tree.rs`, `schemas.rs` (xem Rủi ro). · Nguồn: U3 ("xuất được cho AI coder: CSS @keyframes/transition hoặc JS Web Animations tương đương")

Phase thứ hai của P07:

- P07a: dữ liệu;
- **P07b (file này)**: export;
- P07c: render tại t;
- P07d: viewer preview.

Riêng P07b có giá trị độc lập: AI coder nhận CSS chạy được ngay, có ngữ nghĩa **trùng khít** với dữ liệu design. Không cần renderer mới.

## Vấn đề

- Sau P07a, `get_design_context` trả `props.animations` dạng WAAPI-ish. AI coder vẫn phải tự map:
  - `translateX/Y` → CSS `translate`;
  - `fill` → `background-color` hay `color` tuỳ kind;
  - keyframe ngầm định;
  - easing theo đoạn;
  - `fillMode` mặc định `both` (khác mặc định `none` của CSS);
  - hover → transition;
  - click → JS.
- Mỗi điểm trên là một chỗ dễ sai, có tài liệu công khai. Ví dụ:
  - CSS áp `animation-timing-function` cho **từng đoạn** keyframe, còn `easing` trong options của `element.animate()` áp cho **cả vòng**. Hai cách chỉ trùng nhau khi có đúng 2 keyframe.
  - Animation đang `fill: forwards` sẽ **đè** giá trị `:hover`/transition của cùng thuộc tính.
- Nguyên tắc của dự án là "ít token, khó sai cho agent". Server biết chính xác ngữ nghĩa, nên server nên sinh code tương đương và có test khẳng định nó tương đương.

## Bằng chứng

Code tham chiếu: `night/integration` @ `f2b0376`, cộng P07a.

- `crates/server/src/reads/tree.rs:58-80`: node trong context có `props` authored và `resolvedProps`. Đây là chỗ gắn thêm một field cho từng node (giống `computedBounds` ở `:77-79`).
- `crates/server/src/reads.rs:16-44` `ReadRequest` (`deny_unknown_fields`). Field opt-in kiểu `compact` được chuẩn hoá `false → absent` (`:41-43`, `:47`) để giữ hash scope của cursor. `animationCss` làm y hệt.
- `crates/server/src/schemas.rs:93-98`: chỗ thêm param và câu mô tả cho `get_design_context` (`compact`, `viewports`).
- `crates/server/src/reads/tree.rs:94-102`: **`diagnostics` bị ghi đè thành `[]` khi `reflow == true`** (`:98-99`). Diagnostic animation phải được nối vào **sau** đoạn này, nếu không sẽ mất.

**Prototype** (`/private/tmp/claude-501/-Volumes-MySpace-Workspace-OpenCanvas/6450e799-5289-4221-a46f-6264064be06e/scratchpad/p07/anim.js`, hàm `exportCss`; `run.mjs`):

- CSS xuất theo quy tắc ở dưới được nạp vào Chrome 156. Sampler của P07a/P07c được so với `getComputedStyle` ở 9 ca × 561 mẫu, có tua `currentTime`.
- Lệch tối đa: opacity 5,4e-7, translate 4,9e-5 px, rotate 4,9e-5°, scale 4,9e-6, RGB 0, alpha 0,003.
- Không có declaration nào bị Chrome bỏ.
- Độ dài: tổng CSS 1373 ký tự so với JSON authored 1242 ký tự, **×1,11** trên 9 ca. Ví dụ `enter`: 106 → 128 ký tự.
- Ví dụ một ca nhiều keyframe, đúng output prototype:
  ```css
  .oc-n1{animation:oc-n1-pop 1000ms ease-in-out 0ms 1 normal both}
  @keyframes oc-n1-pop{0%{opacity:0;scale:0.8}60%{scale:1.1;animation-timing-function:cubic-bezier(0.34,1.56,0.64,1)}100%{opacity:1;scale:1}}
  ```

**Tham khảo**

- MDN `animation` shorthand (thứ tự name, duration, timing-function, delay, iteration-count, direction, fill-mode): https://developer.mozilla.org/en-US/docs/Web/CSS/animation.
- Keyframe ngầm định và easing theo keyframe trong WAAPI: https://developer.mozilla.org/en-US/docs/Web/API/Web_Animations_API/Keyframe_Formats.
- CSS transitions và hành vi đảo chiều khi bị ngắt: https://www.w3.org/TR/css-transitions-1/#reversing.
- Thuộc tính riêng `translate`/`rotate`/`scale`: Chrome 104+, Safari 14.1+, Firefox 72+. https://developer.mozilla.org/en-US/docs/Web/CSS/scale.

## Giải pháp

**Tham số mới:** `get_design_context {…, animationCss: true}`, boolean, mặc định `false`.

- Bật thì mỗi node **có animation** nhận thêm field `animationCss`, là một string.
- Nếu node có animation `on:"click"`, node nhận thêm `animationJs`, là một string.
- Node không có animation không có field nào mới.
- Không bật thì response không đổi byte nào.

**Selector:** `.oc-<nodeId>` là placeholder rõ ràng. `implement-frame.md` dặn "thay bằng selector/class của component".

**Tên keyframes:** `oc-<nodeId>-<name>`. Node ID chỉ gồm `[A-Za-z0-9_-]` và name là slug, nên ghép lại luôn là CSS ident hợp lệ.

**Quy tắc sinh code.** Hàm thuần `animation_css(kind, authored, resolved) -> Option<{css, js}>`.

1. **Thuộc tính.**
   - `opacity` → `opacity:<n>`.
   - `translateX/Y` → `translate:<x>px <y>px`. Nhờ P07a R6, X/Y luôn nằm cùng keyframe; component không có trong animation thì ghi `0`.
   - `rotate` → `rotate:<n>deg`.
   - `scale` → `scale:<n>`.
   - `fill` → `background-color`, hoặc `color` nếu node là text.
   - Số in tối đa 4 chữ số thập phân, bỏ số 0 thừa.
2. **Token.**
   - `fill` authored là `{tokenId:T}` → `var(--T,<giá trị resolved>)`. Giữ được tham chiếu, đúng nguyên tắc "code theo tham chiếu", và vẫn chạy khi chưa định nghĩa biến.
   - `{styleId}` → literal resolved.
   - Số dùng token → literal resolved. Lý do: biến số trong `animation` cần `calc(var()*1ms)`, rối và dễ sai.
3. **`on:"load"`.**
   - `@keyframes oc-<id>-<name>{<p>%{…}…}`, chỉ gồm các keyframe authored, **không** thêm keyframe ngầm định: CSS tự lấy giá trị gốc của phần tử, đúng ngữ nghĩa P07a.
   - Keyframe có `easing` riêng thì thêm `animation-timing-function:<e>` vào khối đó.
   - Rule: `.oc-<id>{animation:<name> <duration>ms <easing> <delay>ms <iterations> <direction> <fillMode>[, …]}`. Mọi animation load của node gộp trong một danh sách.
   - Luôn ghi đủ 7 thành phần, nên không phụ thuộc mặc định CSS (`fillMode both` khác mặc định CSS `none`).
4. **`on:"hover"`.** P07a R7: một keyframe, `offset` 1.
   - `.oc-<id>{transition:<prop> <duration>ms <easing> <delay>ms[, …]}`;
   - `.oc-<id>:hover{<giá trị keyframe>}`.

   Đây đúng ngữ nghĩa P07a: vào thì chạy tới keyframe, ra thì chạy về props authored, cùng duration/easing. Bị ngắt giữa chừng thì CSS tự rút ngắn theo "reversing shortening".

   Nếu node cũng có animation load, `transition` nằm cùng rule `.oc-<id>` với `animation`. P07a R5 bảo đảm hai bên không cùng thuộc tính, nên `:hover` không bị animation đang fill đè.
5. **`on:"click"`.** Mỗi click chạy lại từ đầu.
   - CSS: `@keyframes` như load, cộng rule `.oc-<id>.oc-<id>-<name>{animation:…}`;
   - `animationJs`: `el.addEventListener("click",()=>{el.classList.remove("oc-<id>-<name>");void el.offsetWidth;el.classList.add("oc-<id>-<name>")})`.

   Chọn cách đổi class thay vì `el.animate()`. Lý do: giữ một nguồn ngữ nghĩa là CSS đã kiểm, đúng keyframe ngầm định và easing theo đoạn. `el.animate()` lấy easing của keyframe ngầm định là `linear`, khác CSS.
6. **Thứ tự output:** rule base, `:hover`, rule click, rồi các `@keyframes`, nối bằng `\n`.

Ví dụ: card có ba animation `enter` (load), `lift` (hover, scale), `pulse` (load, fill token).

```css
.oc-card{animation:oc-card-enter 400ms ease-out 0ms 1 normal both,oc-card-pulse 800ms ease 0ms infinite alternate both;transition:scale 200ms ease-out 0ms}
.oc-card:hover{scale:1.02}
@keyframes oc-card-enter{0%{opacity:0;translate:0px 12px}}
@keyframes oc-card-pulse{100%{background-color:var(--color-brand-soft,#dbeafe)}}
```

**Diagnostic `ANIMATION_DOES_NOT_SETTLE`**, nối vào `diagnostics` của `get_design_context`, luôn bật vì rẻ.

- Điều kiện: animation `on:"load"` có `iterations` hữu hạn, `fillMode` là `forwards` hoặc `both`, và **trạng thái giữ sau khi xong khác props authored**.
- Trạng thái cuối:
  - tính chiều của vòng cuối theo `direction` và số vòng (công thức WAAPI);
  - tiến độ cuối 1 thì lấy keyframe `offset 1`, tiến độ cuối 0 thì lấy keyframe `offset 0`;
  - thiếu keyframe ở đầu đó thì là giá trị authored, tức không lệch;
  - `iterations` lẻ phần (ví dụ 2,5) thì dừng giữa vòng, coi như lệch mọi prop đang animate.
- Hậu quả: render tĩnh và `render_frame` không có `timeMs` vẽ props authored, còn code thật sau khi load lại hiện giá trị khác. Agent tự kiểm bằng ảnh sẽ thấy sai mà không biết vì sao.
- Ví dụ:
  ```json
  {"code":"ANIMATION_DOES_NOT_SETTLE","nodeId":"badge","ref":"badge:dim","props":["opacity"],
   "message":"Load animation 'dim' holds opacity 0.5 after it ends, but the node's authored opacity is 1, which static renders show. Author the settled state in props, or use fillMode none or backwards."}
  ```

**Vì sao không trả CSS mặc định** (luôn kèm, không cần opt-in):

- Thêm khoảng ×1,1 số ký tự của animation cho mọi lần đọc context. Phần lớn lần đọc là để dựng layout.
- Opt-in giống `compact`: agent bật khi bắt tay vào code chuyển động. Mô tả tool và `animated` của P07a cho agent biết khi nào cần bật.

**Vì sao không tạo tool `get_animation_code` riêng:**

- Thêm một tool làm `tools/list` dài thêm cho mọi client.
- Dữ liệu đã nằm cạnh node trong context, nơi AI coder đang đọc.

## Phạm vi file

| File | Thay đổi | Dòng ước tính |
|---|---|---|
| `crates/server/src/reads/animation_css.rs` | mới: `animation_css(kind, authored, resolved)`, định dạng số, `var()` cho token | ~170 |
| `crates/server/src/reads/animation_settle.rs` | mới: diagnostic `ANIMATION_DOES_NOT_SETTLE` cho scope | ~70 |
| `crates/server/src/reads.rs` | `ReadRequest.animation_css: Option<bool>` (skip khi absent, `false → absent`), khai báo module | 92 → ~96 |
| `crates/server/src/reads/tree.rs` | gắn `animationCss`/`animationJs` cho node; nối diagnostic sau đoạn reflow | ~144 (sau P07a) → ~152 |
| `crates/server/src/schemas.rs` | `properties["animationCss"]` + một câu mô tả (≤ 260 ký tự) | 159 → ~162 |
| `crates/server/skills/implement-frame.md` | mục Animation: bật `animationCss`, thay selector, `prefers-reduced-motion` là quyết định của người dùng | +8 |
| `docs/SERVER.md` | param và field mới, quy tắc sinh code (tóm tắt) | +12 |
| `crates/server/tests/fixtures/animation_css.json` | mới: ≥ 10 ca `{id, kind, props, css, js?}`, dùng chung với P07c | ~110 |
| `crates/server/tests/animation_css.rs` | mới: golden và plumbing (AC1–AC6, AC8) | ~150 |
| `spikes/pipeline/animation-css.test.mjs` | mới: nạp CSS golden vào Chromium, kiểm parse và giá trị dạng đóng (AC7) | ~110 |

Mọi file viết tay ≤ 200 dòng.

## Tương thích MCP

- `tools/list`: `get_design_context` thêm một property optional `animationCss: {"type":"boolean"}`. Vẫn là object phẳng, `tool_schemas.rs` vẫn 15 tool.
- Param mặc định false, response không đổi. Cursor cũ vẫn dùng được: hash scope không đổi khi param absent hoặc false.
- `animationCss`, `animationJs` và diagnostic mới là field cộng thêm. Không field cũ nào đổi nghĩa.
- Tài liệu: `docs/SERVER.md`, `implement-frame.md`. Test `skill_layer.rs` so nội dung qua `include_str!`, nên tự khớp.

## AC

1. **Golden.** Với mỗi ca trong `crates/server/tests/fixtures/animation_css.json`, `animation_css()` trả **đúng từng byte** `css` (và `js` nếu có). Fixture gồm tối thiểu:
   - (a) `enter` một keyframe offset 0;
   - (b) 3 keyframe có easing riêng ở keyframe giữa;
   - (c) `steps(4, end)`;
   - (d) `infinite alternate` với fill trên rectangle;
   - (e) fill trên text → `color`;
   - (f) `fillMode none` có `delay`;
   - (g) `iterations 2.5 alternate-reverse` với translate + rotate;
   - (h) hai animation load trên một node;
   - (i) load + hover trên một node;
   - (j) click;
   - (k) `fill` token → `var(--T,#hex)`.
2. **Plumbing.**
   - `get_design_context {animationCss:true}` trên frame có 2 node động: đúng 2 node có `animationCss`, đúng node có click có `animationJs`, các node khác không có field mới.
   - Không có param, hoặc `animationCss:false`: response **byte-identical** với bản trước P07b, kể cả `nextCursor`.
3. **Theme.** Cùng frame, `theme:"dark"`: phần `var(--color-brand-soft,<hex>)` đổi fallback sang giá trị theme dark, tên biến giữ nguyên.
4. **Token.**
   - Tổng số ký tự `animationCss` / tổng số ký tự JSON authored `animations` trên fixture ≤ **1,3** (prototype 1,11).
   - Với `compact:true`, token chỉ dùng trong animation vẫn có trong `tokens` (giữ AC5 của P07a).
5. **Diagnostic.**
   - `ANIMATION_DOES_NOT_SETTLE` có mặt cho:
     - (a) load `[{offset:1, opacity:0.5}]` trên node opacity 1, `fillMode both`;
     - (b) `iterations 2.5`.
   - Không có mặt cho:
     - (c) entrance `[{offset:0, opacity:0}]`;
     - (d) cùng (a) nhưng `fillMode backwards`;
     - (e) `alternate` 2 vòng (kết thúc ở offset 0 = ngầm định);
     - (f) `infinite`;
     - (g) hover.
   - Diagnostic còn nguyên trên frame có `viewports` cho `reflow:true`: test khẳng định không bị đoạn `tree.rs:98-99` xoá.
6. **Schema.** `tools/list` có `animationCss` boolean. `tool_schemas.rs` pass. `animationCss:"yes"` bị lỗi schema.
7. **Chạy được trong Chrome.** `node --test spikes/pipeline/animation-css.test.mjs` dùng Chromium của repo. Mỗi ca golden:
   - (a) `CSSStyleSheet` có đúng số rule. Mỗi `CSSKeyframeRule` giữ đủ số declaration đã xuất, tức không có declaration nào bị bỏ vì sai cú pháp.
   - (b) `el.getAnimations().length` bằng số animation load.
   - (c) Giá trị dạng đóng sau `pause()` và `currentTime = t`, sai số ≤ 1e-3:
     - linear fade 1000 ms tại 250 → opacity 0,25;
     - `steps(4, end)` 0→100 px tại 600 → translate `50px`;
     - alternate infinite scale 1→2 linear 1000 ms tại 1300 → scale 1,7;
     - `fillMode none` delay 200 tại 100 → opacity authored 0,5.
   - (d) Hover: `page.hover()` rồi chờ `duration + 100` ms → computed `scale` = 1,02. Rời chuột rồi chờ → 1.
   - (e) Click: chạy `animationJs` với `el` rồi `el.click()` → `getAnimations().length === 1`.
8. **Không hồi quy.** `cargo test -p opencanvas-server --test context --test compact_context --test multi_viewport` pass.
9. **Modular.** `wc -l` các file mới hoặc sửa đều ≤ 200. Báo số dòng `animation_css.rs`, `tree.rs`.

## Cách kiểm

```sh
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server --test animation_css --test tool_schemas --test context --test compact_context --test multi_viewport
node --test spikes/pipeline/animation-css.test.mjs     # PLAYWRIGHT_BROWSERS_PATH=.runtime/browsers như các test pipeline
wc -l crates/server/src/reads/{animation_css,animation_settle,tree}.rs crates/server/src/reads.rs crates/server/src/schemas.rs
```

Kiểm live trên server tạm (`--port 0 --data-root "$(mktemp -d)"`):

1. tạo card có `enter` + `lift`;
2. gọi `get_design_context {frameId, animationCss:true}`;
3. dán `animationCss` vào một file HTML nháp có `<div class="oc-card">`, mở bằng Chromium của repo, rê chuột vào;
4. dọn process và data root.

## Rủi ro

- **Lệch CSS và sampler.** Sampler TS ở P07c phải trùng output này. Golden fixture dùng chung là chỗ khoá: P07c chạy parity với Chrome trên chính fixture đó. Sửa fixture thì cả hai phase cùng thấy.
- **Hover dạng transition.** Rời chuột thì chạy về với **cùng** easing (CSS transition), không phải chạy ngược thời gian. P07a định nghĩa đúng như vậy. Viewer P07d mô phỏng cùng cách.
- **`var(--tokenId)` giả định tên biến CSS trùng token ID.** Đây chỉ là gợi ý. AI coder map sang token của mình theo `implement-frame.md`. Fallback literal giữ cho code chạy đúng.
- **Thuộc tính riêng `translate`/`scale`/`rotate` cần trình duyệt 2022+.** Nếu dự án đích cần `transform`, coder tự gộp. Không sinh `transform` vì gộp làm hỏng nội suy riêng từng thuộc tính.
- **File nóng.** `reads/tree.rs` và `schemas.rs` cũng bị P02a (notes) và P06a (navigation) sửa; mỗi bên chỉ thêm vài dòng. Dev orchestrator xếp tuần tự hoặc rebase.

## Ngoài phạm vi / phase sau

- Xuất JS Web Animations đầy đủ (`el.animate` với keyframe ngầm định được materialize). Có thể thêm `animationCode:"waapi"` nếu agent cần React không CSS.
- Xuất cho Framer Motion. Không xuất SwiftUI hay Compose.
- Block `@media (prefers-reduced-motion)` tự động: đây là quyết định sản phẩm, chỉ ghi trong skill.
- Stagger, `transform-origin`.
