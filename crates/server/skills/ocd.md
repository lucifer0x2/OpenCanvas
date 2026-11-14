# OCD source for apply_batch

Send `source` (OCD v1) alone or before `operations`: compiled to the same JSON
operations, both run as one atomic batch (`pageIds`, `dryRun`, idempotency unchanged).
Parse errors: `SOURCE_PARSE_FAILED` with `line`, `column`, `suggestion`. Errors on a
compiled operation add its `line`/`column`; on a JSON one, `opIndex` counts in `operations`.

```
line  := indent (node | directive) [// comment]   indent = 2 spaces per level
node  := kind[#id][.styleId] {attr | flag | WxH} ["text"]
kind  := page | frame | group | rect | text | image | comp | use <componentId>
attr  := key=value   value: number | fill | hug | 50% | #hex | $token | a,b,c
                            | "json string" | {json} | [json]   (no spaces outside quotes)
flag  := row | col | free | wrap
@token id value [type=] [name=] {theme=value}   @style id [name=] {attr}
@set id {attr}   @del id   @move id parentId [index]   @order id index
@note nodeId "text" [id=] [author=]   @reply noteId "text" [author=]   @resolve noteId
@note-set noteId ["text"] [resolved=true|false] [node=]   @note-del noteId
@in parentId   (indented new nodes, each with #id, under an existing node)
```

| Shorthand | Writes |
| --- | --- |
| `400x44`, `w=`, `h=`, `r=` | `width`, `height`, `radius` |
| `row`/`col`/`free`, `wrap` | `layout.mode`, `layout.wrap` |
| `p=0,12`, `gap=`, `align=`, `justify=between` | `layout.padding/gap/align/justify` (`space-between`) |
| `.body`, `$ink` | `styleId`, `{"tokenId":"ink"}` |
| `stroke=#E5E7EB,2,inside` | `stroke {color,width,align}` |
| `size=24/32`, `font=`, `family=`, `weight=` | `fontSize`/`lineHeight`, `fontId`, `fontFamily`, `fontWeight` |
| `comp#input name=Input` | frame + `component {name}` (name: ID) |
| `use#a input ph="x"`, `ph.r=8` (any node key) | instance `componentId`, `overrides {ph:{text,radius}}` |
| `key={json}`, `props={json}`, `index=2` | raw value; props merged last; create index |

Every catalog key (skill://opencanvas/props.md) works as written; later attributes win.
On `use`, a non-instance key names a master descendant (string: its `text`; `{json}`: its
override); node props, and descendants missing from a `comp` in the source, fail. A
trailing string is a text node's `text`, else `name`. `@set` replaces whole values
(`layout={…}`, `stroke=#000,1,inside`); partial writes (flags, `gap=`, `layout.gap=`,
`stroke.width=`, `stroke=#000`, `ph.fill=`) send `update_node` `merge:true`, a JSON merge
patch keeping the rest (`null` deletes; `{tokenId}`, `{styleId}`, gradients replace).

Defaults: root pages need `#id`; other nodes get `<nearest #id>-<kind>-<hash of
parent, kind, text/name>`, returned as `autoIds` {line: id}: give nodes to edit an `#id`. Text width: `"fill"` in a start-aligned `col` of non-hug
width, unset under `align=stretch`, else `"hug"`; height `"hug"` (unset in a stretched
row); with breakpoints only agreed sizes; none under `@in`. Token/style `name` defaults to
the ID; token `type` to `color` for `#hex`, `dimension` for numbers (else required); `$x`
is an alias. A style of only `fill` is a fill style, else text.

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
      text#title .h1 "Đăng nhập"
      text .body "Nhập email và mật khẩu."
      use#email input ph="email@example.com"
      use#pass input ph="Mật khẩu"
      frame#btn h=44 r=8 fill=$pri row align=center justify=center
        text .body fill=#FFFFFF "Tiếp tục"
  @note card "Card đăng nhập: stack column." author=demo
```

Edit (`pageIds:["login"]`, last revision):

<!-- ocd-edit -->
```ocd
@set title text="Chào mừng trở lại" size=28
@set card gap=24
@del pass
```
