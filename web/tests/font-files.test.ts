import assert from "node:assert/strict";
import test from "node:test";
import { readFontFile, validateFontFile } from "../src/fonts/fontFiles.ts";

test("font selection checks bounded signatures and submit preserves complete original bytes", async () => {
  for (const [extension, signature, mediaType] of [
    ["ttf", "00010000", "font/ttf"],
    ["ttf", "74727565", "font/ttf"],
    ["otf", "4f54544f", "font/otf"],
    ["woff2", "774f4632", "font/woff2"],
  ]) {
    const bytes = Buffer.concat([Buffer.from(signature, "hex"), Buffer.from("original bytes")]);
    const file = new File([bytes], `face.${extension.toUpperCase()}`);
    await validateFontFile(file);
    const request = await readFontFile(file);
    assert.equal(request.mediaType, mediaType);
    assert.deepEqual(Buffer.from(request.base64, "base64"), bytes);
  }
});

test("header selection and full submission enforce the same extension, size and signature constraints", async () => {
  for (const [file, message] of [
    [new File(["invalid"], "font.woff"), /Choose a TTF, OTF or WOFF2/],
    [new File([], "font.ttf"), /empty/],
    [new File([new Uint8Array(10 * 1024 * 1024 + 1)], "font.ttf"), /10 MiB/],
    [new File(["OTTO"], "font.ttf"), /contents do not match/],
    [new File([new Uint8Array([0, 1])], "font.ttf"), /contents do not match/],
  ] as const) {
    await assert.rejects(validateFontFile(file), message);
    await assert.rejects(readFontFile(file), message);
  }
});
