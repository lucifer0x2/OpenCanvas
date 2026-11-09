// JSON Schema for the capture handoff. Exported verbatim to ignored out/report-schema.json.
const string = { type: "string" }, number = { type: "number" }, boolean = { type: "boolean" };
const nullableString = { type: ["string", "null"] };
const array = (items) => ({ type: "array", items });
const object = (properties, required = Object.keys(properties)) =>
  ({ type: "object", properties, required, additionalProperties: false });
const record = (ref) => ({ type: "object", additionalProperties: { $ref: `#/$defs/${ref}` } });
const ref = (name) => ({ $ref: `#/$defs/${name}` });
const metrics = Object.fromEntries(["width", "actualBoundingBoxLeft", "actualBoundingBoxRight",
  "actualBoundingBoxAscent", "actualBoundingBoxDescent", "fontBoundingBoxAscent", "fontBoundingBoxDescent",
  "emHeightAscent", "emHeightDescent", "hangingBaseline", "alphabeticBaseline", "ideographicBaseline"].map((k) => [k, number]));

export const REPORT_SCHEMA = {
  $schema: "https://json-schema.org/draft/2020-12/schema",
  title: "OpenCanvas Task10 Chromium measurement oracle (capture only)",
  ...object({ schemaVersion: { const: 1 }, kind: { const: "chromium-measurement-oracle-only" },
    generatedAt: string, environment: ref("Environment"), fonts: array(ref("Face")),
    corpus: ref("Corpus"), requests: array(ref("Request")), cases: record("Case"), browser: ref("Browser") }),
  $defs: {
    Request: object({ key: string, text: string, fontId: string, fontHash: string, family: string,
      weight: number, style: string, fontSize: number, lineHeight: number,
      width: { type: ["number", "null"] }, letterSpacing: number,
      textTransform: { enum: ["none", "uppercase", "lowercase", "capitalize"] } }),
    Bounds: object({ x: number, y: number, width: number, height: number }),
    Line: object({ text: string, x: number, y: number, width: number, inkBounds: ref("Bounds") }),
    Measurement: object({ width: number, height: number, lines: array(ref("Line")), ascent: number,
      missing: array({ type: "integer" }), coverageUnavailable: boolean }),
    Metrics: object(metrics, ["width", "actualBoundingBoxLeft", "actualBoundingBoxRight",
      "actualBoundingBoxAscent", "actualBoundingBoxDescent", "fontBoundingBoxAscent", "fontBoundingBoxDescent"]),
    Probe: object({ transformedText: string, cssFont: string, letterSpacing: string,
      empty: ref("Metrics"), fullText: ref("Metrics"), lines: array(ref("Metrics")) }),
    Manifest: object({ id: string, family: string, hash: string, url: string, weight: number, style: string }),
    Face: object({ manifest: ref("Manifest"), privateFamily: string, sourceURL: string, sourceBytes: string,
      byteSize: { type: "integer" }, tableTags: array(string),
      license: object({ name: string, sourceURL: nullableString, sha256: nullableString, file: nullableString }) }),
    Case: object({ group: string, label: string, declaredFallback: boolean, thresholdOf: nullableString }),
    Eligibility: object({ status: { enum: ["eligible", "excluded"] },
      reason: { enum: [null, "missing-glyph", "coverage-unavailable", "declared-platform-fallback"] } }),
    Corpus: object({ path: string, sha256: string, advanceCases: { type: "integer" }, wrapCases: { type: "integer" } }),
    Environment: object({ node: string, platform: string, arch: string, playwrightVersion: string,
      chromiumVersion: string, chromiumRevision: string, headless: { const: true }, args: array(string),
      sfTrak: boolean, sourceHashes: { type: "object", additionalProperties: string } }),
    Browser: object({ rendererVersion: string, sceneVersion: string,
      readiness: array(object({ id: string, kind: { const: "font" }, status: { const: "ready" } })),
      diagnostics: array(string), faces: array(object({ family: string, weight: string, style: string, status: string })),
      userAgent: string, language: string, devicePixelRatio: number, domAvailable: { const: true },
      measurements: record("Measurement"), probes: record("Probe"), eligibility: record("Eligibility") }),
  },
};

export function eligibility(measurement, testCase) {
  const reason = measurement.coverageUnavailable ? "coverage-unavailable"
    : measurement.missing.length ? "missing-glyph"
      : testCase.declaredFallback ? "declared-platform-fallback" : null;
  return { status: reason ? "excluded" : "eligible", reason };
}
