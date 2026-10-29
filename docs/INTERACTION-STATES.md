# Flat interaction states (P27a+b)

Every non-page node may author `props.states`, an object keyed by `hover`,
`pressed`, `focus` and `disabled`. Each value is a flat visual override object.
Only `fill`, `opacity`, `radius` and `stroke` (frame/group/rectangle/image) are
accepted. Text color uses `fill`. Base property types, ranges and typed references
apply. The real stroke schema is `{color,width?,align?}`, including four-side
arrays; `strokeColor` and `strokeWidth` are aliases rejected by the base catalog.
Font weight and shadow have no base property and are unsupported in states.

```json
{"op":"update_node","id":"button","props":{"states":{
  "hover":{"fill":{"tokenId":"brand"}},
  "pressed":{"opacity":0.8},
  "focus":{"stroke":{"color":"#93C5FD","width":2}},
  "disabled":{"opacity":0.4}
}}}
```

`update_node` replaces the whole `states` object; `{}` clears it. A missing state
or empty override leaves resting visuals. No nesting, inheritance, selectors or
state combinations: activating a parent never changes a child's props. A state
stroke replaces the entire stroke object, using width 1 and align inside when
omitted. There is no deep merge with the resting stroke.

Unknown names/keys fail `VALIDATION_FAILED`, with full paths such as
`props.states.hover.width` and `props.states.foo`, `allowedKeys`, node identity and
operation index. Invalid refs and ranges fail at the state property path.
Validation covers every theme and dependent token/style update. Failure leaves
revision, authored data and idempotency state unchanged.

## Reading implementation context

`get_subtree` and context `props.states` always retain the authored references
(unless the caller's `properties` filter excludes states). Context's normal
`resolvedProps.states` contains theme-resolved values, alongside resting props.
Optional `statesMode:"resolved"` adds per-node `resolvedStates`, containing each
declared state with only keys differing from the resting resolved properties.
Nodes without states omit that field. `compact:true` keeps this same differing
rule and retains resources referenced only inside states. A properties filter
that excludes states also excludes `resolvedStates`.

Omitted mode and `statesMode:"none"` preserve the old response shape, including
cursor scope. Resolved mode binds pagination: reusing its cursor with another
mode fails `INVALID_CURSOR`. Map declared hover/pressed/focus/disabled to CSS
`:hover`, `:active`, `:focus-visible` and `[disabled]`; never invent variants.

## Rendering and geometry

`render_frame state` and the HTTP scene query accept
`default|hover|pressed|focus|disabled`, defaulting to default. A named state applies
to every node declaring it in that frame. This is a whole-frame variant preview,
not a simulation of one pointer or browser focus. Different states produce
different scene keys; an identical request reuses its job.

Layout and text measurement always use resting props. After compute, the server
overlays resolved visual values onto scene nodes before PNG rendering. Bounds,
child positions and text lines therefore stay identical, including when an
inside stroke changes width. Only the resting border reserves layout space;
state borders paint over that geometry. Outside/center strokes may paint outside
the node bounds according to their normal stroke semantics.

If no node in the frame declares a requested non-default state, the image equals
default and scene/job diagnostics contain the string `STATE_NOT_DECLARED`.
The concrete state remains in provenance and cache identity. Viewer rendering
uses the same computed scene; no viewer state picker or pointer activation is
added in this slice. No schema/DB migration is needed.

## P07a animation compatibility

P07a has no `to` property: animations require explicit `keyframes`; hover requires
exactly one keyframe. This existing contract stays intact. P27 does not invent an
implicit `states.hover` animation target or allow missing keyframes. When
authoring both, supply compatible explicit hover keyframes. Static state renders
show the selected variant and do not run animations.
