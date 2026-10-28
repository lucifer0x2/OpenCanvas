# P20 — `codeSyntax` trên token: tên biến trong code (CSS var, Android, iOS) đi kèm token khi coder đọc
Ưu tiên: P1 · Kích cỡ: ~50 phút dev · Phụ thuộc: không. Chạm `schemas/operations.rs`, nên xếp sau P19 nếu P19 merge trước; xung đột chỉ một dòng · Nguồn: tự đề xuất (so sánh với Figma Dev Mode MCP, Penpot MCP, Framelink)

## Vấn đề
Coder viết code từ `get_design_context` thấy authored props tham chiếu `{tokenId}` và nhận `tokens[]` với `id/name/type/value/resolvedValue`. Họ không biết token đó tương ứng với biến nào trong code. Chỉ dẫn hiện tại bảo coder tự map sang "your own named token", nên agent tự đặt tên mới (`--accent`, `--color-accent-500`…) và lặp lại việc này ở mỗi phiên, kể cả khi design được vẽ lại từ UI có sẵn (trường hợp QueryNest), nơi biến CSS gốc đã tồn tại. Figma giải bài này bằng `codeSyntax` của variable. Tool `get_variable_defs` trả `codeSyntax` cho mỗi biến để agent dùng đúng `var(--…)`. OpenCanvas chưa có trường tương đương và chưa có đề xuất nào về nó.

## Bằng chứng
- Token chỉ có 4 trường và `deny_unknown_fields`: `crates/storage/src/model.rs:47-55`.
- Schema `upsert_token` chỉ nhận `id/name/type/value`: `crates/server/src/schemas/operations.rs:35-37`.
- Coder được bảo tự map: `crates/server/src/instructions.rs:26`, `crates/server/src/schemas.rs:89` (mô tả `get_design_context`), `crates/server/skills/implement-frame.md:9-10`.
- Token đi vào `get_design_context` bằng `serde_json::to_value(s)` rồi thêm `resolvedValue`: `crates/server/src/reads/tree.rs:107-118`. Một trường serde optional mới sẽ tự xuất hiện ở đây và trong `list_tokens` (`crates/server/src/mcp.rs:119`) mà không cần sửa code đọc.
- Token lưu dưới dạng JSON serde: `crates/storage/src/persistence.rs:47`. Thêm field `#[serde(default)]` không cần migration.
- Validate token: `crates/storage/src/validate/candidate.rs:195-199` gọi `validate_tokens` (`crates/storage/src/validate/tokens.rs:66`).
- Đo `tools/list`: thuộc tính thêm vào schema `upsert_token` (`"codeSyntax":{"type":"object","propertyNames":{"enum":["web","android","ios"]},"additionalProperties":{"type":"string","minLength":1,"maxLength":256}}`) dài **151 B** (`printf %s … | wc -c`). Nếu viết theo dạng `object()` với 3 property thì là 255 B.

**Bảng khoảng trống** (tool hoặc thông tin mà coder/designer agent dùng nhiều ở MCP khác, so với OpenCanvas và `.night/proposals/`):

| # | Khả năng ở MCP khác | OpenCanvas hiện có | Đã có đề xuất? |
| --- | --- | --- | --- |
| 1 | `get_variable_defs` trả biến cùng **`codeSyntax`** (WEB/ANDROID/iOS) — [Figma tools](https://developers.figma.com/docs/figma-mcp-server/tools-and-prompts/), [Variable.codeSyntax](https://developers.figma.com/docs/plugins/api/Variable/) | Token không có tên code (`model.rs:47-55`) | **Chưa → chọn (P20)** |
| 2 | `get_code_connect_map` / `add_code_connect_map`: component → file và props của component trong code (cùng link Figma) | Không có component/instance; `PLAN.md:399` hoãn component/variant | Chưa. Cần khái niệm component trước, để phase sau |
| 3 | `get_metadata`: outline XML gọn có x/y/w/h | `get_outline` không có bounds; vị trí phải đọc qua `get_design_context viewports`/`inspect_layout` (MCP instructions) | Chưa (giá trị trung bình) |
| 4 | `search_design_system`: tìm component/biến/style theo truy vấn | `list_tokens`/`list_styles` liệt kê toàn bộ, không có filter | Chưa (giá trị thấp, ≤ 4096 token) |
| 5 | Prompt `create_design_system_rules`: sinh file rules cho repo | Chỉ có skill tĩnh `skill://opencanvas/implement-frame.md` | Chưa (giá trị thấp) |
| 6 | `download_assets` (Figma), `download_figma_images` ([Framelink](https://github.com/GLips/Figma-Context-MCP)), export shape (Penpot MCP, [ancrz](https://glama.ai/mcp/servers/@ancrz/penpot-mcp-server/blob/57d1f93bd3eaf6c846210fd0f51e40d234664319/README.md), [zcube](https://lobehub.com/en/mcp/zcube-penpot-mcp-server)) | — | Có: P03a/b/d |
| 7 | `get_motion_context` (keyframes + CSS) | — | Có: P07b |
| 8 | `html_to_figma`, `generate_figma_design` (UI web → layer) | — | Có: P18a |
| 9 | `get_screenshot` | `render_frame` | Đã có trong sản phẩm (so sánh ảnh: P13a) |
| 10 | Framelink `get_figma_data`: gom style lặp vào `globalVars` | `compact:true` cùng `resources:"referenced"` | Gần đủ, không cần |

Danh sách tool chính thức của Penpot MCP không xác minh được trong phiên này. Chỉ đọc được README của hai server cộng đồng nêu trên; các nhóm tool của chúng (shape, export, component, comment) đều đã nằm trong các hàng 6–9.

## Giải pháp
Thêm vào token một trường optional, đặt tên theo Figma để agent đã quen Figma đoán được:

```json
{"op":"upsert_token","token":{"id":"accent","name":"Accent","type":"color","value":"#3b82f6",
  "codeSyntax":{"web":"var(--color-accent)"}}}
```

Coder sẽ nhận được (`get_design_context`, cả khi `compact:true`; `list_tokens` cũng vậy):

```json
{"id":"accent","name":"Accent","type":"color","value":"#3b82f6",
 "codeSyntax":{"web":"var(--color-accent)"},"resolvedValue":"#3b82f6"}
```

1. **Model** (`model.rs`): `#[serde(default, skip_serializing_if = "Option::is_none")] pub code_syntax: Option<BTreeMap<String, String>>`, serde camelCase thành `codeSyntax`. Token không có trường này serialize giống hệt hiện nay, nên không tốn thêm token. 12 chỗ khởi tạo `Token { … }` trong code/test cần thêm `code_syntax: None`.
2. **Validate** (`validate/tokens.rs`, trong vòng lặp của `validate_tokens`):
   - Khoá chỉ được là `web`, `android` hoặc `ios`. Khoá sai báo lỗi với field `token.codeSyntax.<key>` và nêu đủ 3 khoá hợp lệ, cùng dạng lỗi `allowedKeys` của props nếu helper có sẵn.
   - Giá trị phải là chuỗi 1–256 ký tự, không có ký tự điều khiển hay xuống dòng.
   - `{}` bị từ chối với lời nhắn "omit codeSyntax instead of {}".
   - Server **không** kiểm cú pháp CSS/Kotlin/Swift. Đây là chuỗi khai báo, giống Figma.
3. **Schema** (`operations.rs:36`): thêm thuộc tính 151 B ở trên, không thêm vào `required`.
4. **Alias không kế thừa**: token alias có `codeSyntax` riêng. Nếu thiếu, coder theo `aliasId` tới token gốc. Cách này giống Figma, đơn giản và không đổi resolver M3.
5. **Hướng dẫn**:
   - `instructions.rs:26`: "…mapped to the token's `codeSyntax.web` when present, else your own named token…" (+~45 B trong `initialize`, không thuộc `tools/list`).
   - `implement-frame.md:9-10`: dùng `codeSyntax` thay vì tự đặt tên.
   - `author-frame.md`: khi vẽ lại UI có sẵn, đặt `codeSyntax.web` theo biến CSS gốc.

**Vì sao không chọn phương án khác:**
- Tool mới `get_variable_defs`: tốn ~1 KB `tools/list` mỗi phiên và trùng `list_tokens`/`get_design_context resources:"referenced"`.
- Quy ước đặt `name` = `--color-accent`: `name` là nhãn cho người đọc, chỉ chứa được một nền tảng, và coder không phân biệt được nó có phải tên code hay không.
- Bảng mapping riêng ở cấp project: cần thêm một lần đọc và một nơi đồng bộ, trong khi trường trên token tự đi theo `resources:"referenced"`.

## Phạm vi file
| File | Thay đổi | Dòng (hiện → sau) |
| --- | --- | --- |
| `crates/storage/src/model.rs` | thêm field | 132 → ~136 |
| `crates/storage/src/validate/tokens.rs` | `validate_code_syntax` + gọi trong vòng lặp | 163 → ~195 (nếu > 200 thì tách `validate/code_syntax.rs` ~45 dòng) |
| `crates/server/src/schemas/operations.rs` | 1 thuộc tính | 67 → ~68 |
| `crates/server/src/instructions.rs`, `crates/server/skills/implement-frame.md`, `author-frame.md` | 1–2 dòng mỗi file | |
| `docs/RESOLUTION.md` | mục hình dạng token: `codeSyntax`, không kế thừa qua alias | +6 |
| 12 chỗ `Token { … }` (`crates/storage/examples/spike/fixture.rs:29`, …) | `code_syntax: None` | +12 |
| `crates/server/tests/token_code_syntax.rs` (mới) | test AC1–AC5, AC7 | ~130 |

## Tương thích MCP
- `tools/list`: chỉ thêm một thuộc tính optional trong schema `upsert_token`, **+151 B (≤ 160 B)**. Không thêm tool. Field cũ giữ nguyên nghĩa.
- Đầu ra `get_design_context`, `list_tokens`, `get_changes`: token không có `codeSyntax` cho JSON byte-identical với hiện nay.
- Dữ liệu cũ đọc được ngay nhờ `#[serde(default)]`. Bundle export/import (M4) mang theo field qua serde.
- Cần cập nhật: `instructions.rs`, 2 skill, `docs/RESOLUTION.md`. `docs/SERVER.md` không mô tả shape token nên không đổi.

## AC
1. `apply_batch` có `upsert_token` kèm `codeSyntax:{"web":"var(--color-accent)"}` thành công. Sau đó `get_design_context {frameId, compact:true}` của một frame có fill `{accent}` trả token `accent` mang `codeSyntax` đúng bằng input, kèm `resolvedValue`.
2. `list_tokens` trả cùng `codeSyntax`.
3. Token không khai báo `codeSyntax` không có khoá `codeSyntax` trong `get_design_context`/`list_tokens`. Các test hiện có `crates/server/tests/context.rs` và `multi_viewport.rs` pass mà không sửa expected.
4. Mỗi input sau bị từ chối với lỗi validation có `field` đúng:
   - `{"css":"x"}` → `token.codeSyntax.css`, message có `web`, `android`, `ios`;
   - `{"web":""}`, chuỗi 257 ký tự, chuỗi có `\n` → `token.codeSyntax.web`;
   - `{}` → `token.codeSyntax`.
   Revision không đổi sau batch lỗi.
5. Bền vững: dừng rồi mở lại server trên cùng data root thì `list_tokens` còn `codeSyntax`. Upsert lại token không có `codeSyntax` thì trường bị xoá.
6. `tools/list` tăng ≤ 160 B so với trước thay đổi (đo bằng `serde_json::to_vec(&tools_list).len()`, ghi số đo vào report). `crates/server/tests/tools_budget.rs` pass.
7. Token alias có `codeSyntax` riêng giữ `resolvedValue` như cũ. Alias không có `codeSyntax` không thừa hưởng của token gốc.
8. `cargo test -p opencanvas-storage`, `cargo test -p opencanvas-server` và `cargo clippy --all-targets -D warnings` pass. File sửa không vượt 250 dòng.

## Cách kiểm
```bash
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server --test token_code_syntax
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server --test tools_budget --test context --test multi_viewport
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-storage
```
- Test mới `token_code_syntax.rs` dùng helper tạo project/batch như `tests/context.rs:11-12`. Viết mỗi AC một hàm `#[tokio::test]`. AC5 mở lại store trên cùng thư mục tạm.
- AC6: chạy cùng phép đo `tools/list` trên commit nền và trên nhánh (không dùng `git stash`), lấy hiệu số rồi ghi vào report.
- Kiểm live: chạy server tạm `--port 0 --data-root $(mktemp -d)`, gọi `apply_batch` rồi `get_design_context` bằng curl MCP, xong thì dọn process và thư mục tạm. Không đụng cổng 3847.

## Rủi ro
- `Token` có `deny_unknown_fields`, nên binary cũ hơn không đọc được data hoặc bundle đã có `codeSyntax` (hạ cấp). Ghi rõ trong `docs/BACKUP.md` nếu file đó có mục tương thích ngược.
- `codeSyntax` chỉ là lời khai báo: designer có thể ghi sai tên biến và server không kiểm. Hướng dẫn cho coder nói rõ "kiểm biến tồn tại trong repo".
- Đụng `schemas/operations.rs` cùng P19, P12a, P15: xung đột một dòng, rebase dễ.

## Ngoài phạm vi / phase sau
- `codeSyntax` cho style, ví dụ text style → class `.text-body`.
- Code Connect cho node: `codeComponent {source, name, props}`. Cần component/instance trước (`PLAN.md:399`), là khoảng trống số 2.
- `get_design_context tokenCss`: sinh sẵn khối `:root{--…}` từ token có `codeSyntax.web`.
- `get_outline` có bounds (khoảng trống 3), filter `query` cho `list_tokens`/`list_styles` (khoảng trống 4), MCP prompt sinh rules (khoảng trống 5).
