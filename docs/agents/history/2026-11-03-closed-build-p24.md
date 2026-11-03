# Nhật ký đã đóng — build, design memory và P24

Chuyển nguyên văn từ LOG.md ngày 2026-11-03 để giữ ngân sách nhật ký.

### Commit và push viewer/design memory — 2026-10-29

- Người dùng cho phép commit/push nhánh hiện tại và yêu cầu ghi nhận contributor Codex. Chốt phạm vi viewer actions/custom select/export, nghiên cứu design rules và English design-memory instructions đã kiểm; commit có trailer `Co-authored-by: OpenAI Codex <noreply@openai.com>`. Giữ các ghi chú LOG đã có từ phiên khác ngoài commit, không sửa author Git hoặc lịch sử cũ. Diff whitespace sạch; các kiểm chứng và release build đã đạt ở hai mục task dưới đây.

### Server instruction cho design memory — 2026-10-29

- Người dùng chốt thêm instruction bằng tiếng Anh: `AGENTS.md` riêng theo project là memory cho đọc/chỉnh sửa design, không dùng cho code/build/test hoặc công việc khác. Giao fresh worker sửa instruction và tài liệu MCP trong phạm vi nhỏ; giữ các thay đổi viewer đang có, không commit/push hoặc chạm dữ liệu/server thật.
- Nhận kết quả: CONNECT và hai skill MCP có policy English đọc/tạo/cập nhật memory từ quyết định design đã xác nhận, ưu tiên memory project trong phạm vi design. Không bịa công cụ memory MCP; khi không có authorized file access thì báo limitation. Worker kiểm MCP skill_layer/protocol_2026 4/4, rustfmt/diff sạch; ba file 86/168/157 dòng. Root đã đọc đối chiếu, đang kiểm độc lập và build release.
- Điểm dừng: root chạy độc lập skill_layer/protocol_2026 4/4 đạt; release build thành công. `AGENTS.md` 64 dòng/7.861 byte, mọi file sửa task này dưới250 dòng. Chưa commit/push hoặc restart server3847; cần restart server và reconnect MCP để client nhận instruction mới.

### Build toàn bộ app và launch scripts — 2026-10-29

- Người dùng yêu cầu root `package.json`: `build` toàn app debug, `build:release` toàn app release, `dev` chạy kết quả build debug và `start` chạy kết quả build release. Không dùng dev server FE cho hai lệnh chạy này.
- Giao fresh worker thêm scripts và runtime profile wiring cần thiết (layout worker/fingerprint đang mặc định debug), cập nhật README và kiểm debug/release trên server/data tạm. Giữ nhánh P24 hiện tại, không push hoặc chạm server/data thật.
- Nhận kết quả: 4 scripts build/run toàn app, launcher pin server/layout cùng profile và fingerprint băm layout được chọn. Worker chạy đủ hai build, hai live smoke, CLI/error/shutdown probes và Clippy/format đạt; chi tiết ở `docs/APP-BUILD-REPORT.md`.
- Root kiểm độc lập `dev` và `start` với target tạm chỉ có đúng profile: compiled viewer/assets HTTP200, MCP text → PNG320×200 đạt, data/process tạm đã dọn. Dọn symlink dependency QA, commit trên nhánh hiện tại, không push.

### P24 merge main — 2026-10-29

- Fetch `origin/main` @ `f901a31` (merge P27); rebase gặp conflicts. Theo yêu cầu mới của người dùng, đã abort rebase hoàn toàn, giữ commit P24 `236d116`, chuyển sang merge `origin/main` trên nhánh `codex/p24-components-instances`; không rebase tiếp.
- Merge có 5 content conflicts; giao fresh worker xử lý và kiểm integration P24/P27, giữ cả instance và interaction states. Root điều phối/kiểm chứng; không push hoặc chạm server/data thật.
- Nhận kết quả: giữ schema4/instance và P27 states; thêm regression và sửa `resolvedStates` trong component definitions, bảo toàn `visible:false` khi state đổi opacity. Root kiểm độc lập server44/44 và storage16/16; worker strict library Clippy/fmt đạt. Báo cáo: `docs/P24-MERGE-REPORT.md`.
- Chốt merge `f901a31` vào nhánh P24, giữ lịch sử commit `236d116`, dọn 4 symlink dependency tạm, tạo merge commit; không rebase hoặc push.

### P24 — 2026-10-29

- Người dùng yêu cầu triển khai P24 sau khi lấy main mới, trên nhánh riêng. Đã fetch `origin/main` @ `a0d7743`, tạo worktree riêng và nhánh `codex/p24-components-instances`; giữ nguyên checkout P27 có thay đổi đang làm.
- Giao fresh worker triển khai P24a+b theo `docs/recommend-features/tasks/P24-components-instances.md`, gồm storage/expand, MCP reads và kiểm AC. Root điều phối và kiểm chứng; không push, không chạm dữ liệu/server thật.
- Worker báo MCP/Chromium kiểm pixel, master invalidation, byte budgets và virtual reads đạt; root storage instances 4/4 pass, migration 2/4 còn assertion cũ schema3 cần cập nhật4 (base main đã dùng3 cho notes). Baseline MCP context6900 nodes đã đo trên main. Worker bị ngắt do model capacity, đã yêu cầu tiếp tục cùng task để hoàn tất kiểm và report.
- Root kiểm độc lập MCP/Chromium P24 3/3 pass. Worker tiếp tục bị capacity lần hai; giao fresh worker hoàn tất regression/migration/docs/lint/report từ diff hiện có, giữ nguyên nhánh/worktree.
- Nhận kết quả hoàn tất: `docs/P24-REPORT.md` và `artifacts/p24/` ghi đủ AC, schema4/legacy compatibility, pixel equality, byte budgets và hai workload latency. Root kiểm độc lập 22 storage tests (qua các lượt), MCP cuối 4/4, fmt/diff và benchmark mở rộng thực 6.900 descendants đạt; worker kiểm thêm server liên quan 19/19, unit/golden 35/35 và strict library Clippy.
- Chốt nghiệm thu P24 trên `codex/p24-components-instances`, lưu benchmark tái lập trong `spikes/p24/context-benchmark.mjs`; dọn các symlink dependency tạm, commit nhánh riêng, không push. Không đổi main, checkout P27 hoặc server/data thật.
