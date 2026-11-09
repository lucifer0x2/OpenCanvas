# Viewer: tối ưu frontend — 2026-11-09

Bản frontend mới đã được đưa vào bản test riêng tại
[http://127.0.0.1:3867](http://127.0.0.1:3867). Reload trang; dùng Cmd+Shift+R nếu
tab còn giữ bundle cũ. Người dùng tự kiểm trải nghiệm pan/zoom, All frames,
Layers, Inspector và cập nhật snapshot.

## Nguyên nhân và bằng chứng từ mã nguồn

- Painter gọi lại `indexScene` khi paint/pan, dù scene chưa đổi. Chưa có culling
  tổng quát theo viewport; nhánh bị ancestor clip loại bỏ đã có thể bỏ qua con.
- Các Shape chung Konva Layer vẫn được vẽ lại khi Layer redraw, kể cả khi React
  đã memo. `batchDraw` gom các yêu cầu vẽ; không có bằng chứng hai paint/event.
- Layers quét lại mảng node nhiều lần; identity props/callback chưa ổn định làm
  mất lợi ích của memo ở các ranh giới UI.
- Ở zoom 10–20%, nhiều nội dung có thể nằm trong cùng vùng nhìn, tăng việc paint.
  Đây là suy luận về khối lượng hiển thị, không có ngưỡng zoom này trong mã.

## Các thay đổi

- `CanvasFrame` chuẩn bị cây scene một lần cho mỗi ready scene/resources.
  `paintScene`/`paintTree` nhận cây đã chuẩn bị tùy chọn, giữ đường gọi mặc định.
  Culling dùng device viewport với bounds bảo thủ cho overflow, text ink,
  stroke/shadow và guard anti-alias theo device pixel; placeholder chưa xác định
  bounds không bị loại bỏ. Giữ clip/opacity; sửa extent của transparent inset
  shadow và dùng lại transform giữa các sibling.
- Giữ ownership/publication của camera. Geometry, frame props và `CanvasScenes`
  ổn định để pan không lặp việc chuẩn bị danh sách.
- Một `WeakMap` chia sẻ index cho mỗi mảng node bất biến, tra ID và children đã
  sắp xếp theo số rồi ASCII khi hòa; Layers duyệt index thay các lần quét O(V×N).
  Giữ hành vi mở hàng mặc định.
- Ổn định App selectors/callback/frame IDs; memo Layers/Inspector. Tách commands
  ổn định của NodeActions khỏi busy state, đọc selection đã commit mới nhất và
  giữ đúng project/revision đã chọn khi export; giữ identity nếu selection không đổi.
- Conditional GET dùng strong HTTP ETag sau xác thực và tra project có thẩm quyền.
  `304` giữ reference snapshot đã nhận, xóa transient state; `200` vẫn nhận đổi
  privacy/metadata dù revision không đổi. `useSnapshot` giữ `{snapshot, etag}`
  nguyên tử; reset khi đổi project/reload/404; giữ coalescing, abort, chống trả
  kết quả cũ và kiểm session khi `401`. Authority/cache IndexedDB không đổi.

## Ranh giới state

Đã rà khoảng 18 state cells phối hợp trong App: camera/expansion cục bộ,
selection có owner chung, workflow action nằm trong provider. Ownership hiện có
của FontManager/Preview vẫn gắn với chức năng tương ứng. Không cần thêm reducer
hay store chung chỉ để giảm số hook; thay đổi tập trung vào identity và ranh giới
render. Các source/test được giao đều không quá 250 dòng (lớn nhất 233 dòng).

## Kiểm chứng đã chạy

Các worker chạy kiểm thuần đúng vùng thay đổi; phiên chính kiểm build tổng hợp:

```sh
# Từ root repo: canvas 17/17
node --test --test-concurrency=1 renderer/viewport-paint.test.mjs renderer/text-extent.test.mjs
# Từ web/: state/UI model 19/19
node --test --test-concurrency=1 tests/state.test.ts tests/frame-scope.test.ts tests/node-links.test.ts tests/preview-model.test.ts
# Từ root repo: snapshot HTTP 10/10
node --experimental-transform-types --test --test-concurrency=1 web/tests/snapshot-http.test.ts
# Từ root repo: typecheck + production bundle tổng hợp
npm --prefix web run build
git diff --check
```

Tất cả lệnh trên đạt; mỗi worker cũng đã kiểm web typecheck. Vite còn cảnh báo
chunk lớn: bundle chính 666,36 kB theo build. Đây chưa phải bằng chứng FPS hay
mức cải thiện thời gian.

## Bản test và giới hạn

Chỉ đồng bộ generated static assets vào `source/web/dist` của bản test riêng;
giữ các hash cũ cho tab đang mở, thay index sau cùng bằng rename nguyên tử.
HTTP index và `index-CZExainG.js` khớp từng byte với build vừa hoàn tất.
App test PID17418 và PG riêng PID17389 giữ nguyên thời điểm khởi động.
Native PID23980 trên 3857/3858 và PostgreSQL nguồn trên 2703 giữ nguyên;
không restart, đổi launchd/Funnel, commit hoặc push.

Chromium đã bị chặn khi launch bởi MachPort `bootstrap_check_in` 1100/SIGTRAP;
đã dừng thử lại browser. Chưa đo FPS/thời gian, xác nhận pixel parity hoặc chạy
browser/runtime/DB/stress/benchmark campaign cho thay đổi này. Lượt cập nhật bản
test chỉ GET static index/bundle và đối chiếu process/listener; không render job.
