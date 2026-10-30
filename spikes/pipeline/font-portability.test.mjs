import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { startRuntime } from "../../worker/runtime.mjs";

test("real fonts with identical authored family remain isolated across later resource loads", async () => {
  const runtime = await startRuntime();
  try {
    const fonts = await Promise.all(
      [
        ["noto", "renderer/fonts/NotoSans.ttf"],
        ["lato", "fixtures/fonts/Lato-Regular.ttf"],
      ].map(async ([id, file]) => {
        const bytes = await readFile(file);
        return {
          id,
          family: "Shared Family",
          weight: 400,
          style: "normal",
          hash: createHash("sha256").update(bytes).digest("hex"),
          url: `data:font/ttf;base64,${bytes.toString("base64")}`,
        };
      }),
    );
    const result = await runtime.page.evaluate(async (fonts) => {
      const first = await renderer.loadResources([fonts[0]], []);
      const measure = (font, resources) =>
        renderer.measureBatch(
          [
            {
              key: "copy",
              text: "Tiếng Việt OpenCanvas",
              fontId: font.id,
              fontHash: font.hash,
              family: font.family,
              weight: 400,
              style: "normal",
              fontSize: 20,
              lineHeight: 28,
              width: null,
            },
          ],
          resources,
        ).copy;
      const before = measure(fonts[0], first);
      const second = await renderer.loadResources([fonts[1]], []);
      const after = measure(fonts[0], first);
      const other = measure(fonts[1], second);
      const both = await renderer.loadResources(fonts, []);
      const widths = fonts.map((font) => measure(font, both).width);
      const canvas = document.createElement("canvas").getContext("2d");
      canvas.fontKerning = "normal";
      const canvasWidths = fonts.map((font) => {
        canvas.font = renderer.fontCSS(font, 20);
        return canvas.measureText("Tiếng Việt OpenCanvas").width;
      });
      const familyNames = [...document.fonts].map((font) => font.family);
      const failures = [];
      for (const font of [
        { ...fonts[0], hash: "0".repeat(64) },
        {
          ...fonts[0],
          hash: "5e6ed95031c41c0c3c678d67c25b7fb67c229f8e3d51d8a2e92145e0ac077b29",
          url: "data:font/ttf;base64,bm90LWEtZm9udA==",
        },
      ]) {
        try {
          await renderer.loadResources([font], []);
          failures.push("unexpected ready");
        } catch (error) {
          failures.push(error.code);
        }
      }
      return {
        before,
        after,
        other,
        widths,
        canvasWidths,
        familyNames,
        failures,
      };
    }, fonts);
    assert.deepEqual(result.before, result.after);
    assert.ok(Math.abs(result.before.width - result.other.width) > 1);
    assert.deepEqual(result.widths, [result.before.width, result.other.width]);
    result.canvasWidths.forEach((width, i) =>
      assert.ok(
        Math.abs(width - result.widths[i]) < 0.02,
        JSON.stringify(result),
      ),
    );
    assert.deepEqual(
      result.familyNames.sort(),
      fonts.map((font) => `oc_${font.hash}_400_normal`).sort(),
    );
    assert.deepEqual(result.failures, [
      "RESOURCE_NOT_READY",
      "RESOURCE_NOT_READY",
    ]);
  } finally {
    await runtime.close();
  }
});
