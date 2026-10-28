/// What a client is told on connect. It covers both directions: authoring a design, and reading
/// an existing one to implement it in code, which the tool list alone does not make obvious.
pub const CONNECT: &str = "Versioned design tools over a local design store. Every read is one \
consistent revision.

Authoring: get_outline first, then scoped reads. Create a page, then a frame, then its \
descendants. Mutations need caller-chosen IDs, expectedRevision and a stable idempotencyKey; \
replay the exact body after a lost response, and read back after a conflict. Text must declare a \
valid font, size and line height, or take them from a text style. Render jobs never change the \
authored revision.

Implementing a design in code: get_outline for the frame, then get_design_context for that \
frameId. Each node returns authored props beside resolvedProps. Authored props hold the token and \
style references the design was built from; resolvedProps hold what those references evaluate to \
for one theme. Write code against the reference, mapped to your own named token, so a theme change \
in the design stays one change in the code; treat resolvedProps as today's value, not as the \
design. list_tokens, list_styles and list_fonts give the named system behind those references.

Build the structure the design declares, not the pixels it measures. A container's layout mode \
says it: 'row' and 'column' are stacks with padding and gap, so implement them as stacks; 'free' \
positions children at authored x/y. Sizing is authored width/height with min/max, which may be a \
fixed number or hug. Computed bounds and text lines are measurements of one viewport, already \
wrapped, so use them to check an implementation rather than to derive it, and re-read at another \
viewport to see how the design reflows instead of inventing breakpoints. The design covers theme 'default' and \
state 'default' only, and says nothing about interaction, animation, routing or data; ask rather \
than invent those. To check an implementation, render_frame at the same revision and viewport and \
compare it with inspect_layout bounds and text lines for that job's scene key.";
