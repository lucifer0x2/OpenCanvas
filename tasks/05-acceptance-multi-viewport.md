# Task 05 — Nghiệm thu đa viewport: thôi thưởng pixel-perfect một kích thước

**Trạng thái:** hoàn tất qua Codex CLI, root kiểm độc lập15 nhóm E2E và3 bounds tests (2026-10-28); xem `docs/TASK05-REPORT.md`.
**Phụ thuộc:** Task 02 (cần frame stacks để có cái mà kiểm reflow).
**Phạm vi sở hữu:** `spikes/e2e/`, `artifacts/e2e/`, `docs/ACCEPTANCE.md`, npm script `test:e2e`.
Không sửa `crates/`, không sửa `web/`.
**Workspace:** `/Users/luis/Workspace/OpenCanvas`, branch `main`.

## Bối cảnh

Đây là phần dễ bỏ qua nhất nhưng quyết định hành vi của mọi agent về sau.

Thước đo nghiệm thu hiện tại thưởng đúng một thứ: pixel diff bằng 0 tại một viewport.
Nhật ký `AGENTS.md` cũ (nay ở `docs/agents/history/2026-10-28.md`) ghi kết quả đạt là
"2.995.200 RGBA channels, 0 sai khác", và `docs/ACCEPTANCE.md`
ghi "exact RGBA 0 sai khác desktop/mobile/restart".

Nhưng với frame `free`, "0 sai khác ở mobile" **không** chứng minh thiết kế chạy được trên
mobile. Nó chỉ chứng minh renderer cắt khung một cách tất định: node ở `x=744` vẫn ở `x=744`,
chỉ là nằm ngoài khung 390 và bị `clip:true` cắt đi. Hai lượt render khớp nhau tuyệt đối trong
khi nửa giao diện biến mất.

Còn giữ thước đo này thì agent sẽ luôn chọn absolute positioning — đó là cách duy nhất đạt điểm
tuyệt đối. Sửa Task 01, 02, 03 mà không sửa Task 05 thì vòng phản hồi vẫn kéo mọi thứ về chỗ cũ.

## Mục tiêu

Bổ sung các kiểm mà một thiết kế cứng **không** vượt qua được, để "đạt" nghĩa là "dùng được",
không phải "render lại giống hệt".

## Việc phải làm

1. **Kiểm overflow ngang.** Ở mỗi viewport kiểm, so bounds mọi node con với khung frame. Node
   nằm ngoài khung phải được báo, kèm id và số px tràn. Hiện tượng này đang bị `clip` che và
   pixel diff không thấy.
2. **Kiểm reflow.** Render cùng frame ở ít nhất hai viewport. Nếu bounds mọi node con **không
   đổi**, báo rõ: frame này cố định, không phải responsive. Không coi là fail của renderer —
   đây là thông tin về thiết kế, và phải xuất hiện trong report.
3. **Giữ nguyên kiểm pixel hiện có.** Nó vẫn đúng việc của nó: chứng minh tính tất định của
   renderer. Chỉ là không được một mình nó định nghĩa "đạt".
4. **Cập nhật `docs/ACCEPTANCE.md`.** Ghi rõ "0 sai khác RGBA ở viewport hẹp" chứng minh điều gì
   và **không** chứng minh điều gì. Đây là chỗ sửa hiểu nhầm cho mọi phiên sau.

## Acceptance Criteria

- `npm run test:e2e` chạy kiểm overflow và kiểm reflow ở ít nhất hai viewport.
- Frame `free` cũ: kiểm pixel pass, kiểm overflow báo đúng các node tràn, kiểm reflow báo
  "không reflow". Report nêu cả ba.
- Frame stacks của Task 02: kiểm reflow báo có reflow, overflow sạch hoặc chỉ còn chỗ đã biết
  và được giải thích.
- `docs/ACCEPTANCE.md` nêu rõ giới hạn của thước đo pixel.
- Mọi file mã viết tay ≤250 dòng.

## Cách kiểm

```bash
npm run test:e2e
```

Dùng data root tạm, dọn process và data sau khi chạy. Không để server thật chạy lại sau lượt
nghiệm thu. Không chạm cổng 3847 của người dùng.

E2E hiện là 12 nhóm kiểm — con số đó phải giữ hoặc tăng.

## Đầu ra

- Delta trong `spikes/e2e/`.
- `artifacts/e2e/` tái tạo được, có report hai viewport cho cả hai frame.
- `docs/ACCEPTANCE.md` cập nhật.
- Báo cáo: lệnh thật, số nhóm kiểm, và kết quả đối chiếu hai frame cạnh nhau.

## Ghi chú ngoài phạm vi repo

Bản HTML mà Codex xuất ra nằm ngoài repo này
(`~/Documents/Codex/2026-10-28/add-mcp-c-a-open-th/outputs/index.html`), nên không thuộc task nào
ở trên. Nếu cần dùng ngay trước khi Task 01–03 xong, có một cách chữa triệu chứng rẻ: bọc
`<main>` trong wrapper dùng `transform: scale()` + `aspect-ratio` để bản export vừa màn hình.
Nó không làm thiết kế responsive — chỉ thu nhỏ toàn bộ bức tranh — nhưng đủ để xem preview.
Ghi ở đây để khỏi quên, không phải việc của task này.

## Ngoài phạm vi

- Không sửa renderer hay layout engine để làm kiểm pass. Kiểm mới mà fail là thông tin, không
  phải lỗi cần giấu.
- Không thêm GC, pruning hay bất cứ thứ gì PLAN đã loại trừ.
