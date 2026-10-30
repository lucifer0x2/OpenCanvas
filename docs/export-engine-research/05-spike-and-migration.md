# 05 — Handoff spike tối thiểu và migration có rollback

Ngày 2026-10-29. Đây là công việc **đề xuất chưa thực hiện**, không tự phê duyệt
thay engine. Quy tắc PLAN và HTTP/MCP/job contracts tiếp tục là ràng buộc.

## Task S0: corpus + baseline dùng chung

**Mục tiêu:** tách sai khác text/layout khỏi raster/encoder, có kết quả có thể
tái chạy. Workspace/branch: tạo nhánh `codex/export-engine-spike` theo quy trình
repo khi giao triển khai, không checkout/reset nhánh đang có thay đổi của người khác.
Chỉ một fresh worker cho mỗi task độc lập; worker không spawn tiếp.

**Phạm vi dự kiến:** thư mục mới `spikes/export-engine/` chứa runner/corpus/metrics;
fixtures reference dùng data root tạm. Không sửa authored DB/PLAN/schema/tools,
không restart3847, không chép dữ liệu project thật. Font redistribution phải rõ.
Các file mới focused 100–200 dòng, tách khi >250; không gom painter/tests thành
file lớn. Runtime implementation ở tasks S1/S2 riêng, chưa trộn S0.

**Corpus tối thiểu (gợi ý 30–50 cases có IDs và expected properties):**

- Tiếng Việt NFC/NFD: `Tiếng Việt ă â đ ê ô ơ ư`, `Ấp ỹ`, combining marks nhiều
  lớp; nguyên string và equivalent normalized sequence. Không normalize authored
  text ngầm; kiểm shape hiển thị và giữ đúng strings.
- `AV To office ffi`, Arabic/bidi mixed digits, Devanagari, CJK; ký hiệu
  `⌘C ⇧⌘V ↵ ✓ →`, emoji modifier/flag/ZWJ `👩🏽‍💻`, variation selector.
- CRLF/LF/empty line, leading/trailing/multiple spaces, long unbroken word;
  width null/0/finite, width own max-content, break width±1/64, near hug/fill.
- P10a regression strings `Prefer`, `customers`, `Filter objects` và 7
  fontSize/lineHeight pairs từ handoff; current baseline và desired DOM baseline
  là **hai nhãn**, không dùng kết quả proposed như code đã pass.
- Font TTF/OTF/WOFF2, missing/corrupt/wrong hash, exact file name collision với
  host font; fallback manifest, supplementary-plane missing glyph. Dùng ít nhất
  bundled Noto + redistributable symbol/emoji/complex-script font pinned.
- Rounded fill/clip nested, alpha overlap, uniform/per-side inside/outside stroke,
  stroke over child, subpixel x/y, canvas edge clipping và scale0.5/1/2/4.
- PNG/JPEG/WebP **input**, EXIF oriented JPEG, alpha checkerboard, normalized crop,
  fit/contain/cover và downscale; color profile/gamma case có attribution.
- 16/182/~850/2048 nodes, nhiều text dòng và repeated fonts; 390×844,
  1040×844, 1440×900; case sát16M output pixels/layers/depth128 và over-budget.
- Frame PNG, node PNG nhiều settings, SVG text/image/stroke diagnostic,
  hover/pressed state; repeat/revision/theme/font/asset invalidation.

**AC S0:** save generated inputs/scenes/manifests/reference PNG và decoded RGBA
hash, browser/runtime/OS/build/font identities; deterministic runner produces
raw metrics JSON/CSV, comparison images/diff masks và Markdown summary. Không
lưu bytes font không có quyền phân phối. Baseline Chromium đã pin, cold/warm
phân biệt; control run cùng backend phải tự so bằng0 trước cross-backend.

## Task S1: hai painter CPU trên computed scene cố định

**Mục tiêu:** so resvg/tiny-skia và skia-safe CPU, không đổ lỗi metrics layout cho
rasterizer. Scope: `spikes/export-engine/` modules/crate riêng; dependency spike
có manifest/lock riêng theo kiến trúc repo, không sửa production lock/runtime
trước khi task được giao. File scene/resources đọc từ corpus có ID/hash.

**AC:** cả hai render supported corpus scene sang transparent RGBA+PNG đúng ceil
dimensions; support đủ fill/image/crop/clip/opacity/stroke current semantics;
không scan host fonts, không network fetch từ SVG/font. resvg internal serializer
không đi qua public SVG thiếu stroke. Dùng canonical font selection/options để
measure rồi paint cùng shaper; unsupported font/emoji phải lộ rõ, không báo pass.
Đo raster và encode riêng; decoded pixels so reference, lưu diff heatmaps và
node-level crops. **S1 vẫn dùng scene đo bởi browser, chưa bỏ Chromium toàn pipeline.**

**Đầu ra:** commands thực, exact dependency/source/prebuilt hashes và feature set,
failure/support matrix, metrics thô + summary; ranking cập nhật theo bằng chứng.
Nếu một painter fail hard gate, dừng nó, không mở rộng abstraction chỉ để giữ
đối thủ. Không cần vello/Python/Qt implementation nếu hai đối thủ chính đủ kết luận.

## Task S2: text-native + Taffy, hoặc CDP khi cần giữ fidelity

**Mục tiêu:** candidate thắng S1 phải compute text-native → Taffy → scene → PNG
không launch Node/Chromium. Handoff riêng vì đây là rủi ro lớn nhất. Scope: adapter
measurement/shaping/fonts/wrapping trong spike; vẫn Taffy box engine, giữ cache-only
callback/pending/batch/6 passes. Không sửa public scene thành glyph format mới
trong spike; glyph runs có thể là internal payload để kiểm reuse.

**AC:** all corpus measure requests trả widths/height/lines, exact current wrapper
semantics hoặc versioned delta list có giải thích; linebreak/grapheme/baseline
gates dưới đây đạt. Native compute-produced scene vẽ qua browser viewer hiện tại
cũng phải so; same font files chưa đủ. Process test phải chứng minh không exec
Node/Chromium, Linux CPU container không browser binaries, macOS build riêng.

**Nếu native không đạt:** task CDP riêng dùng Rust driver + pinned Chromium,
giữ renderer JS và PNG Canvas capture; AC all original scenes/lines/RGBA bằng
baseline cùng browser/fonts/flags, lifecycle/restart/error behavior tương đương,
không Node ở production runtime. Đo savings thật trước quyết định ship CDP;
Node build frontend và E2E có thể giữ.

## Cách đo công bằng

| Phép đo | Cách thực hiện trong spike |
| --- | --- |
| Cold end-to-end | Từ worker launch tới PNG ready, **không** gồm install/build; thêm service startup-to-first-request riêng. Ít nhất10 lần fresh worker, không giả filesystem cache đã cold nếu chưa kiểm soát. |
| Warm uncached | Worker alive, job key thay đổi để tránh server trả cached PNG; ≥100 mẫu/case lớn sau5 warmups, concurrency1. Cache fonts/shapes có thật thì log. |
| Cache hit | Repeat identical scene key riêng: cached job latency khác repeat compute with measured-text cache. Báo cả hai, không dùng cache hit thay warm render. |
| Stage timing | Queue wait, resources+decode, layout passes, measure+shape/wrap, paint, encode, IPC/base64/copies; instrumentation same boundaries. |
| Memory | Worker+browser/process-tree aggregate, steady RSS và high-water peak; Linux cgroup memory.peak hoặc sampling documented, macOS equivalent có caveat. Không cộng peak từng process ở thời điểm khác rồi gọi tổng peak. |
| Size/build | Release executable+native libs+fonts, installed runtime, compressed/unpacked Docker image; clean/cached build time & peak disk/RSS. Không so debug layout hiện tại với release native rồi gán toàn bộ lợi ích cho ngôn ngữ. |
| Correctness | Exact IDs/revision/provenance/options; bounds/line metrics, decoded RGBA, masks/text baselines; PNG encoded bytes size riêng. |
| Saturation | Sau single-job baseline, burst8 requests+background tier, queue-full/retry time, timeout/cancel và recovery; không cần pool đầu tiên. |

Ghi CPU/RAM/OS/architecture, compiler flags/profile, worker count, font bytes,
pipeline and artifact hashes. Đo macOS arm64 và Linux arm64 CPU container độc lập;
Docker VM resource limits cố định. Không gộp hai OS thành một p95. Báo median/p95,
sample counts/raw data và outliers; mọi số trong nghiên cứu hiện tại là **chưa đo**.

## Gates đề xuất — chưa được đo hoặc phê duyệt làm chuẩn thay thế

| Gate | Ngưỡng khởi điểm để ra quyết định |
| --- | --- |
| Contract/correctness hard | 100% current corpus API status/error/IDs/revision/ceil dimensions/limits đúng; không silent resource/glyph fallback; repeat same backend stable RGBA; không sửa authored data. |
| Text hard | Không tách grapheme, không thêm/mất line ngoài delta typography được chấp nhận; width sai≤1/64px trong fixed-font parity corpus, baseline≤0.5px; cases sát break width phải same decision. Nếu không đạt, không tự nới tolerance để chọn native. |
| Pixel candidate | Khởi điểm: exact flat interior colors/alpha, geometry edge≤0.5px; toàn ảnh≤1% pixels có max-channel delta>8/255, từng text crop≤5%. Chỉ là ngưỡng triage, không tự thay acceptance RGBA0; xem diff bằng mắt và approve policy trước rollout. |
| Determinism | Với pinned fonts/options, repeated native output decoded RGBA bằng0; cross-platform differences report rõ; nếu hứa identical mac/Linux thì phải bằng0 cả corpus. |
| Performance utility | Chỉ promote nếu warm p95 không tệ hơn baseline>10% và đạt ít nhất một mục tiêu: cold p95 giảm≥30%, process-tree peak RSS giảm≥40%, hoặc runtime footprint giảm≥50%. Đây là budget đề xuất, không forecast. |
| Operational hard | CPU-only Docker arm64 và macOS đều qua; corrupted inputs bounded; timeout kill/reap và next valid job ready; server/DB sống sau worker crash; không worker orphan hoặc cache key cross-engine. |

Nếu resvg qua packaging nhưng pixel/text gates fail, skia-safe có thể thắng.
Nếu cả hai fail, Chromium/CDP hoặc tối ưu pipeline cũ là kết luận hợp lệ. Không
hy sinh viewer parity chỉ để đạt mục tiêu “Rust thuần”. PDF/JPG/WebP không phải
gate bắt buộc current scope; report khả năng/phần việc bổ sung riêng.

## Migration giữ contracts và rollback

1. B0 instrumentation/baseline giữ backend cũ. Không đổi `render_frame`,
   `export_nodes`, HTTP POST export, file URL/MIME/filename/job states, queue tier,
   bounded wait, strict readiness và SSE semantics. Mark errors client-safe.
2. B1 backend adapter bên trong worker boundary; chọn engine ở startup config,
   không thêm public MCP tools. Engine name/build/features/fonts/policy vào startup
   fingerprint và cache namespace mới. Scene/renderer versions bump khi semantics
   đổi; update viewer version gate cùng release. Không giả old rendererVersion.
3. Shadow run corpus/temp data trước; native failure không poison Chromium cache.
   Không thử trên project thật khi chưa được giao. Nếu có fallback runtime, record
   actual engine identity, tránh job/pixel provenance báo native nhưng vẽ Chromium.
4. Canary explicit native, default cũ; compare metrics/errors/downloads/viewer trên
   corpus và user-approved sample. Retain native diagnostics/support matrix.
5. Bỏ Node/browser chỉ ở image native sau S2; chỉnh fingerprint Node probe, Docker,
   binary launch checks, README/runtime packaging đồng bộ. Build web/test có thể
   còn Node. Keep legacy build/image có pinned artifacts để rollback.
6. Rollback: chọn lại legacy engine + restart backend theo quyền orchestrator;
   tạo fresh job/cache identity, client refetch scene. Authored DB/assets không
   migration nên không cần restore dữ liệu. Mixed binaries/hot replacement chưa
   được hỗ trợ. Không reuse cache hoặc job scene từ engine/version khác.

## Đầu ra và điều kiện dừng spike

Deliver `report.md` + raw metrics + pinned source/artifact/license manifest +
PNG/diffs, exact rerun commands và cleanup evidence. Báo supported/unsupported
corpus từng engine, failures không đổi thành skips kín, mọi file>250 dòng và lý do.
Không viết “pass” Linux/macOS/WOFF2/emoji nếu chưa chạy. Một bộ metrics đầy đủ cho
hai candidate hoặc một hard failure chứng minh không phù hợp là đủ ra quyết định;
không mở rộng engine thứ tư trước khi giải thích được hai kết quả đầu.
