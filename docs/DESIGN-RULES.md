# Protocol thiết kế dùng chung cho hai phiên AI — bản nháp v1

Ngày: 2026-10-29. Áp dụng cho AI tạo/sửa hoặc đọc thiết kế OpenCanvas.
Đây là **quy ước agent**, chưa phải policy do backend enforce. Chỉ áp dụng sau khi
manifest của project đã được điền/chốt; không tự chọn palette hoặc font từ bản nháp.
Nền tảng và giới hạn: [nghiên cứu](DESIGN-RULES-RESEARCH.md),
[resolution](RESOLUTION.md), [components](COMPONENTS.md).

## 1. Một manifest chuẩn cho mỗi project

Chỉ có một bản authoritative do người dùng/đầu mối thiết kế chỉ định. Cả hai AI
nhận cùng nội dung hoặc cùng đường dẫn/ID. Mẫu dưới đây là hồ sơ làm việc, **không
phải** JSON schema của `apply_batch` hoặc field mới trong project:

```yaml
projectId: <ID đã xác minh bằng list_projects>
rulesVersion: <phiên bản quy tắc>
verifiedRevision: <revision project khi kiểm registry>
theme: <default hoặc tên đã khai báo>
rulesSource: <file được giao hoặc rulesFrameId + note IDs>
semanticTokens: <role -> token ID; background, text, action, border, status...>
spacingTokens: <role -> dimension token ID>
textStyles: <role -> style ID; heading, body, label, caption...>
fonts: <font ID, family, weight, style, content hash và fallback được chấp thuận>
components: <role -> master ID, allowed overrides, các biến thể được chấp thuận>
viewports: <width/height của các kích thước phải kiểm>
exceptions: <scope/property, lý do, người chốt>
systemOwner: <người/phiên được giao thay đổi tài nguyên chung>
```

ID/value trong project là dữ liệu chuẩn. Manifest chỉ định vai trò và phạm vi sử
dụng; nếu hai nguồn mâu thuẫn thì đọc lại và giải quyết trước thay đổi liên quan.
Không dùng `verifiedRevision` như kho snapshot cũ: API read chỉ có revision hiện tại.
Phiên mới phải kiểm registry ở revision mới nhất và cập nhật bằng chứng tương ứng.

Nếu registry chưa có vai trò cần thiết, gửi đề xuất cụ thể gồm tên, kiểu, giá trị,
theme, font/style và phạm vi ảnh hưởng cho đầu mối đã giao; không tự dựng system
mới trong một frame. Công việc độc lập với quyết định đó có thể tiếp tục.

## 2. Bootstrap bắt buộc ở mỗi phiên

1. Đọc manifest và ghi nhận projectId/rulesVersion/theme. Với MCP-only: đọc rules
   frame đã được giao bằng `get_design_context {projectId, frameId, notes:"all"}`;
   đọc đủ mọi trang/part. Không có `get_design_rules` tool hiện tại.
2. Đọc MCP connect instructions; qua `resources/read` đọc
   `skill://opencanvas/props.md` và `skill://opencanvas/author-frame.md` hoặc
   `skill://opencanvas/implement-frame.md`. Chỉ gọi khả năng server thực sự công bố.
3. `list_projects` để kiểm đúng project; `get_outline`/`find_nodes` để xác định frame.
   Không tạo project mới chỉ vì không nhớ ID. Không dùng dữ liệu demo vào project thật.
4. Đọc `list_tokens`, `list_styles`, `list_fonts` theo IDs đã ghi (mỗi lần tối đa
   100 IDs), và các tài nguyên cần khám phá. Lấy revision R từ response đầu; truyền
   `revision:R` cho các read tiếp, theo cursor đến hết. Nếu `REVISION_UNAVAILABLE`
   hoặc `INVALID_CURSOR`, bắt đầu lượt đọc mới; không ghép các revision khác nhau.
5. Đọc `get_design_context` cho frame công việc với cùng R/theme,
   `compact:true`, `instances:"reference"`, `notes:"all"` và viewport đã chốt.
   Đọc hết pagination; compact chỉ trả resources được tham chiếu, không thay thế
   bước đọc registry nếu cần chọn thêm token/style/component.
6. Kiểm font record/hash/status và dependency masters. Bắt đầu khi đã hiểu cả
   authored refs, resolved values, notes và các ngoại lệ liên quan.

Prompt mở phiên có thể dùng nguyên ý sau, điền thông tin thực:

> Làm project `<projectId>`, scope `<frameId>`. Đọc protocol và manifest
> `<nguồn chuẩn>` phiên bản `<rulesVersion>` trước khi ghi. Dùng token/style/font/
> master IDs đã chốt; không tự chọn palette/font hoặc sửa system chung ngoài
> phạm vi được giao. Đọc dữ liệu ở cùng revision/theme, báo gap và ngoại lệ.

File này không tự xuất hiện trong MCP resources. Người điều phối phải đưa con trỏ
vào cả hai phiên. Note rules nên giữ unresolved và ghi “quy tắc thường trực”; dùng
`notes:"all"` để không mất quy tắc nếu ai đó đã resolve. Notes ở page khác phải
đọc riêng qua rules frame; context frame công việc không tự kế thừa chúng.

## 3. Quy tắc authoring

- **Màu:** dùng semantic token IDs đã chốt qua `fill:{"tokenId":"<id>"}` hoặc
  fill style đã chốt. Giữ alias tới primitive nếu registry có; không sao chép
  resolved hex thành literal ở mỗi node. Ngoại lệ phải có trong manifest.
- **Typography:** dùng `styleId` theo vai trò. Style gồm `fontId`, `fontSize`,
  `lineHeight` (px), và tùy chọn `fill`; fontId phải trỏ font record phù hợp.
  Không tự ghi `fontFamily`, `fontWeight`, `letterSpacing` hay `textAlign`: hiện
  không phải node props hợp lệ. Muốn weight khác dùng font record được chấp thuận.
  Tránh override size/lineHeight/fontId riêng trên node vì chúng thắng style;
  nếu cần, ghi ngoại lệ hoặc tạo style chung theo phạm vi được giao.
- **Spacing/radius:** dùng dimension token của đúng role trên gap/padding/radius
  và các property cho phép. Không biến mọi số geometry thành token. Không suy
  “8px scale” hoặc một scale khác nếu manifest chưa chốt.
- **Layout:** row/column cho nội dung, free cho overlay/decor có chủ đích. Lưu
  fixed/fill/hug, min/max, padding/gap thay vì x/y tính từ screenshot. Root frame
  có width/height số; row wrap cần definite width và minWidth dương cho direct
  width-fill child. Không tự thêm %, Grid, breakpoint hoặc column wrap.
- **Component:** tái sử dụng master ID bằng `instance`; chỉ override fields đang
  được hỗ trợ/chấp thuận. Không sửa virtual child IDs; cập nhật toàn bộ
  `props.overrides` của instance khi cần. Đổi master ảnh hưởng mọi instance,
  phải thuộc phạm vi systemOwner. Tên “Button/Large” không tự tạo variant axis.
- **State/behavior:** chỉ khai báo/map state, navigation và animation được chốt.
  `props.states` không phải component-set variants. Intent chưa biểu diễn được
  ghi note, không bịa property hoặc hành vi thay thế.
- **Font/asset thiếu:** ghi rõ thiếu ID/hash/trạng thái. Chỉ fallback theo manifest;
  không âm thầm đổi font hoặc icon rồi đánh dấu hoàn tất.

Không tự upsert token/style/master hiện có chỉ để frame riêng “trông đẹp hơn”.
Quyền systemOwner ở đây là quy ước phối hợp; backend chưa phân quyền đó.

## 4. Hai AI cùng làm và revision conflict

Chia scope frame/node rõ, thống nhất ai sửa tài nguyên chung; prefix IDs theo
task tránh trùng. Không dựa vào việc hai frame khác nhau để suy ra không phụ thuộc:
chúng có thể dùng cùng style, alias, font, master hoặc rules note.

`apply_batch` dùng `expectedRevision:R`, key riêng cho đúng request body và ordered
operations (token trước style trước node dùng chúng). Mất response: replay nguyên
body và key. Đổi body hoặc expectedRevision: dùng key mới.

Khi `REVISION_CONFLICT`, kiểm `changedSince` nếu có; dùng `get_changes` để bắt kịp
hoặc đọc lại khi thiếu/truncated/RESET_REQUIRED. Kiểm cả nodes/ancestors **và**
token alias chains, styles, fonts/assets, component definitions, rules notes mà
batch dựa vào. Nếu bất kỳ dependency nào đổi, đọc lại context/registry ở revision
mới, đối chiếu rulesVersion rồi dựng lại batch. Chỉ rebase nhanh khi đã xác minh
thay đổi độc lập và danh sách đầy đủ; luôn key mới vì body đổi.

Nếu cùng sửa semantic decision, giữ quyết định đã được chốt hoặc chuyển đầu mối
thiết kế xử lý; không để “ghi sau thắng” chọn palette/font. Validation thành công
chỉ chứng minh model hợp lệ, chưa chứng minh đúng quy tắc thiết kế.

## 5. Kiểm và bàn giao

Đọc lại authored refs và resolved context ở committedRevision; nếu có write mới
trong lúc kiểm, đọc một lượt thống nhất mới và nêu revision đã nghiệm thu.
Render cùng theme/viewport đã chốt; pending job thì chờ `render_frame` với
`projectId`, `jobId`, `waitMs` tối đa 10.000, không polling dày. Inspect dùng đúng
jobId/sceneKey để so bounds/text lines. Kiểm overflow, text wrap, missing font,
state và các viewport theo scope; screenshot đơn lẻ không chứng minh responsive.

Bàn giao: projectId, rulesVersion, committed/verified revision, theme/viewports,
frame/master IDs đã đổi, token/style/font IDs dùng hoặc đổi, ngoại lệ/gap và bằng
chứng kiểm. Người nhận phải đọc lại dữ liệu hiện tại; không dùng báo cáo thay API.
Không hứa parity đầy đủ với Figma, pixel-perfect mọi OS/browser, hoặc policy
enforcement khi mới có protocol bằng văn bản.
