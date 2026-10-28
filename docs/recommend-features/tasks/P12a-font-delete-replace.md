# P12a — Xoá font, thay font theo id, tự dọn asset mồ côi
Ưu tiên: P1 (dữ liệu thật đang kẹt 18 font cũ, và chặn P10c khi trùng mặt) · Kích cỡ: ~85 phút dev · Phụ thuộc: không về code. P10c (chọn mặt theo family/weight) chỉ chạm cùng vùng ở `validate/refs.rs`; xem mục "Quan hệ với P10c" · Nguồn: note #11 (`/Users/luis/Documents/opencanvas.md:51-52`)

## Vấn đề

- Không có cách gỡ một font khỏi project. Phiên QueryNest phát hiện subset thiếu U+2212 nên phải import bộ mới `font_sf2_*`. 18 record `font_sf_*` cùng asset của chúng vẫn nằm lại trong `list_fonts`/`list_assets`. Chúng tốn token mỗi lần đọc danh sách, và agent sau dễ chọn nhầm.
- Không có đường chính thức để **thay file** của một font đang dùng. Muốn sửa glyph, agent phải tạo id mới rồi `update_node` từng text (hàng trăm node), trong khi chỉ cần đổi bytes sau cùng một `fontId`.
- Asset không còn ai tham chiếu thì không gỡ được.
- Hai record cùng `(family, weight, style)` nhưng khác bytes còn gây lỗi render. Renderer coi chúng là một mặt; record nạp sau thắng (xem Bằng chứng).

## Bằng chứng

Code ở `night/integration@0847b4b`.

| Vị trí | Hiện trạng |
| --- | --- |
| `crates/storage/src/model/requests.rs:4-64` | `Operation` có `delete_subtree`, `upsert_token`, `upsert_style`, `*_note`. **Không có op nào xoá token/style/font/asset.** |
| `crates/storage/src/validate/candidate.rs:100-211` | `apply` khớp đúng các op trên. |
| `crates/storage/src/persistence.rs:42-69` | tokens/styles/assets/fonts chỉ có `INSERT … ON CONFLICT DO UPDATE`. Chỉ notes có `DELETE` (`:70-72`). Xoá record font/asset hiện không thể ghi xuống DB. |
| `crates/storage/src/assets.rs:93-122` | `import_asset` + `font` gọi `candidate.fonts.insert(font.id, …)`. **Font id đã tồn tại thì bị ghi đè lặng lẽ**: không kiểm trùng (`:94-100` chỉ kiểm định dạng và `font_bundled`). Persist upsert (`persistence.rs:63-69`). Như vậy "thay font theo id" đã chạy được tình cờ (đã kiểm live, xem dưới), nhưng không được ghi trong tài liệu hay schema. Asset cũ cũng không được dọn. |
| `crates/storage/src/assets.rs:58` | `asset_id = "asset_" + sha256[..32]`: asset đánh địa chỉ theo nội dung. Nhiều font record có thể dùng chung một asset (cùng bytes, khác metadata). |
| `crates/storage/src/assets.rs:124-125, 168-198` | Binary **cố ý không GC**, vì handle render/export có thể còn giữ. `orphan_asset_keys` chỉ liệt kê file trên đĩa không còn record. |
| `crates/storage/src/validate/properties.rs:66-81` (`fontId`, `assetId`), `validate/node.rs:26-32` (text style `fontId`) | Tham chiếu được kiểm khi validate. Nếu font bị gỡ trong khi còn người dùng, `candidate.validate()` dừng ở node đầu tiên với `field:"fontId"`, `Referenced font does not exist`. Lỗi không nói còn bao nhiêu node, cũng không nói cách sửa. |
| `crates/storage/src/validate/properties.rs:10` | Node được validate trên `resolve_props` (đã merge style). Text kế thừa font từ style cũng là "người dùng" của font. |
| `crates/storage/src/project.rs:101` | Batch áp op theo thứ tự, lỗi gắn `opIndex`. Agent trỏ text sang font khác ở op trước `delete_font` trong **cùng batch** là hợp lệ. |
| `crates/server/src/jobs/input.rs:24-37` | Scene đưa vào worker `fonts[{id, family, hash, url:/assets/<assetId>, weight, style}]`. Thay bytes làm đổi `hash` và `url`, nên scene của revision mới tự nạp file mới. |
| `renderer/resources.ts:78-92` | Renderer nạp `FontFace(family, bytes, {weight, style})` và **xoá mặt cũ cùng family/weight/style** trước khi `add`. Thay theo id nên chạy đúng. Hệ quả phụ: `font_sf_600` và `font_sf2_600` (cùng family/weight/style, khác bytes) trong cùng frame sẽ đè nhau, mặt nạp sau thắng. |
| `crates/server/src/reads/changes.rs:16-18,176` | `get_changes` xếp id không còn trong snapshot vào `removed`. Font/asset bị xoá tự hiện đúng, không phải sửa. |
| `crates/storage/src/bundle_history.rs:84-95` | Bundle import đọc lại `ops_json`: commit `import_asset` phải có **đúng một** op `{op:"import_asset", asset, font}`; commit khác phải parse được thành `Vec<Operation>`. |
| `crates/server/src/reads.rs:66-81` | `list_assets`/`list_fonts` là snapshot thô (bỏ `relativeKey`). Agent không biết asset/font nào đang không được dùng. |
| `crates/server/src/schemas/operations.rs:3-63`, `schemas.rs:134-146` | Schema op của `apply_batch` và mô tả `import_asset`. Ngân sách: `crates/server/tests/tools_budget.rs:18-20` (tổng 32 KiB, mỗi tool 8 KiB). |

**Kiểm live** (02:04, server tạm `--port 0`, data root tạm, binary `night/integration@0847b4b`; script `check.mjs` ở thư mục tạm của phiên, đã dọn):
- `import_asset` NotoSans với `font {id:"f_a"}` → `changedIds [asset_bfb7…, f_a]`. Tạo text `fontId:"f_a"`.
- `import_asset` lại bytes B (A + 1 byte) với **cùng** `font.id:"f_a"`:
  - commit rev 3, `changedIds [asset_39a0…, f_a]`;
  - `list_fonts`: `f_a.assetId = asset_39a0…`, nghĩa là **đã ghi đè**;
  - `list_assets` vẫn còn `asset_bfb7…` mồ côi;
  - `get_changes` từ rev 2: `assets [asset_39a0…]`, `fonts [f_a]`, `removed []`.
- `apply_batch [{op:"delete_font", id:"f_a"}]` → `INVALID_REQUEST`: `Arguments do not match the published tool schema`.

Kết luận: (2) đã chạy nhưng không ai biết và để lại rác; (1) và (3) chưa có.

**Tham khảo**
- Figma: modal "Missing fonts" liệt kê font và layer bị ảnh hưởng. Chọn family/style thay thế rồi "Replace fonts" thì mọi text trong file đổi theo (https://help.figma.com/hc/en-us/articles/360039956994-Missing-font-alert-in-Figma-Design). Figma có "Manage conflicting fonts" cho các phiên bản khác nhau của cùng một font (https://help.figma.com/hc/en-us/articles/4403175325719). OpenCanvas đi theo id nên "thay" là đổi bytes sau cùng id; hiệu quả giống nhau: một thao tác, mọi text đổi theo.
- Penpot: menu của custom font family cho phép đổi tên, tải về và xoá (https://help.penpot.app/user-guide/custom-fonts/). Trang không nói text đang dùng font bị xoá sẽ ra sao. Ở đây chọn **chặn** khi còn người dùng, vì note #12 cho thấy fallback âm thầm là thứ tốn thời gian nhất.
- Figma variable fonts (cho phase sau): https://help.figma.com/hc/en-us/articles/5579502031511-Use-variable-fonts.

## Giải pháp

Ba thay đổi, không thêm tool:
- 2 op mới trong `apply_batch`;
- `import_asset` chính thức hoá upsert theo `font.id`;
- `list_fonts`/`list_assets` đánh dấu `unused`.

### 1. Op `delete_font`

```json
{"op":"delete_font","id":"font_sf_600"}
```

Đọc trên candidate tại thời điểm của op (đã áp các op trước trong batch):
1. Font không tồn tại → `field:"id"`, `Font does not exist`.
2. `id == "font_bundled"` → `field:"id"`, `font_bundled is the built-in fallback and cannot be deleted`.
3. **Người dùng** của font:
   - text node có `props.fontId == id` (authored);
   - text node có `resolve_props(n).fontId == id`, tức kế thừa từ style, hoặc sau P10c được chọn theo family/weight;
   - text style có `value.fontId == id`.
   Có người dùng → `VALIDATION_FAILED`, `field:"id"`, kèm `opIndex`. Message liệt kê tối đa 10 node ID (theo thứ tự id), số còn lại, mọi style ID, và cách sửa:
   ```
   Font font_sf_600 is used by 14 text nodes (t_a, t_b, …, t_j and 4 more) and text styles body, label.
   Point them at another font with update_node/upsert_style earlier in this batch, or re-import
   this font id with import_asset to replace its file instead of deleting it.
   ```
4. Gỡ record khỏi `candidate.fonts`, `changed += id`.
5. **Tự dọn asset:** nếu `asset_id` của font không còn được font nào khác hay image node nào tham chiếu thì gỡ asset record luôn, `changed += assetId` (helper `release_asset`, mục 3).

### 2. Thay/upsert font theo id (qua `import_asset`, không đổi schema)

```json
{"projectId":"p","expectedRevision":41,"idempotencyKey":"sf-600-v2","mediaType":"font/woff2",
 "base64":"…","font":{"id":"font_sf_600","family":"SF Pro Text","weight":600,"style":"normal"}}
```

- Giữ hành vi upsert hiện có (`assets.rs:107`), nay thành hợp đồng có tài liệu và test. Mọi text dùng `font_sf_600`, trực tiếp hay qua style, đo và vẽ bằng file mới ở `committedRevision` mới. Không phải sửa node nào.
- Metadata (`family`/`weight`/`style`/`license`) cũng được thay. `candidate.validate()` chạy lại toàn bộ như hiện nay, nên nếu đổi metadata làm hỏng cách chọn mặt của P10c thì batch bị từ chối nguyên tử.
- Asset cũ khác asset mới mà không còn ai tham chiếu thì gỡ trong **cùng commit** (`release_asset`). `changedIds = [newAssetId, fontId, oldAssetId]`; `get_changes` đặt `oldAssetId` vào `removed`.
- `ops_json` của commit giữ đúng một op `{op:"import_asset", asset, font}`, để `bundle_history.rs:84-93` đọc lại được. Việc gỡ asset chỉ thể hiện trong `changedIds`.
- Re-import cùng bytes và cùng id thì asset trùng, không gỡ gì. Đây là cách đổi metadata, gửi lại bytes. Op riêng chỉ đổi metadata để phase sau.

### 3. Op `delete_asset` và helper `release_asset`

```json
{"op":"delete_asset","id":"asset_3fa1…"}
```

- Asset không tồn tại → `field:"id"`, `Asset does not exist`.
- Asset còn được dùng → `field:"id"`, chặn với message liệt kê:
  - font có `assetId == id` (mọi font);
  - image node có `props.assetId == id` (tối đa 10 node + số còn lại).
  Cách sửa: `delete_font those fonts or point image nodes at another asset first`.
- Gỡ record, `changed += id`.
- `release_asset(id)`, dùng chung cho 1.5 và 2: gỡ asset nếu không font/image node nào dùng, trả `Option<id>`.
- **Chỉ gỡ record trong DB**: `persistence.rs` thêm `DELETE FROM fonts|assets WHERE id=?` cho id có ở `old` mà không có ở `after`, giống notes `:70-72`. Binary trên đĩa giữ nguyên theo chính sách no-GC (`assets.rs:124-125`). Nó xuất hiện trong `orphan_asset_keys` và được GC ở phase sau. Bundle export (`bundle.rs:40-50`) chỉ lấy asset trong snapshot, nên tự gọn.

### 4. `unused` trong `list_fonts` / `list_assets`

- Mỗi item **không còn ai dùng** có thêm `"unused": true`, đúng các luật người dùng ở 1.3 và 3. Item đang dùng không đổi byte nào.
- `font_bundled` không bao giờ được đánh dấu.
- Agent thấy ngay 18 `font_sf_*` cần xoá mà không phải `find_nodes`.
- Tính trên snapshot đang đọc trong module mới `reads/usage.rs`: một lượt qua nodes và styles. Text dùng `resolve_props` không có ở server, nên dùng authored `fontId` cộng với `fontId` của style được tham chiếu. Sau P10c cần thêm resolved, xem "Quan hệ với P10c".

### Quy trình agent cho note #11 (2 lời gọi)

1. `list_fonts` thấy `font_sf_*` có `unused:true`.
2. `apply_batch` với 18 op `delete_font`. Asset của chúng tự được gỡ trong cùng revision.

Cách đúng hơn từ đầu: re-import `font_sf_*` với file mới, cùng id. Không cần bộ `font_sf2_*` và không cần sửa node.

### Vì sao chọn như vậy

| Phương án | Quyết định |
| --- | --- |
| **Op `delete_font`/`delete_asset` trong `apply_batch`** | Chọn. Nguyên tử với các op trỏ lại font trong cùng batch; có sẵn idempotency, conflict, `get_changes`; mỗi op ~196 B trong `tools/list`. |
| Tool mới `delete_font`, `replace_font` | Loại. Mỗi tool ≥ 500 B mô tả và schema cho việc hiếm gặp; không nguyên tử với các `update_node` trỏ lại font. |
| Thêm op `upsert_font` (metadata) trong `apply_batch` + `import_asset` chỉ tải bytes | Loại ở phase này. Upsert theo `font.id` đã có sẵn trong `import_asset` và đã atomic với asset. Thêm op chỉ để trùng chức năng. |
| `delete_font` tự trỏ node sang font khác (`replaceWith`) | Loại. Ẩn thay đổi trên hàng trăm node. Thay bytes theo id đã giải quyết việc "đổi font cho mọi text" mà không đụng node. |
| Xoá font đang dùng thì rơi về `font_bundled` | Loại. Fallback âm thầm làm sai đo và wrap (note #12). |
| `delete_font` không tự dọn asset, chỉ có `delete_asset` | Loại. 18 font thành 36 op, và để lại rác nếu agent quên. |
| Op `prune_assets` xoá mọi asset unused | Loại. Ảnh vừa import mà chưa gắn vào node sẽ bị xoá nhầm. `unused` + `delete_asset` tường minh an toàn hơn. |

### Quan hệ với P10c (`fontFamily`/`fontWeight`/`fontStyle`, đã giao dev)

- P10c chọn mặt ở `resolve_props` (`validate/refs.rs`). P12a chỉ **gọi** `resolve_props` để tìm người dùng, không sửa nó. Hai nhánh không sửa cùng hàm.
- Nếu P10c vào trước:
  - bước 1.3 tự bắt node chọn font theo family, vì đọc resolved `fontId`;
  - lỗi trùng mặt của P10c (`Fonts font_sf_600 and font_sf2_600 are both …`) được giải bằng `delete_font` một bên;
  - `reads/usage.rs` cần resolved `fontId` cho node dùng `fontFamily`. Server đã có resolved ở `reads/tree.rs:40-45`; nếu dev P12a đến sau P10c thì gọi cùng đường đó, còn không thì ghi FOLLOWUP.
- Nếu P12a vào trước: không có gì thay đổi với P10c.
- Thay font theo id giữ nguyên `(family, weight, style)` thì P10c chọn mặt như cũ.

## Phạm vi file

| File | Việc | Dòng (hiện → ước) |
| --- | --- | --- |
| `crates/storage/src/model/requests.rs` | `DeleteFont {id}`, `DeleteAsset {id}` | 156 → ~162 |
| `crates/storage/src/validate/resources.rs` (mới) | `delete_font`, `delete_asset`, `release_asset`, `font_users`, `asset_users`, format message | ~140 |
| `crates/storage/src/validate.rs` | `mod resources;` | 14 → 15 |
| `crates/storage/src/validate/candidate.rs` | 2 arm gọi sang `resources.rs` | 213 → ~217 |
| `crates/storage/src/persistence.rs` | `DELETE` fonts/assets đã gỡ | 91 → ~99 |
| `crates/storage/src/assets.rs` | ghi nhớ asset cũ của font trước `insert`, gọi `candidate.release_asset` sau, push vào `changed` | 213 → ~219 (đã > 200 từ trước; không thêm logic khác, helper nằm ở `resources.rs`) |
| `crates/server/src/schemas/operations.rs` | 2 variant `{id}` | 63 → ~65 |
| `crates/server/src/schemas.rs` | 1 câu trong mô tả `apply_batch`, 1 câu trong `import_asset` (mục MCP) | 161 → 161 |
| `crates/server/src/reads/usage.rs` (mới) + `reads.rs` | đánh dấu `unused` cho `list_fonts`/`list_assets` | ~60; 97 → ~102 |
| `crates/storage/tests/font_lifecycle.rs` (mới) | AC1–AC5, AC8 | ~180 |
| `crates/server/tests/font_lifecycle.rs` (mới) | AC6, AC7, AC9 qua MCP + render job | ~140 |
| `docs/SERVER.md`, `crates/server/skills/author-frame.md`, `crates/storage/README.md` | luật xoá/thay/unused | vài dòng mỗi file |

- Không đụng `crates/layout`, `renderer/`, `worker/`, `web/`, `validate/refs.rs`.
- File mới ≤ 180 dòng; `candidate.rs` và `assets.rs` chỉ thêm vài dòng.

## Tương thích MCP

- **Không thêm tool.** `tools/list` tăng khoảng **480 B**, ước tính bằng `printf … | wc -c` theo đúng hàm `object()`/`id()` ở `schemas.rs:38-49`:
  - variant `delete_font`: 196 B;
  - variant `delete_asset`: 197 B;
  - câu thêm vào mô tả `apply_batch`: ` delete_font/delete_asset refuse while anything uses them.` (~58 B);
  - câu thêm vào mô tả `import_asset`: ` An existing font.id is replaced for every text using it.` (~57 B).
  Còn xa trần 32 KiB/8 KiB.
- Op cũ, field cũ, response cũ giữ nguyên. `list_*` chỉ **thêm** `unused:true` cho item không dùng.
- Hành vi `import_asset` với font id đã có không đổi (vẫn upsert). Khác biệt duy nhất: asset cũ không còn ai dùng biến khỏi `list_assets` và id của nó có trong `changedIds`.
- Bundle: commit `delete_*` parse được bằng `Vec<Operation>` mới. Server **cũ** import bundle có op mới sẽ từ chối (giống notes trước đây), ghi trong `docs/BACKUP.md`.
- Tài liệu:
  - `SERVER.md` mục ops và `import_asset`;
  - `author-frame.md`: "Sửa font sai/thiếu glyph: import lại **cùng font.id**. Dọn: `list_fonts` → `delete_font` những font `unused`";
  - `crates/storage/README.md`: no-GC binary vẫn đúng.

## AC

Fixture: NotoSans bundled bytes (`renderer/fonts/NotoSans.ttf`) làm bytes A. Bytes B là A với một byte đuôi bị đổi, đủ để đổi hash. Signature TTF vẫn hợp lệ ở `assets.rs:208`; nếu dev cần font render được ở AC6 thì dùng một TTF thứ hai có sẵn trong repo. Import `fa` (font `f_a`, "Test Sans" 400) và `fb`.

1. **AC1 — Xoá font không dùng** (`storage/tests/font_lifecycle.rs`):
   - `delete_font f_b` → commit; `snapshot.fonts` không còn `f_b`.
   - `changedIds` chứa `f_b` và asset của nó, vì không font nào khác dùng asset đó.
   - `snapshot.assets` không còn asset đó.
   - Mở lại project (đọc từ SQLite), kết quả vẫn vậy.
2. **AC2 — Chặn khi còn dùng.** Có 12 text `fontId:"f_a"` và style `body {fontId:"f_a"}`:
   - `delete_font f_a` → `VALIDATION_FAILED`, `field:"id"`, `opIndex` đúng;
   - message chứa đúng 10 node ID đầu theo thứ tự, `and 2 more`, `body` và `import_asset`;
   - revision không đổi.
   - Text **chỉ** kế thừa `f_a` qua style (không có `fontId` authored) cũng tính là người dùng. Kiểm bằng một node `{styleId:"body"}` khi style đã trỏ font khác trong batch: node đó không còn bị liệt kê.
3. **AC3 — Trỏ lại rồi xoá trong cùng batch.** `[update_node × 12 {fontId:"f_b"}, upsert_style body {fontId:"f_b"}, delete_font f_a]` → commit một revision; mọi text resolve ra `f_b`. Đảo thứ tự (`delete_font` đứng đầu) → lỗi `opIndex:0`.
4. **AC4 — Không xoá được `font_bundled`** → `field:"id"`, message chứa `font_bundled`. Font không tồn tại → `Font does not exist`.
5. **AC5 — Thay theo id.** `import_asset` bytes B với `font {id:"f_a", …}` khi 12 text đang dùng `f_a`:
   - commit; `fonts[f_a].assetId` = asset của B, `contentHash` = hash B;
   - `changedIds` = `[assetB, "f_a", assetA]`; `assetA` không còn trong `snapshot.assets`, nếu không font nào khác dùng;
   - **không** node nào đổi props, các byte `props_json` giống trước;
   - replay đúng body và key → `replayed:true`, không commit thêm.
   - Biến thể: font `f_c` dùng chung asset A → thay `f_a` thì **giữ** assetA, và `changedIds` không chứa assetA.
6. **AC6 — Render theo revision mới** (`server/tests/font_lifecycle.rs`, server tạm như `tests/wrap.rs`):
   - `render_frame` ở revision trước khi thay → scene `fonts[f_a].hash` = hash A;
   - sau khi thay, ở `committedRevision` mới → `hash` = hash B, `url` kết thúc bằng `/assets/<assetB>`, `status:"ready"`;
   - `sceneKey` hai lần khác nhau.
7. **AC7 — `get_changes`.** Từ revision trước AC1 → `removed` chứa `f_b` và asset của nó. Từ trước AC5 → `fonts` chứa `f_a`, `assets` chứa assetB, `removed` chứa assetA.
8. **AC8 — `delete_asset`:**
   - asset của font đang có → lỗi liệt kê font id;
   - asset của image node → lỗi liệt kê node id;
   - image asset chưa ai dùng → commit, rồi `orphan_asset_keys()` chứa đúng key của nó (binary còn trên đĩa, không GC);
   - Bundle export → import roundtrip (`tests/bundle_roundtrip.rs`) sau AC1/AC5/AC8 → pass.
9. **AC9 — `unused` + MCP:**
   - `list_fonts` sau khi trỏ 12 text sang `f_b` → item `f_a` có `unused:true`; `f_b` và `font_bundled` không có key `unused`;
   - `list_assets` đánh dấu asset image chưa gắn.
   - `tools/list`:
     - không thêm tool (`tool_names` pass);
     - byte `apply_batch` + `import_asset` tăng ≤ 600 B so với trước, in bằng `cargo test -p opencanvas-server --test tools_budget -- --nocapture` và ghi vào `.night/dev/P12.md`;
     - `tools_budget` pass.
10. **AC10 — Không hồi quy:**
    - toàn bộ test hiện có của `opencanvas-storage`, `opencanvas-server` pass mà không sửa;
    - `cargo clippy` (lệnh ACCEPTANCE) và `cargo fmt --all --check` pass;
    - file mới ≤ 180 dòng; báo số dòng `candidate.rs`, `assets.rs`.
11. **AC11 — Live** trên server tạm:
    - `create_project` → `import_asset` NotoSans `font {id:"live_f"}` → text dùng `live_f` → `render_frame` `ready`;
    - re-import cùng `live_f` bằng TTF khác → `render_frame` revision mới `ready`, `inspect_layout` có text; `delete_font live_f` → lỗi liệt kê text;
    - `delete_subtree` text → `delete_font live_f` → commit; `list_fonts` không còn `live_f`;
    - dọn process và data.

## Cách kiểm

```bash
W=<worktree>; cd $W && export CARGO_BUILD_JOBS=4
cargo test --offline -p opencanvas-storage --test font_lifecycle --test bundle_roundtrip --test assets
cargo test --offline -p opencanvas-server --test font_lifecycle --test tools_budget --test changes -- --nocapture
cargo test --offline -p opencanvas-storage -p opencanvas-server
cargo clippy --offline -p opencanvas-storage -p opencanvas-server --lib --bin opencanvas-server -- -D warnings
cargo fmt --all --check
```

**Live:** `target/debug/opencanvas-server --port 0 --data-root $(mktemp -d)`, script node gọi MCP HTTP, dựa trên `.night/qa/mcp-tools.mjs`. Không đụng 3847, `~/.opencanvas` hay container.

## Rủi ro

- **Job render của revision cũ đang chạy khi asset bị gỡ.** Worker tải font qua `/assets/<id>`, route này tra record (`assets.rs:17-27`), nên sẽ gặp `ASSET_NOT_FOUND` và job đó `failed`. Job ấy đã thuộc revision cũ (render chỉ chấp nhận revision hiện tại, `jobs/input.rs:60-63`); job ở revision mới không bị ảnh hưởng. Chấp nhận được, ghi trong `SERVER.md`. Nếu QA thấy viewer nhấp nháy lỗi thì phase b cho route phục vụ theo `relative_key` còn trên đĩa.
- **Ghi đè font id ngoài ý muốn** đã có từ trước (`assets.rs:107`). P12a ghi thành hợp đồng, không thêm cờ `replace:true` để không đổi hành vi hiện có. Câu trong mô tả `import_asset` giúp agent biết.
- **Chi phí** `delete_font`: `resolve_props` trên mọi text cho mỗi op. 18 op × ≤ 20 000 node vẫn nhỏ so với `candidate.validate()` vốn đã resolve mọi node trên mọi theme. Nếu đo thấy chậm thì gom người dùng của mọi font một lần mỗi batch.
- **Bundle mới với server cũ:** op mới không parse được ở server cũ (`bundle_history.rs:95`). Đây là chiều tiến, giống notes.
- **`reads/usage.rs` trước P10c** dựa trên authored `fontId` + style. Sau P10c, node chỉ có `fontFamily` có thể làm font hiện `unused` sai cho tới khi usage đọc resolved. Khi đó `delete_font` vẫn chặn đúng vì storage dùng `resolve_props`, nên không mất dữ liệu.

## Ngoài phạm vi / phase sau

- **GC binary trên đĩa** (`orphan_asset_keys` → xoá file): cần lease cho handle render/export. Thuộc giới hạn đã công bố "log pruning/GC" (`docs/ACCEPTANCE.md`).
- **Op chỉ đổi metadata font** (`update_font {id, family?, weight?, style?}`) không gửi lại bytes: thêm khi có nhu cầu thật.
- **`replaceWith` khi xoá:** không làm, dùng thay theo id.
- **Variable font (`wght`/`opsz`/`wdth`), hướng thiết kế (P12b):**
  - Lúc `import_asset`, đọc bảng `fvar` (vài chục dòng parse OpenType, không cần crate nặng). `Font` có thêm `axes: {"wght":[100,900],"opsz":[17,28]}`, optional nên dữ liệu cũ không đổi.
  - `weight` của record biến thành mặc định trong dải.
  - P10c chọn record variable khi `fontWeight` nằm trong dải `wght` (ưu tiên mặt tĩnh khớp chính xác). Scene gửi `weight` thực + `axes`; renderer đăng ký `FontFace(family, bytes, {weight:"100 900"})` (dải descriptor CSS Fonts 4) và đặt `ctx.font` với weight số.
  - `opsz` theo `fontSize` (`font-optical-sizing:auto`). Canvas 2D chưa có `fontVariationSettings` chuẩn, nên **cần spike** xác nhận Chrome headless áp `opsz` khi vẽ canvas trước khi cam kết. Nếu không áp được thì bỏ `opsz`, chỉ hỗ trợ `wght`.
  - `get_design_context` cho coder: `font-variation-settings` chỉ xuất khi khác mặc định.
- Báo cáo font trùng `(family, weight, style)` khác bytes (renderer đè nhau, `resources.ts:84-91`): nên là diagnostic ở P11 (`MISSING_GLYPH` cùng nhóm) hoặc lỗi P10c. P12a cung cấp cách dọn.
