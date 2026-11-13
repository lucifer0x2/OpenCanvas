# Task26 report — OCD source input for `apply_batch`

Branch `feat/ocd-source` from `main` @ `0f42dd1`, uncommitted. Spec: `tasks/26-compact-authoring-dsl.md`.

## What changed

- `crates/server/src/ocd/`: `compile(&str) -> Result<Compiled, OcdError>` (operations plus
  the statement position of each); lexer, parser, value, keys (shorthand names), props,
  defaults (text sizes), expand (auto IDs), directives, set, error, and `apply` (the
  `apply_batch` glue: byte caps and error positions).
- `apply_batch` takes exactly one of `operations` or `source` (≤ 2 MiB). `tools/list`
  shows a flat object (both optional, like `render_frame`); `schemas::validate` enforces
  the one-of. `domain/batches.rs` calls `ocd::compile_into`, which writes the compiled
  `operations`, re-validates and checks the batch byte cap; the request then continues on
  the unchanged inline path (dryRun, pageIds, idempotency, storage errors).
- Parse errors: `SOURCE_PARSE_FAILED`, `field:"source"`, `line`, `column` (characters),
  `message`, `suggestion` (same top-level field as property errors). Errors on a compiled
  operation keep their code and fields and add `line`/`column` (see below).
- Docs: `skills/ocd.md` (4,082 B, registered in `SKILLS`), one paragraph in
  `author-frame.md`, one sentence in `instructions.rs` and the tool description.

## Review fixes (round 2)

| # | Finding | Fix | Covered by |
| --- | --- | --- | --- |
| 1 | `split` re-scanned the token per part (O(k·n)) | `value::split` cuts each part from the one scan (`Token::part`); no other per-part re-scan in lexer/parser/expand | `tests_rules::long_lists_compile_in_linear_time` (200k-item `p=` and `size=` lists, each < 2 s in debug; suite runs in ~0.1 s) |
| 2 | `@in id row gap=8 …` / `@in id "x"` dropped parts | `@in` takes one ID; any other token is `SOURCE_PARSE_FAILED` at that token; `@in` without children fails at the `@` | `tests_rules::in_takes_exactly_one_parent_id_and_needs_children`, `ocd_source_errors::in_rejects_anything_but_one_parent_id` |
| 3 | `note-<node>-<line>` collided across batches, could exceed 128 | `note-<nodeId cut to 110>-<12 hex FNV-1a of node+text>` (≤ 128); `id=` wins | `tests_rules::default_note_ids_depend_on_node_and_text_and_stay_valid` |
| 4 | Root `page` got `page-N` per source | Root pages need `#id` | `tests_rules::root_pages_need_an_id` |
| 5 | Post-compile errors had no position | Per-operation positions kept internally; schema errors locate the first failing operation (`schemas::invalid_operation`), storage errors with `opIndex` get its statement's `line`/`column`; empty result is `SOURCE_PARSE_FAILED` | `tests::every_operation_keeps_its_statement_position`, `ocd_source_errors::errors_after_compilation_point_at_their_statement` (index 30000, note > 2000 chars, `.missing` style; JSON input still has no position) |
| 6 | Text defaults fought the parent `align` | `defaults::text_size` (rule below) | `tests::text_size_defaults_never_defeat_the_parent_alignment`; golden updated |
| 7 | `ph.r=8`, `ph.stroke=…` skipped shorthands | `desc.key=` goes through the node-prop mapping into `overrides[desc]` | `tests_rules::dotted_descendant_keys_use_node_shorthands` |
| 8 | `@style` dotted-key guard was dead; `props={…}` skipped `STYLE_KEYS` | Both rejected at the key / value token | `tests_rules::style_rejects_dotted_and_non_style_raw_keys` |
| 9 | `@set x stroke=#000` reset width/align | `@set` accepts `stroke=` only as `color,width,align` or `{json}` | `tests_set::a_partial_stroke_is_rejected_and_a_full_one_compiles` |
| 10 | Parser edges | Non-ASCII whitespace in indent fails (`U+00A0`); non-string `comp name=` fails; `@token ink "#111"` accepted, missing arguments name the usage; `.5x10` is a size, a `.style` ID starts with a letter or `_`; first-line indent and indent under a childless directive say so | `tests_rules::parser_edges` |
| 11 | Caps and wording | Source cap checked in UTF-8 bytes (`SOURCE_PARSE_FAILED` at the crossing line); a source whose typed request (+ `pageIds`) exceeds 2 MiB fails at the statement that crosses it, `field:"source"`; instructions reworded and shortened | `tests_rules::source_cap_counts_utf8_bytes`, `ocd_source_errors::both_size_caps_fail_as_source_errors` |

Text size defaults (rule 6), checked against `validate/layout.rs` (only `fill` under a
hug/unset parent axis is rejected; width/height are optional for text) and the layout
engine (`align:"stretch"` stretches children whose cross size is unset): in a `col`
parent, width is `fill` with start/unset align and a non-hug width, unset under
`stretch`, else `hug`; height `hug`. In a `row`, width `hug`, height `hug` unless the row
stretches (unset). Free/no layout: `hug`/`hug`. Under `@in`: no size defaults, since the
stored parent is unknown and storage needs none. Authored sizes always win.

## Review fixes (round 3)

| # | Finding | Fix | Covered by |
| --- | --- | --- | --- |
| 1 | Cycle `directives` → `expand::position` → `directives` | Sibling-index parser moved to `value::index`; `expand` and `directives` (`@move`) call it; `directives` no longer imports `expand` | existing `index=`/`@move` tests, unchanged |
| 2 | Text defaults read only the base `layout`, so a stretching breakpoint was defeated by `width:"hug"` | `defaults::Parent` keeps one rule per layout (base + each `breakpoints[].layout`); each rule gives the round-2 default and an axis is written only when all rules agree. A malformed `breakpoints` (not an array, an entry without an object `layout`) writes no defaults; storage reports it. `definite_width` stays the parent's own width; parents without breakpoints and `@in` unchanged | `tests::text_size_defaults_hold_for_every_breakpoint_layout` (center+stretch → width unset; start+start → `fill`; column vs row → width unset; row-stretch vs column → both unset; 3 malformed shapes → none), `ocd_source::breakpoint_parents_and_numeric_styles_validate` (dry run + commit on temp SQLite) |
| 3 | `.123`, `.-ink`, `rect.123` rejected although `@style`/storage accept those IDs | `parser::parts` takes a complete `WxH` first (`.5x10` stays a size), then any other `.x` is a style; `props::style_id` uses `value::id` only (also for the head form); `.` alone is `SOURCE_PARSE_FAILED` "A .style needs an ID after the dot" at the dot | `tests_shorthands::dotted_styles_take_any_valid_id` (equal to `styleId=` form, `@set a .-ink`, `.5x10` size, `.`/`rect.` line+column); `tests_rules::parser_edges` now expects `rect.5x10` → `styleId:"5x10"` and drops the `.5` error case, which encoded the old restriction |

`ocd.md` defaults sentence now says breakpoints keep only sizes all layouts agree on; the
file stays 4,082 B (wording trimmed elsewhere). No limit or budget changed.

Module graph after round 3 (`A → B`: A uses B; acyclic among siblings): `lexer → error`;
`value, keys, parser → error, lexer`; `defaults →` none; `props → error, keys, lexer,
parser, value`; `set → props, keys, parser, value, lexer, error`; `directives → set, props,
keys, parser, value, lexer, error`; `expand → directives, defaults, props, parser, value,
lexer, error`; `mod::compile → lexer, parser, expand`; `apply → mod::compile`. Only the
module root's facade items (`Compiled`, `MAX_OPERATIONS`, `compile`) are used by children.

Checks (round 3): `--lib ocd::` 37 passed; `ocd_source` 5, `ocd_source_errors` 3,
`structured_authoring` 3, `structured_batch` 3, `tool_schemas` 2, `skill_layer` 2,
`protocol_2026` 2, `tools_budget` 1, `discovery_workflow` 1, `scoped_concurrency` 2 passed
(1 ignored); clippy `-D warnings` and `fmt --check` clean.

## `@set` and object props

`update_node` merges props only at the top level, so an object prop is replaced whole.
`@set` (`ocd/set.rs`) rejects, with `SOURCE_PARSE_FAILED` at the offending token, every
shorthand that would write part of an object, with a full example.

| `@set` input | Result |
| --- | --- |
| `row` `col` `free` `wrap`, `p=` `gap=` `align=` `justify=` (and `padding=`) | rejected → `layout={…}` |
| `desc.prop=` (e.g. `ph.fill=`) | rejected → `overrides={…}`; `layout.gap=`/`stroke.width=` name their own object |
| `ph="x"`, `ph={json}` (bare descendant key) | unknown key, message adds the `overrides={…}` hint, keeps the suggestion |
| `stroke=#000`, `stroke=$line`, `stroke=#000,2` | rejected → `stroke=#E5E7EB,1,inside` or `stroke={…}` |
| `stroke=c,w,a`, `stroke={json}`, `layout={json}`, `overrides={json}`, `props={json}`, any catalog `key={json}` | accepted: explicit full value |
| `fill=`, `r=a,b,c,d`, `w=$tok`, `size=a/b`, `WxH`, `.style`, `font=`/`family=`/`weight=`, scalars | accepted |

`@style` builds `upsert_style`, which replaces the style value whole; it rejects layout,
non-style keys, dotted keys and non-style keys inside `props={…}` (the earlier statement
that it "already rejects dotted keys" was wrong: that guard never fired; fixed in round 2).
`@token` writes whole tokens.

## Bytes

| | Bytes |
| --- | --- |
| Sample as OCD (`ocd.md` block) | 828 |
| Golden operations, compact JSON (`tests/fixtures/ocd-login.json`) | 2,622 (−68.4 % for OCD) |
| `tools/list` before Task26 → now | 29,516 → 29,734 (limit 32,768; unchanged by round 2) |
| `apply_batch` tool | 6,155 (limit 8,192; unchanged by round 2) |
| `discovery_workflow` total | 99,645 (< 100,000; unchanged by round 2) |
| `ocd.md` | 3,749 → 4,082 (≤ 4,096, asserted in `tests::guide_sample_…`) |
| `instructions.rs` authoring sentence | 267 → 240 (round 2) |

## Checks run (`CARGO_TARGET_DIR=/private/tmp/opencanvas-task26-target CARGO_BUILD_JOBS=4`, `--offline`)

Round 2 (after the fixes above):

| Command | Result |
| --- | --- |
| `cargo test -p opencanvas-server --lib ocd::` | 35 passed |
| `--test ocd_source --test ocd_source_errors --test structured_authoring --test structured_batch --test tool_schemas --test skill_layer --test protocol_2026` | 4 / 3 / 3 / 3 / 2 / 2 / 2 passed |
| `--test tools_budget --test discovery_workflow -- --nocapture` | pass; 29,734 B; total 99,645 B |
| `cargo clippy -p opencanvas-server --lib -- -D warnings` | clean |
| `cargo fmt -p opencanvas-server -- --check` | clean |

Earlier rounds: same suites passed (20 → 25 unit tests); `notes
note_operations_cost_little_in_tools_list` passed in round 1 and was not rerun.

## Line counts (hand-written)

After round 3. `ocd/`: mod 86, error 45, lexer 149, parser 223, value 110, keys 76,
props 188, defaults 93, expand 227, directives 172, set 83, apply 92; tests 168,
tests_errors 93, tests_rules 153, tests_set 127, tests_shorthands 179. `tests/ocd_source.rs` 215,
`tests/ocd_source_errors.rs` 100, `schemas/batch.rs` 54, `schemas.rs` 200,
`domain/batches.rs` 85. `ocd.md` 78 lines. Over 200 but under 250: `parser.rs`,
`expand.rs`. Pre-existing over threshold, grown by 3 lines only: `skills/author-frame.md`.

## Deviations from the draft

1. Published schema is flat with neither input required; strict one-of is server-side
   (`tool_schemas` forbids top-level combinators).
2. Added `@in parentId`: exactly one ID; indented lines under it create nodes inside an
   existing node; each needs `#id`, and at least one is required.
3. Root pages need `#id`.
4. Text size defaults follow the parent's layout (rule above), not only "fill under a
   column".
5. A trailing string on a non-text node sets `name`; it must be the last item on its line.
6. `index=` on a node sets the `create_node` index (operation field, not a prop).
7. `@note nodeId [id=noteId] "text"`; default note ID `note-<nodeId>-<hash of node+text>`.
8. One `.style` per node (the grammar allowed repeats; `styleId` is single).
9. `@token`/`@style` accept `name=`; `$x` as a token value is `{aliasId}`; a quoted token
   value is accepted; a style with only `fill` is a fill style, else a text style.
10. `rectangle` accepted as `rect`. `text`, `name`, `theme`, `fontId`, `fontFamily`,
    `styleId`, `assetId`, `componentId`, `flowStart` keep bare values as strings.
11. On `use`, a descendant key takes a string (text) or `{json}` (whole override);
    `desc.key=` takes any node key and shorthand. `stroke={json}` passes through,
    `stroke=[…]` maps positionally (on nodes; `@set` needs three items or `{json}`).
12. The 500-operation and 2 MiB limits are reported as parse errors at the crossing line.

## Limitations

- A source over 2 Mi characters fails the published `maxLength` first and gets the generic
  `INVALID_REQUEST`; only a source within that but over 2 MiB of UTF-8 gets
  `SOURCE_PARSE_FAILED`.
- Schema errors on compiled operations stay `INVALID_REQUEST` with no field; the position
  names the first operation that fails the published operation schema.
- `@set` edits of one layout field, one override or one stroke field need the whole object
  written out (no read-merge in the compiler, which never touches storage).
- Auto IDs depend on sibling position in the source, so editing a source and re-running
  it creates different IDs; edits use `@set`/`@del` on the IDs already created. Two
  identical `@note` lines on one node share a default ID; the second fails as a duplicate.
- Strings and JSON values must close on their line; `//` starts a comment only after
  whitespace. No `.ocd` in `apply_batch_file` (out of scope).
- `discovery_workflow` total is 99,645 B against its 100,000 B limit.
