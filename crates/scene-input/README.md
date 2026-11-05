# Scene input

Pure authored models, property catalogs, validation, token/theme resolution,
component expansion and standalone frame input assembly. Storage reexports these
same types and functions, so native callers retain model and error identity.

```rust,ignore
let input_json = opencanvas_scene_input::make_input(&snapshot, &SceneOptions {
    frame_id: "frame".into(), width: 320., height: 200.,
    theme: "default".into(), state: "default".into(),
})?;
let input: opencanvas_layout::LayoutInput = serde_json::from_value(input_json)?;
```

`make_input` validates the whole authored snapshot in every declared theme,
expands instances, resolves the selected and scoped themes, then selects the frame
subtree in sibling order. It detaches the selected frame's parent and preserves
the server's 2,048-node limit and existing errors. Schema-1 input remains distinct
from storage schema version 4. JSON preserves resolved fields at the existing
layout boundary, including interaction state metadata.

Resource manifests contain only used resources and **empty `url` strings**.
Callers supply controlled URLs or caller-owned resource bytes at their transport
boundary. This crate performs no resource loading and invents no URLs. Request
revision, viewport/scale budgets, caching and transport checks remain caller
responsibilities, as before extraction.

`scene_diagnostics` exposes the shared bounded measured overflow append and note
intent resolution used by native server jobs and browser scene JSON. It reads the
resolved frame nodes from the same prepared input, never loads resources, rewrites
geometry, or includes authored text/notes in advisory output.

Default features have no database or async runtime dependencies. The optional
`sqlite-errors` feature implements the existing `From<rusqlite::Error>` conversion
on the shared `StorageError`; native storage enables it. Do not enable that native
feature when building the pure WASM pipeline.

```sh
cargo check -p opencanvas-scene-input -p opencanvas-layout -p opencanvas-text \
  --target wasm32-unknown-unknown --locked --offline
cargo test -p opencanvas-scene-input --locked --offline
```

`opencanvas_text::FontSetMeasurer` adapts caller-owned exact fonts to layout's
`TextMeasurer`. `new(&fonts)` requires no runtime; `with_check(&fonts, callback)`
runs the caller's cancellation/deadline guard before and after each successful
measurement, with the same safe error mapping and count used by the native server.
