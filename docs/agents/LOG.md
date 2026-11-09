# Nhật ký điều phối OpenCanvas

- Các mục đã đóng về PR #7, Compose và WASM mặc định/Docker: [archive](history/2026-11-06-closed-viewer.md).

### Viewer WASM scene — 2026-11-05

- Người dùng giao compute scene bằng Rust WASM, giữ Canvas2D paint và server default/fallback; yêu cầu mới thay phạm vi painter của Task12 cũ. Worktree riêng `/private/tmp/opencanvas-viewer-wasm-scene`, nhánh `feat/viewer-wasm-scene` từ `feat/task11-native-png`; không commit/push/deploy hoặc dữ liệu thật.
- Giao fresh worker tách resolver/model/input thuần WASM và adapter text; root điều phối và kiểm chứng. Các bước WASM binding, viewer worker và parity sẽ nối theo API đã đóng băng.
- Giao fresh worker binding/build WASM và fresh worker viewer/FontFace dùng chung bytes. API ghim `SceneComputer`, URL maps do adapter cung cấp trước compute; font discovery dùng cùng resolver để không tải font ngoài frame. Native check core đạt; chưa nghiệm thu parity/browser.
- Nhận core extraction và WASM binding: workers báo native/WASM check, resolver regression, binding smoke và strict Clippy đạt. WASM đã sinh, raw khoảng 3,55 MB; root sẽ kiểm độc lập và giao fresh worker làm matrix parity/timing trên fixture tạm.
- Giao worker parity 5 frame × 4 viewport. Root review phát hiện native bổ sung text overflow advisories sau layout; giao fresh worker tách cùng logic thuần cho WASM để giữ JSON diagnostics, không bỏ qua khác biệt đó trong comparator. Workspace suite đang chạy; sẽ kiểm lại khi nguồn cuối ổn định.
- Nhận advisory parity và gate fixes: logic overflow/note intent dùng chung, golden thêm đúng effectiveTheme có từ nền, test deadline dùng semaphore riêng, smoke chọn preference notices rõ ràng. Root đã kiểm WASM smoke/35 web tests/typechecks/build/continuity; matrix worker báo 20/20 scene exact sau khi giữ HTTP JSON nguyên bản, đang kiểm báo cáo và chạy workspace suite cuối.
- Root kiểm lại độc lập 20 scene đã capture: 0 mismatch và mọi source/artifact hash còn khớp. Actions smoke cuối đạt. Giao fresh fix nhỏ cho worker error reply để Retry khởi tạo worker mới, tránh giữ module import lỗi trong worker cũ; không đổi Rust/parity.
- Nhận worker recovery đạt; root kiểm 37 web tests, build và WASM smoke cuối, thêm production viewer/backend tạm với instance: 0 request scene và MIME WASM đúng. Full workspace kết thúc 528 pass/1 fail/36 ignored, chỉ test catalog stroke cũ loại Instance trong khi nền đã cho phép; giao fresh worker sửa đúng test, không đổi hành vi sản phẩm.
- Điểm dừng: full workspace cuối **529 pass/0 fail/36 ignored**, 37 web tests, native/WASM check, strict library Clippy, scoped fmt và browser/continuity/actions/WASM/production smokes đạt; parity 20/20 exact. [Báo cáo triển khai](../VIEWER-WASM-SCENE.md), [parity/timing](../VIEWER-WASM-PARITY.md). Mọi source/test ≤236 dòng; viewer reference cũ 254 dòng. Không commit/push/deploy, main giữ nguyên source; dữ liệu kiểm đều tạm.
- Follow-up: giao fresh worker sửa đường dẫn source đã di chuyển và fresh worker cải thiện hủy request/cache snapshot. Chỉ làm tại worktree này; người dùng cho phép commit sau khi kiểm đủ gate, không push hoặc chạm main checkout. Email Git đã xác nhận đúng yêu cầu.
- Nhận sửa đường dẫn: renderer 11/11, không còn `storage/src` trong source/script được yêu cầu. Lượt Rust trong sandbox bị chặn socket loopback; chạy lại với quyền phù hợp. Giao fresh worker chuẩn bị nhóm commit có kiểm build từng bước, chờ root xác nhận đủ gate trước khi commit.
- Nhận worker reuse/cache: root kiểm web 43/43, renderer 11/11, hai typecheck, production build, WASM browser smoke, Clippy năm crate và parity 20/20 không mismatch; source/test tối đa 236 dòng. Rust suite còn chạy; cho worker chuẩn bị index/build bước đầu, chưa commit.
- Rust cuối đạt 448 pass/0 fail/36 ignored (server 243 pass); tất cả gate người dùng yêu cầu đã đạt. Cho worker build từng index export rồi commit bảy nhóm phụ thuộc, không push. Cập nhật [báo cáo follow-up](../VIEWER-WASM-SCENE.md#follow-up-verification--2026-11-05) và parity; chỉ working tree riêng được sửa.
- Nhận commit extraction `7694b8f`: build từ index export đạt cùng renderer typecheck/build/image-fit. Worker tiếp tục các nhóm còn lại; source sản phẩm không đổi sau gate cuối.
- Nhận commit shared measurer `a9c6679`, native build từ index export đạt; bước WASM dùng binary wasm-bindgen đã ghim tại thư mục công cụ tạm, không cài thêm toolchain.
- Nhận WASM `30669b1` và viewer `d5835d2`: native/WASM build từ index export đạt, WASM hash giữ nguyên; renderer/web build, web 43/43 và renderer 11/11 đạt trên tree viewer đã stage đủ regression cache.
- Nhận regression `5707967` và housekeeping `83fa8b0`: native/web build, traversal 3/3 và stroke 6/6 đạt; chỉ bỏ symlink dependency đã track, không thay thư mục dependency cục bộ.
- Điểm dừng trước commit cuối: mọi gate yêu cầu đạt, sáu nhóm source/housekeeping đã commit cục bộ; duyệt nhóm parity harness/tài liệu sau kiểm compile capture trên index export. Source/test tối đa 236 dòng, AGENTS 69 dòng/9476 byte; không push, không deploy, không sửa main checkout. Rust parse-once là tùy chọn chưa làm; snapshot đã encode/cache trong Worker.
- Review tiếp: giao fresh workers sửa recovery/timeout và phân loại lỗi Worker, dùng chung viewport budget giữa server/WASM, cùng cache font theo byte. Chỉ worktree riêng, không push/main checkout; email Git vẫn đúng. Các regression mới phải kiểm được lỗi review, source/test ≤250 dòng, chờ đủ gate trước commit fix.
- Nhận cache font 64 MiB +32 entry và hook Retry invalidation: focused font test 6/6 đạt; chờ API Worker cuối trước typecheck toàn web. Shared viewport giữ server downscale hợp lệ, WASM dùng scale1.
- Shared guard/neutral measurer/path docs đã sẵn sàng: root kiểm binding WASM mới 2/2, renderer 11/11 và typecheck đạt; WASM đã rebuild từ source mới. Giao fresh worker chuẩn bị một commit fix, chờ đủ gate. Recovery được gia cố để replay request còn sống sau lỗi init của request đã hủy.
- Nhận runtime cuối: root kiểm web 62/62, gồm r1 abort/r2 replay, real Worker chạy obsolete compute lâu hơn timeout và một budget chung init+compute; font/code-error/Retry regressions đạt. Native capture mới + parity đã đạt 20/20 không mismatch, strict Clippy năm crate đạt; final Rust/browser gates còn đang hoàn tất.
- Kiểm font mirror phát hiện lỗi sau khi font hash mới được nạp nhưng compute trả structured error: client có thể giữ ack hash cũ. Giao fresh worker sửa invalidation của mirror và regression quay lại hash cũ; giữ Worker, không đổi Rust. Chưa commit.
- Nhận font mirror fix: hai regression fail trước/đạt sau sửa, worker 27/27; root chạy cuối web 64/64, typecheck/build và WASM browser smoke đạt. Rust cuối 456 pass/0 fail/36 ignored (scene-input11, scene-wasm9, text38, storage153, server245); renderer11/11, binding2/2, Clippy và parity20/20 không mismatch đều đạt.
- Điểm dừng trước commit fix: đủ tám mục review, source/test tối đa206 dòng, AGENTS69 dòng/9476 byte; tài liệu tham chiếu cũ chỉ sửa path và cập nhật báo cáo. Duyệt một commit cục bộ từ source đã kiểm; không push, deploy hoặc sửa main checkout. Artifact/capture/binary generated vẫn ignore.

Các mục đã đóng về push main/ruleset và khảo sát app: [archive](history/2026-11-02-publish-notes.md).

Các mục đã đóng về build, design memory và P24: [archive](history/2026-11-03-closed-build-p24.md).

Append-only. Cách ghi theo `AGENTS.md` mục "Nhật ký và memory": mỗi task một heading có ngày, mỗi lần ghi vài dòng (quyết định, kết quả cuối, blocker, link báo cáo).
Lịch sử đến hết Task01–08 và commit `215313c`: `docs/agents/history/2026-10-28.md`.

- Các mục đã đóng 2026-10-28 → 2026-11-02 (design rules, P27, export Rust, thuộc tính Figma, public viewer, storage/PG spike): [archive](history/2026-11-03-log-1028-1102.md).

### Task19 — PostgreSQL runtime backend và Redis binary cache — 2026-11-03

- Người dùng duyệt triển khai bước1: PostgresBackend đầy đủ qua StorageBackend, tích hợp Redis nếu phù hợp; ưu tiên một key/project và zero-copy thay JSON. Root khảo sát domain/contract, chốt [handoff](../../tasks/19-postgres-runtime-backend.md), chia fresh workers backend, binary cache và runtime contracts.
- Redis dùng authoritative identity/revision trước cache và chỉ là read acceleration; API Snapshot owned còn bước materialization. Không migration/cutover, CLI engine selection, restart dịch vụ thật hoặc commit/push.
- Người dùng chốt dùng bitcode thay zero-copy; root chuyển yêu cầu cho cả ba workers và cập nhật handoff. Cache giữ một binary snapshot key/project, decode về owned Snapshot, PostgreSQL vẫn authoritative.
- Nhận adapter PostgreSQL đã đóng băng: full domain/shared validation, entity delta cùng durable revision/history/replay/resources, bounded reusable workers và cache namespace binding. Worker kiểm unit/SQLite resource regression/strict Clippy; root đang kiểm độc lập và giao fresh reviewer chỉ đọc cho source Task19.
- Root kiểm unit13 và SQLite regression19 đạt. Review độc lập phát hiện hai điểm cần sửa trong PG mới: PartialEq có thể bỏ thay đổi dấu của float0 và sort tài nguyên phụ thuộc localeDB. Giao fresh fix worker cho cùng Task19; chưa nghiệm thu trước sửa/lượt kiểm cuối. Bộ chạy Docker tạm được gia cố sau lỗi startup không output, không có lỗi backend trong lượt17 bài trước đó.
- Nhận binary cache và contract lanes hoàn tất; fresh fix worker đã sửa serialized delta và Rust ordering, thêm fidelity regression/ICU fixture. Root đọc delta và chạy độc lập final:20 PG/Redis +1 HTTP/MCP,13unit,19SQLite regression đạt; strict production Clippy/fmt/diff sạch. Xem [báo cáo Task19](../TASK19-REPORT.md).
- Điểm dừng: bước1 và Redis bitcode opt-in trong runtime source đã nghiệm thu phạm vi báo cáo; mọi source/test mới ≤250dòng, không giữ generated output. Docker Task19 đã dọn, inventory label rỗng. Chưa CLI chọn engine/bước2, migration/cutover/bước3, TLS/Kubernetes, benchmark runtime mới hoặc commit/push; không restart dịch vụ hay chạm dữ liệu thật.

### Task19b — Số đo runtime một writer — 2026-11-03

- Người dùng hỏi số liệu; root chốt [phạm vi đo](../../tasks/19b-runtime-single-writer-benchmark.md): release runtime thật, SQLite/PG/PG+Redis,100/5.000nodes, một writer và HOT reads, byte bitcode/JSON. Giao fresh benchmark worker và reviewer chỉ đọc; không sửa production engine hoặc dùng số spike thay runtime mới.
- Phiên tiếp tục tìm thấy đủ raw của hai lượt release. Đã kiểm lại percentile, fingerprint nguồn hiện tại, snapshot/revision, kích thước và Redis HOT counters. Docker inventory Task19 trống. Bổ sung lượt B và kết luận đối chiếu vào [báo cáo runtime](../RUNTIME-STORAGE-BENCHMARK.md).
- Điểm dừng: Task19b đã chốt benchmark một writer. SQLite dẫn về median đọc; Redis HOT làm chậm snapshot trực tiếp ở 5.000 node trong cả hai lượt. Chỉ cập nhật tài liệu tiến độ và báo cáo; CLI selection và migration vẫn ngoài Task19b.

- Mốc Redis 8, Moka và nghiên cứu Ferra đã đóng. Xem [nhật ký lưu trữ](history/2026-11-03-moka.md).

### JSON dạng hàng và GET snapshot revalidation — 2026-11-03

- Người dùng duyệt thử JSON theo schema và yêu cầu cache GET bằng Moka, kiểm revalidation kỹ. Giao hai worker mới cho MCP + encoder chung và HTTP + authoritative cache identity. Root sẽ kiểm parity, hành vi revision/ETag và số đo.
- Giao worker riêng bổ sung benchmark release với GET 200 và conditional 304 cho từng format. Nhận harness đã compile/lint, chờ nguồn HTTP đóng băng. MCP đã qua parity và paging; golden cũ lệch trường `effectiveTheme` có từ trước.
- Nhận HTTP + cache: kiểm unit/SQLite/PostgreSQL đạt, gồm race, nguồn lỗi, asset mất và đổi incarnation. Root chạy release và kiểm raw, 304 body rỗng và HOT vẫn probe nguồn. [Báo cáo](../RUNTIME-STORAGE-BENCHMARK.md#results--release-run-d-with-http-response-cache-2026-11-03).
- Điểm dừng: opt-in rows và cache GET đã kiểm, default body giữ tương thích. Dọn fixture thành công, chưa phát hành. Xem [contract](../CONTRACT.md#http-viewer-surface).

### Chọn backend và cổng qua env — 2026-11-03

- Lưu điểm bàn giao sau benchmark và revalidation. Người dùng duyệt `DATABASE_PROVIDER` mặc định SQLite, `DATABASE_URL` bắt buộc cho PostgreSQL + Moka và `PORT` cho cổng riêng. Giao worker mới nối startup và kiểm bằng binary thật với storage tạm.
- Nhận startup hoàn tất: test cấu hình và smoke debug hai backend đạt; PG có bằng chứng Moka snapshot dùng chung giữa hai format HTTP. Root rebuild server release và kiểm `npm start` trên PG/SQLite tạm, xác nhận ghi đúng engine, cổng env và ưu tiên CLI.
- Điểm dừng: đã dọn fixture, giữ tiến trình người dùng đang chạy. Không migration hay commit/push. [Cách chạy](../../README.md).

### Tối ưu tích hợp PostgreSQL theo review — 2026-11-03

- Root review chỉ đọc storage/postgres, hot cache, snapshot HTTP và SSE; người dùng duyệt sửa cả 7 mục: SSE đọc full snapshot chỉ để lấy revision, lỗi kết nối PG bị coi là không retry được, thiếu write-through/gộp miss cache, đường ghi O(kích thước project), prepare lại câu lệnh mỗi lần và probe lặp, dry run giữ `FOR UPDATE`, cấu hình pool/timeout qua env, clone sâu snapshot khi hit, readiness font O(n²).
- Chia 6 worker tuần tự trên cùng working tree (thay đổi Task19 chưa commit nên không dùng worktree): lỗi kết nối → SSE → env/readiness → round trip → cache → đường ghi/dry run. Root chạy baseline release `task19-bench.sh` trước khi sửa và đo lại sau cùng. Không commit/push, không chạm cổng 3847/3848 hay `~/.opencanvas`.
- Mục 2 xong: classifier `postgres/failure.rs`, worker bỏ client sau `STORAGE_UNAVAILABLE`, test refused/reconnect PG thật; root chạy lại unit 8/8. Phát hiện HTTP trả 400 cho `STORAGE_UNAVAILABLE`, gộp sửa 503 vào worker SSE.
- Mục 1 xong: SSE và import font dùng `project_meta`, `STORAGE_UNAVAILABLE` → 503; test `events_metadata` bắt hồi quy. Root chạy lại đạt. Suite server 194/196: golden outline lệch có từ trước và `workspace_events` flaky trên bản gốc (5/6 lần), không thuộc thay đổi này. Clippy all-targets còn dead code test support có sẵn.
- Mục 7 (vận hành) xong: `DATABASE_POOL_SIZE` và 4 biến `DATABASE_*_TIMEOUT_MS` (module `startup/postgres_env.rs`, README), readiness font dùng `partition_point`. Worker smoke binary debug với PG tạm; root chạy lại `startup_env` 9/9.
- Mục 5 xong: `postgres/statements.rs` (Session + cache prepared statement theo worker, xoá khi 0A000/26000), gộp probe và đọc miss vào một `pool.run`. Round trip: hit 2→1, apply_batch 16→9. Release p50 5.000 node: snapshot pgonly 7,0→5,2 ms, Moka hit 1,33→0,90 ms; commit gần như không đổi. Root chạy lại lib 17/17. Test `stroke_props` fail là của thay đổi props_catalog đang dở, ngoài phạm vi.
- Mục 3 + Arc xong: `postgres/write_through.rs` dựng snapshot sau commit từ đúng payload đã ghi, put trên thread caller sau commit thành công; replay/dry run/lỗi không put; cache trả `Arc<Snapshot>`, `snapshot_with_cache_identity` trả Arc. Bỏ single-flight vì chờ trong worker có thể cạn pool. Root chạy lại lib 20/20. `docs/MOKA-SNAPSHOT-CACHE.md` cần cập nhật cuối đợt.
- Mục 4 + 6 xong: đường ghi so payload mới với text đã lưu (`snapshot/stored.rs`), bỏ serialize `before` và clone Candidate thứ hai, gộp 5 câu ghi thành một CTE; dry run chạy RR read-only không `FOR UPDATE`, test `postgres_dry_run` chứng minh không bị khoá. Không bỏ serialize `after` vì không chứng minh được tập thay đổi đủ.
- Root kiểm độc lập: lib 24/24, 12 binary PG storage 24/24, server `postgres_backend` 2/2, events/startup/snapshot_http/storage_backend đạt, clippy storage all-targets + server lib/bins sạch, file chạm ≤233 dòng, không còn container. Release p50 5.000 node: commit PG 30,2→25,4 ms, snapshot PG 7,0→5,2 ms, Moka hit 1,33→1,05 ms. [Run E](../RUNTIME-STORAGE-BENCHMARK.md#results--release-run-e-after-the-postgresql-review-fixes-2026-11-03).
- Điểm dừng: 7 mục đã triển khai, chưa commit/push/restart dịch vụ thật. Còn mở: validate tăng dần (~10 ms/commit, dùng chung SQLite), `get_changes` vẫn đọc full snapshot, single-flight miss bị bỏ có chủ đích.
- Người dùng bỏ kế hoạch validate tăng dần (scout bị dừng, không để lại file) và yêu cầu commit/push. Nhánh hiện tại chứa `claude` và lịch sử cũ, nên root tạo `feat/postgres-runtime-storage` từ `origin/main` (cùng tree với HEAD) và commit toàn bộ thay đổi thành 2 commit: spike/research (giống hệt nhánh spike đã push) và runtime storage. Đã quét secret/attribution, push branch; chưa mở PR. Test `stroke_props` của props_catalog vẫn fail trong commit.

### Task20 — Kubernetes local — 2026-11-03

- Người dùng yêu cầu nhánh mới từ `feat/postgres-runtime-storage` và chốt triển khai local. Root tạo `codex/kubernetes-local` từ `d93db07`; Docker/cluster OrbStack sẵn có, chưa có namespace ứng dụng.
- Giao fresh worker theo [handoff](../../tasks/20-local-kubernetes.md): cấu hình module nhỏ, PostgreSQL PVC, một app replica, truy cập loopback cổng riêng, build/deploy/smoke và kiểm persistence. Không chạm native server/dữ liệu thật, không commit/push.
- Image nguồn hiện tại đã build; root kiểm độc lập app/PG Ready, PVC Bound, viewer200, Host/Origin guard403, MCP initialize/list, revision1 và conditional GET304 body rỗng. Worker đã kiểm render PNG và persistence qua thay Pod app/PG; đang hoàn thiện lifecycle, kiểm apply lặp và dọn fixture trước nghiệm thu.
- Nhận kết quả cuối: apply giữ Secret/PVC, dừng SIGINT có `Stopping…`, DB outage làm NotReady/503 nhưng viewer200 và không restart app; phục hồi giữ dữ liệu. Fixture được xoá đúng ID+slug, app thay lần cuối để dọn cache/jobs.
- Điểm dừng: root kiểm lại app/PG Ready1/1, PVC cũ Bound2Gi, HTTP200 với danh sách project rỗng; deployment/port-forward3857 giữ chạy. [Runbook](../KUBERNETES.md), toàn bộ source/config mới dưới200 dòng; chỉ PLAN438 dòng là tài liệu yêu cầu gốc. Không commit/push, không HA/migration hay đổi native server/dữ liệu thật.

### Trao đổi kiến trúc GPU renderer — 2026-11-03

- Root đọc viewer, painter, text, compositing và worker export: nội dung scene đi qua shared Canvas2D `paintScene`; Konva giữ stage/camera/events và overlay, hit-test riêng theo scene.
- Kết luận tư vấn: có thể thêm GPU backend giữ buffers/textures và cập nhật delta; cần xử lý glyph shaping, clip/group opacity và parity với PNG. Chưa triển khai hoặc giao task; Task09–13 vẫn gác lại, chưa có benchmark GPU để kết luận mức tăng tốc.

### Task20b — Kubernetes public viewer 3858 — 2026-11-03

- Người dùng yêu cầu thêm listener chỉ đọc3858 để forward Funnel. Root lấy HTTPS origin từ Tailscale local, xác nhận chưa có serve config; giao fresh worker theo [handoff](../../tasks/20b-kubernetes-public-viewer.md).
- Phạm vi: cấu hình public router hiện có, triển khai namespace local, kiểm read/export và chặn MCP/mutation, giữ port-forward3857/3858. Không tự bật Funnel, không đổi native/DB thật, không commit/push.
- Nhận overlay đã rollout; worker kiểm34 đường MCP/ghi bị chặn, read/font/scene/PNG8544byte/export SVG+PNG và snapshot bất biến. Root kiểm độc lập viewer200/readOnly=true, Host/Origin403, MCP/create/import/delete403 dù spoof headers, font200, project parity và MCP local hoạt động. Đang kiểm apply lặp và dọn fixture trước điểm dừng.
- Điểm dừng: apply lặp và smoke lần2 đạt, Secret/PVC giữ UID, fixture đã dọn và cache/jobs đã xoá qua restart app. Root xác nhận app/PG Ready, PVC Bound2Gi, hai forward chỉ bind127.0.0.1 và Tailscale chưa có serve config. [Runbook](../KUBERNETES-PUBLIC-VIEWER.md); mọi module mới dưới200 dòng, không rebuild/commit/push hoặc đổi native. Listener sẵn cho người dùng tự bật Funnel.

### Đổi kết nối MCP sang 3857 — 2026-11-03

- Người dùng yêu cầu đổi cổng MCP OpenCanvas sang3857. Giao worker mới chỉ sửa URL kết nối Codex và kiểm MCP initialize chỉ đọc; giữ cấu hình approval của tool và các dịch vụ hiện có.
- Nhận kết quả: chỉ URL3847→3857 thay đổi trong cấu hình Codex, approval giữ nguyên. Root kiểm `codex mcp get opencanvas` xác nhận3857; worker kiểm initialize HTTP200 và đóng session HTTP202. Điểm dừng: cấu hình đã đổi, không restart dịch vụ hoặc chạm dữ liệu.

### Chuyển hai project 3847 sang 3857 — 2026-11-03

- Người dùng yêu cầu lấy hai project trên server cũ và migrate sang MCP mới. Root xác nhận nguồn có `axis-courses` revision206 và `axis-workspace-login` revision21; đích trống. Giao worker mới capture nhất quán, nhập PostgreSQL đích nguyên tử và kiểm parity, giữ ID/revision/resources/history; nguồn chỉ đọc, không sửa cấu hình hoặc restart dịch vụ.
- Người dùng chỉnh phạm vi: chỉ đọc API nguồn và ghi bằng MCP đích. Đã ngắt worker DB và huỷ hướng import trực tiếp; giao worker mới theo phạm vi API/MCP, không đọc thêm code hoặc DB. Native MCP handle của chat vẫn trỏ3847 nên worker phải kết nối MCP tường minh3857 để tránh ghi nhầm nguồn.
- Nhận kết quả API/MCP: Courses đích `p_493b127cceb1474d9c0ce8a0dae32227` revision60; Login đích `p_96bb45df27b649abb2bc04cfba803864` revision18. Root đối chiếu toàn bộ snapshot: khớp khi chỉ chuẩn hoá projectId/revision và thứ tự mảng; nguồn vẫn206/21. Worker kiểm62binary cùng bundled font khớp hash/size, MCP readback và render cả hai đạt. Lịch sử cũ không chuyển qua create/apply MCP; xem [báo cáo](../API-MCP-MIGRATION.md).
- Người dùng phản hồi client Node và10cảnh báo U07. Root xác nhận script đã gửi Structured Operations qua MCP nhưng dùng Node client; native tool handle vẫn3847. Kiểm cùng frame `u07-a3-tile`: nguồn0cảnh báo, đích10; authored nodes/font hashes khớp nhưng ink metrics khác. Render smoke trước thiếu frame này; fidelity dữ liệu không chứng minh parity render. Điểm dừng: đã ghi bằng chứng vào báo cáo, chưa cô lập nguyên nhân nền tảng hoặc sửa renderer/design.

### Task21 — U07 text ink diagnostics — 2026-11-03

- Người dùng yêu cầu rà và sửa10cảnh báo. Root đọc measurement/diagnostics: dùng cùng ngưỡng0,5px cho advance, ink và clip;10ca thực tế có advance vừa hộp, không clip, ink nhô tối đa1px trên Linux. Giao fresh worker theo [handoff](../../tasks/21-text-ink-rounding.md): điều tra và sửa có đối chứng overflow thật; root review trước cập nhật app Kubernetes, giữ dữ liệu/native/public overlay.
- Nhận bản sửa: chỉ own-box ink ngang được nới tối đa1px khi hai đầu advance khớp hộp trong1/64px; clip/advance/vertical giữ0,5px. Root review diff và tự chạy14test đạt, gồm10ca thực tế và overflow/clip đối chứng. Cho worker build image/cập nhật riêng app Kubernetes, chưa nghiệm thu runtime trước khi so PNG/snapshot.
- Root kiểm độc lập sau rollout: U07 ready, diagnostics rỗng; PNG146169byte giữ nguyên SHA256 và hai snapshot đích khớp byte trước sửa, revision60/18. Nguồn vẫn206/21; GET trực tiếp3858 bị guard403 như thiết kế, worker đang xác nhận public origin và identity hạ tầng trước chốt báo cáo.
- Điểm dừng: worker xác nhận public3858 đúng origin200/readOnly, sai Host403, PG/Secret/PVC và public overlay giữ nguyên, hai loopback forward hoạt động; tài nguyên/snapshot không đổi. Task21 đã kiểm và deploy riêng app,14test/fmt/clippy đạt, code chạm tối đa132dòng. [Báo cáo](../TEXT-INK-DIAGNOSTICS-FIX.md); không sửa design/native, không commit/push.

### Đối chiếu cấu hình MCP sau migration — 2026-11-03

- CLI trong profile CodexPortable `account-2` báo OpenCanvas ở 3847; cấu hình `~/.codex` trỏ 3857. Kết quả khớp với việc phiên migration phải dùng client MCP tường minh để gửi Structured Operations tới đích. Điểm dừng ở chẩn đoán cấu hình.

- Người dùng xác nhận migration chạy ở profile khác và yêu cầu đổi endpoint của profile phiên hiện tại. Giao worker chỉ đổi URL OpenCanvas của `account-2`; root tách điều tra phiên migration khỏi cấu hình này.
- Nhận thay đổi đúng một dòng ở `account-2`; CLI đọc 3857 và endpoint trả danh sách tool. Root gọi `list_projects` trực tiếp trong chat đang mở, vẫn trả ID SQLite từ 3847. Settings và CLI hiện tại chưa cung cấp đường nạp lại cho chat này.
- Điểm dừng: trace của phiên migration trong profile kia cũng cho thấy `list_projects` trực tiếp trả hai ID nguồn sau khi file cấu hình đã đổi sang 3857. Worker dùng Node để khởi tạo kết nối đích mới rồi gửi Structured Operations qua `apply_batch`. Sự khác biệt profile giải thích các file cấu hình riêng; kết nối còn giữ cổng cũ trong chat mới là nguyên nhân của fallback.

### Commit và push Kubernetes/Task21 — 2026-11-03

- Người dùng yêu cầu commit và push. Root xác nhận nhánh `codex/kubernetes-local`, đích `origin`; giao worker mới kiểm các file chờ version, gom commit theo phạm vi và push nhánh hiện tại. Không thay code/runtime/data, không force-push hoặc mở PR; giữ thay đổi đồng thời của phiên khác.
- Đã commit và push `origin/codex/kubernetes-local`: `bb200ea` triển khai/public viewer, `f470f0b` ink fix và `c91c209` bằng chứng migration/tiến độ. Bash/Node syntax, hai Kustomize render offline và diff check đạt; không chạy lại build/test hoặc smoke có ghi dữ liệu. Không đưa hostname riêng, credential hoặc capture vào commit; mọi module mới dưới200 dòng, PLAN440 dòng là tài liệu yêu cầu hiện có.
- Điểm dừng: push chính thành công, cập nhật trạng thái Task20/20b/21 và lưu kết quả bằng commit tài liệu cuối. Không mở PR hoặc thay đổi runtime/dữ liệu; các kết quả kiểm chứng Task21 đã có vẫn giữ phạm vi báo cáo.

### Khảo sát chuyển Kubernetes sang Docker Compose — 2026-11-03

- Giao và nhận khảo sát chỉ đọc về Compose, dữ liệu PostgreSQL và public viewer. Root xác nhận hai project trên 3857, PostgreSQL 16 và PVC đang hoạt động. Compose mặc định dùng SQLite.
- Điểm dừng: phương án là dump/restore toàn bộ PostgreSQL vào volume Docker riêng, kiểm parity trước khi mở ghi, giữ PVC nguồn để quay lại và giữ cổng 3857/3858. Chưa tạo backup hay chuyển dịch vụ.

### Chuyển runtime PostgreSQL từ Kubernetes sang Compose — 2026-11-03

- Người dùng duyệt dừng ghi và chuyển stack để giảm tài nguyên nền Kubernetes. Giao worker riêng cho cấu hình Compose và backup thử. Chỉ đóng băng nguồn sau khi khôi phục thử đạt; giữ PVC nguồn và endpoint 3857/3858.
- Nhận cấu hình Compose và backup thử cùng fingerprint theo snapshot PostgreSQL. Kiểm schema, archive và hai cổng loopback đạt; giao worker vận hành khôi phục thử trên volume riêng trước khi chuyển cổng.
- Bản thử qua đối chiếu SQL, snapshot HTTP, MCP, PNG và public viewer sau restart hai container. Root duyệt bắt đầu cutover đã được người dùng cho phép. Worker OrbStack xác minh `orb stop k8s` chỉ dừng cụm, chờ Compose qua kiểm tra trước khi thực hiện.
- Cutover hoàn tất. Dừng writer, giữ archive đã kiểm checksum, restore và đối chiếu toàn DB trước mở cổng. Hai project giữ ID/revision; snapshot, lịch sử, binary, MCP, PNG U07 và public guard đạt. PVC/Secret nguồn giữ nguyên và rehearsal đã dọn. [Runbook](../COMPOSE-POSTGRES.md).
- `orb stop k8s` dừng toàn bộ container Kubernetes nhưng OrbStack cũng restart Docker ngắn. Compose tự lên lại; root kiểm lại snapshot/MCP/3858. Task15 PG cùng container đã được khởi động lại với port động mới; container test AXIS tạm bị Docker dọn. Điểm dừng ở hai container Compose healthy và cụm Kubernetes đã dừng.

### Đo RAM Compose và dừng PostgreSQL thử nghiệm Task15 — 2026-11-03

- Người dùng yêu cầu giải thích icon/RAM và dừng PostgreSQL phụ nếu không còn dùng. Giao worker kiểm tra consumer và dừng đúng container Task15 khi an toàn; root kiểm nhãn và RAM của Compose.
- Điểm dừng: Task15 benchmark có 0 client, đã dừng sạch và giữ volume. Root kiểm cả hai container Compose healthy và hai revision giữ nguyên. App vẫn dùng tag image `kubernetes-local` dưới Compose; Kubernetes tắt. Node/Chromium và Rust chiếm phần lớn RAM tiến trình, cgroup có cache file và đỉnh cao hơn mức lúc đo.

### Phạm vi đọc khi agent cập nhật design — 2026-11-03

- Điểm dừng: đối chiếu mã và hợp đồng MCP. Agent có thể đọc theo ID và dùng `committedRevision` cho lần ghi kế tiếp. PostgreSQL vẫn nạp và kiểm tra candidate toàn project ở phía server. Chưa đo token hoặc độ trễ ở 10.000 node.

### Kiểm xung đột PR #1–#3 — 2026-11-03

- Điểm dừng: xác minh HEAD từ GitHub và mô phỏng merge trong thư mục Git objects tạm. Cả ba sạch với main hiện tại; từng cặp xung đột ở tài liệu. PR #3 đã chứa các thay đổi code của #1 và #2. Không merge hay push.

### Xuất bản nhánh Docker Compose PostgreSQL — 2026-11-03

- Người dùng yêu cầu commit và push nhánh mới. Giao worker tách cấu hình/runbook Compose và nhật ký liên quan từ `codex/kubernetes-local`; giữ các ghi chú khảo sát khác ở working tree.

### Khảo sát hover của nhóm kích hoạt animation con — 2026-11-03

- Fast-forward `main` đến `256554b` và khảo sát trong worktree riêng do cây Kubernetes/Compose đang có thay đổi cục bộ. Đề xuất `triggerNodeId` tùy chọn trên từng hover animation, xuất qua `get_design_context.animated`; agent nhóm theo node nguồn và chọn cơ chế hover của framework đích.
- Điểm dừng: ưu tiên validation, MCP và hướng dẫn agent. Preview để sau. Danh sách `animated` hiện lặp trên mọi trang và chiếm ngân sách metadata; cần kiểm trường hợp frame nhiều animation. Chưa triển khai.

- Người dùng duyệt triển khai. Giao worker mới cho hợp đồng storage/MCP và hướng dẫn agent trên `codex/hover-animation-trigger` từ `256554b`. Preview là phần tiếp theo sau khi kiểm hợp đồng.
- Giao worker riêng chuẩn bị preview theo tên trường đã chốt. Smoke qua server thật phụ thuộc phần validation/MCP và dùng data root tạm.
- Nhận phần preview đã qua Chromium và 26 web unit. Hai worker chạm giới hạn phiên sau khi để lại mã và test. Root chạy lại kiểm tra Rust/MCP trong worktree riêng và giao một lượt hoàn thiện phần còn dở.
- Nhận bản hoàn thiện: MCP công bố `triggerNodeId`, giữ nguyên self-hover cũ, giới hạn summary theo ngân sách và hướng dẫn truy vấn lại theo frame/revision khi có `animatedTruncated`. Preview dùng chung vùng hover theo ID nguồn. Đã sửa fixture cho ca ngân sách và ca nguồn hover thiếu.
- Điểm dừng: root kiểm 9 test storage, 7 test MCP, 26 web unit, 11 bước Chromium, typecheck, fmt và Clippy lib đạt. Fixture 240 animation giữ giới hạn 64 KiB và đọc đủ khai báo. `tools/list` 26.575→26.825 byte (baseline tính từ delta mô tả), dưới 32 KiB. Mã nguồn và test chạm tối đa 232 dòng. Thay đổi nằm trong worktree `codex/hover-animation-trigger`, chưa commit hay cập nhật runtime.

### Đồng bộ nhánh hover với main — 2026-11-03

- Fast-forward `main` và nhánh hover lên `62f258a`. Merge ba chiều tự gộp `PLAN.md` và chỉ xung đột ở hai đoạn nối cuối nhật ký. Giữ cả hai đoạn, đặt mục hover sau nhật ký từ main. Các thay đổi hover vẫn ở worktree chưa commit.
- Điểm dừng: không còn mục unmerged. `git diff --check`, Rust fmt và 6 test MCP hover đạt sau khi cấp dependency Playwright tạm cho worktree.
- Người dùng duyệt commit và push `codex/hover-animation-trigger`. Phạm vi gồm hợp đồng hover, preview, test và tài liệu liên quan trên nền `62f258a`.

### Task09 — Spike WASM renderer Rust — 2026-11-04

- Người dùng yêu cầu làm tiếp Task09 để chuyển renderer sang Rust. Máy là VM Linux aarch64 4 nhân không GPU (tiny-skia raster CPU nên không cần GPU); handoff gốc nhắm macOS, số đo ghi rõ nền tảng. Người dùng duyệt cài rustup stable 1.99.1 + `wasm32-unknown-unknown` (root cài thêm `build-essential`, `brotli` qua apt) và bỏ phần đo Chromium/Worker vì máy chưa có Node/Playwright.
- Spike không build được trên `main` @`c2bf260` vì `opencanvas-layout` đã đổi `TextLine`/`Measurement`/`fill`/`radius`; root sửa 4 dòng để build lại native. Worktree `../OpenCanvas-task09`, nhánh `feat/task09-wasm-spike` từ `main`. Giao fresh worker: tách lib C-ABI, build WASM hai biến thể, đo size/native, so pixel native↔WASM qua runtime WASM nhúng trong spike.

### Xác thực public viewer — 2026-11-03

- Giao worker backend cho xác thực riêng public listener, lưu phiên trong SQLite/PostgreSQL, công cụ quản trị MCP và IP qua proxy tin cậy. Giao worker frontend cho dialog và khóa mount viewer theo trạng thái phiên. Kiểm trên dữ liệu tạm trước khi đánh giá rollout.
- Nhận frontend, đã qua Chromium mock, typecheck và smoke viewer hiện có. Probe OrbStack xác nhận peer là gateway mạng Compose. Giao worker vận hành build image riêng, chạy thử với PostgreSQL tạm và chờ root kiểm độc lập trước khi cập nhật app.
- Ngày 2026-11-04, người dùng yêu cầu tiếp tục sau khi worker vận hành dừng do giới hạn sử dụng. Root xác nhận Chromium qua HTTPS và MCP với SQLite, cùng phiên PostgreSQL qua restart, đổi IP, đổi mật khẩu và thu hồi. Giao lại bước vận hành cho worker mới.
- Nhận rehearsal trên image build lại từ mã hiện tại. Compose/PostgreSQL tạm kiểm đủ vòng đời phiên và proxy, 12 kiểm thử Rust đạt, Clippy sạch. Root ghi fingerprint chỉ đọc cho toàn bộ project trước khi worker cập nhật riêng container app.
- Điểm dừng: Compose đã chạy image auth mới, PostgreSQL giữ container và volume. Root xác nhận bốn snapshot khớp checksum, 3858 mặc định mở và chỉ đọc, 3857 có ba công cụ auth MCP. Rehearsal đã dọn, Funnel chưa cấu hình. [Báo cáo](../PUBLIC-VIEWER-REPORT.md#password-protection--2026-11-04).

### Quyền xem theo project — 2026-11-04

- Giao worker storage cho metadata và chuyển đổi dữ liệu cũ mặc định private. Giao worker server cho HTTP/MCP và ranh giới 3858, kể cả SSE và job/resource đã tạo. Giao worker web cho cập nhật danh sách và bỏ project đã chuyển private khỏi màn hình đang mở.
- Hai worker storage/web dừng giữa chừng do giới hạn model, đã giao lại phần còn thiếu cho hai worker mới. Server đã build, root chạy kiểm PostgreSQL riêng. Đã lấy fingerprint bốn dự án hiện tại để đối chiếu khi rollout.
- Nhận web: Chromium HTTPS với MCP thật xác nhận hai tab tự ẩn rồi hiện lại khi đổi quyền. Root kiểm PostgreSQL qua restart, PATCH/MCP, ETag và JWT. Giao worker vận hành build image riêng và kiểm nâng cấp từ schema hiện tại trên PostgreSQL tạm.
- Nhận storage: migration và bundle đã kiểm cả hai backend. Trạng thái Funnel dạng JSON xác nhận proxy foreground đang trỏ 3858. Không dùng image cũ với public listener nếu phải rollback khi proxy còn hoạt động.
- Nhận server: qua kiểm pagination/ETag, URL job đã cấp, SSE và export đang xử lý. Golden traversal hiện có cũng lỗi trên bản trích sạch từ HEAD. Rehearsal nâng cấp từ image cũ giữ snapshot và phiên đăng nhập, nhưng trả 404 cho project cũ mặc định private. Cho worker thay riêng container app.
- Điểm dừng: Compose đã chạy image privacy mới, app và PostgreSQL healthy. Root đối chiếu bốn snapshot giữ nguyên phần thiết kế, cả bốn private trên 3857. URL HTTPS Funnel trả danh sách rỗng và link riêng tư 404. Rehearsal đã dọn. [Báo cáo](../PUBLIC-VIEWER-REPORT.md#project-visibility--2026-11-04).

### Rà secret và commit public viewer — 2026-11-04

- Người dùng yêu cầu rà nhanh secret và commit cục bộ trên `codex/compose-postgres`. Giao worker rà diff/staging đã che giá trị và commit đúng phạm vi auth/privacy, giữ hai ghi chú khảo sát ngoài phạm vi trong working tree.
- Root xác nhận tag image dùng cấu hình runtime đã ignore. Kiểm so khớp chính xác mật khẩu PostgreSQL đang dùng trên toàn bộ file thuộc phạm vi Git không có kết quả. Khóa ký viewer sinh ngẫu nhiên khi đặt mật khẩu và lưu trong database.

### Khôi phục render dữ liệu responsive trên Compose — 2026-11-04

- Tái hiện HTTP 400 ở revision 137. Snapshot giữ giá trị width phần trăm. Root xác nhận rollout Compose build từ nhánh thiếu phần hỗ trợ responsive đã có ở worktree riêng. Hai project hiện dùng kích thước phần trăm, một project dùng breakpoint. Giao worker tích hợp phần responsive vào nhánh Compose và kiểm bằng dữ liệu tạm.
- Giao worker vận hành dựng bản sao PostgreSQL tạm và tái hiện lỗi bằng image hiện tại trước khi kiểm image tương thích, giữ nguyên dữ liệu thật và Funnel.
- Phiên tiếp tục xác nhận nhánh `codex/compose-postgres` có phần tích hợp responsive chưa commit và bản thử `opencanvas-responsive-7f7b5d96` đang healthy. Giao worker mới chốt tích hợp/kiểm hồi quy, cùng worker vận hành tiếp quản bản thử rồi cập nhật riêng app sau khi root kiểm đạt; giữ PostgreSQL, dữ liệu authored và Funnel.
- Kiểm tích hợp tìm thấy helper `Candidate::ancestors` còn thiếu trên nền Compose, khiến storage chưa compile; worker tích hợp bổ sung phần thiếu trước khi dựng image. Layout đã qua kiểm, chưa coi tích hợp hoặc rollout hoàn tất.
- Nhận tích hợp hoàn tất: 70 test Rust đạt, production Clippy/fmt/diff sạch; root kiểm độc lập 10 test pipeline Chromium đạt. Canary image đồng bộ server/layout-step đã render Training và Responsive Lab, giữ bốn snapshot và 84 asset binary; ranh giới auth/privacy giữ nguyên. Root kiểm riêng ngưỡng 721/720/390, inspect/PNG, viewer desktop/mobile GET-only và duyệt thay riêng app theo phạm vi đã được người dùng cho tiếp tục.
- Blocker: bộ duyệt tự động từ chối lệnh thay app thật vì yêu cầu tiếp tục chưa phải xác nhận rollout rõ ràng. Chưa thay container thật; root đã gửi form xin phép cụ thể cho image đã kiểm. Giữ canary và bằng chứng để có thể tiếp tục đúng thao tác khi người dùng trả lời; không thử lại hay vượt chặn.
- Điểm dừng theo chỉ đạo người dùng: “App thật tôi sẽ tự cài, không tự deploy vào app thật”. Dừng bước rollout, bàn giao image đã kiểm và [báo cáo](../RESPONSIVE-COMPOSE-REPORT.md); app thật/PG/Funnel giữ nguyên. Giữ canary hiện có và bằng chứng, không commit/push.
- Người dùng tự build `kubernetes-local` rồi chạy Compose nhưng browser còn lỗi. Kiểm chỉ đọc xác nhận `prod.env` vẫn chọn `project-privacy-local`, container thật còn image ID cũ và chưa được tạo lại. Bàn giao cách chọn đúng tag; không sửa env hay tự deploy.

### Commit tích hợp responsive — 2026-11-04

- Người dùng yêu cầu commit cục bộ và chốt cập nhật log trước commit, không ghi log sau commit. Giao worker mới rà/stage đúng phần responsive, test và tài liệu hiện có trên `codex/compose-postgres`, rồi tạo một commit; không push hoặc deploy.
- Kiểm trước bàn giao: diff sạch whitespace, `AGENTS.md` trong ngân sách; bằng chứng 70 test Rust, 10 test pipeline Chromium, fmt/Clippy và canary ở [báo cáo](../RESPONSIVE-COMPOSE-REPORT.md). Source không đổi sau lượt kiểm đã ghi. Sau commit chỉ kiểm đọc Git và báo kết quả trong chat; commit hash tra bằng Git history.

### Task09 — khôi phục nhánh và merge nền — 2026-11-04

- Khôi phục `feat/task09-wasm-spike` từ `c560961` trong worktree riêng, merge nền Compose `dbd37a0` vào Task09. Giữ hướng dẫn hover mới, responsive và cả hai đuôi nhật ký; không sửa working tree Compose hoặc push/deploy.
- Sandbox sạch cùng commit, không tìm thấy số đo WASM hoặc source chưa push trong phạm vi đã kiểm. WASM type-check và 5 test responsive đạt; native spike bị chặn ở adapter viewport `f64`/`f32`, để worker tiếp theo sửa đúng phạm vi spike. Chưa benchmark hoặc kết luận go/no-go.
- Điểm dừng và lệnh tiếp tục: [Task09 recovery](../../spikes/rust-export/RECOVERY.md). Source spike giữ nguyên; có whitespace cũ từ notice bar trên nền Compose, không chỉnh ngoài phạm vi.
- Nhận merge cục bộ `9e19d38`; root chuyển phiên sang worktree Task09 và giao worker mới hoàn thiện adapter, gia cố runner rồi đo native/WASM trong Chromium Worker. Target WASM đã có sẵn; không cần cài toolchain, không push/deploy hoặc thay engine sản phẩm.
- Nhận source/benchmark và bảng Phần E: size/parity đạt nhưng latency vượt gate trên host tải cao; tile DPR1 còn lệch crop dù native/WASM khớp. Root giao reviewer fresh chỉ đọc và chạy lại phép đo độc lập trước chốt; không coi toàn CLI lint/fmt xanh vì lỗi/style cũ đã công bố.
- Reviewer không có Required; root chạy lại native/Chromium, kiểm hash input và 12 phép so RGBA đều trùng, 10 Node + 1 Rust test đạt. Hai lượt vẫn vượt gate thời gian trên host tải cao; giữ no-go hiện tại, cần đo máy rảnh trước quyết định dài hạn. [Bảng số và giới hạn](../../spikes/rust-export/README.md#phần-e--task09-wasm-trong-chromium-web-worker-2026-11-04).
- Điểm dừng: native release rebuild và diff-check đạt, source tạo/sửa dưới 200 dòng, AGENTS trong ngân sách. Merge nền đã commit cục bộ; phần hoàn thiện/kiểm chứng để chưa commit, không push/deploy. Compose gốc giữ commit và checksum log sau bàn giao; generated evidence ignored, symlink tooling chỉ dùng cục bộ. Task10–13 chưa triển khai.

### Task10 nền → Task11 — Rust native PNG — 2026-11-04

- Người dùng chọn làm nền Task10 trước Task11, ưu tiên native khi gate tốc độ WASM chưa đạt; xác nhận ngưỡng ngoài text có pixel Δ>8 dưới 0,5%, text xem ảnh đối chiếu. Giữ Chromium mặc định, không làm Task12 hoặc deploy app thật.
- Giao worker khảo sát fresh để đối chiếu handoff cũ với text/paint hiện tại và đề xuất ranh giới module/lát triển khai. Giữ nguyên phần Task09 chưa commit; không push hoặc thay dữ liệu/runtime thật.
- Nhận khảo sát: phải giữ typography và paint hiện tại, không dùng adapter spike bỏ mất thuộc tính; startup fingerprint cũng phải bỏ Node cho đường Rust. Người dùng đồng ý diagnostic MISSING_GLYPH/.notdef không fallback hệ thống. Tạo nhánh `feat/task11-native-png` giữ nguyên Task09 dirty; cập nhật chốt handoff và giao hai worker độc lập cho text foundation và synchronous layout.
- Giao worker riêng tạo oracle Chromium theo semantics hiện tại và font bytes xác định; giao worker render crate native thuần. Các lane có ownership riêng, kiểm nặng giới hạn và không dùng dữ liệu thật; mọi kết quả parity/paint chưa được coi đạt trước khi tích hợp/kiểm độc lập.
- Giao lát startup/config riêng: cờ backend và fingerprint native không gọi Node từ constructor; chưa nối runner Rust trong lát này, không coi cờ cấu hình là bằng chứng render native đã hoạt động.
- Nhận layout provider đồng bộ giữ chung engine/cache và mã lỗi; worker báo 41 test đạt. Nhận startup/config backend opt-in, fingerprint Rust không probe Node; worker báo 16 test/startup đạt. Root đang kiểm độc lập; chưa có runner native.
- Nhận oracle Chromium pin với font Noto/Inter/Lato và SF có trak: 3.422 request, 56 trường hợp loại vì glyph/fallback; repeat ổn định. Đây là chuẩn đối chiếu, chưa phải parity text native.
- Nhận text foundation thuần dùng chung positioned runs, WOFF2/hash và policy glyph đã chốt; worker báo 16 test/lint/fmt/WASM build đạt. Root tự kiểm lại layout 6 và startup/config 16 test đạt, giao worker fresh đối chiếu text native với oracle và chỉ sửa mismatch có chứng cứ.
- Giao lát nối compute native vào jobs theo provider đã chốt, nạp font từ store và giữ states/diagnostics/readiness. Đây là bước nền Task10; đường PNG còn Chromium cho tới khi render crate bàn giao, không nhận nghiệm thu Task11 sớm.
- Sau compact, người dùng xác nhận việc hủy worker chỉ để thu gọn context và yêu cầu chạy tiếp. Nối lại đúng ba phiên render crate, parity text và compute jobs từ mã dở; không giao trùng ownership hoặc bỏ thay đổi. Chưa có kết quả nghiệm thu mới, không deploy/push.
- Nhận F2: compute trong Rust trước khi khởi động worker, giữ scene-ready khi strict PNG lỗi và drain blocking task sau timeout. Worker báo unit2, integration3, Chromium2 và lint/fmt đạt; root đã đọc seam và đang chạy lại đúng nhóm kiểm. PNG/export vẫn tạm dùng Chromium, chờ render crate để giao F3.
- Root chạy lại F2 unit2/integration3 và Chromium render2 đều đạt (helper test cũ có warning dead_code); chưa có bằng chứng no-Node PNG. Giao worker fresh làm fixture/đối chiếu paint trong spike riêng, không sửa render/text/server đang có chủ sở hữu.
- Nhận render crate: worker báo unit28/lint/WASM và native↔WASM đạt; root chạy độc lập unit28 đạt. Chromium pixel gate chưa chạy. Giao F3 fresh nối render/export native và giữ default Chromium, readiness, provenance, sizing; cần kiểm riêng scale thập phân vì API render hiện dùng f32.
- Nhận G2: geometry exact trên corpus chính và probes, 56 exclusions chỉ glyph thiếu; ink còn khác, đặc biệt dấu kết hợp trong Inter trộn Greek/Latin. Root chạy lại comparator trước khi chốt và giao lát fresh sửa itemization/mark attachment có bằng chứng, không nới gate hoặc đổi API text.
- Root chạy lại hai comparator G2: geometry 3.366/3.366 và 2.160/2.160 khớp tuyệt đối, lỗi/metadata mismatch đều 0; xác nhận ink lớn nhất 3,7978/6,609375 px còn là gap thật. Các số này chưa chứng minh PNG hoặc full-scene bounds.
- Nhận G3: 88 captures có 47 PASS, 28 FAIL ngoài text và 13 REVIEW_TEXT; ngưỡng/mask không đổi, ICC chưa có fixture. Root xem strip shadow/ảnh/stroke/text, giao fresh worker sửa paint theo bằng chứng; F3 tiếp tục server và chỉ sở hữu API/scale/readiness để tránh đụng module paint.
- Giao kiểm full-scene layout hai backend trên fixture repo/font upload và viewport responsive trong test riêng; comparator text không thay thế AC bounds mọi node. Test không sửa server/native helper do F3 đang sở hữu.
- Nhận H1: đã sửa attachment Inter bằng itemization, giữ geometry corpus cũ; stress mới còn 120 lỗi tracking dấu khác script và 11 lỗi tích lũy dòng dài. Root chạy lại stress ở output riêng và giao fresh H2 sửa đúng các nhóm đã phát hiện; không mở rộng ngoại lệ hoặc tiếp tục sinh stress vô hạn.
- Root xác nhận độc lập H1 stress: 5.089/5.220 eligible exact, 131 mismatch, 72 exclusions glyph thiếu, không lỗi native/metadata. Ink/height lệch lớn do vị trí dấu và wrap vẫn chặn nghiệm thu; H2 đang xử lý đúng nhóm lỗi này.
- Worker paint chạm usage limit trước bàn giao; người dùng yêu cầu tiếp tục. Root giữ mã dở (có module coverage/gradient/image_sample mới, chưa có báo cáo hay lượt parity mới) và giao fresh worker tiếp quản đúng scope. Kết quả 47/28/13 vẫn là baseline, không coi sửa dở đã đạt.
- F3 đã dừng sửa và xác nhận giữ nguyên paint work-in-progress. Root đọc native dispatch/export/deadline/provenance, bắt đầu kiểm độc lập native binary/PNG/export/unsupported và default Chromium; chưa ghi pass trước kết quả chạy.
- Root kiểm F3: 3 native unit, 9 native integration và 6 default Chromium render/export đều đạt. Binary Rust thật với PATH rỗng, workspace không Node/layout-step tạo design, tải PNG và export PNG+SVG thành công; scale/target sizing, readiness và cache bytes đã kiểm. Nhận L1 16 cặp scene/296 node geometry exact, ink còn lệch nhỏ; root chưa chạy lại L1 sau các sửa text đang làm.
- Cập nhật hướng dẫn opt-in/provenance và gắn rõ trạng thái experimental chưa đạt pixel. Giao reviewer fresh chỉ đọc phần tích hợp server đã ổn định; không review mã paint/text đang sửa như thể đó là bản cuối.
- Nhận H2 sửa toàn bộ 131 geometry failures trên stress cũ, giữ corpus chính/probes và API dùng chung. Root đang chạy lại bốn comparator, text/captured regressions và full-scene layout. Giao kiểm ICC độc lập bằng fixture tổng hợp có nguồn rõ trong spike riêng để lấp gap chưa được Chromium đối chứng; không sửa painter đang có chủ sở hữu.
- Root chạy lại H2: 11.166 eligible phép đo exact, 31 text test và 3 captured regression đạt; full-scene layout chạy riêng đạt sau khi lượt ghép trước hết timeout tổng. Probe bổ sung Inter variable weight400/700/900 phát hiện 4/6 lệch do trục weight bị bỏ qua (max width 42,390625 px). Giao fresh worker sửa lựa chọn instance chung đo/vẽ, không coi corpus mặc định400 là đủ static-face/variable coverage.
- Nhận P2: parity lên 71 PASS/4 FAIL ngoài text/13 REVIEW_TEXT; ảnh và gradient đều đạt trên matrix, source ổn định. Còn 4 ca stroke/clip/shadow 1× vượt ngưỡng, giao fresh P3 khảo sát raster/coverage có giới hạn; không thay fixture, mask hoặc tolerance. Unit/lint/WASM build của P2 chưa thay cho execution parity trên painter mới.
- Nhận review server R1 có 3 Required: SVG-only bị chặn theo khả năng raster (COLRv1 hợp lệ), PNG có thể publish sau deadline, và provenance PNG thiếu scale. Root đọc bằng chứng và giao fresh S1 sửa + regression; không coi test cũ mong SVG fail là thay đổi AC được duyệt.
- Nhận I1: 13/14 ICC captures đạt, RGB/gray + alpha max Δ4; CMYK ICC lệch toàn bộ fixture, native khớp reference tự tính còn Chromium khác nên chưa kết luận lỗi phía nào. Giao fresh I2 kiểm tính hợp lệ/PCS và chỉ sửa CMS nếu có căn cứ; profile hỏng vẫn là probe riêng, không tính vào pass.

### Đồng bộ PR #6 với main — 2026-11-04

- Merge `main` (`c2bf260`, hover animation trigger) vào `codex/compose-postgres`. Xung đột ở `capabilities.rs`, `implement-frame.md` và nhật ký: giữ `restrictions.layout` responsive và `restrictions.animations` hover dạng khóa riêng, giữ bullet overflow mới cùng hướng dẫn hover theo `triggerNodeId`, đặt các mục hover của main trước mục nhánh.

- Các mục đã đóng All frames, dựng bản test riêng và progressive cache: [archive](history/2026-11-09-closed-viewer.md).

### Stress All frames 10.000 nodes/page — 2026-11-06

- Người dùng yêu cầu page cao khác nhau, mỗi page10.000nodes. Giao fresh worker tạo stress harness và fixture thật qua API trong data root/server tạm; một project/page để giữ cap20.000/project,9frames/page để giữ cap2.048/frame, thêm probe10.000nodes trong một frame. Không tăng giới hạn hoặc chạm runtime native thật.

### Commit và push viewer All/progressive cache — 2026-11-06

- Người dùng đã tự thử bản viewer riêng và chấp nhận, yêu cầu “ổn áp đấy, push đi”. Cho phép commit/push phần All frames và progressive IndexedDB cache trên `feat/viewer-wasm-scene`; giữ riêng stress harness của phiên khác.
- Dùng kết quả typecheck/production build đã đạt cùng kiểm trước commit bằng Git; không chạy lại tests/benchmark, deploy hoặc chạm runtime/DB. Chuẩn bị một commit và push thường sau khi đối chiếu nhánh remote; chưa ghi push thành công trước khi xác minh.

### Viewer FE performance — 2026-11-09

- Root giao khảo sát canvas chỉ đọc và audit toàn frontend, sau đó ba worker độc lập: canvas/prepared tree và viewport culling, React/indexed Layers và ranh giới state, HTTP ETag/snapshot lifecycle. Giữ source của phiên stress riêng, không tạo campaign benchmark hoặc triển khai app thật.
- Nhận cả ba: bỏ index scene mỗi paint, culling bảo thủ, props/commands ổn định và index node dùng chung; revalidation ETag giữ auth/privacy và lifecycle. Các kiểm thuần lần lượt17/17,19/19,10/10 đạt; worker typecheck và root build tổng hợp/diff-check đạt. Source/test được giao tối đa233dòng. [Báo cáo, lệnh kiểm và giới hạn](../VIEWER-FE-PERFORMANCE.md).
- Chromium launch bị MachPort1100/SIGTRAP, dừng retry; chưa đo FPS/timing hoặc xác nhận pixel parity. Giữ kiến trúc state hiện có sau rà ownership; không ép reducer/store chung.
- Chỉ đồng bộ generated static assets vào bản test3867, giữ hash cũ, thay index nguyên tử sau cùng; HTTP index/bundle khớp byte. App17418/PG17389 và native23980 giữ thời điểm khởi động, nguồnPG2703 vẫn nghe; không restart/đổi launchd/Funnel, kiểm DB, render job, commit/push. Để app+PG test chạy cho người dùng tự thử.
- Người dùng yêu cầu “push code đi”, cho phép commit và push thường phần tối ưu FE trên `feat/viewer-wasm-scene`. Dùng kết quả kiểm đã ghi, kiểm phạm vi/whitespace trước commit và đối chiếu remote; giữ riêng stress harness, không đổi runtime/DB hoặc generated assets. Chưa ghi push thành công trước khi xác minh remote.

### Native runtime và backup PostgreSQL — 2026-11-06

- Người dùng yêu cầu chuyển Docker sang service native và PostgreSQL native, ưu tiên backup database. Host hiện tại là macOS; đang làm rõ máy Linux/systemd hay Mac/launchd và nơi lưu backup ngoài máy.
- Giao worker khảo sát chỉ đọc phạm vi dữ liệu PostgreSQL, runtime và cách kiểm restore; chưa thay service, dữ liệu hoặc container thật. Giữ nguyên file nghiên cứu Graphite ngoài phạm vi.
- Người dùng chốt chạy ngay trên Mac bằng launchd, backup local trước và giữ 14 bản. Giao hai task độc lập: bộ backup/restore có kiểm thử, và chuẩn bị runtime native từ đúng commit image đang chạy (`727cd33`) cùng rehearsal restore; root kiểm trước cutover.
- Người dùng chốt PostgreSQL native cổng `2703`; MCP/public giữ `3857/3858`, Funnel giữ nguyên. Homebrew PostgreSQL 16.15 đã có; dùng cluster riêng, không thay cluster Homebrew hiện hữu.
- Root kiểm độc lập bộ backup trên cluster PostgreSQL 16 tạm: restore cả bytea/history/auth/sequence/large object, phát hiện hỏng archive/fingerprint, failed dump không prune, giữ 14 bản hoàn tất và lịch local-day đều đạt. Runtime native đang chuẩn bị; chưa cutover.
- Nhận bộ backup và review độc lập: hai lỗi cleanup khi nhận TERM đã sửa, root chạy lại test TERM/collision đạt và ShellCheck sạch. Giao worker nhỏ bổ sung runbook recovery để app và lịch backup cùng chuyển sang database khôi phục.
- Rehearsal native đạt đối chiếu toàn DB; root kiểm riêng cả 5 snapshot, ETag304, asset WASM/JS đúng image, quyền file và public auth/read-only. Đang kiểm lifecycle/render trước chuyển cổng thật.
- Nhận runbook recovery đã sửa đồng bộ cấu hình app/backup; lifecycle graceful và crash khi worker đang chạy đạt, không orphan. Root duyệt cutover trong phạm vi yêu cầu setup native.
- Hoàn tất chuyển native: root xác nhận PostgreSQL `2703` chỉ loopback, app `3857/3858` do launchd quản lý, 5 checksum/revision giữ nguyên và HTTPS Funnel hoạt động. Backup đầu tiên đã restore kiểm chứng, checksum đạt; lịch 03:00 đã nạp và lần chạy đầu exit 0, không còn scratch DB. Hai container Docker dừng/restart=no, giữ volume và dump nguồn riêng để khôi phục.
- Điểm dừng: [runtime](../NATIVE-RUNTIME.md), [backup/recovery](../NATIVE-BACKUP.md); LaunchAgents cần user đăng nhập. Chưa commit/push; file nghiên cứu Graphite của người khác giữ nguyên. Native là nguồn dữ liệu hiện tại, không quay lại database Docker cũ sau khi đã nhận thay đổi.

### Domain public viewer — 2026-11-06

- Người dùng yêu cầu duyệt domain custom cho cổng 3858, ưu tiên giữ cả domain Funnel. Bản hiện tại chỉ nhận một HTTPS origin; giao worker bổ sung allowlist tường minh, kiểm auth theo từng domain và chuẩn bị cập nhật riêng native app, giữ PostgreSQL/backup.
- Review độc lập đã đóng lỗi parser port; root chạy riêng 3 integration test allowlist/auth/export đạt. Domain mới chưa có DNS A/AAAA/CNAME public; vẫn kiểm được approval bằng Host/Origin trên loopback. Đang dựng canary từ đúng nền native 727cd33 trước khi thay riêng app.
- Canary clone đạt MCP/PNG/WASM và auth theo từng domain; root kiểm riêng 14 trường hợp Host/Origin/local isolation cùng 5 snapshot, toàn DB live không đổi. Đã duyệt thay riêng native app/config, giữ PostgreSQL, backup và Funnel.
- Người dùng đổi domain mới sang subdomain custom, yêu cầu gỡ apex khỏi allowlist và giữ Funnel. Subdomain có DNS A/AAAA Cloudflare, nhưng curl và Node HTTPS đều lỗi TLS handshake; đường Funnel vẫn 200/readOnly. Đang nạp delta cấu hình app, không sửa DNS/proxy.
- Hoàn tất cấu hình subdomain: root kiểm 12 trường hợp trên 3858 đạt, apex bị từ chối, hai domain được nhận và cross-origin bị chặn; toàn DB/static assets và 5 revision giữ nguyên. TLS subdomain sau đó hoạt động lại; root kiểm HTTPS cả subdomain mới và Funnel đều 200/readOnly, không tắt xác minh chứng chỉ. Lịch backup giữ nguyên, đang dọn clone thử và chốt runbook.
- Điểm dừng: canary/clone đã dọn, native app/PG/backup giữ hoạt động; cả hai HTTPS đã kiểm auth/read-only. Bản binary/config cũ giữ để rollback riêng app; runbook ở [native runtime](../NATIVE-RUNTIME.md#public-viewer-domain-allowlist). Không commit/push.

### Domain lấy từ ENV — 2026-11-06

- Người dùng yêu cầu không hardcode domain và gỡ commit chứa domain nếu agent đã tạo. Kiểm xác nhận chưa có commit mới; runtime đã dùng ENV nhưng installer và fixture/ví dụ còn tên host thật. Giao worker bỏ giá trị thật khỏi các file triển khai/test/tài liệu vừa tạo, giữ cấu hình riêng và dịch vụ đang chạy.
- Hoàn tất: installer bắt buộc primary ENV và nhận optional allowlist ENV; test/ví dụ và ghi chú tiến độ dùng host mẫu hoặc tên vai trò. Worker kiểm 2 unit + 6 integration đạt, fmt/ShellCheck sạch; root kiểm độc lập thiếu/empty ENV dừng trước tạo root, scan hostname thật trong phần vừa sửa không còn kết quả.
- Điểm dừng: domain thật chỉ giữ trong cấu hình runtime riêng, không reload app/PG/backup. HEAD vẫn là commit trước phiên này, không có commit/staged change cần xóa và không rewrite history; chưa commit/push.

### Khảo sát hai agent cùng design — 2026-11-09

- Đọc cơ chế batch SQLite/PostgreSQL, conflict history, tài nguyên dùng chung và bundle: hiện revision/commit guard ở cấp project; bundle SQLite chưa thuộc hợp đồng StorageBackend dùng chung.
- Đề xuất để người dùng cân nhắc: giữ project revision cho snapshot/events/cache, bổ sung precondition theo page và dependency để giảm conflict khác page; clone phù hợp thử phương án độc lập nhưng cần xử lý dependency/ID và kiểm bản gốc khi nhập về.
- Điểm dừng: mới khảo sát và tư vấn, chưa chốt hướng hoặc giao triển khai; không sửa code, dữ liệu thiết kế hay runtime, không chạy test vì không đổi hành vi.
- Người dùng chốt triển khai revision theo page và dependency trước, do hai phiên design đang bị chặn. Giao một worker triển khai lát dọc storage SQLite/PostgreSQL → MCP và targeted tests; giữ global revision và strict legacy semantics, không deploy/restart dịch vụ thật.
- Chốt phạm vi kỹ thuật với worker: guard theo page + dependency tự suy ra, dấu phạm vi ghi bền trong commit envelope có version; thiếu lịch sử/metadata thì từ chối an toàn. Giao reviewer chỉ đọc rà dependency, side effects và replay/bundle để bổ sung ca đối kháng trong khi triển khai.
- Nhận audit kiến trúc: yêu cầu bổ sung dependency font qua instance override/theme, kiểm envelope hỏng, side effects điều hướng/notes và replay sau commit với base cũ. Worker đang xử lý trong cùng task; chưa nghiệm thu hoặc cập nhật runtime.
- Nhận source ổn định và review mã độc lập: không còn P1/P2; sửa ca append root sau sort-key gaps và bổ sung wildcard cho thay đổi điều hướng toàn project. Root kiểm riêng SQLite, MCP/HTTP và PostgreSQL tạm đạt; kiểm lại 8 scoped/history/navigation/bundle test sau sửa cuối đạt, không chạm DB thật.
- Do checkout cũ hơn runtime đang chạy, giao worker mới chuẩn bị candidate riêng từ đúng binary/provenance live và chỉ ghép delta tính năng. Chưa cho phép kích hoạt; candidate phải giữ renderer/artifacts hiện tại và kiểm canary trên PG/cổng tạm trước bước đồng ý cập nhật runtime.

### Kiểm tra All frames trên public — 2026-11-09

- Kiểm chỉ đọc runtime: HTML trên 3857 và 3858 với Host hợp lệ giống hệt, cùng bundle `index-CZExainG.js`; bundle public có All frames. Nhật ký worktree viewer cũng ghi nhận frontend native đã được cập nhật.
- Chưa xác minh được đường HTTPS primary do không kết nối được cổng 443 từ môi trường kiểm. Không đổi source ứng dụng, runtime, cấu hình hoặc dữ liệu; chưa kết luận nguyên nhân giao diện người dùng thiếu option.
- Người dùng đính chính public qua Cloudflare Tunnel. Kiểm HTTPS domain alternate thành công: HTML thường và thêm query đều `CF-Cache-Status: DYNAMIC`, tham chiếu `index-CZExainG.js`; bundle trả 200/MISS và chứa All frames, có browser cache TTL 14.400 giây. Không có bằng chứng edge đang trả frontend cũ tại lượt kiểm; chưa kiểm cache/tab trong browser người dùng. Đường Funnel primary là cấu hình cũ, không đại diện public đang dùng.

### Sửa cache trình duyệt viewer — 2026-11-09

- Người dùng xác nhận ẩn danh có All frames và yêu cầu sửa. Giao fresh worker xử lý chính sách cache static dùng chung local/public, thêm kiểm hồi quy bằng workspace/data root tạm và báo cáo cách đưa delta vào đúng nền native hiện tại.
- Phạm vi: HTML/SPA entry và static URL thay đổi nội dung phải kiểm lại; giữ cache hiệu quả cho bundle có content hash và giữ API/auth/privacy. Chưa cho phép deploy/restart runtime thật; không commit/push hoặc xóa dữ liệu trình duyệt.
- Giao fresh worker vận hành chuẩn bị candidate từ đúng provenance release native đang chạy, chỉ thêm delta cache sau khi source ổn định và kiểm canary bằng SQLite/workspace tạm; không kích hoạt dịch vụ thật hoặc dùng database người dùng.
- Nhận source: module static riêng, HTML/stable URLs `no-store`, bundle Vite có hash `immutable`; giữ Brotli/gzip/Vary và API cache/auth/privacy. Worker kiểm cache + public CLI 6/6, scoped Clippy/rustfmt/diff-check đạt; root chạy riêng cache 3/3 đạt. Mã mới/sửa tối đa196 dòng. [Chính sách và cách tích hợp](../VIEWER-CACHE-POLICY.md).
- Candidate native từ đúng release live đã build; 12 test cache/auth/privacy/CLI trên nền này đạt, payload ngoài server giữ byte-for-byte. Canary nền cũ tái hiện stale304; candidate trả HTML mới với cùng mtime. Root chạy độc lập canary hai listener đạt cache/compression/MCP/auth/privacy và xác nhận release thật vẫn khớp baseline; canary/dữ liệu tạm đã dọn.
- Điểm dừng: code và candidate đã kiểm, chưa kích hoạt native/đổi cấu hình/commit/push. Giữ yêu cầu người dùng trước đây về không tự deploy app thật; cần đồng ý rõ ràng trước bước kích hoạt riêng bản cache đã kiểm. Cache trình duyệt cũ có thể cần hard reload một lần trước khi nhận chính sách mới.
- Candidate, fingerprint và cách kích hoạt/rollback: [báo cáo native cache](../VIEWER-CACHE-CANDIDATE.md). Root đối chiếu độc lập manifest giữ nguyên566 payload và hash server/patch đạt; AGENTS.md còn trong ngân sách.

### Hoàn thiện revision theo page — 2026-11-09

- Người dùng yêu cầu hoàn thiện nốt phiên Codex. Source scoped concurrency và candidate riêng đã xong; hai candidate (page concurrency, cache viewer) cùng nền live nhưng tách rời, cài một bản sẽ mất bản kia.
- Kiểm lại checkout chính: fmt sạch, storage scoped/bundle 11 và server liên quan 19 test đạt. Lỗi có sẵn ngoài phạm vi: `stroke_props` (Instance có `stroke`) và lint Clippy `--all-targets` ở test không sửa.
- Ghép một candidate trên nền live: delta không chồng file, `COMBINED_PATCH` áp lại khớp. PG tạm: storage 9 + server 26 test, smoke hai backend render và canary cache đều đạt; 566 payload ngoài binary giữ nguyên live.
- Điểm dừng: [candidate ghép](../NATIVE-COMBINED-CANDIDATE.md), chưa kích hoạt, chưa commit/push. Cần backup ngay trước khi cài vì binary cũ không đọc envelope lịch sử mới.
- Người dùng yêu cầu deploy native và backup. Backup thủ công `backup-20261109T052151Z` restore kiểm chứng đạt; cài release `+scope-cache.f271654aad80`, chỉ đổi `OPENCANVAS_RELEASE` (giữ bản `server.env` cũ), restart riêng server. Sau cài: 5 snapshot giống hệt, MCP có `pageIds`/`get_edit_context`, PNG/WASM đạt, HTML `no-store`, bundle hash `immutable`, public 401/403 đúng, HTTPS Cloudflare `DYNAMIC`. Các phiên design cần kết nối lại MCP; trình duyệt có thể cần hard reload một lần.

### Sửa React runtime sau deploy native — 2026-11-09

- Production bundle từ checkout main với dependencies được copy chứa hai React dispatcher: hooks của app và renderer nạp hai đường dẫn khác nhau, cùng phiên bản. Thêm `resolve.dedupe` cho `react`/`react-dom` ở Vite; không đổi component, hooks hoặc quyền sở hữu state.
- Typecheck/build và audit module graph đạt: một React runtime, bundle trở lại `index-CZExainG.js`. Root tái hiện browser trắng trên native, kiểm bản staging rồi kiểm lại native thật: `All frames · 3`, `Scene r10` hiển thị. Chuyển release frontend riêng, giữ binary/overlay native, assets cũ và rollback; không đổi PostgreSQL, auth, domain hoặc bản test 3867. Chưa commit/push sửa này.

### Native deploy latest main — 2026-11-09

- Theo yêu cầu mới của người dùng, fetch `origin/main` tại `4905dd5` (chứa `97f0f5e`), build đồng bộ server/layout release, renderer, WASM và web trong checkout/target riêng. Giữ nguyên compatibility overlay allowlist đang cài (`33bbe9f75b87`); không commit/push hoặc sửa các checkout khác.
- Backup restore-verified, canary PostgreSQL riêng và kiểm MCP/PNG/WASM/auth đạt trước app-only cutover. Sau deploy, năm snapshot và fingerprint đầy đủ của database giữ nguyên; PostgreSQL và phiên test khác giữ nguyên. Primary HTTPS route từ máy trả connection refused; không đổi proxy/Funnel. Chi tiết và rollback ở báo cáo riêng `outputs/native-main-deploy.md` của chat vận hành.

### Gộp nhánh và dọn nhánh — 2026-11-09

- Người dùng yêu cầu làm FE trên nhánh hiện tại thay vì worktree từ `origin/main`; dừng subagent cũ. Commit công việc chưa commit thành 4 commit (allowlist domain, cache viewer, revision theo page, runtime native), merge `origin/main`, thêm `resolve.dedupe` React; `crates/`/`web/` khớp source bản native đang chạy. Sau merge: typecheck web, 82 test web, 11 storage + 29 server test đạt. Lệch rustfmt ở `crates/render` có sẵn từ main.
- Theo yêu cầu chỉ giữ `main` và nhánh task09: đưa harness stress All frames + nhật ký từ worktree cũ vào nhánh, tag `archive/pg-delta-redis-hot-reads` cho spike Redis, gỡ worktree `OpenCanvas-viewer-wasm-scene`/`OpenCanvas-frame-cache`, xóa 12 nhánh local (nội dung đã có trong nhánh, xét theo patch), đổi tên thành `feat/page-concurrency`, fast-forward `main`. Chỉ local: không push, không xóa nhánh remote. Worktree Codex detached `native-main-20261109` giữ nguyên (không phải nhánh).
