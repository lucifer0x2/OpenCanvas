/// What a client is told on connect. It covers both directions: authoring a design, and reading
/// an existing one to implement it in code, which the tool list alone does not make obvious.
pub const CONNECT: &str = "Versioned design tools over a local design store. Every read is one \
consistent revision.

Finding: when you know a node's name, visible text, kind, ID or parent (parentId lists direct \
children), call find_nodes; use get_outline when you need structure: depth counts from the scope \
root, so nodeId <page> with depth 1 lists that page's root frames in one call. totalMatches 0 means nothing matched, not an error. Position ('the card on \
the right') is not searchable: read bounds with get_design_context viewports or inspect_layout nodeIds.

Authoring: get_outline first, then scoped reads. Create a page, then a frame, then its \
descendants. Default to row/column stacks; reserve free for overlays and decoration. \
Mutations need caller-chosen IDs, expectedRevision and a stable idempotencyKey; \
replay the exact body after a lost response. A REVISION_CONFLICT carries changedSince \
{fromRevision, toRevision, changedIds, truncated}: when truncated is false and no changed ID is a \
node you edit or one of its ancestors, resend with expectedRevision toRevision and a new \
idempotencyKey (the body changed); otherwise re-read those IDs with find_nodes ids, or catch up \
with get_changes, before rebuilding the batch. The server validates every batch again, so a wrong \
rebase fails rather than corrupting data. Node props accept only the keys listed per kind in \
skill://opencanvas/props.md; a wrong key fails with field props.<key>, allowedKeys and a suggestion. Text must declare a valid font, size and line height, or take them from a text style. Render jobs never change the \
authored revision.

Implementing a design in code: find_nodes kinds [frame] or get_outline for the frame, then \
get_design_context for that frameId. Each node returns authored props beside resolvedProps. Authored props hold the token and \
style references the design was built from; resolvedProps hold what those references evaluate to \
for one theme. Write code against the reference, mapped to your own named token, so a theme change \
in the design stays one change in the code; treat resolvedProps as today's value, not as the \
design. list_tokens, list_styles and list_fonts give the named system behind those references. \
For a smaller read pass compact:true: resolvedProps then keep only keys that differ from authored \
props (resolvedPropsMode differing; the rest equal authored, without styleId) and \
tokens/styles/fonts/assets only those the frame references (resources referenced).

Build the structure the design declares, not the pixels it measures. A container's layout mode \
says it: 'row' and 'column' are stacks with padding and gap, so implement them as stacks; 'free' \
positions children at authored x/y. Sizing is authored width/height with min/max, which may be a \
fixed number, fill or hug. Computed bounds and text lines are measurements of one viewport, already \
wrapped, so use them to check an implementation rather than to derive it. Request viewports \
(up to four) in get_design_context to compare bounds across sizes; while computed.status is \
pending, wait on its job IDs with render_frame {jobId, waitMs} (up to 10000), then repeat the read. reflow compares every descendant, excluding the resized root, and \
is null until a successful comparison of at least two viewports. An all-free frame with unchanged bounds is a picture at one size, \
not a responsive specification: heed NO_REFLOW_INFORMATION, state assumptions or ask the user; \
do not silently consider absolute positioning complete. Read skill://opencanvas/implement-frame.md \
or skill://opencanvas/author-frame.md through resources/read for detailed checklists. \
Only interaction state 'default' is supported; named token themes may be requested. \
The design says nothing about interaction, animation, routing or data; ask rather \
than invent those. To check an implementation, render_frame at the same revision and viewport and \
compare it with inspect_layout bounds and text lines for that job's scene key. Poll a pending \
job with waitMs rather than in a tight loop: it returns as soon as the job is ready or failed.";
