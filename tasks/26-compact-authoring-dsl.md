# Task26 — OCD source input for `apply_batch`

**Status:** ready to assign. Not started. Requested by the user on 2026-11-13; scope
narrowed the same day to: a DSL → JSON wrapper in `apply_batch` plus agent docs.

## Why

A small login card is 2,905 bytes as JSON operations (key names 49 %, copy text 6 %;
`op`, `parentId` and `props` repeat on every node). The same design in the OCD sample
below is 828 bytes (−71 %). Byte counts only; tokens were not measured.

## Scope

One worker, workspace `/Users/luis/Workspace/OpenCanvas`, own worktree and branch
`feat/ocd-source` from the integration branch the root names at handoff. No deploy,
service restart, DB manipulation or push. Files below 250 lines.

- New module `crates/server/src/ocd/`: `mod.rs` (public `compile(&str) -> Result<Value,
  OcdError>` returning an `operations` array), `lexer.rs`, `parser.rs`, `expand.rs`
  (shorthands, defaults, auto IDs).
- `crates/server/src/schemas/batch.rs`: optional `source` string (≤ 2 MiB); exactly one of
  `operations` or `source`. The MCP dispatch validates the schema first, so `operations`
  must stop being unconditionally required. One sentence in the tool description.
- `crates/server/src/domain/batches.rs`: when `source` is present, compile it into
  `operations` and continue exactly as today (the file path already does this
  conversion before `schemas::validate("apply_batch")`). Validation, atomicity, `pageIds`,
  `dryRun` and idempotency then work unchanged on the compiled operations.
- Docs: new resource `crates/server/skills/ocd.md` (≤ 4 KB, registered in `SKILLS` in
  `resources.rs`) with grammar, shorthand table, defaults and the sample; a short pointer
  in `author-frame.md` and `instructions.rs` saying `source` is the preferred input and
  JSON `operations` remains supported.

Wrapper rules:

- A node without `#id` gets `<parentId>-<position>` (1-based among its siblings);
  duplicate IDs in one source are parse errors.
- Defaults written into the output (stored data stays explicit): text under a column gets
  `width:"fill",height:"hug"`, elsewhere `width:"hug",height:"hug"`; token/style `name`
  defaults to the ID; token `type` is `color` for `#hex`, `dimension` for a number,
  otherwise `type=` is required.
- Parse errors return `SOURCE_PARSE_FAILED` with `line`, `column`, `message` and, for an
  unknown key, a suggestion. Storage errors pass through unchanged.
- Every shorthand target is an existing key in `props_catalog.rs` (one test).

## OCD v1

```
line      := indent (directive | node) [ '//' comment ]      indent = 2 spaces per level
node      := kind ['#' id] {'.' styleId} {attr | flag} [string]
kind      := page | frame | group | rect | text | image | comp | use <componentId>
attr      := key '=' value
value     := number | W 'x' H | percent | fill | hug | '#'hex | '$'token
           | a,b[,c,d] | "json string" | {json} | [json]
flag      := row | col | free | wrap
directive := @token id value {theme=value} | @style id {attr}
           | @set id {attr} | @del id | @move id parentId [index] | @note nodeId string
```

| Shorthand | Expands to |
| --- | --- |
| `400x44`, `w=`, `h=` | `width`, `height` |
| `r=` | `radius` |
| `row` / `col` / `free`, `wrap` | `layout.mode`, `layout.wrap` |
| `p=32`, `p=0,12`, `gap=`, `align=`, `justify=` (`between` = `space-between`) | `layout.*` |
| `.body` / `$ink` | `styleId` / `{"tokenId":"ink"}` |
| `stroke=#E5E7EB,2,inside` | `stroke {color,width,align}` |
| `size=24/32`, `font=`, `family=`, `weight=` | `fontSize`/`lineHeight`, `fontId`, `fontFamily`, `fontWeight` |
| `comp#input name=Input` | frame with `component {name}` |
| `use#email input ph="…"`, `ph.fill=$pri` | instance `componentId`, `overrides` |
| `key={json}`, `props={json}` | raw value / props merged last (gradients, states, animations) |

Full catalog keys are always accepted. On `use`, a key that is not an instance prop is a
master descendant ID: a string overrides its `text`, `desc.prop=value` any other
override-capable prop; nested virtual IDs use `overrides={json}`.

```
@token ink #111827
@token muted #6B7280
@token pri #2563EB
@token line #D1D5DB
@style h1 font=font_bundled size=24/32 fill=$ink
@style body font=font_bundled size=14/20 fill=$muted

page#login
  comp#input name=Input 336x44 r=8 stroke=$line row p=0,12 align=center
    text#ph .body "Placeholder"
  frame#screen 1280x800 fill=#F3F4F6 col align=center justify=center
    frame#card w=400 h=hug fill=#FFFFFF r=16 stroke=#E5E7EB col p=32 gap=16 align=stretch
      text .h1 "Đăng nhập"
      text .body "Nhập email và mật khẩu để tiếp tục."
      use#email input ph="email@example.com"
      use#pass input ph="Mật khẩu"
      frame#btn h=44 r=8 fill=$pri row align=center justify=center
        text .body fill=#FFFFFF "Tiếp tục"
  @note card "Mẫu demo: card đăng nhập dựng bằng stack column."
```

Edit batch (with `pageIds`): `@set card-1 text="Chào mừng trở lại" size=28`,
`@set btn fill=$ink`, `@del pass`.

## Acceptance

- The sample compiles to a checked-in golden operation list that passes `dryRun`; one test
  per shorthand; parse-error tests (unknown key, bad indent, unterminated string,
  duplicate ID) report the right line and column; `ocd.md` sample is executed by a test.
- Without `source`, `apply_batch` behavior and responses are unchanged. `structured_batch`,
  `tool_schemas`, `skill_layer` and `tools_budget` pass without raising limits.
- Short report `docs/TASK26-REPORT.md`: sample bytes before/after, tests run, line counts.
  Root reviews the diff before commit.

## Out of scope

`.ocd` files in `apply_batch_file`, token benchmark, a JSON tree format, changes to
storage or the `Operation` enum.
