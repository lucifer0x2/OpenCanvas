/** Use the browser's CSS casing rules (including capitalize and expansions such as ß).
 * A rendered, offscreen element is necessary: display:none innerText returns source.
 */
export function transformText(source: string, transform = "none"): string {
  if (transform === "none") return source;
  const span = document.createElement("span");
  span.style.cssText = "position:fixed;left:-100000px;top:0;opacity:0;white-space:pre;pointer-events:none";
  span.style.textTransform = transform;
  span.textContent = source;
  document.body.append(span);
  try { return span.innerText; } finally { span.remove(); }
}
