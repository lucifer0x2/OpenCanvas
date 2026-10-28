# P07d — Viewer: Play/scrub animation, chỉ vẽ lại vùng của phần tử động (dirty region trên layer vector của P01)
Ưu tiên: P1 · Kích cỡ: ~90 phút dev · Phụ thuộc: **P07c** (`sampleScene`, `nodeMatrix`, `drawScene {timeMs, play}`, scene có `animations`), **P01** (đã merge `f2b0376`). Nếu P05 đã merge thì dùng `paintTree` có `clipRect` (xem Giải pháp). File nóng `web/src/components/CanvasView.tsx` chung với P02b, P06c, P06d và phiên "OpenCanvas UI redesign" ngoài đội: xếp tuần tự. · Nguồn: U3 ("không vẽ lại toàn trang mỗi frame, chỉ phần tử đó"; "viewer: nút play/scrub")

Phase cuối của P07:

- P07a: dữ liệu;
- P07b: CSS export;
- P07c: render tại t;
- **P07d (file này)**: preview trên viewer.

## Vấn đề

- Sau P07c, agent xem được ảnh tại t qua MCP. Người dùng thì chưa xem được animation chạy trên viewer.
- Nếu viewer làm theo cách đơn giản nhất, tức mỗi frame `layer.batchDraw()` để `sceneFunc` vẽ lại **toàn bộ scene** qua `paintScene` (`web/src/components/CanvasView.tsx:106-112`, `renderer/paint.ts:116-133`), thì đúng là "vẽ lại toàn trang mỗi frame", điều người dùng đã nói không muốn:
  - prototype đo: 226 lệnh vẽ và 3,3 ms mỗi frame cho dashboard 230 node;
  - 901 lệnh và 14 ms cho cảnh 917 node, sát ngân sách 16,7 ms ở 60 Hz.
- Chi phí đó tăng theo kích thước trang, không theo kích thước phần tử đang chuyển động.

## Bằng chứng

Code tham chiếu: `night/integration` @ `f2b0376` (P01 đã merge).

**Viewer hiện tại**

- `web/src/components/CanvasView.tsx:95-121`: **một** `Layer` chứa cả `Rect` trang (nền trắng có shadow), `Shape` vẽ vector scene (`sceneFunc` → `paintScene`) và `Rect` viền chọn.
  - Vẽ lại một phần trên layer này sẽ xoá luôn viền chọn.
  - Vì vậy phải tách overlay sang layer riêng.
- `paintScene` vẽ theo đơn vị scene dưới transform hiện có của context, không clear (`renderer/paint.ts:112-115`). Nhờ đó vẽ một vùng chỉ cần clip, clear rồi gọi lại với bộ lọc node.
- Harness test của P01 dùng lại được:
  - `web/tests/zoom-harness.mjs:12,52` (`withViewer`, `open(browser, dpr)`);
  - `web/tests/fixture-dense.mjs:8` (`denseFixtureData(hash, kind)`);
  - helper so sánh `web/tests/zoom-probe.mjs:3`.

**Prototype đo** (`/private/tmp/claude-501/-Volumes-MySpace-Workspace-OpenCanvas/6450e799-5289-4221-a46f-6264064be06e/scratchpad/p07/run2.mjs`, `scene.js`, `anim.js`)

Điều kiện:

- Chromium 156.0.8078.4 headless, DPR 2.
- Canvas cỡ frame.
- Mỗi phép đo có `getImageData(0,0,1,1)` để ép raster, nên thời gian gồm cả rasterize.
- Mỗi frame 16,7 ms, chạy 1 s.
- Cảnh giống dashboard dense của P01: sidebar, topbar, 4 thẻ KPI, bảng 25×6.

| Cảnh | Vẽ lại toàn cảnh mỗi frame | Vẽ lại theo vùng (đề xuất) | Pixel so với vẽ toàn cảnh cùng t |
|---|---|---|---|
| dense 1440×900, 230 node, 1 thẻ KPI động (opacity + translateY + scale) | 226 lệnh vẽ, p50 3,3 ms, p95 3,9 ms | **5 lệnh**, p50 0,2 ms, p95 1,1 ms, vùng ≤ 142 546 px (**2,75 %** canvas) | **0 kênh khác** (9 frame) |
| dense, 2 node xa nhau (thẻ KPI + ô search fill/rotate) | 226 lệnh, p50 3,8 ms | **8 lệnh**, p95 1,4 ms, vùng ≤ 204 106 px (3,9 %) | 0 |
| stress 2880×1800, 917 node, 1 thẻ động | 901 lệnh, p50 14 ms, p95 15 ms | **5 lệnh**, p50 0,2 ms, p95 1,3 ms, max 3,2 ms | 0 |

Kết luận:

- Chi phí mỗi frame chỉ còn phụ thuộc phần tử động. Số lệnh vẽ không đổi khi trang lớn gấp 4.
- Vì clip vùng là hình chữ nhật pixel nguyên dưới transform đơn vị, và mọi node giao vùng được vẽ lại theo đúng thứ tự, nên pixel trong vùng **trùng khít** với vẽ toàn cảnh.

Tham khảo:

- Konva performance tips (layer riêng cho phần động, tránh redraw cả layer): https://konvajs.org/docs/performance/All_Performance_Tips.html.
- Partial invalidation và dirty rect là cách trình duyệt repaint không qua compositor; transform/opacity là thuộc tính rẻ: https://web.dev/articles/animations-guide.

## Giải pháp

**Chọn: dirty region trên chính layer vector của P01.**

Mỗi frame, với **mỗi gốc animation**, tính hộp thiết bị của subtree ở mẫu trước ∪ mẫu sau. Clip vùng đó, clear, rồi vẽ lại **chỉ các node giao vùng**, đúng thứ tự vẽ. Phần còn lại của canvas giữ nguyên.

| Phương án | Lệnh vẽ/frame | Đúng thứ tự z | Bộ nhớ | Kết luận |
|---|---|---|---|---|
| A. `batchDraw()` toàn layer mỗi frame | ~N node (226 / 901) | Có | 0 | Trái yêu cầu người dùng |
| B. Cache bitmap phần tĩnh (Konva `cache()`) + phần động trên layer riêng | ít | **Sai** nếu node phía trên phần tử động chồng lên nó, vì lớp động luôn nằm trên. Phải cắt thành nhiều dải tĩnh/động | +1 bitmap cỡ màn hình × số dải; phải cache lại khi zoom | Phức tạp, sai thứ tự hoặc tốn bộ nhớ |
| **C. Dirty region trên layer vector** | **5–8** | **Có**: mọi node giao vùng được vẽ lại theo preorder | 0 | **Chọn**. Prototype khớp 0/255 |

**Thiết kế**

1. **`renderer/regions.ts`** (mới, ~120):
   ```ts
   export function motionIndex(scene: Scene): MotionIndex;   // chuỗi tổ tiên, con, ink box tĩnh, gốc animation
   export function motionRegions(ix: MotionIndex, prev: Samples, next: Samples, m: Matrix, inner: DeviceRect): DeviceRect[] | "full";
   export function paintRegions(ctx: CanvasRenderingContext2D, scene: Scene, resources: Resources,
                                ix: MotionIndex, samples: Samples, regions: DeviceRect[], m: Matrix): { painted: number; areaPx: number };
   ```
   - **Ink box của node:** bounds ∪ hộp từng dòng text, cộng 1 px AA. Hộp dòng là `x..x+width`, `y − fontSize .. y + 0,4·fontSize`.
     - Placeholder khi thiếu font/ảnh, hoặc node mà P04/P09 vẽ tràn (stroke, shadow): ink = ∞, nên trả `"full"`.
   - **Vùng của gốc:**
     - hợp ink box của mọi node trong subtree, biến đổi bằng `m × chuỗi nodeMatrix` của mẫu trước và mẫu sau;
     - làm tròn ra pixel nguyên (floor−1, ceil+1);
     - gộp các vùng chồng nhau.
   - **Vùng không nằm trọn trong `inner`** (pixel thiết bị nguyên bên trong frame) → `"full"`.
     - Lý do: mép frame có AA với shadow của `Rect` trang, và phần ngoài frame không thuộc scene.
     - Hệ quả: vẽ lại cả layer cho frame đó, vẫn đúng, chỉ không tối ưu.
   - **`paintRegions`**, cho mỗi vùng:
     1. `setTransform(1,0,0,1,0,0)`, `rect` + `clip`, `clearRect`, `fillStyle = "#fff"`, `fillRect` (nền trang trong vùng);
     2. `setTransform(m)`;
     3. vẽ các node mà hộp thiết bị (đã cache cho node tĩnh, tính lại cho node trong subtree động) giao vùng, bằng đúng hàm vẽ của P07c (`paintScene` có `samples` và bộ lọc).
   - **Nếu P05 đã merge:** thay bước 3 bằng `paintTree(..., {clipRect: region, samples})`. P05 đã cắt subtree theo extent và tạo layer opacity trong phạm vi `clipRect`.
2. **`web/src/hooks/useMotionPreview.ts`** (mới, ~140):
   - Trạng thái `playing`, `timeMs`, `play` (ref hoặc vắng = mọi animation load), `durationMs` nằm trong **ref**, không nằm trong state của `CanvasView`.
   - Vòng `requestAnimationFrame`: `t += dt`; `infinite` thì lặp; animation hữu hạn thì dừng ở T.
   - Mỗi frame:
     1. `next = sampleScene(scene, t, play)`;
     2. `regions = motionRegions(...)`;
     3. nếu `"full"` thì `layer.batchDraw()`;
     4. ngược lại thì `paintRegions(layerCtx, …)` thẳng vào context native của layer scene (`layer.getContext()._context`), **không** qua `batchDraw`.
   - Transform `m` là transform mà `sceneFunc` ghi lại ở lần Konva vẽ gần nhất (`ctx.getTransform()` trong `sceneFunc`). Như vậy khớp tuyệt đối với Konva (pixelRatio × stage), không tự tính lại.
   - Mỗi frame ghi `performance.measure("oc-motion-paint")`; mỗi lần fallback ghi `"oc-motion-full"`. Chi phí gần 0, và test đo được.
   - `sceneFunc` đọc `samplesRef.current`. Khi Konva tự vẽ lại (zoom, pan, đổi selection), toàn layer được vẽ ở đúng t hiện tại.
   - Scene đổi (revision mới, đổi frame) → dừng, về trạng thái nghỉ.
3. **`web/src/motionStore.ts`** (mới, ~40):
   - store nhỏ cho `useSyncExternalStore`: thời gian hiện, đang chạy, lựa chọn;
   - **chỉ** `MotionBar` re-render theo thời gian. `CanvasView` và cây Konva không re-render mỗi frame (AC9).
   - Nếu re-render, react-konva sẽ vẽ lại cả layer.
4. **`web/src/components/MotionBar.tsx`** (mới, ~80). Chỉ hiện khi `scene.nodes.some(n => n.animations?.length)`. `aria-label="Animation preview"`, overlay dưới canvas cạnh zoom controls. Gồm:
   - nút Play/Pause (`aria-pressed`);
   - nút Reset (về trạng thái nghỉ, tức vẽ lại toàn layer không mẫu);
   - thanh `input[type=range]` 0..T;
   - nhãn `0.42 / 1.20 s`;
   - chip lựa chọn: "Load" hoặc `card:lift ×`.

   T = max(`delay + duration × iterations`) của các animation được chọn. `infinite` tính 2 vòng, và khi play thì lặp. Kéo thanh dùng cùng đường vẽ theo vùng.
5. **`web/src/components/AnimationList.tsx`** (mới, ~50), gắn vào `Inspector.tsx`:
   - mục "Animations" của node đang chọn;
   - mỗi dòng `name · on · 400 ms` có nút ▶ (`aria-label="Play card:lift"`);
   - ▶ chạy riêng ref đó, cùng ngữ nghĩa `play` của `render_frame`.
6. **`CanvasView.tsx`:**
   - tách `Layer` overlay cho viền chọn (sau này cả pin note của P02b và hotspot của P06d);
   - `Layer` scene giữ `Rect` trang + `Shape`;
   - `sceneFunc` truyền `samplesRef.current` và ghi lại transform;
   - nối hook và `MotionBar`.

Không đổi: `useCanvasTransform`, hit-test (theo bounds tĩnh, như CSS: transform không đổi layout; chọn node lúc đang chạy vẫn theo vị trí nghỉ), HTTP API, MCP.

## Phạm vi file

| File | Thay đổi | Dòng ước tính |
|---|---|---|
| `renderer/regions.ts` | mới | ~120 |
| `renderer/index.ts` | export regions | +1 |
| `web/src/hooks/useMotionPreview.ts` | mới | ~140 |
| `web/src/motionStore.ts` | mới | ~40 |
| `web/src/components/MotionBar.tsx` | mới | ~80 |
| `web/src/components/AnimationList.tsx` | mới | ~50 |
| `web/src/components/CanvasView.tsx` | tách overlay layer, `samplesRef`, hook, `MotionBar` | 137 → ~160 |
| `web/src/components/Inspector.tsx` | gắn `AnimationList` | 100 → ~104 |
| `web/src/canvas.css` | style MotionBar, nút ▶ | 157 → ~185 |
| `web/tests/fixture-dense.mjs` | kind `dense-anim`, `dense-anim2`, `stress-anim` (animation như bảng Bằng chứng) | 75 → ~95 |
| `web/tests/motion-probe.mjs` | mới: đếm lệnh vẽ và `clearRect` trên canvas layer scene theo rAF, đọc `performance.measure`, chụp/so pixel | ~90 |
| `web/tests/motion.mjs` | mới: AC1–AC10 qua `withViewer`/`open` của P01 | ~170 |
| `web/package.json` | `"test:motion": "node tests/motion.mjs"` | +1 |
| `docs/VIEWER.md` | mục "Animation preview": dirty region, fallback, ref và store | +15 |
| `web/dist/*` | build lại như các lần trước | generated |

Mọi file viết tay ≤ 200 dòng.

## Tương thích MCP

- Không đổi `tools/list`, schema, HTTP API hay SSE.
- Viewer đọc `scene.nodes[].animations` mà P07c đã thêm.
- Worker PNG không đổi. `drawScene` không dùng `regions.ts`.

## AC

Mọi AC chạy trong `web/tests/motion.mjs`:

- Chromium Playwright của repo, cửa sổ 1440×1000, `deviceScaleFactor` 2, harness `withViewer` của P01.
- Zoom 100 % đặt bằng phím `0` hoặc API transform của harness.

Định nghĩa:

- **"lệnh vẽ"**: số lời gọi `fill`, `fillText`, `fillRect`, `drawImage`, `stroke` trên canvas DOM của layer scene, đếm theo từng rAF bằng cách patch `CanvasRenderingContext2D.prototype` trong trang.
- **"vùng"**: tổng diện tích `clearRect` trên canvas đó mỗi rAF.
- **"diện tích frame"**: `viewport.width × viewport.height × zoom² × dpr²`.

1. **Một phần tử động (dense).** Cảnh `dense-anim`: thẻ KPI `kpi1` có `enter` (opacity 0 + translateY 16 + scale 0,96, 600 ms ease-out). Bấm Play, đo mọi rAF trong 1 s:
   - lệnh vẽ **≤ 12** mỗi frame (prototype 5);
   - vùng **≤ 5 %** diện tích frame (prototype 2,75 %).

   Đối chứng: một lần Konva vẽ lại toàn layer trong cùng cảnh ≥ 200 lệnh (prototype 226). Test in cả hai số.
2. **Hai phần tử xa nhau** (`dense-anim2`, thêm `search` đổi fill + rotate 3°, 500 ms ×2 alternate): lệnh vẽ **≤ 20**/frame, vùng **≤ 8 %** (prototype 8 / 3,9 %).
3. **Không phụ thuộc kích thước trang** (`stress-anim` 2880×1800, ~900 node, 1 thẻ động):
   - lệnh vẽ ≤ 12/frame;
   - `oc-motion-paint` **p95 ≤ 4 ms**, **max ≤ 8 ms** (prototype p95 1,3, max 3,2, đã gồm raster);
   - **0** long task ≥ 50 ms trong 2 s play.
4. **Đúng pixel so với vẽ toàn cảnh.** Tại 6 thời điểm (3 lần Pause khi đang chạy, 3 vị trí kéo thanh), trên cả 3 cảnh:
   - chụp canvas layer scene;
   - gọi `window.Konva.stages[0]` → layer scene `.draw()` (vẽ toàn cảnh ở cùng t);
   - chụp lại;
   - kết quả **0 kênh khác** (prototype 0 ở 27 frame).
5. **Khớp renderer tham chiếu.** Tại các thời điểm của AC4, dùng `compare()` của `web/tests/zoom-probe.mjs` với `drawScene(…, {scale: zoom×dpr, timeMs, play})` của P07c: **off ≤ 0,1 %**, **sharp ≥ 0,95**.
6. **Về trạng thái nghỉ.** Bấm Reset sau khi chạy: canvas giống ảnh chụp trước khi Play, **0 kênh khác**.
7. **Chạy riêng một animation.** Thêm hover `lift` (scale 1,02) lên `kpi2`. Bấm ▶ của `kpi2:lift` trong Inspector:
   - chỉ `kpi2` đổi; vùng chỉ phủ `kpi2`; `kpi1` vẫn ở trạng thái nghỉ;
   - Pause rồi so như AC5 với `play:["kpi2:lift"]` → off ≤ 0,1 %.
8. **Fallback.** Animation `translateX` 400 px đưa node ra ngoài mép frame:
   - có ít nhất một `oc-motion-full`, không exception;
   - AC4 vẫn 0 kênh khác.

   Zoom bằng wheel ×1,5 trong 300 ms khi đang chạy: không exception; sau cử chỉ AC4 vẫn đúng.
9. **Không vẽ lại cả layer mỗi frame.** Trong 1 s play không có input, số lần `Konva.Layer.prototype.draw`/`batchDraw` thật sự chạy trên layer scene **≤ 2** (lúc bắt đầu và kết thúc). Số lần render React của `CanvasView` (đếm bằng một ref trong test build, hoặc bằng `React.Profiler` trong harness) **≤ 2**. AC này bắt lỗi re-render mỗi frame.
10. **Không hồi quy.**
    - Cảnh không có animation: không có `[aria-label="Animation preview"]`.
    - `npm --prefix web run test:zoom` (P01) và `node web/tests/smoke.mjs` pass.
    - Click chọn node vẫn đúng (Inspector hiện "Headline" như AC7 của P01).
11. **Kiểm tra mã.** `npm --prefix web run typecheck`, `npm --prefix web test`, `npm run test:renderer` pass.
12. **Modular.** `wc -l` mọi file viết tay đã sửa hoặc tạo đều ≤ 200. Báo số dòng `regions.ts`, `useMotionPreview.ts`, `CanvasView.tsx`.

## Cách kiểm

```sh
npm run build:renderer && npm run test:renderer
npm --prefix web run typecheck && npm --prefix web test
npm --prefix web run test:motion      # AC1–AC10, in JSON số đo để QA dán vào .night/qa/P07d.md
npm --prefix web run test:zoom && node web/tests/smoke.mjs
npm --prefix web run build            # web/dist
wc -l renderer/regions.ts web/src/hooks/useMotionPreview.ts web/src/motionStore.ts web/src/components/{CanvasView,MotionBar,AnimationList,Inspector}.tsx web/tests/motion*.mjs
```

Kiểm live (tuỳ chọn) trên server tạm `--port 0 --data-root "$(mktemp -d)"`:

1. tạo project có card `enter` + hover `lift` qua MCP;
2. mở viewer của server tạm, bấm Play, kéo thanh, bấm ▶ `lift`;
3. bật Chrome DevTools "Paint flashing": chỉ vùng card nhấp nháy;
4. dọn process và data root.

## Rủi ro

- **Ink box thiếu.**
  - Glyph tràn quá margin `0,4·fontSize`, hoặc stroke/shadow (P04/P09) vẽ ngoài bounds, sẽ để lại pixel cũ.
  - Giảm rủi ro:
    - AC4 so toàn canvas;
    - node có hiệu ứng tràn trả ink ∞, dẫn tới fallback `"full"`;
    - P04/P09 phải cập nhật `inkBox` (ghi trong `docs/VIEWER.md`; cùng ý với ghi chú `extent` của P05).
- **Group opacity (P05).** Nếu P05 merge trước, vẽ theo vùng phải dùng `paintTree` có `clipRect`, để layer opacity tạo trong vùng. Nếu P07d merge trước, P05 phải nối `clipRect`. Dev orchestrator nhắc bên làm sau.
- **Konva internals.** `layer.getContext()._context` là thuộc tính công khai có kiểu (P01 đã dùng `_context` trong `sceneFunc`). Vẽ thẳng không qua Konva là hợp lệ vì layer scene không có node nào khác nhận hit (`listening={false}`).
- **File nóng `CanvasView.tsx`** (P02b, P06c, P06d, UI redesign). Tách overlay layer có lợi cho cả P02b/P06d (pin và hotspot không bị xoá khi vẽ theo vùng), nên xếp P07d trước hai đề xuất đó nếu được.
- **Số đo headless.** Ngưỡng AC rộng gấp 2–3 lần số prototype. Số lệnh vẽ không phụ thuộc GPU.

## Ngoài phạm vi / phase sau

- Hover/click thật trên canvas: chế độ "Preview" rê chuột vào node thì chạy hover. Phase sau, sau P06d hotspot.
- Timeline nhiều track kiểu After Effects; chỉnh keyframe trên viewer (viewer vẫn chỉ đọc).
- Xuất GIF/video.
- `prefers-reduced-motion` cho preview: viewer không tự chạy nên chưa cần.
