// Helpers shared by scenario modules.
import { byPhase, totals } from "../meter.mjs";

/** Calls a paginated read and follows nextCursor until exhausted. */
export async function readAll(client, tool, args, limitPages = 500) {
  const pages = [];
  let cursor;
  do {
    const page = await client.call(tool, {
      ...args,
      ...(cursor ? { cursor } : {}),
    });
    pages.push(page);
    cursor = page.nextCursor;
    if (pages.length > limitPages)
      throw new Error(`${tool}: more than ${limitPages} pages`);
  } while (cursor);
  return pages;
}

/** Node entries of every page, with a uniqueness check (no dupes across pages). */
export function nodesOf(pages) {
  const nodes = pages.flatMap((page) => page.nodes ?? []);
  const unique = new Set(nodes.map((n) => n.id));
  return { nodes, count: nodes.length, unique: unique.size };
}

/** Standard scenario output: totals, phases, wall time, facts, then raw calls. */
export function summarize(id, title, rec, result) {
  return {
    id,
    title,
    totals: { ...totals(rec.calls), wallMs: rec.wallMs() },
    phases: byPhase(rec.calls),
    result,
    calls: rec.calls,
  };
}

export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
