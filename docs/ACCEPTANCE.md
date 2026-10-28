# OpenCanvas — bản đồ nghiệm thu

Nguồn yêu cầu: `PLAN.md`. Tài liệu này do phiên điều phối quản lý. Trạng thái ban đầu là **chưa kiểm**, chỉ đổi sau khi có lệnh và kết quả thực tế.

| Mốc | Bằng chứng bắt buộc | Trạng thái |
| --- | --- | --- |
| M0 pipeline | Taffy + batch measurement/cache có giới hạn; hai width, Vietnamese wrap/hug; shared scene/renderer/font manifests; missing font/delayed image strict readiness | Phiên chính chạy Chromium 6/6 pass; chờ báo cáo/quyết định cuối |
| M0 storage | SQLite patched runtime, WAL/FULL/FK; budget trước benchmark; small/large create/update/reparent; 1/4/8 clients + reader; queue/conflict/WAL/checkpoint; correctness/restart | Chưa kiểm |
| M1 domain/storage | Create/open project guards; typed atomic ops, revision/idempotency; resource refs; durable assets; tokens/styles resolver | Storage crate 15 tests pass độc lập; E2E MCP/browser chưa có |
| M1 API/MCP | Streamable HTTP MCP schemas; list/create, outline/subtree/context/resources, apply/import/render/inspect; HTTP versioned snapshot/SSE/assets/jobs; caps và client-safe errors | Chưa kiểm |
| M1 viewer | Light editor shell, collapsible layers/inspector, project/page/frame/viewport, read-only selection/highlight/pan/zoom/fit; scene revision rõ | Chưa kiểm |
| M1 E2E | MCP → DB → viewer → PNG → restart/reload giữ IDs/refs/revision; invalid batch không commit | Chưa kiểm |
| M2 transaction/recovery | Concurrent base revision; lost-response retry/key reuse; crash trước/sau commit; asset publish/commit durable/orphan | Storage targeted tests pass; transport lost-response E2E chưa có |
| M2 events/jobs | Snapshot/replay không rơi commit; cursor reset; reconnect; old job không thay scene mới; worker failure/missing resource statuses | Chưa kiểm |
| M3 context/resolution | Cursor pagination, consistent revision, typed alias/theme cycle checks/invalidation; layout fixtures and diagnostics | Chưa kiểm |
| M4 hardening | Versioned migration giữ DB khi lỗi; consistent backup DB + resource manifest; round-trip new project/revision reset; hash/archive/symlink guards | Chưa kiểm |

## Quy tắc báo cáo

- Test phải đi qua seam thật; mock không thay bằng chứng browser worker hoặc SQLite durable.
- Không lấy ảnh đẹp làm bằng chứng persistence/events. Scene line lists và provenance phải kiểm riêng.
- Root data tests tạm, không dùng project thật trong `~/.opencanvas`.
- Phân biệt chưa triển khai, chưa kiểm, thất bại và đã đạt. Không ghi toàn bộ milestone đạt chỉ vì một happy path xanh.
- Ghi command, runtime, fixture, kết quả, giới hạn vào báo cáo của task; giữ nhật ký tóm tắt trong `AGENTS.md`.
