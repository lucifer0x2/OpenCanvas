```markdown
# Cấu trúc Database PostgreSQL Tối ưu cho OpenCanvas

> **Bối cảnh:** OpenCanvas cần chuyển từ SQLite sang PostgreSQL để hỗ trợ rolling update trên Kubernetes. Thách thức cốt lõi không nằm ở việc lưu hàng nghìn node, mà ở chỗ mỗi thao tác của agent phải nhanh, nguyên tử (atomic) và không gây khóa cục bộ trên toàn bộ cây.

---

## 1. Nguyên tắc thiết kế

| Nguyên tắc | Lý do |
| :--- | :--- |
| **Mỗi node là một hàng** (không phải một document JSONB khổng lồ) | Row-level lock cho phép nhiều agent sửa các node khác nhau song song; tránh ghi lại toàn bộ canvas mỗi khi sửa một thuộc tính nhỏ |
| **Adjacency List + Materialized Path (ltree)** | Adjacency List đảm bảo toàn vẹn tham chiếu; ltree cho phép truy vấn subtree/ancestor tức thời mà không cần recursive CTE |
| **JSONB chỉ cho thuộc tính linh hoạt** | Các trường cấu trúc (`parent_id`, `path`, `node_type`, `sort_order`) tách thành cột quan hệ để index B-tree và FK hoạt động hiệu quả |
| **Batch update qua `jsonb_to_recordset`** | Một round-trip duy nhất cho hàng nghìn node, planner tối ưu toàn bộ phép join |
| **Optimistic concurrency + Idempotency** | Chốt revision toàn cục và operation log chống ghi đè, chống trùng lặp khi retry |

---

## 2. Mô hình dữ liệu cốt lõi

### 2.1. Bảng `projects` — trạng thái gốc và revision

```sql
CREATE TABLE projects (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name             TEXT NOT NULL,
    current_revision BIGINT NOT NULL DEFAULT 0,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

`current_revision` là "chốt chặn" cho mọi thao tác ghi của agent. Mỗi batch update phải kèm `expected_revision`; nếu không khớp, transaction bị từ chối. Đây là cơ chế **optimistic concurrency** đã được OpenCanvas áp dụng qua MCP, giờ được hiện thực hóa ở tầng database.

### 2.2. Bảng `nodes` — mỗi node là một hàng

```sql
CREATE EXTENSION IF NOT EXISTS ltree;

CREATE TABLE nodes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id  UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    parent_id   UUID REFERENCES nodes(id) ON DELETE CASCADE,
    path        LTREE NOT NULL,            -- ví dụ: 'proj_abc.frame_1.title_2'
    node_type   TEXT NOT NULL,             -- 'frame', 'text', 'rect', ...
    props       JSONB NOT NULL DEFAULT '{}'::jsonb,
    sort_order  INT NOT NULL DEFAULT 0,    -- thứ tự giữa các sibling
    revision    BIGINT NOT NULL DEFAULT 1, -- revision gần nhất của riêng node
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_nodes_project_parent ON nodes (project_id, parent_id);
CREATE INDEX idx_nodes_project_path    ON nodes USING GIST (path);
CREATE INDEX idx_nodes_props_gin       ON nodes USING GIN (props jsonb_path_ops);
```

#### Vì sao dùng `ltree` thay vì chỉ `parent_id`?

- Với `parent_id` thuần túy (adjacency list), mọi truy vấn "lấy toàn bộ subtree" đều phải dùng `WITH RECURSIVE`. Với cây vài nghìn node, recursive CTE vẫn chạy được, nhưng **sâu và thường xuyên truy vấn ancestor** sẽ chậm dần.
- `ltree` lưu toàn bộ đường dẫn tổ tiên trong một cột, cho phép toán tử `@>` (ancestor) và `<@` (descendant) chạy trên **GIST index**. Ví dụ: lấy toàn bộ node con của `frame_1` chỉ là `WHERE path <@ 'proj_abc.frame_1'`, không cần đệ quy.
- **Đánh đổi:** Di chuyển subtree (đổi `parent_id` của một node) đòi hỏi cập nhật `path` cho tất cả node con. Tuy nhiên, trong OpenCanvas, thao tác **thêm/sửa/xóa node** phổ biến hơn nhiều so với **di chuyển cả subtree**, nên `ltree` mang lại lợi ích đọc lớn hơn chi phí ghi không thường xuyên.

#### Vì sao `props` là JSONB nhưng tách khỏi `path` và `parent_id`?

- Các trường cấu trúc (`parent_id`, `path`, `node_type`, `sort_order`) được tách thành cột quan hệ để **index B-tree và ràng buộc khóa ngoại** hoạt động hiệu quả.
- Chỉ những thuộc tính **linh hoạt, thay đổi thường xuyên** (màu sắc, font, kích thước, nội dung text) mới nằm trong `props` JSONB.
- GIN index trên `props` chỉ nên dùng `jsonb_path_ops` (nhỏ hơn, nhanh hơn cho truy vấn containment) và **tránh index toàn bộ** nếu không có truy vấn cụ thể, vì GIN trên JSONB lớn sẽ bị bloat khi cập nhật thường xuyên.

### 2.3. Bảng `node_operations` — nhật ký batch và idempotency

```sql
CREATE TABLE node_operations (
    id              BIGSERIAL PRIMARY KEY,
    project_id      UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    idempotency_key TEXT NOT NULL,
    revision        BIGINT NOT NULL,
    operations      JSONB NOT NULL,   -- mảng operation đã áp dụng
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (project_id, idempotency_key)
);

CREATE INDEX idx_node_operations_project_rev
    ON node_operations (project_id, revision DESC);
```

Bảng này không chỉ là log — nó là **cơ chế idempotency thực sự**. Khi agent gọi `apply_batch` với một `idempotency_key`, backend thực hiện `INSERT ... ON CONFLICT DO NOTHING` vào bảng này **trong cùng transaction** với việc cập nhật `nodes` và `projects.current_revision`. Nếu key đã tồn tại, transaction trả về kết quả cũ mà không áp dụng lại thay đổi. Điều này giải quyết triệt để vấn đề retry khi mạng chập chờn hoặc agent gửi trùng lặp.

---

## 3. Chiến lược Batch Update cho Agent

### 3.1. Cập nhật nhiều node trong một câu lệnh

Agent không nên gọi `UPDATE` riêng lẻ cho từng node. Toàn bộ batch phải nằm trong **một transaction**, nhưng cần **chia nhỏ nếu batch quá lớn** (ví dụ > 5000 node) để tránh giữ lock lâu và tạo WAL quá lớn.

```sql
WITH updates AS (
    SELECT *
    FROM jsonb_to_recordset($1::jsonb) AS t(
        id         UUID,
        parent_id  UUID,
        path       LTREE,
        node_type  TEXT,
        props      JSONB,
        sort_order INT
    )
)
UPDATE nodes n
SET parent_id  = u.parent_id,
    path       = u.path,
    node_type  = u.node_type,
    props      = u.props,
    sort_order = u.sort_order,
    revision   = n.revision + 1,
    updated_at = now()
FROM updates u
WHERE n.id = u.id
  AND n.project_id = $2;
```

> **Quan trọng:** Không dùng `UPDATE ... FROM` với một danh sách dài các `VALUES` rời rạc, vì PostgreSQL sẽ phải xử lý từng hàng riêng lẻ. `jsonb_to_recordset` cho phép truyền toàn bộ batch dưới dạng một tham số JSON duy nhất, giảm round-trip và cho phép planner tối ưu toàn bộ phép join.

### 3.2. Tính `path` ở backend, không ở agent

Nếu agent cần **thêm node mới** với `path` được tính từ parent, hãy để backend tính `path` thay vì agent. Agent chỉ gửi `parent_id` và `node_type`; backend đọc `path` của parent, nối thêm ID mới, rồi insert. Điều này tránh agent phải tự xây dựng đường dẫn và giảm nguy cơ sai `ltree`.

---

## 4. Vì sao không nên lưu toàn bộ canvas trong một cột JSONB duy nhất?

Mặc dù JSONB rất tiện cho việc đọc toàn bộ canvas để render, việc dùng nó làm **nguồn dữ liệu chính** sẽ tạo ra hai vấn đề nghiêm trọng khi agent thao tác với hàng nghìn node:

1. **Chi phí ghi lại toàn bộ document:** Khi agent sửa `props.fontSize` của một node, PostgreSQL phải ghi lại toàn bộ JSONB document (có thể vài MB), đọc lại document đó cho mỗi index liên quan, và cập nhật GIN index. Với batch hàng nghìn thay đổi, chi phí này nhân lên theo cấp số nhân.
2. **Khóa và bloat:** Một `UPDATE` trên document lớn sẽ khóa toàn bộ hàng, ngăn các agent khác đọc/ghi đồng thời trên các node khác nhau trong cùng project. GIN index cũng nhanh chóng bị bloat bởi các dead tuple.

Mô hình **mỗi node một hàng** cho phép PostgreSQL khóa ở mức hàng (row-level lock), nghĩa là agent A sửa node X và agent B sửa node Y **có thể chạy song song** mà không chặn nhau. Đây là lợi ích kiến trúc quan trọng nhất khi làm việc với hàng nghìn node.

> **Gợi ý bổ sung:** Bạn vẫn có thể giữ một bảng `canvas_snapshots` **chỉ để đọc** (materialized view hoặc bảng snapshot cập nhật bất đồng bộ) phục vụ render nhanh. Snapshot này được rebuild sau mỗi batch thành công, không tham gia vào đường ghi của agent.

---

## 5. Tóm tắt cấu trúc đề xuất

| Thành phần | Vai trò | Lý do |
| :--- | :--- | :--- |
| `projects.current_revision` | Chốt revision toàn cục | Optimistic concurrency, chặn ghi đè |
| `nodes` (mỗi node một hàng) | Nguồn dữ liệu chính | Row-level lock, cập nhật từng phần hiệu quả |
| `nodes.parent_id` (Adjacency List) | Toàn vẹn tham chiếu | FK enforcement, truy vấn sibling đơn giản |
| `nodes.path` (ltree + GIST) | Truy vấn subtree/ancestor | Tránh recursive CTE, đọc nhanh trên cây sâu |
| `nodes.props` (JSONB + GIN `jsonb_path_ops`) | Thuộc tính linh hoạt | Cân bằng giữa schema linh hoạt và index hiệu quả |
| `node_operations` | Idempotency & audit log | Chống trùng lặp batch, hỗ trợ debug và rollback |
| Batch update qua `jsonb_to_recordset` | Ghi hàng nghìn node | Một round-trip, planner tối ưu toàn batch |

---

## 6. Kết luận

Với cấu trúc này, agent có thể thao tác trên hàng nghìn node thông qua các batch nguyên tử, trong khi PostgreSQL vẫn giữ được khả năng mở rộng đọc/ghi đồng thời — điều mà cả mô hình "một JSONB khổng lồ" lẫn "chuẩn hóa quá mức" đều không làm tốt.

**Điểm mấu chốt cần ghi nhớ:**

- **Mỗi node một hàng** → row-level lock → multi-agent concurrency.
- **ltree** → truy vấn cây tức thời, không recursive CTE.
- **JSONB có chọn lọc** → linh hoạt mà không phá vỡ index.
- **`node_operations` + `expected_revision`** → idempotency và optimistic concurrency ở tầng database.
- **`jsonb_to_recordset`** → batch update hàng nghìn node trong một round-trip.
```

Tôi đã tổng hợp toàn bộ đề xuất thành một file markdown hoàn chỉnh, có cấu trúc rõ ràng với 6 phần: nguyên tắc thiết kế, mô hình dữ liệu, chiến lược batch update, phân tích trade-off, bảng tóm tắt và kết luận. Bạn có thể lưu trực tiếp thành `docs/database-schema.md` trong repository.