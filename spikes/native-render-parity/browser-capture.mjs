import * as renderer from "/renderer.js";
import { transformText } from "/transform.js";

const metrics = (m) => Object.fromEntries([
  "width", "actualBoundingBoxLeft", "actualBoundingBoxRight", "actualBoundingBoxAscent",
  "actualBoundingBoxDescent", "fontBoundingBoxAscent", "fontBoundingBoxDescent",
  "emHeightAscent", "emHeightDescent", "hangingBaseline", "alphabeticBaseline", "ideographicBaseline",
].filter((name) => typeof m[name] === "number").map((name) => [name, m[name]]));

export async function initialize(fonts) {
  if (!document.body || getComputedStyle(document.body).display === "none") throw new Error("Rendered DOM body required");
  const resources = await renderer.loadResources(fonts, [], { strict: true, timeoutMs: 10_000 });
  await document.fonts.ready;
  globalThis.oracleResources = resources;
  return { rendererVersion: renderer.RENDERER_VERSION, sceneVersion: renderer.SCENE_VERSION,
    readiness: resources.readiness, diagnostics: resources.diagnostics,
    faces: Array.from(document.fonts, (face) => ({ family: face.family, weight: face.weight,
      style: face.style, status: face.status })),
    userAgent: navigator.userAgent, language: navigator.language, devicePixelRatio, domAvailable: true };
}

export function capture(requests) {
  const resources = globalThis.oracleResources;
  if (!resources) throw new Error("Fonts not initialized");
  // Preserve this result directly. No regenerated/normalized widths, lines, ink or missing metadata.
  const measurements = renderer.measureBatch(requests, resources);
  const context = document.createElement("canvas").getContext("2d");
  if (!context || !("letterSpacing" in context)) throw new Error("Canvas letterSpacing unavailable");
  const probes = {};
  for (const request of requests) {
    const face = resources.fonts.get(request.fontId);
    context.font = renderer.fontCSS(face, request.fontSize);
    context.fontKerning = "normal";
    context.letterSpacing = `${request.letterSpacing}px`;
    const transformedText = transformText(request.text, request.textTransform);
    probes[request.key] = { transformedText, cssFont: context.font, letterSpacing: context.letterSpacing,
      empty: metrics(context.measureText("")), fullText: metrics(context.measureText(transformedText)),
      lines: measurements[request.key].lines.map((line) => metrics(context.measureText(line.text))) };
  }
  return { measurements, probes };
}
