# Nhật ký điều phối OpenCanvas

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

### Xuất bản nhánh Docker Compose PostgreSQL — 2026-11-03

- Người dùng yêu cầu commit và push nhánh mới. Giao worker tách cấu hình/runbook Compose và nhật ký liên quan từ `codex/kubernetes-local`; giữ các ghi chú khảo sát khác ở working tree.

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
