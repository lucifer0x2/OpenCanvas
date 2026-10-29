import type { StyleDef, TokenDef } from "./types.ts";

/** Token and fill-style lookup at the default design theme (docs/RESOLUTION.md). */
export interface TokenTable {
  tokens: Map<string, TokenDef>;
  styles: Map<string, StyleDef>;
}

export type Resolved<T> = { value: T } | { warning: string };

const MAX_ALIAS_DEPTH = 32;

export function tokenTable(
  tokens: TokenDef[] = [],
  styles: StyleDef[] = [],
): TokenTable {
  return {
    tokens: new Map(tokens.map((token) => [token.id, token])),
    styles: new Map(styles.map((style) => [style.id, style])),
  };
}

const isObject = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

/** Follows `{aliasId}` chains and `{default, themes}` envelopes (default branch only). */
function tokenValue(
  table: TokenTable,
  id: string,
  type: string,
): Resolved<unknown> {
  let token = table.tokens.get(id);
  for (let depth = 0; depth < MAX_ALIAS_DEPTH; depth++) {
    if (!token) return { warning: `token "${id}" does not exist` };
    if (token.type !== type)
      return { warning: `token "${token.id}" is ${token.type}, not ${type}` };
    let value = token.value;
    if (isObject(value) && "default" in value) value = value.default;
    if (isObject(value) && typeof value.aliasId === "string") {
      token = table.tokens.get(value.aliasId);
      continue;
    }
    return { value };
  }
  return { warning: `token "${id}" alias chain is too deep` };
}

/** A number literal or `{tokenId}` of a number token. */
export function resolveNumber(
  table: TokenTable,
  value: unknown,
): Resolved<number> {
  if (typeof value === "number" && Number.isFinite(value)) return { value };
  if (isObject(value) && typeof value.tokenId === "string") {
    const resolved = tokenValue(table, value.tokenId, "number");
    if ("warning" in resolved) return resolved;
    return typeof resolved.value === "number"
      ? { value: resolved.value }
      : { warning: `token "${value.tokenId}" has no number value` };
  }
  return { warning: `unsupported number value ${JSON.stringify(value)}` };
}

/** A color literal, `{tokenId}` of a color token or `{styleId}` of a fill style. */
export function resolveColor(
  table: TokenTable,
  value: unknown,
): Resolved<string> {
  if (typeof value === "string") return { value };
  if (isObject(value) && typeof value.tokenId === "string") {
    const resolved = tokenValue(table, value.tokenId, "color");
    if ("warning" in resolved) return resolved;
    return typeof resolved.value === "string"
      ? { value: resolved.value }
      : { warning: `token "${value.tokenId}" has no color value` };
  }
  if (isObject(value) && typeof value.styleId === "string") {
    const style = table.styles.get(value.styleId);
    if (!style) return { warning: `style "${value.styleId}" does not exist` };
    if (style.kind !== "fill")
      return { warning: `style "${style.id}" is not a fill style` };
    if (isObject(style.value) && "styleId" in style.value)
      return { warning: `style "${style.id}" aliases another style` };
    return resolveColor(table, style.value);
  }
  return { warning: `unsupported color value ${JSON.stringify(value)}` };
}
