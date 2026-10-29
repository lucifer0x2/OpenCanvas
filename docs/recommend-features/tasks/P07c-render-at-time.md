# P07c — Render tại thời điểm t: `render_frame {timeMs, play}`; renderer áp transform/opacity/fill theo phần tử
Ưu tiên: P1 · Kích cỡ: ~90 phút dev · Phụ thuộc: **P07a** (dữ liệu), **P07b** (golden `crates/server/tests/fixtures/animation_css.json` cho test parity), **P01** (đã merge `f2b0376`: vòng vẽ ở `renderer/paint.ts`). Nên làm **sau P05** (`paintTree` đệ quy, nên transform của subtree áp tự nhiên); nếu P05 chưa merge thì áp trong vòng tổ tiên, xem Giải pháp. · Nguồn: U3 ("preview và render theo phần tử"; agent tự kiểm bằng ảnh)

Phase thứ ba của P07:

- P07a: dữ liệu;
- P07b: CSS export;
- **P07c (file này)**: render tại t;
- P07d: viewer.

Phase này có giá trị riêng:

- AI designer tự kiểm bằng ảnh: render khung đầu, giữa, cuối của một animation qua MCP, không cần viewer.
- Renderer dùng chung có hàm `sampleScene` và đường vẽ có transform theo phần tử. P07d chỉ còn phần lập lịch và vùng vẽ lại.

## Vấn đề

- Sau P07a/P07b, animation có trong dữ liệu và trong CSS xuất ra, nhưng **không ảnh nào cho thấy nó**. `render_frame` chỉ vẽ trạng thái nghỉ. Agent không kiểm được:
  - "ở 200 ms card đã hiện một nửa và còn lệch 6 px";
  - "hover scale 1,02 không tràn khỏi khung clip";
  - "spinner quay quanh tâm chứ không quanh góc".
- Renderer không có khái niệm transform:
  - `SceneNode` (`renderer/types.ts:47-63`; `crates/layout/src/model.rs:147-163`) chỉ có `bounds/opacity/fill/clip/radius`;
  - vòng vẽ `renderer/paint.ts:116-133` (`paintScene`), cùng `enterAncestors` `:88-110`, chỉ áp clip và nhân opacity theo tổ tiên.
- Scene không mang animation:
  - `Props` của layout bỏ qua key lạ (`crates/layout/src/model.rs:53-77`, không có `deny_unknown_fields`);
  - `engine.rs:120-151` chỉ chép các field tĩnh.

## Bằng chứng

Code tham chiếu: `night/integration` @ `f2b0376` (đã có B01–B05, P01).

**Đường render hiện tại**

- MCP `render_frame` dẫn tới `SceneRequest` (`crates/server/src/jobs.rs:27-40`, `deny_unknown_fields`), rồi `check_request` (`jobs/input.rs:58-87`), rồi `make_input` dùng node đã resolve theo theme (`jobs/input.rs:9-12`). Token trong `animations` vì vậy đã thành literal (resolver của P07a).
- Key job = sha256 của `{"input","scale","pipeline"}` (`jobs.rs:105-108`). Chỉ khi request có `timeMs/play` mới thêm vào key; như vậy key của request cũ giữ nguyên.
- Runner gọi worker `compute` rồi `render` với `scale` (`jobs/runner.rs:61-67`).
- Worker `cli.mjs:20-24` dẫn tới `pipeline.mjs:181-195` `renderer.drawScene(canvas, scene, resources, {scale, strict})`.
- Viewer gọi `drawScene` một lần cho canvas parity (`web/src/hooks/useScene.ts:88`), và gọi `paintScene` mỗi lần Konva vẽ (`web/src/components/CanvasView.tsx:106-112`).
- Test renderer `renderer/*.test.mjs` chạy bằng **Node** (`package.json:8`, `renderer/image-fit.test.mjs`). Vì vậy sampler không được dùng `DOMMatrix` hay DOM; ma trận tự tính 6 số.

**Prototype** (`/private/tmp/claude-501/-Volumes-MySpace-Workspace-OpenCanvas/6450e799-5289-4221-a46f-6264064be06e/scratchpad/p07/anim.js`, `run.mjs`, `run2.mjs`; Chromium 156.0.8078.4):

- **Parity sampler và CSS của Chrome**, 9 ca × 561 mẫu:

  | Thuộc tính | Lệch tối đa |
  |---|---|
  | opacity | 5,4e-7 |
  | translate | 4,9e-5 px |
  | rotate | 4,9e-5° |
  | scale | 4,9e-6 |
  | RGB | 0 |
  | alpha | 0,003 |

  Các ca gồm: keyframe ngầm định, easing riêng từng keyframe, `steps`, `alternate-reverse` 2,5 vòng, `fillMode none` có delay, `infinite`, màu có alpha, hai animation trên một node.
- **Vẽ có transform theo phần tử:**
  - ma trận CSS `translate · rotate · scale` quanh tâm bounds, áp theo chuỗi tổ tiên (transform trước, rồi clip của tổ tiên đó);
  - opacity mẫu **thay** opacity authored rồi nhân xuống con;
  - fill mẫu thay fill.

  Ảnh vẽ lại theo vùng khớp **0/255** với vẽ toàn cảnh ở cùng t (mục 2 của P07d). Đường vẽ trong prototype là chính đường đề xuất ở đây.

## Giải pháp

### 1. Hợp đồng `render_frame` (chế độ enqueue)

```json
{"projectId":"p","revision":12,"frameId":"home","viewport":{"width":1440,"height":900},"timeMs":200}
{"projectId":"p","revision":12,"frameId":"home","viewport":{"width":1440,"height":900},"timeMs":120,"play":["card:lift"]}
```

| Field | Kiểu | Ý nghĩa |
|---|---|---|
| `timeMs` | integer 0..600000, optional | **Vắng** = trạng thái nghỉ: props authored, byte-identical với hiện nay. **Có** = mọi animation được chọn lấy mẫu tại `timeMs` tính từ lúc trigger |
| `play` | mảng 1..16 chuỗi `"<nodeId>:<name>"`, duy nhất; cần `timeMs` | **Vắng**: chọn mọi animation `on:"load"` của frame (trang vừa mở). **Có**: chỉ chọn đúng các animation liệt kê, đều bắt đầu ở 0; animation khác **không** áp, node của chúng ở trạng thái nghỉ. Đây là cách xem hover/click, hoặc xem riêng một animation load |

Lỗi (`VALIDATION_FAILED`, có `field`):

| Trường hợp | `field` | Message |
|---|---|---|
| `play` mà không có `timeMs` | `play` | "play needs timeMs: the time since these animations started" |
| Ref không tồn tại trong subtree frame | `play[i]` | "No animation 'card:hover' in frame 'home'; animations here: card:enter, card:lift, btn:press" (tối đa 20 ref, cùng danh sách `animated` của P07a) |
| Có `timeMs` nhưng frame không có animation load và không truyền `play` | `timeMs` | "Frame 'home' has no load animation; pass play with one of: card:lift, btn:press, or omit timeMs for the rest state" |

Ca cuối báo lỗi thay vì render ảnh trùng với trạng thái nghỉ, để agent không tưởng animation đã chạy.

- Key job: chỉ khi có `timeMs` mới thêm `"timeMs"` và `"play"` (giữ thứ tự authored) vào JSON băm.
- Response `provenance` có thêm `timeMs`, `play`. Worker cũng trả `draw.animatedNodes` (số node có mẫu khác rỗng).
- Poll mode không đổi.

### 2. Scene mang animation (pass-through, layout không diễn giải)

- `crates/layout/src/model.rs`:
  - `Props.animations: Option<serde_json::Value>`;
  - `SceneNode.animations: Option<serde_json::Value>` với `#[serde(default, skip_serializing_if = "Option::is_none")]`.
- `engine.rs` chép `p.animations.clone()`.
- Scene không có animation **serialize y hệt** hiện nay. Giữ `SCENE_VERSION`/`RENDERER_VERSION`: đây là field optional, pipeline identity lúc khởi động đã đổi cache khi build mới.
- Viewer nhận animation qua chính scene (`/scene` HTTP), nên P07d không cần API mới.

### 3. Renderer

**`renderer/easing.ts`** (~60): `easing(spec): (x: number) => number`.

- Có cache theo chuỗi.
- Gồm: 5 keyword (giá trị cubic-bezier của CSS), `cubic-bezier` (Newton 8 bước, rồi chia đôi, sai số 1e-7), `steps(n, start|end|jump-start|jump-end)`.

**`renderer/animation.ts`** (~150):

```ts
export type NodeSample = { opacity?: number; translateX?: number; translateY?: number; scale?: number; rotate?: number; fill?: string };
export function sampleScene(scene: Scene, timeMs: number, play?: string[]): Map<string, NodeSample>;
export function nodeMatrix(node: SceneNode, s: NodeSample): [number, number, number, number, number, number];
```

- Chuẩn hoá theo P07a:
  - default;
  - offset tính như WAAPI;
  - một keyframe = offset 1.
- Tiến độ theo mô hình thời gian WAAPI:
  - pha before/active/after theo `delay`, `duration × iterations`, `fillMode`;
  - tiến độ 1 ở cuối vòng cuối;
  - chiều theo `direction` và vòng chẵn/lẻ.
- Mỗi prop:
  - lấy các keyframe có prop đó;
  - thiếu đầu 0 hoặc 1 thì thêm keyframe ngầm định = giá trị authored (opacity/fill của node; translate 0, scale 1, rotate 0), dùng easing của animation;
  - easing của đoạn = easing của keyframe đầu đoạn.
- Màu nội suy sRGB premultiplied. Opacity kẹp [0,1].
- Ma trận: `M = T(cx+tx, cy+ty) · R(θ) · S(s) · T(−cx, −cy)`, tâm = tâm `bounds`.
  - a = s·cosθ, b = s·sinθ, c = −s·sinθ, d = s·cosθ;
  - e = cx + tx − (a·cx + c·cy), f = cy + ty − (b·cx + d·cy).

**`renderer/color.ts`** (~35): parse `#RGB/#RGBA/#RRGGBB/#RRGGBBAA` và trộn premultiplied. Màu xuất ra dạng `rgba()`.

**Vẽ** (`renderer/paint.ts` của P01; nếu P05 đã merge thì `renderer/layers.ts`). `paintScene(ctx, scene, resources, samples?)`:

- **Nếu có P05 `paintTree` (đệ quy):** tại node `n` có mẫu `s`:
  1. `ctx.save(); ctx.transform(...nodeMatrix(n, s))`;
  2. alpha của node = `s.opacity ?? n.opacity` (thay cho `n.opacity` trong công thức `a′` của P05);
  3. node vẽ là `s.fill ? {...n, fill: s.fill} : n`;
  4. vẽ node và con;
  5. `restore`.

  Layer group-opacity của P05 lấy `M = getTransform()` nên tự tính transform. Extent không đổi vì extent ở toạ độ node.
- **Nếu chưa có P05 (vòng phẳng):** trong vòng tổ tiên, đi **từ gốc xuống**. Với mỗi tổ tiên:
  1. `transform` nếu có mẫu;
  2. clip nếu `clip`;
  3. `opacity *= s?.opacity ?? a.opacity`.

  Sau đó transform của chính node, rồi vẽ với `fill` mẫu. Đúng ngữ nghĩa CSS: clip của một phần tử bị biến đổi cùng phần tử đó. Đây chính là code prototype.
- `samples` vắng hoặc rỗng: **không** gọi `transform`, không tạo object mới, nên lệnh vẽ và output byte-identical với hiện nay.

**`drawScene(target, scene, resources, {scale, strict, timeMs?, play?})`:**

- có `timeMs` thì `samples = sampleScene(scene, timeMs, play)`;
- trả thêm `animatedNodes`.

### 4. Server và worker

| File | Thay đổi |
|---|---|
| `jobs.rs` | `SceneRequest` thêm `time_ms: Option<u32>`, `play: Option<Vec<String>>` (skip khi `None`); key thêm 2 field khi có |
| `jobs/motion.rs` (mới) | kiểm `play`/`timeMs` với node trong input (các lỗi ở mục 1); trả mảnh JSON cho key và cho worker |
| `jobs/queue.rs` | `Work` thêm `motion: Option<Value>` |
| `jobs/runner.rs:66` | op `render` thêm `"timeMs","play"` khi có |
| `jobs/input.rs` | `check_request` gọi `motion::check` |
| `schemas/render.rs` | thêm `timeMs` và `play` vào `enqueue()`; một câu mô tả |
| `worker/cli.mjs`, `worker/pipeline.mjs` | chuyển `timeMs/play` vào `drawScene`; provenance thêm `timeMs`, `play` |

Câu mô tả thêm vào `render_frame` (≤ 300 ký tự):

> "timeMs renders the frame that many milliseconds after its load animations start (omit for the rest state); play lists 'nodeId:name' animations to run alone from 0 instead, e.g. a hover."

## Phạm vi file

| File | Thay đổi | Dòng ước tính |
|---|---|---|
| `renderer/easing.ts` | mới | ~60 |
| `renderer/animation.ts` | mới: normalize, tiến độ, prop, `sampleScene`, `nodeMatrix` | ~150 |
| `renderer/color.ts` | mới | ~35 |
| `renderer/types.ts` | `SceneAnimation`, `SceneKeyframe`, `SceneNode.animations?` | 115 → ~140 |
| `renderer/paint.ts` (133 dòng; nếu đã có P05 thì `renderer/layers.ts`) | áp mẫu trong `enterAncestors` và `paintScene` | 133 → ~155 |
| `renderer/drawing.ts` | option `timeMs/play`, `animatedNodes` | 75 → ~85 |
| `renderer/index.ts` | export `animation`, `easing` | +2 |
| `crates/layout/src/model.rs`, `engine.rs` | pass-through `animations` | +5 |
| `crates/server/src/jobs.rs` | field và key | 193 → ~199 |
| `crates/server/src/jobs/motion.rs` | mới | ~70 |
| `crates/server/src/jobs/{queue,runner,input}.rs` | nối dây | +6 |
| `crates/server/src/schemas/render.rs` | 2 field, 1 câu | 107 → ~110 |
| `worker/cli.mjs`, `worker/pipeline.mjs` | nối dây, provenance | +8 |
| `crates/server/skills/author-frame.md` | "Kiểm animation: `render_frame timeMs` ở 0, giữa, cuối; hover/click dùng `play`" | +5 |
| `docs/SERVER.md`, `docs/CONTRACT.md` (scene node `animations`, option render), `docs/VIEWER.md` (renderer) | mô tả | ~20 |
| `renderer/animation.test.mjs` | mới: unit Node, dạng đóng (AC1) | ~120 |
| `spikes/pipeline/animation-render.test.mjs` | mới: probe pixel qua worker thật (AC2–AC5) | ~170 |
| `spikes/pipeline/animation-parity.test.mjs` | mới: sampler vs CSS Chrome trên golden P07b (AC6) | ~110 |
| `crates/server/tests/render_motion.rs` | mới: hợp đồng MCP (AC7, AC8) | ~140 |

Không file viết tay nào vượt 200 dòng. `jobs.rs` đang 193: logic mới phải vào `jobs/motion.rs`, `jobs.rs` chỉ thêm khai báo.

## Tương thích MCP

- `tools/list`: `render_frame` thêm 2 property optional ở `published()`, và trong nhánh enqueue của `validation()`. Test `schemas/render.rs:49-62` (published = hợp của hai mode) vẫn đúng. Vẫn 15 tool.
- Không có `timeMs`: request, key, job, PNG **không đổi**. Field response cũ không đổi nghĩa; `provenance.timeMs/play` là field cộng thêm.
- Scene: `animations` là field optional. Client đọc `includeScene` hay `inspect_layout` không vỡ.
- Tài liệu: `docs/SERVER.md`, `docs/CONTRACT.md`, `author-frame.md`, `implement-frame.md`. Test `skill_layer.rs` tự khớp.

## AC

Chung cho AC2–AC5:

- Probe chạy qua worker thật (`compute` + `render`, scale 1).
- Frame 200×200 nền trắng, `layout free`.
- Node `box` là rectangle 100×100 tại (50,50), fill `#000000`, trừ khi ghi khác.
- Giá trị ở dạng R (= G = B với ảnh xám), sai số ±1.

1. **Sampler (Node, không browser).** `node --test renderer/animation.test.mjs`:
   - linear fade `[{offset:0,opacity:0}]` 1000 ms: t = 0, 250, 1000, 9999 → 0; 0,25; 1; 1;
   - `ease-out` tại 0,5 → 0,6846 ±1e-4;
   - `steps(4,end)` 0→100 tại 600 → 50;
   - `infinite alternate` linear 1000 tại 1300 → tiến độ 0,7;
   - `iterations 2.5 alternate-reverse` sau khi kết thúc → giữ tiến độ của nửa vòng cuối;
   - `fillMode none` trước delay và sau khi kết thúc → không có mẫu;
   - `[{opacity:0},{opacity:0.5},{opacity:1}]` → offset 0/0,5/1;
   - fill `#ff000000` → `#0000ffcc` ở 0,5 → `rgba(0,0,255,0.4)`, đúng premultiplied;
   - `play` chỉ chọn ref liệt kê.
2. **Opacity và easing.**
   - `[{offset:0,opacity:0}]` linear 1000, tâm (100,100): t=0 → 255; t=250 → 191; t=500 → 128; t=1000 → 0.
   - Cùng animation với `ease-out`, t=500 → 80 (255 × (1 − 0,6846)).
   - `infinite alternate` `[{offset:1,opacity:0}]` linear 1000, t=1300 → 179.
   - `fillMode none` delay 200 với `opacity` authored 0,5, t=100 → 128.
3. **Transform quanh tâm.**
   - (a) `[{offset:0,translateY:40}]` linear 400: t=0 → (100,85)=255, (100,95)=0; t=200 → (100,65)=255, (100,75)=0; t=400 → (100,45)=255, (100,55)=0.
   - (b) Thanh 160×20 tại (20,90), `[{offset:1,rotate:90}]` linear 1000: t=1000 → (100,30)=0, (30,100)=255; t=0 thì ngược lại; t=500 → (60,60)=0.
   - (c) `[{offset:1,scale:0.5}]` linear 1000, t=1000 → (100,100)=0, (70,100)=255, (80,100)=0.
4. **Fill, subtree, clip.**
   - (a) `fill #ff0000` → `[{offset:1,fill:"#0000ff"}]` linear 1000, t=500 → RGB (128,0,128) ±1.
   - (b) Group `g` (x50 y50 100×100, không fill) chứa rect đen 20×20 tại (0,0), animation trên `g` `[{offset:0,translateX:30}]` linear, t=0 → (85,55)=0, (55,55)=255. Con đi theo cha.
   - (c) Group `c` `clip:true` (x50 y50 100×100) chứa rect đen 100×100 có `[{offset:0,translateX:80}]`, t=0 → (140,100)=0, (160,100)=255. Clip của tổ tiên vẫn áp.
5. **`play` và trạng thái nghỉ.**
   - (a) `box` có `fade` (load `[{offset:0,opacity:0}]` linear 100 ms) và `shrink` (hover `[{scale:0.5}]` linear 200).
     - `timeMs:200` không `play` → (70,100)=0: hover không áp, fade đã xong.
     - `timeMs:200, play:["box:shrink"]` → (70,100)=255.
     - `timeMs:0, play:["box:shrink"]` → (100,100)=0: fade không áp vì không được chọn.
   - (b) Render **không** `timeMs` của frame có animation → PNG giống từng byte bản không có `animations` (giữ AC11 của P07a).
   - (c) `npm run test:pipeline` (gồm parity viewer/worker hiện có) pass, không đổi golden.
6. **Parity với CSS của Chrome.** `node --test spikes/pipeline/animation-parity.test.mjs`. Với mỗi ca trong golden P07b, nạp `css` vào Chromium của repo, pause, đặt `currentTime` theo bước 23 ms từ 0 tới hết, cộng 200 ms (`infinite` thì 3 vòng). Đo:
   - **≥ 500 mẫu** trên **≥ 10 ca**;
   - lệch tối đa opacity ≤ 1e-4, translate ≤ 0,01 px, rotate ≤ 0,01°, scale ≤ 1e-4, kênh màu ≤ 1/255, alpha ≤ 0,005.

   Prototype: 561 mẫu / 9 ca, lệch ≤ 5e-5.
7. **Hợp đồng MCP** (`cargo test -p opencanvas-server --test render_motion`):
   - (a) `tools/list`: `render_frame` có `timeMs` (integer 0..600000) và `play` (array ≤ 16). `tool_schemas.rs` pass.
   - (b) `play` không có `timeMs` → `field:"play"`.
   - (c) Ref sai → `field:"play[0]"`, message chứa ref đúng `card:lift`.
   - (d) Frame không có animation load, có `timeMs`, không `play` → `field:"timeMs"`, message chứa danh sách ref.
   - (e) Cùng tham số → cùng `jobId`. `timeMs` 200 và 300 → khác `sceneKey`.
   - (f) Request không `timeMs` cho `sceneKey` **bằng** key tính bằng công thức cũ (test so với `json!({"input","scale","pipeline"})`).
   - (g) Response ready có `provenance.timeMs` và `provenance.play` đúng giá trị gửi.
   - (h) `timeMs: 600001` → lỗi schema.
8. **Không hồi quy.**
   - `cargo test -p opencanvas-server --test render --test render_wait --test render_queue --test render_response --test tool_schemas` pass;
   - `npm run test:renderer` pass;
   - `npm --prefix web run typecheck` pass;
   - `npm run test:e2e` pass, 0 khác biệt RGBA.
9. **Modular.** `wc -l` mọi file viết tay đã sửa hoặc tạo đều ≤ 200. Báo số dòng `animation.ts`, `easing.ts`, `paint.ts`/`layers.ts`, `jobs.rs`, `jobs/motion.rs`.

## Cách kiểm

```sh
npm run build:renderer && npm run test:renderer                              # AC1
CARGO_BUILD_JOBS=4 cargo build -p opencanvas-layout --bin layout-step
node --test --test-concurrency=1 spikes/pipeline/animation-render.test.mjs spikes/pipeline/animation-parity.test.mjs   # AC2–AC6
npm run test:pipeline                                                        # AC5c
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server --test render_motion --test render --test render_wait --test render_queue --test render_response --test tool_schemas   # AC7, AC8
npm --prefix web run typecheck && npm run test:e2e                           # AC8
wc -l renderer/*.ts crates/server/src/jobs.rs crates/server/src/jobs/motion.rs   # AC9
```

Kiểm live trên server tạm (`--port 0 --data-root "$(mktemp -d)"`):

1. tạo card có `enter` 400 ms;
2. `render_frame timeMs` 0 / 200 / 400 và xem PNG: 0 thì trong suốt, 200 thì mờ và lệch, 400 thì bằng trạng thái nghỉ;
3. `play:["card:lift"]`;
4. dọn process và data root.

## Rủi ro

- **Ngữ nghĩa CSS.** AC6 khoá trên Chrome của repo. Safari/Firefox cùng spec, khác biệt nếu có chỉ ở làm tròn.
- **Ma trận và AA.** Transform không nguyên làm mép AA khác đi. Đây là đúng: Chrome cũng vậy. AC dùng probe ở giữa vùng, không dùng mép.
- **Text bị scale/rotate** được vẽ bằng transform của canvas, nên glyph biến dạng đúng như CSS transform. Chrome DOM có thể hint glyph khác một chút; không so DOM text.
- **P05 chưa merge.** Có hai đường áp mẫu, nhưng chỉ triển khai một (theo trạng thái lúc làm). Bên làm sau phải chuyển code mẫu vào `paintTree`. Viết test AC3/AC4 độc lập với đường vẽ để khoá hành vi.
- **Node giữ chỗ khi thiếu font hoặc ảnh** (`renderer/paint.ts:24-28`, `:46-52`) cũng bị transform. Chấp nhận được.
- **Khối lượng ~90 phút.** Phần lớn là port từ prototype. Nếu trễ, cắt AC6 sang lượt sau, vì AC1–AC5 đã khoá ngữ nghĩa bằng dạng đóng. Không cắt AC5b.

## Ngoài phạm vi / phase sau

- P07d: preview trên viewer.
- `inspect_layout` trả bounds đã biến đổi tại t (`animatedBounds`). Hiện ảnh đã đủ để tự kiểm.
- Render nhiều thời điểm trong một lần gọi (dải ảnh/sprite), xuất GIF/video.
- Trạng thái leave của hover (chạy về) qua `play`. Hiện chỉ có pha vào, vì pha về là transition ngược.
