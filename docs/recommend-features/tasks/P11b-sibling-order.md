# P11b — Thứ tự sibling rõ ràng: `sortKey` luôn là index 0..n−1, MCP nói rõ thứ tự vẽ (note #14)
Ưu tiên: P2 · Kích cỡ: ~40 phút dev · Phụ thuộc: không. Khác vùng P11a; chạm `schemas.rs` (file nóng: xếp tuần tự hoặc rebase một dòng) · Nguồn: note #14

Phase b của P11, độc lập với `P11a-missing-glyph.md`.

## Vấn đề

Note #14: "`sortKey` của các sibling tạo không có `index` đều là `0` trong outline — thứ tự vẽ dựa vào gì thì không rõ từ output."

Đo trên integration, **không tái hiện được "đều là 0"**: create không có `index` cho ra 0..n−1. Nhưng đo ra hai vấn đề thật, đủ để agent hiểu sai như người dùng:

1. **MCP không nói `sortKey` nghĩa là gì, cũng không nói thứ tự vẽ.** Không chỗ nào cho agent biết rằng:
   - `sortKey` là vị trí trong cùng `parentId`, không phải z-order toàn cục;
   - sibling sau vẽ đè sibling trước;
   - bỏ `index` là thêm vào cuối, tức nằm trên cùng.
   Outline liệt kê preorder, nên con đầu của mỗi container đều hiện `0`. Cây UI nhiều container một con (nút → nhãn) và mỗi page một root frame cho ra cả loạt `sortKey: 0`. Đây là giả thuyết có khả năng nhất cho quan sát "đều là 0".
2. **`sortKey` có lỗ sau `delete_subtree` và `reparent_node`**, nên không còn bằng `index` mà `reorder_node` nhận. Agent lấy `sortKey` làm `index` sẽ đặt sai chỗ.

## Bằng chứng

Code ở `night/integration@900e0dc`.

| Vị trí | Hiện trạng |
| --- | --- |
| `crates/storage/src/validate/candidate.rs:68-99` | `place`: sắp sibling theo `(sort_key,id)`, chèn tại `index` hoặc cuối (`:82`), đánh số lại 0..n−1 cho **parent mới** (`:89-96`). |
| `crates/storage/src/validate/candidate.rs:145-158` | `ReparentNode` đổi `parent_id` rồi `place` ở parent mới; **parent cũ không được đánh số lại**. |
| `crates/storage/src/validate/candidate.rs:169-193` | `DeleteSubtree` xoá node; **sibling còn lại giữ số cũ**. |
| `crates/server/src/jobs/input.rs:46-57` | `collect`: preorder, con sắp theo `sort_key`. Đây là thứ tự scene, và `renderer/scene-tree.ts:107-132` vẽ theo thứ tự đó: sau đè trước. `spikes/pipeline/geometry.test.mjs:19` đã khoá "paint order". |
| `crates/server/src/reads/index.rs:2-3`, `:25-27` | Mọi read dạng cây sắp sibling theo `sort_key`. |
| `crates/server/src/reads/tree.rs:56` | Outline trả `sortKey` thô từ storage. |
| `crates/server/src/schemas.rs:78` | Description `get_outline`: "Ordered id/parentId/kind/name/sortKey …", không giải thích. |
| `crates/server/src/schemas/operations.rs:12`, `:15`, `:25`, `:30` | `index` chỉ là `{"type":"integer","minimum":0,"maximum":20000}`, không description. |
| `crates/server/skills/author-frame.md`, `implement-frame.md` | `rg "sortKey\|index\|on top\|paint"`: không có kết quả. |
| `docs/CONTRACT.md:11` | Hợp đồng ghi "`sortKey` is a number (integer sibling index)", nên lỗ hổng trái hợp đồng. |

**Đo live** (binary `target/debug/opencanvas-server` của integration, `--port 0`, data root tạm, đã dọn; script ở `/private/tmp/claude-501/-Volumes-MySpace-Workspace-OpenCanvas/14034e76-758d-42c7-8c11-41ff9b05d4f4/scratchpad/p11/sortkey.mjs`, `sortkey2.mjs`):

- Batch 1 tạo `zeta, alpha, mid` (không `index`). Batch 2 tạo `late` (không `index`) và `first` (`index:0`). Outline: `first:0, zeta:1, alpha:2, mid:3, late:4`. Không có "đều 0".
- Tạo `a,b,c,d`, rồi `delete_subtree b` và `reparent_node c → other`. Outline: `frame/a:0, frame/d:3, other/c:0`. Sau restart server vẫn `d:3` (lỗ được lưu bền). `reorder_node` với `index:3` lúc này sẽ báo "Sibling index is out of bounds" (`candidate.rs:83-87`), vì chỉ còn 2 sibling.

## Giải pháp

### 1. Storage: `sortKey` luôn đặc 0..n−1

Tách `place` khỏi `candidate.rs` (213 dòng) sang module mới `crates/storage/src/validate/order.rs`, thêm `renumber(parent, changed)`:

```rust
/// Siblings of `parent` keep their order and take sort keys 0..n-1; only nodes whose key changes are marked changed.
pub(super) fn renumber(nodes: &mut BTreeMap<String, Node>, parent: &Option<String>, changed: &mut BTreeSet<String>)
```

- `ReparentNode`: ghi nhớ `old_parent` trước khi đổi; sau `place` thì `renumber(old_parent)` nếu khác parent mới.
- `DeleteSubtree`: ghi nhớ parent của gốc bị xoá; sau khi xoá thì `renumber(parent)`. Chỉ parent của gốc, vì con cháu bị xoá cùng.
- `place` giữ nguyên hành vi, chuyển sang `order.rs`.

**Hệ quả có chủ đích:** sibling được đánh số lại vào `changedIds`. Hôm nay chèn `index:0` đã làm thế (`candidate.rs:92-95`), nên đây là cùng một quy tắc.

**Dữ liệu cũ có lỗ:** không migration. Readers vẫn sắp đúng. Lỗ tự được lấp lần kế tiếp con của parent đó thay đổi. Ghi trong `docs/SERVER.md`.

**Phương án loại:**

- *Đặc hoá lúc đọc* (outline tính `sortKey` = vị trí): phải sửa outline, `get_subtree`, snapshot HTTP và bundle; storage vẫn trái `CONTRACT.md:11`.
- *Bỏ `sortKey` khỏi outline*: vỡ field cũ, viewer `CONTRACT.md:40` đang dùng.

### 2. MCP: nói rõ thứ tự (chỉ description, không đổi schema)

`get_outline` (`schemas.rs:78`) nối thêm:

> "Children are listed back to front: a later sibling paints over earlier ones. sortKey is the 0-based position within parentId."

`apply_batch` (`schemas.rs:134`) nối thêm:

> "index is the 0-based position among the parent's children; omitted appends last, painted on top."

Không gắn description vào `index` trong `operations.rs:12`: giá trị đó dùng chung cho 3 op, nên cùng câu sẽ lặp 3 lần trong `tools/list`.

`crates/server/skills/author-frame.md`, một dòng ở mục tạo node:

> "Create children in back-to-front order (background first); later siblings paint on top. Pass index only to insert before an existing sibling; read sortKey from get_outline as that position."

### 3. Tài liệu

`docs/SERVER.md` (mục apply_batch / outline):

- `sortKey` đặc 0..n−1 sau mọi op;
- sibling được đánh số lại có trong `changedIds`;
- dữ liệu cũ có thể còn lỗ đến lần sửa kế tiếp.

## Phạm vi file

| File | Thay đổi | Dòng ước lượng |
| --- | --- | --- |
| `crates/storage/src/validate/order.rs` | mới: `place` (chuyển) + `renumber` + unit test | ~90 |
| `crates/storage/src/validate/candidate.rs` | gọi `order::place/renumber`, bỏ thân `place` | 213 → ~190 |
| `crates/storage/src/validate.rs` | `mod order;` (cạnh `mod candidate;` ở dòng 1) | +1 |
| `crates/storage/tests/sibling_order.rs` | mới: AC1–AC5 qua API storage | ~110 |
| `crates/server/src/schemas.rs` | 2 câu description | ±0 dòng (dài hơn) |
| `crates/server/skills/author-frame.md` | 1 dòng | +1 |
| `docs/SERVER.md` | 3 dòng | +3 |

## Tương thích MCP

- Không thêm tool hay field. Schema giữ nguyên; chỉ hai description dài thêm. Ước lượng `tools/list` +~190 B (AC7 đo thật).
- `sortKey` giữ kiểu và nghĩa "sibling index" theo hợp đồng; chỉ hết lỗ.
- `changedIds`/`get_changes` của batch có `delete_subtree`/`reparent_node` có thể dài hơn (các sibling phía sau). Client theo quy tắc rebase trong instructions vẫn đúng: chỉ có thêm ID để xét.
- Bundle và migration không đổi. Bundle cũ có lỗ vẫn import được (`bundle_history.rs:77-78` chỉ chặn số âm).

## AC

1. Storage test: tạo `a,b,c,d` dưới `frame` (không `index`) cho `sortKey` 0,1,2,3. Batch `delete_subtree b` cho `a:0, c:1, d:2`, và `changedIds` của batch chứa `b, c, d`, không chứa `a`.
2. `reparent_node c → other` (không `index`): `frame` còn `a:0, d:1`; `other` có `c` ở cuối với `sortKey` = số con cũ của `other`. `changedIds` chứa `c, d`.
3. Sau AC1–2, `reorder_node d index:0` cho `d:0, a:1`. `reorder_node` với `index` = số sibling khác của node (n−1, tức chèn cuối) hợp lệ; `index` = n báo `Sibling index is out of bounds` như hiện nay.
4. Tạo không `index` luôn được `sortKey` = số sibling trước đó; `index:0` đẩy các sibling khác +1. Khoá hành vi hiện có, kể cả hai batch liên tiếp như phép đo live.
5. Đóng/mở lại store (persist + load) giữ nguyên các `sortKey` đặc của AC1–3. `crates/storage/tests/bundle_roundtrip.rs` pass không sửa.
6. Live (server tạm): lặp lại kịch bản đo `sortkey2.mjs`. Outline cho `frame/a:0, frame/d:1, other/c:0` (hiện `d:3`).
7. `tools/list` (`serde_json::to_vec`) tăng ≤ 250 B so với trước. `cargo test -p opencanvas-server --test tools_budget --test tool_schemas` pass; ghi hai số trong `.night/dev/P11b.md`.
8. Description `get_outline` chứa "later sibling paints over"; description `apply_batch` chứa "omitted appends last" (test chuỗi trong `tool_schemas.rs` hoặc `skill_layer.rs`).
9. `wc -l crates/storage/src/validate/candidate.rs` ≤ 213 (không phình); `order.rs` ≤ 120.

## Cách kiểm

```bash
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-storage
CARGO_BUILD_JOBS=4 cargo test -p opencanvas-server --test tools_budget --test tool_schemas --test skill_layer
node /private/tmp/claude-501/-Volumes-MySpace-Workspace-OpenCanvas/14034e76-758d-42c7-8c11-41ff9b05d4f4/scratchpad/p11/sortkey2.mjs http://127.0.0.1:<port> 1
```

Lệnh cuối chạy với server tạm `--port 0 --data-root $(mktemp -d)`. Dev chép script vào thư mục tạm của mình nếu scratchpad đã bị dọn: cần `create_project`, 2 `apply_batch`, `get_outline`.

## Rủi ro

- Batch xoá phần tử đầu của container lớn làm `changedIds` dài (tối đa số sibling, ≤ 20 000 theo schema `index`). Quy tắc chèn `index:0` đã có chi phí này; `changedSince` đã có `truncated`.
- Hai agent sửa sibling khác nhau cùng lúc sẽ gặp `REVISION_CONFLICT` thường hơn, vì sibling đánh số lại nằm trong `changedIds`. Rebase vẫn an toàn vì server validate lại.
- Chưa xác định chắc nguồn quan sát "đều là 0" của người dùng: không tái hiện được trên integration, và không được đọc dữ liệu 3847. Nếu Context Watch có outline gốc, đối chiếu trước khi đóng note #14.

## Ngoài phạm vi / phase sau

- Layers panel viewer hiển thị trên cùng trước (kiểu Figma, đảo thứ tự): việc của viewer, không cần đổi dữ liệu.
- Field `z`/`zIndex` tách khỏi thứ tự stack: không cần cho tới khi có yêu cầu thứ tự vẽ khác thứ tự layout.
- Op "đưa lên trên/xuống dưới" (`index:"last"`): `reorder_node` với `index` = n−1 (số sibling khác) đã làm được.
