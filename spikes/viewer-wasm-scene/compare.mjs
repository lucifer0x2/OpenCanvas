// No epsilon or rounded output: every shared scene field is compared recursively.
export const PROVENANCE_EXCLUSIONS = [
  "sourceSceneKey",
  "engine",
  "engineVersion",
  "runtimeVersion",
];

export function differences(a, b, path = "$", result = []) {
  if (a === b) return result;
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length)
      result.push({ path: `${path}.length`, native: a.length, wasm: b.length });
    for (let i = 0; i < Math.max(a.length, b.length); i++)
      differences(a[i], b[i], `${path}[${i}]`, result);
  } else if (a && b && typeof a === "object" && typeof b === "object") {
    for (const key of new Set([...Object.keys(a), ...Object.keys(b)]))
      differences(a[key], b[key], `${path}.${key}`, result);
  } else
    result.push({
      path,
      native: a ?? null,
      wasm: b ?? null,
      nativeType: typeof a,
      wasmType: typeof b,
    });
  return result;
}

export function compare(native, wasm) {
  const a = structuredClone(native),
    b = structuredClone(wasm);
  const excluded = [];
  const remove = (nativeObject, wasmObject, field, path) => {
    excluded.push({
      path: `${path}.${field}`,
      native: nativeObject?.[field] ?? null,
      wasm: wasmObject?.[field] ?? null,
    });
    if (nativeObject) delete nativeObject[field];
    if (wasmObject) delete wasmObject[field];
  };
  for (const field of PROVENANCE_EXCLUSIONS)
    remove(a.provenance, b.provenance, field, "$.provenance");
  // The same job cache key and runtime labels are copied into overflow advisories.
  // Their diagnostic code/order/content/geometry stay in the exact gate.
  for (
    let i = 0;
    i < Math.max(a.diagnostics.length, b.diagnostics.length);
    i++
  ) {
    const na = a.diagnostics[i],
      wb = b.diagnostics[i];
    if (
      na?.code === "TEXT_CONTENT_OUTSIDE_BOX" ||
      na?.code === "TEXT_OVERFLOW_SUMMARY"
    ) {
      remove(na, wb, "sourceSceneKey", `$.diagnostics[${i}]`);
      if (na.code === "TEXT_OVERFLOW_SUMMARY")
        for (const field of PROVENANCE_EXCLUSIONS)
          remove(
            na.provenance,
            wb?.provenance,
            field,
            `$.diagnostics[${i}].provenance`,
          );
    }
  }
  return {
    mismatches: differences(a, b),
    excluded,
    nodes: a.nodes.length,
    texts: a.nodes.filter((n) => n.kind === "text").length,
    lines: a.nodes.reduce((sum, n) => sum + n.lines.length, 0),
    diagnostics: { native: a.diagnostics, wasm: b.diagnostics },
  };
}

export function distribution(values) {
  const sorted = [...values].sort((a, b) => a - b);
  return {
    count: sorted.length,
    min: sorted[0],
    median: sorted[Math.floor(sorted.length / 2)],
    p95: sorted[Math.ceil(sorted.length * 0.95) - 1],
    max: sorted.at(-1),
  };
}
