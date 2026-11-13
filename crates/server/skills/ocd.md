# OCD source for apply_batch

Send `source` (OCD v1) instead of `operations`; the server compiles it to the same JSON
operations; validation, `pageIds`, `dryRun` and idempotency are unchanged.
Parse errors return `SOURCE_PARSE_FAILED` with `line`, `column`, `message`, `suggestion`.
Errors on a compiled operation (schema, storage `opIndex`) keep their code and add its
statement's `line`/`column`; a source or compiled batch over 2 MiB is a parse error.

```
line  := indent (node | directive) [// comment]      indent = 2 spaces per level
node  := kind[#id][.styleId] {attr | flag | WxH} ["text"]
kind  := page | frame | group | rect | text | image | comp | use <componentId>
attr  := key=value   value: number | fill | hug | 50% | #hex | $token | a,b,c
                            | "json string" | {json} | [json]   (no spaces outside quotes)
flag  := row | col | free | wrap
@token id value [type=] [name=] {theme=value}   @style id [name=] {attr}
@set id {attr}   @del id   @move id parentId [index]   @note nodeId [id=] "text"
@in parentId     (ID only; indented new nodes under an existing node, each with #id)
```

| Shorthand | Writes |
| --- | --- |
| `400x44`, `w=`, `h=`, `r=` | `width`, `height`, `radius` |
| `row`/`col`/`free`, `wrap` | `layout.mode`, `layout.wrap` |
| `p=32`, `p=0,12`, `gap=`, `align=`, `justify=between` | `layout.padding/gap/align/justify` (`space-between`) |
| `.body`, `$ink` | `styleId`, `{"tokenId":"ink"}` |
| `stroke=#E5E7EB,2,inside` | `stroke {color,width,align}` |
| `size=24/32`, `font=`, `family=`, `weight=` | `fontSize`/`lineHeight`, `fontId`, `fontFamily`, `fontWeight` |
| `comp#input name=Input` | frame + `component {name}` (name defaults to ID) |
| `use#a input ph="x"`, `ph.r=8` (any node key) | instance `componentId`, `overrides {ph:{text,radius}}` |
| `key={json}`, `props={json}`, `index=2` | raw value; props merged last; create index |

Every catalog key (skill://opencanvas/props.md) works as written; later attributes
win. On `use`, a key that is not an instance prop is a master descendant ID: a string sets
its `text`, `{json}` its override object. A trailing string is a text node's `text`, else
`name`. `@set` replaces object props whole (`update_node`), so it rejects layout flags/keys,
descendant keys and a partial `stroke`: write it in full, e.g.
`@set card layout={"mode":"column","padding":32,"gap":8}`, `@set email overrides={"ph":{"text":"x"}}`, `stroke=#000,1,inside`.

Defaults (written to operations): root pages need `#id`; other nodes without `#id` are
`<parentId>-<position>` (1-based); duplicate IDs fail. Text width is `"fill"` in a `col`
with a non-hug width and start align, unset under `align=stretch`, else `"hug"`; height
`"hug"` (unset in a stretched row); breakpoints: only sizes all layouts agree on;
none under `@in`. Token/style `name` is the ID. Token `type` is `color` for `#hex`,
`dimension` for a number, else required; `$x` is an alias. A style with only `fill` is a
fill style, else a text style. `@note` IDs default to `note-<nodeId>-<hash of node+text>`.

<!-- ocd-sample -->
```ocd
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

Edit with `pageIds:["login"]` and the last revision:

<!-- ocd-edit -->
```ocd
@set card-1 text="Chào mừng trở lại" size=28
@set btn fill=$ink
@del pass
```
