# Discovery và semantic text diagnostics — 2026-10-30

Nhóm3 của [implementation](DESIGN-PROPERTIES-IMPLEMENTATION.md), trên checkout
`codex/p24-components-instances`, HEAD nền `8931abd0ad5de2538d433079f7200f65eaf11d03`.
Giữ pipeline Chromium và bản tổng hợp chưa phát hành `scene-3` / `web-4`.
Không commit/push, restart3847 hoặc ghi dữ liệu project thật.

## Kết quả

- `opencanvas://capabilities` là resource JSON tĩnh, private/cacheable, đọc không
  cần tạo project. Accepted keys và keysByKind sinh trực tiếp từ `PROPS`/
  `allowed_props`; fingerprint gồm catalog, consumer facts và pipeline versions.
  Test bắt key chưa phân loại và kiểm versions Rust/TS, diagnostic SVG tương ứng.
- Canvas/PNG, HTML preview và SVG có facts riêng. State đã chọn được áp trước SVG;
  animation nghỉ trong export. Gradient/radius có hỗ trợ; stroke/shadow SVG có
  diagnostic bỏ qua rõ ràng. Text có measured lines + embedded fonts, không outlines.
  Resource phân biệt static support với runtime resource/glyph readiness.
- Author/implement skills và connect instructions giữ paragraph/label, source spaces,
  hard breaks và typography semantic. Character art, wordmark, monogram, per-letter
  motion hợp lệ; ghi approximation/intent bằng notes, không thêm props ngoài catalog.
- `TEXT_FRAGMENTATION_SUSPECTED` quét đầy đủ frame effective đã resolve/expand trước
  depth/properties/pagination. Nó kết hợp ≥8 fragment một chữ cái, baseline/typography
  chung và free placements dày. Font selector, metrics, spacing, case, alignment,
  decoration và fill tham gia nhóm. Không group text còn thiếu metrics đã resolve.
  Chỉ advisory, không reject/merge/reconstruct source hoặc đoán spaces/order.
- Loại heuristic khỏi hidden content, row/column controls, named lettering/art,
  motion và `Intentional lettering:` notes trên node/ancestor/source component.
  Kết quả có virtual instance IDs, tối đa8 groups ×6 IDs và summary khi lược bớt.
- `TEXT_CONTENT_OUTSIDE_BOX` chạy một lần trên immutable measured scene, sau selected
  state và trước render/export. Evidence tách advance width khỏi glyph ink bounds,
  đối chiếu own box và rectangular ancestor clips; bỏ content có ancestor opacity0.
  Tối đa12 nodes ×2 line examples và omission summary, không chứa source text.
  Mỗi item chỉ mang `sourceSceneKey`; một `TEXT_OVERFLOW_SUMMARY` mỗi job
  (`reportedNodes`, `omittedNodes`) mang provenance đầy đủ một lần: scene key,
  viewport, theme/state, versions và font/asset manifest hashes.
- Advisory đo được không bao giờ tự làm read lỗi. Context (và `inspect_layout` có
  `budget`) cho chúng tối đa `budget/4`; vượt thì nén theo bậc cố định: bỏ provenance,
  bỏ evidence của item, bỏ item round-robin theo viewport, cuối cùng một summary
  chung. Summary nén ghi `compaction {step, droppedNodes, retrieve}`; mọi page cùng
  read giống hệt nhau. Sample `welcome` 2 viewport: diagnostics 13.940 B →
  10.179 B (budget mặc định, không nén) → 2.378 B ở budget 8192.
  `Intentional overflow:` notes hạ severity thành info; không đổi authored data.
- Render/inspect/context tái dùng warning của cùng job. Context chỉ có measured
  overflow khi requested viewports đã ready; authored-only read không bịa số đo.
  Node filters của inspect thu hẹp node diagnostics, field/page filters giữ warning.
  Viewer và preview chuyển structured diagnostics thành labels có code/node/message,
  giữ legacy strings, không hiển thị `[object Object]` hay dump examples.

## Kiểm đã chạy lại trên bản tích hợp

Các test MCP mở `127.0.0.1:0`, dùng `tempfile::TempDir`; worker subprocess có
kill-on-drop, server task bị abort khi test kết thúc. Sandbox chặn listener ban đầu;
chạy ngoài sandbox với quyền duyệt sẵn đã thành công. Không dùng port3847/data thật.

| Lệnh | Kết quả và bằng chứng |
| --- | --- |
| `cargo test --offline -p opencanvas-server --test text_discovery --test text_overflow --test discovery_workflow -- --test-threads=1 --nocapture` | 5/5 pass. Catalog/cache hints + unknown-key rejection; resolved styles/dimension token; differing tracking; virtual instances; art/toolbar/monogram/note exceptions; full-scope pages/filters; actual Chromium overflow + context/inspect equality |
| `cargo test --offline -p opencanvas-server --lib text_diagnostics::tests -- --nocapture` | 4/4 pass: advance/ink independent, absent ink explicit, ink overhang despite fitting advance, clip-only evidence, hidden ancestor, bounded examples/omission count |
| `cargo test --offline -p opencanvas-server --lib capabilities::tests -- --nocapture` | 1/1 pass; all live accepted keys classified, kind keys exact, versions and SVG diagnostic declarations checked |
| `node --test web/tests/diagnostics.test.ts` | 1/1 pass; legacy/structured/malformed values have readable labels without exposing evidence. Registered in `web/package.json` test gate |
| `npm --prefix web run typecheck` | pass |
| Targeted `rustfmt`, Prettier check and raw `git diff --check` for this lane | pass; no global full build/suite claim |

Actual measured fixture: `Private paragraph copy` remains only in authored fixture,
never in warning payload. Its 20×8 box at x300 yields advance218.25px versus measured
ink216.2998046875px, and `clipNodeIds:["frame"]`. Its note produces severity info.
The same warning appears in render, inspect and ready context, survives pagination
and properties/fields filters; `nodeIds:["clean"]` excludes it. A later revision2
changes authored bounds, while inspection of the old job keeps revision1 evidence.

## Workflow bytes và provenance

`discovery_workflow` creates one temporary frame + semantic paragraph, then measures
compact JSON results of tools/list, resources/list, author/implement/props reads,
one capability read and one compact authored context. Resource calls measure MCP
`result`; context measures `structuredContent`. Excludes outer JSON-RPC/SSE framing,
requests, initialize instructions, write fixture, rendering/PNG/fonts and retries.
This is a discovery response budget, not total session traffic or billed tokens.

| Bước hiện tại | Bytes |
| --- | ---: |
| tools/list result | 25,199 |
| resources/list result | 543 |
| author-frame read | 13,075 |
| implement-frame read | 12,025 |
| props read | 17,689 |
| capability read, một lần/cacheable | 11,076 |
| compact context structuredContent | 1,315 |
| **Tổng đã đo lại** | **80,922** |

Tools array riêng là25,151B, wrapper result thêm48B; không phải hai lần đo mâu thuẫn.
Mốc trước65597B và tools-array25151B là capture của worker trước khi công cụ bị ngắt;
raw output không còn được giữ, nên phiên hoàn tất **không tái chứng minh baseline**.
Current cost gồm các thay đổi text/face/paint đã tích hợp, không quy toàn bộ delta cho
discovery. Không có claim giảm payload/token/hoá đơn; capability tốn thêm một lần đọc,
được cache theo fingerprint/version và không lặp trong từng node/context.

## Phạm vi và kích thước

Heuristic bảo thủ có false negatives và ngoại lệ theo name/note; không là detector
ngôn ngữ hay xác minh source của QueryNest. Chưa quét general contrast/tap-target/
overlap của P29, chưa rich runs, RTL authored, ellipsis/maxLines hay auto-repair.
Ink không gồm mọi effect/decoration; clip evidence xét hình chữ nhật, không hứa pixel
coverage của rounded clips. Glyph coverage thuộc nhóm text, không do resource tĩnh
chứng minh. Chưa kiểm UI warning bằng browser screenshot ở lane này; unit/typecheck
xác minh mapping dùng chung của viewer/preview. Paint visual/SVG parity thuộc báo cáo
paint; capability ghi subset đã được worker paint xác nhận và checks catalog.

Mỗi source module một nhiệm vụ: capability; fragmentation; intent; measured overflow;
UI label. Source/test mới15–154 dòng. Existing integration source lớn nhất226 dòng
(`jobs.rs`); `reads/tree.rs`204 dòng. Không source của lane này vượt250 dòng.
Skills191/167 dòng; `docs/SERVER.md` là reference doc có sẵn, không áp giới hạn mã.
`AGENTS.md` giữ64 dòng/7,991B. Không sửa dữ liệu/font/research của lane khác.
