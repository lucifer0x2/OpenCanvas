// Source schema; generated evidence JSON belongs exclusively under ignored out/.
const metric = { type: "object", required: ["pixelsOver8", "fractionOver8", "maxDelta", "maxAt", "channelOver8", "meanChannelDelta"], properties: {
  pixelsOver8: { type: "integer", minimum: 0 }, fractionOver8: { type: "number", minimum: 0, maximum: 1 },
  maxDelta: { type: "integer", minimum: 0, maximum: 255 },
  maxAt: { type: ["object", "null"] }, channelOver8: { type: "array", minItems: 4, maxItems: 4, items: { type: "integer", minimum: 0 } },
  meanChannelDelta: { type: "number", minimum: 0, maximum: 255 },
} };
export const reportSchema = {
  $schema: "https://json-schema.org/draft/2020-12/schema", title: "Native paint parity evidence",
  type: "object", required: ["schema", "createdAt", "gate", "pixelPolicy", "os", "node", "sourceHashesBefore", "cases", "probes", "limitations"],
  properties: {
    schema: { const: "native-paint-parity/1" }, createdAt: { type: "string" }, gate: { type: "object" },
    sourceHashesBefore: { type: "object", additionalProperties: { type: "string", pattern: "^[a-f0-9]{64}$" } },
    sourceHashesAfter: { type: "object" }, sourceChangedDuringRun: { type: "boolean" },
    oracle: { type: "object", required: ["browserVersion", "playwrightVersion", "rendererBundleHash", "browserExecutable"] },
    native: { type: "object", required: ["deps", "rustc", "binaryHash"] },
    cases: { type: "array", items: { type: "object", required: ["name", "category", "scale", "sceneHash", "request", "chrome"], properties: {
      name: { type: "string" }, category: { enum: ["nontext", "text"] }, scale: { type: "number" },
      verdict: { enum: ["PASS", "FAIL", "REVIEW_TEXT", "NATIVE_PENDING", "NATIVE_ERROR", "COMPARISON_ERROR"] },
      comparison: { type: "object", required: ["totalPixels", "gate", "rawStraight", "visibleStraight", "compositedWhite", "masks", "thresholdExceeded", "gateApplies", "verdict"], properties: {
        totalPixels: { type: "integer", minimum: 1 }, masks: { const: [] },
        rawStraight: metric, visibleStraight: metric, compositedWhite: metric,
        thresholdExceeded: { type: "boolean" }, gateApplies: { type: "boolean" },
      } },
    } } },
    probes: { type: "array", items: { type: "object", required: ["name", "expectedChromeCode", "chrome", "native", "chromePass", "nativeRejected"] } },
    error: { type: "object", required: ["message", "stack"] }, summary: { type: "object" },
  },
};
