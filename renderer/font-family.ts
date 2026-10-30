import type { FontManifest } from "./types";

/** A content identity, never an installed OS family or user-controlled CSS. */
export function fontFamily(font: FontManifest): string {
  if (
    !/^[a-f0-9]{64}$/.test(font.hash) ||
    !Number.isInteger(font.weight) ||
    font.weight < 100 ||
    font.weight > 900 ||
    !["normal", "italic"].includes(font.style)
  )
    throw new Error(`Font ${font.id} has no valid content identity`);
  return `oc_${font.hash}_${font.weight}_${font.style}`;
}

export const fontCSS = (font: FontManifest, size: number) =>
  `${font.style} ${font.weight} ${size}px "${fontFamily(font)}"`;
