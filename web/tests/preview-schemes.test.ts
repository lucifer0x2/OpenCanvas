import assert from "node:assert/strict";
import test from "node:test";
import { buildAnimation } from "../src/preview/keyframes.ts";
import { tokenTable } from "../src/preview/tokens.ts";

test("scoped animation references resolve each alias, fill style and timing at the node theme", () => {
  const themed = tokenTable(
    [
      { id: "a", type: "color", value: { aliasId: "b" } },
      {
        id: "b",
        type: "color",
        value: { default: "#111111", themes: { night: "#eeeeee" } },
      },
      {
        id: "duration",
        type: "number",
        value: { default: 300, themes: { night: 750 } },
      },
    ],
    [{ id: "fill", kind: "fill", value: { tokenId: "a" } }],
    "night",
  );
  const built = buildAnimation(
    {
      name: "scoped",
      duration: { tokenId: "duration" },
      keyframes: [{ fill: { styleId: "fill" } }],
    },
    { opacity: 1, fill: "#fff", text: false },
    themed,
  );
  assert.deepEqual(built.warnings, []);
  assert.equal(built.animation!.timing.duration, 750);
  assert.equal(built.animation!.paint.at(-1)!.backgroundColor, "#eeeeee");
});
