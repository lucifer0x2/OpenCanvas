# P23 — Lọc `list_tokens` / `list_styles` theo `query` và `type`/`kind`
Ưu tiên: P2 · Kích cỡ: ~40′ dev · Phụ thuộc: merge sau P19 (mô tả/schema `schemas.rs`) và P20 (`codeSyntax` trên token, cùng file) · Nguồn: khoảng trống #4 trong `.night/proposals/P20-token-code-syntax.md:23` (Figma `search_design_system`)

## Vấn đề
Agent designer hỏi "đã có token màu primary chưa?", "có text style heading nào?" thì hôm nay phải đọc toàn bộ `list_tokens`/`list_styles` (100 item/trang, nhiều trang) rồi tự lọc. Bộ lọc duy nhất là `ids` (phải biết ID trước). Figma MCP có `search_design_system` cho đúng việc này. Thêm tool mới tốn `tools/list` cố định mỗi phiên; mở rộng hai tool sẵn có là đủ.

## Bằng chứng
- Schema dùng chung cho 4 tool `list_*`: `crates/server/src/schemas.rs:113` chỉ có `projectId, ids, limit, cursor, revision`; vòng lặp đăng ký `schemas.rs:114-125`.
- Handler: `crates/server/src/reads.rs:67-89` lọc duy nhất theo `req.ids` (`reads.rs:72-76`), rồi `pagination::page` 100 item / 64 KiB.
- `ReadRequest` dùng chung, field mới phải `skip_serializing_if = "Option::is_none"` để giữ scope hash của cursor cũ (`reads.rs:34-49`, hash ở `crates/server/src/reads/pagination.rs:16-20`).
- Không có validate args theo inputSchema lúc chạy (chỉ `additionalProperties:false` công bố, `schemas.rs:48`), nên field lạ trên tool khác bị bỏ qua im lặng → cần guard tường minh.
- Model: `Token {id, name, type, value}` (`crates/storage/src/model.rs:49-55`), `Style {id, name, kind, value}` (`model.rs:59-64`); enum công bố `type ∈ color|dimension|number`, `kind ∈ fill|text` (`crates/server/src/schemas/operations.rs:36,41`).
- Tiền lệ so khớp: `find_nodes` lowercase substring, không fold dấu (`crates/server/src/reads/find.rs:28-29,54`).
- Số đo (ước lượng, AC8 đo thật): 1 token có theme ≈ 125–135 B (`{"id":"color.brand.primary.500","name":"Brand / Primary / 500","type":"color","value":{…}}` = 125 B). Design system 400 token ≈ 52 KB / 4 trang ≈ 13k token LLM; truy vấn khớp 10 token ≈ 1.4 KB. P20 thêm `codeSyntax` (+60–90 B/token) làm khoảng cách lớn hơn.

## Giải pháp
Thêm 2 tham số optional cho mỗi tool (không thêm tool):

| Tool | Tham số mới | Ngữ nghĩa |
| --- | --- | --- |
| `list_tokens` | `query` (string 1–128), `type` (enum `color\|dimension\|number`) | AND với nhau và với `ids` |
| `list_styles` | `query` (string 1–128), `kind` (enum `fill\|text`) | như trên |

- `query`: substring không phân biệt hoa thường (lowercase như `find.rs`, không fold dấu) trên `id` **hoặc** `name`. Không khớp → `items: []`, không lỗi.
- `type`/`kind`: so bằng đúng field model; giữ tên field model thay vì `kinds` để agent không phải nhớ hai tên.
- Thứ tự giữ thứ tự snapshot; lọc **trước** phân trang, nên `limit`/`cursor` phân trang trên tập đã lọc.
- Guard: `type` trên `list_styles` → `INVALID_REQUEST field:"type"`, gợi ý `"kind"`; `kind` trên `list_tokens` ngược lại; `query/type/kind` trên `list_assets`/`list_fonts` → `INVALID_REQUEST` (không bỏ qua im lặng).
- Ví dụ: `{"projectId":"p1","query":"primary","type":"color"}` → `{"schemaVersion":1,…,"items":[{"id":"color.primary.500",…}]}`.

Vì sao không phương án khác: (a) tool `search_design_system` mới ≈ +600–900 B `tools/list` và trùng chức năng; (b) đưa token/style vào `find_nodes` làm lẫn node với tài nguyên, đổi nghĩa `kinds`; (c) lọc theo `value` (tìm token `#1A73E8`) hữu ích nhưng cần chuẩn hoá màu/alias → phase sau.

## Phạm vi file
- Tạo `crates/server/src/reads/resource_filter.rs` (~70 dòng + unit test ~60): `struct Filter {query, token_type, kind}`, `fn check(tool, &Filter) -> Result<()>` (guard), `fn matches(&Value, &Filter) -> bool`.
- Sửa `crates/server/src/reads.rs` (97 → ~107): 3 field `ReadRequest` (`query`, `#[serde(rename="type")] token_type`, `kind`, đều `skip_serializing_if`), gọi `check` + `matches` trong filter `reads.rs:72-76`.
- Sửa `crates/server/src/schemas.rs` (163 → ~175): schema riêng cho `list_tokens`/`list_styles` = `resource` + field mới; câu mô tả thêm ≤ 63 B mỗi tool.
- Tạo `crates/server/tests/resource_filter.rs` (~120 dòng) theo kiểu `tests/context.rs` (seed bằng `upsert_token`/`upsert_style`).
- Sửa `docs/SERVER.md` (mục list_*), `.night` không đụng.

## Tương thích MCP
- Chỉ thêm field optional; gọi không có field mới → response byte-identical, cursor cũ còn hợp lệ (scope hash không đổi vì field vắng không serialize).
- `tools/list`: đo fragment hôm nay: `query` 42 B, `type` 47 B, `kind` 32 B, câu mô tả 63 B/tool → `list_tokens` +152 B, `list_styles` +137 B, tổng ≈ **+289 B**; `list_assets`/`list_fonts` không đổi. Nếu sau P19 `lean()` bỏ `maxLength`, còn ít hơn.
- Không đổi `instructions.rs` (chi phí cố định); mô tả tool đủ. Cập nhật `docs/SERVER.md`; skill `implement-frame.md`/`author-frame.md` thêm 1 dòng "tìm token trước khi tạo mới: `list_tokens query`" (tuỳ chọn, ≤ 1 dòng mỗi file).

## AC
1. Fixture 40 token (12 có "primary" ở id hoặc name, khác hoa thường): `list_tokens {query:"primary"}` trả đúng 12 item, đúng thứ tự snapshot; `query:"PRIMARY"` trả cùng tập.
2. `list_tokens {type:"color"}` chỉ trả token `type=="color"`; `{query, type}` và `{query, ids}` là AND (đếm khớp tập kỳ vọng trong test).
3. `list_styles {kind:"text", query:"heading"}` chỉ trả style text có "heading" ở id/name.
4. Không khớp → `items: []`, không lỗi, không `nextCursor`.
5. `list_styles {type:"color"}` → `INVALID_REQUEST`, `field:"type"`, thông điệp chứa `kind`; `list_tokens {kind:"fill"}` → `field:"kind"`, chứa `type`; `list_fonts {query:"x"}` và `list_assets {query:"x"}` → `INVALID_REQUEST`.
6. Phân trang trên tập lọc: `{query:"primary", limit:5}` → 5 + 5 + 2 qua `nextCursor`, không trùng/thiếu; cursor của trang có `query` đem dùng cho request không `query` → `INVALID_CURSOR` (scope khác, `reads/pagination.rs:41-52`) như hiện hành.
7. Gọi không có field mới: response `serde_json::to_vec` byte-identical trước/sau trên cùng fixture; test pagination/cursor hiện có pass.
8. Đo byte: fixture 400 token có theme `dark`; tổng byte mọi trang `list_tokens` không lọc ≥ 40 000 B, `{query}` khớp 10 token ≤ 2 048 B (in số đo vào report).
9. `tools/list` (đo `serde_json::to_vec` toàn bộ trước/sau trên cùng base): tăng ≤ **+320 B**; `list_assets`, `list_fonts` không đổi byte; `tests/tools_budget.rs` và `tests/tool_schemas.rs` pass.
10. File: `resource_filter.rs` ≤ 200 dòng, `reads.rs` ≤ 120, `schemas.rs` ≤ 200.

## Cách kiểm
- `CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server --test resource_filter --test tools_budget --test tool_schemas`
- `CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server reads::` (unit `resource_filter`, `pagination_tests`).
- `cargo clippy -p opencanvas-server --all-targets -- -D warnings`; `wc -l` các file ở AC10.
- Live (tuỳ chọn): server tạm `--port 0 --data-root $(mktemp -d)`, `create_project` + `apply_batch` vài `upsert_token`, gọi `list_tokens {query}` qua MCP; dọn process/data sau kiểm.

## Rủi ro
- Xung đột `schemas.rs`/`reads.rs` với P19 (lean schema, rút mô tả) và P20 (`codeSyntax`): làm sau, đo AC9 trên base đã merge.
- `type` là từ khoá Rust → `#[serde(rename = "type")]`; `ReadRequest` có `deny_unknown_fields` nên phải khai báo đủ field.
- Không fold dấu: "mau" không khớp "màu" — cùng hành vi `find_nodes`, nêu trong mô tả.

## Ngoài phạm vi / phase sau
- Lọc theo giá trị (`valueEquals:"#1A73E8"`, theo theme, theo alias) để agent tái dùng token thay vì hard-code.
- `query` khớp `codeSyntax` của P20 (vd `--color-primary`) và tìm font theo `family` trong `list_fonts`.
- Tìm component (khi có component/instance, `PLAN.md:399`).
