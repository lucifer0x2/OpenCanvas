# P08b — Stack: `alignSelf` từng con, `justify` space-around/space-evenly
Ưu tiên: P2 · Kích cỡ: ~40 phút dev · Phụ thuộc: **P08a** (bắt buộc; dùng `stack.rs`, `layout_rule.rs`, helper gợi ý giá trị và helper test của P08a) · Nguồn: note "Stack layout" (`/Users/luis/Documents/opencanvas.md:74-76`), phần mở rộng ghi trong P08a

## Vấn đề

**Ngoại lệ căn của một con.** P08a cho cả stack một `align`, nhưng UI thật hay có một con lệch khỏi quy tắc chung:
- nút "Xoá" căn phải trong form căn trái;
- divider kéo hết chiều ngang trong list căn giữa;
- avatar căn đầu trong row căn giữa.

Không có `alignSelf`, agent phải bọc con đó trong group riêng. Mỗi lần như vậy tốn thêm node và token, và AI coder thấy cấu trúc DOM thừa.

**Phân bố đều.** `justify` P08a chưa có `space-around`/`space-evenly`. Đây là các giá trị CSS, Penpot và Figma (`SPACE_AROUND`/`SPACE_EVENLY`) đều có, dùng cho tab bar hoặc bottom navigation chia đều. Agent port từ DOM (computed style) sẽ gặp lỗi validate và phải xấp xỉ bằng padding.

## Bằng chứng

- **Sau P08a:**
  - `align` chỉ ở container;
  - `justify` có 4 giá trị;
  - giá trị lạ bị từ chối với `field:"layout.justify"` (P08a, bảng lỗi).
- **Code hiện tại** (`night/integration` @ `c3ab012`):
  - Taffy `=0.15.1` có `Style.align_self: Option<AlignSelf>` (`taffy-0.15.1/src/style/mod.rs:783`) và `JustifyContent::SPACE_AROUND/SPACE_EVENLY` (`style/alignment.rs:371+`).
  - `Props` layout chưa có field tương ứng (`crates/layout/src/model.rs:53-77`).
- **Prototype** (cùng prototype với P08a, scratchpad `p08/layout`, probe `probe4`):
  - **space-around**, row 400, ba rect 50, gap 0 → x `41.671875, 175, 308.328125`; gap 10 → `38.328125, 175, 311.671875`.
  - **space-evenly**, cùng thiết lập: gap 0 → `62.5, 175, 287.5`; gap 10 → `57.5, 175, 292.5`. Gap vẫn là khoảng tối thiểu, như CSS.
  - **alignSelf**, column 400×300 padding 20 `align:"center"`, rect 100×50:
    - không khai → `(150,20)`;
    - `alignSelf:"start"` → `(20,70)`;
    - `alignSelf:"end"` → `(280,120)`.
  - **alignSelf stretch**, column 300×100 padding 10 `align:"center"`:
    - rect bỏ trống width + `alignSelf:"stretch"` → `(10,10,280,20)`;
    - rect `width:"hug"` + `alignSelf:"stretch"` → width 0 (hug thắng);
    - rect 40 không khai → `(130,50)`.
  - **Parent free**: `alignSelf` không có tác dụng, rect giữ `(5,5)`.
- **Tham khảo:**
  - CSS `align-self`: https://developer.mozilla.org/en-US/docs/Web/CSS/align-self
  - Figma `primaryAxisAlignItems` có `SPACE_EVENLY`, `SPACE_AROUND`: https://developers.figma.com/docs/plugins/api/properties/nodes-primaryaxisalignitems/
  - Figma đã deprecate MIN/CENTER/MAX từng con trong `layoutAlign`, chỉ giữ `STRETCH`: https://developers.figma.com/docs/plugins/api/properties/nodes-layoutalign/. Vì vậy ưu tiên P2.
  - Penpot justify có space-around/evenly: https://help.penpot.app/user-guide/designing/flexible-layouts/

## Giải pháp

**`alignSelf`**: prop cấp đầu, mọi kind.
- Giá trị `"start"`, `"center"`, `"end"`, `"stretch"`.
- Chỉ dùng bởi parent row/column. Ghi trong catalog "used by a row/column parent", giống `x`/`y` "used by a free parent".
- Align hiệu lực = `alignSelf` ?? `parent.layout.align` ?? `"start"`.
- Quy tắc "kích thước tường minh thắng" của P08a giữ nguyên:
  - `stretch` chỉ kéo con bỏ trống kích thước trục phụ;
  - `hug`/số giữ nguyên.

```json
{"id":"delete","kind":"group","parentId":"form","props":{"width":"hug","height":36,"alignSelf":"end","layout":{"mode":"row","padding":[0,16],"align":"center"}}}
```

**`justify`** thêm `"space-around"` và `"space-evenly"`, map `JustifyContent::SPACE_AROUND/SPACE_EVENLY`.
- Thiếu chỗ thì quay về start, như `space-between`. Đây là hành vi Taffy/CSS.

**Validate**
- Storage, qua helper giá trị của P08a:
  - `alignSelf` sai → `VALIDATION_FAILED`, `field:"alignSelf"`, message liệt kê 4 giá trị;
  - `suggestion` từ alias giá trị: `flex-start`/`MIN`/`left`/`top` → `start`, `flex-end`/`MAX`/`right`/`bottom` → `end`;
  - `SPACE_AROUND` → `space-around`, `SPACE_EVENLY` → `space-evenly`.
- Catalog thêm alias key `align-self`, `layoutAlign`, `selfAlign`. B02 sẽ gợi ý `alignSelf` khi agent gõ các tên đó, kể cả sai hoa thường.
- Layout crate: kiểm enum trước measurement → `LAYOUT_INVALID`.

**Không làm**: không từ chối `alignSelf` dưới parent free. Từ chối cần guard chéo node mỗi lần reparent hoặc đổi mode; đổi lại, catalog và skill ghi rõ là không có tác dụng.

## Phạm vi file

| File | Việc | Dòng (sau P08a → ước) |
| --- | --- | --- |
| `crates/layout/src/model.rs` | `Props.align_self: Option<String>` | ~210 → ~211 |
| `crates/layout/src/stack.rs` | `self_align` đọc `alignSelf`; 2 nhánh justify | ~60 → ~70 |
| `crates/layout/src/validation/layout.rs` | enum `alignSelf`, justify mới | ~85 → ~92 |
| `crates/layout/src/self_align_tests.rs` (mới, `#[cfg(test)]`) | AC1–AC3 | ~90 |
| `crates/storage/src/props_catalog.rs` | entry `alignSelf` (Any, value, alias) | +1 |
| `crates/storage/src/validate/layout_rule.rs`, `properties.rs` | enum + alias giá trị; `"alignSelf" => …` | ~150 → ~165, +1 |
| `crates/storage/tests/align_self.rs` (mới) | AC4 | ~80 |
| `crates/server/tests/stack_context.rs` | AC5 (+1 case) | ~130 → ~150 |
| `crates/server/skills/author-frame.md`, `implement-frame.md` | 1–2 dòng mỗi file (`alignSelf` → `align-self`, space-around/evenly) | +2 mỗi file |
| `docs/RESOLUTION.md` (mục P08) | 2 dòng | +2 |

Không file nào vượt 200 dòng. Không đụng `mcp.rs`, `schemas.rs`, renderer.

## Tương thích MCP

- `tools/list` không đổi (`props` tự do).
- Thiếu `alignSelf` → align của parent, nên bounds P08a không đổi.
- `props.md` tự thêm dòng `alignSelf` từ catalog.
- Test `prop_catalog.rs` tự phủ key mới cho mọi kind (`validator_accepts_exactly_the_catalog_keys_per_kind`).
- Không migration, không bump version.

## AC

1. **alignSelf (layout unit)**:
   - Column 400×300 padding 20 `align:"center"`, rect 100×50 → không khai `(150,20)`, `start` `(20,70)`, `end` `(280,120)`.
   - Row 300×60, rect 20×20 → `alignSelf:"center"` y 20, `"end"` y 40.
   - Parent free, `x:5,y:5`, `alignSelf:"center"` → `(5,5)`.
2. **alignSelf stretch (layout unit)**: column 300×100 padding 10 `align:"center"`:
   - bỏ trống width + `stretch` → `(10,10,280,20)`;
   - `width:"hug"` + `stretch` → width 0;
   - `width:40` không khai → `(130,50)`.
3. **justify mới (layout unit)**, row 400×50, ba rect 50×20:
   - `space-around`: gap 0 → x `41.671875, 175, 308.328125`; gap 10 → `38.328125, 175, 311.671875`.
   - `space-evenly`: gap 0 → `62.5, 175, 287.5`; gap 10 → `57.5, 175, 292.5`.
   - Overflow: hai rect 80 trong row 100 với `space-evenly` → x `0`, `80`.
4. **Storage**:
   - Chấp nhận 4 giá trị `alignSelf` trên rectangle, text, group; chấp nhận `justify` space-around/evenly.
   - Từ chối atomic:
     - `alignSelf:"middle"` → `field:"alignSelf"`;
     - `alignSelf:"flex-end"` → `suggestion:"end"`;
     - `justify:"SPACE_EVENLY"` → `field:"layout.justify"`, `suggestion:"space-evenly"`.
   - Key `align-self` và `layoutAlign` → `field:"props.<key>"`, `suggestion:"alignSelf"`.
   - `align_direct` layout crate: `alignSelf:"middle"` → `LAYOUT_INVALID`.
5. **MCP**: `get_design_context` của frame AC1 (viewports 400×300, 800×600):
   - `props.alignSelf` trả nguyên văn;
   - `computedBounds` của con `start` có x `[20,20]`, con không khai có x `[150,350]`;
   - `reflow:true`.
6. **Hồi quy**: toàn bộ test P08a, layout, storage, server, `npm run test:pipeline` pass không sửa; `prop_catalog.rs` pass với key mới; clippy `-D warnings`, `cargo fmt --check` sạch; file đã chạm ≤ 200 dòng.

## Cách kiểm

```bash
CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-layout -p opencanvas-storage
CARGO_BUILD_JOBS=4 cargo build --offline -p opencanvas-layout --bin layout-step
CARGO_BUILD_JOBS=4 cargo test --offline -p opencanvas-server --test stack_context --test props_reference --test tool_schemas
npm run test:pipeline
cargo clippy --offline -p opencanvas-layout -p opencanvas-storage -p opencanvas-server --lib --bin opencanvas-server -- -D warnings
```

**Kiểm live** (QA): server tạm `--port 0 --data-root $(mktemp -d)`.
1. Tạo form column `align:"stretch"` với nút `alignSelf:"end"` `width:"hug"`.
2. Render 1040 và 390 rồi xem PNG: nút sát phải ở cả hai, input kéo hết chiều ngang.
3. Dọn process/data.

## Rủi ro

- **`alignSelf` bị bỏ qua dưới parent free (không lỗi).**
  - Agent có thể tưởng nó có tác dụng.
  - Giảm thiểu: value trong catalog ghi "used by a row/column parent"; skill nhắc.
  - Đây là cùng tiền lệ `x`/`y` trong stack.
- **`alignSelf:"stretch"` + `hug`/số không kéo** (khác Figma `layoutAlign:"STRETCH"`, vốn kéo bất kể).
  - Giữ một quy tắc với P08a: kích thước tường minh thắng.
  - Message catalog: "stretch applies when the cross size is omitted".
- **Xung đột file**: chỉ với P08a (phụ thuộc thẳng) và P04a (`props_catalog.rs`, skills). Dev rebase khi cần.

## Ngoài phạm vi / phase sau

- **`align:"baseline"`**: cần ascent thật từ đo text (`crates/layout/src/engine.rs:40` đang trả baseline 0, note #5). Làm sau đề xuất fidelity baseline.
- **Absolute trong stack** (Figma `layoutPositioning:"ABSOLUTE"`).
- **Không làm**: `order`, `flex-grow` có trọng số (`fill` có hệ số), `align-content`, overflow `safe`.
