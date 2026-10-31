import assert from "node:assert/strict";
import { test } from "node:test";
import { readFontFile } from "../src/fonts/fontFiles.ts";

test("font import bounds and signatures match the actual chosen bytes", async () => {
  const bytes = new Uint8Array([0, 1, 0, 0, 12, 34, 56]);
  assert.deepEqual(await readFontFile(new File([bytes], "sample.TTF")), {
    mediaType: "font/ttf",
    base64: Buffer.from(bytes).toString("base64"),
  });
  for (const [name, signature, mediaType] of [
    ["sample.otf", "OTTO", "font/otf"],
    ["sample.woff2", "wOF2", "font/woff2"],
  ]) {
    assert.equal(
      (await readFontFile(new File([signature], name))).mediaType,
      mediaType,
    );
  }
  await assert.rejects(
    readFontFile(new File([bytes], "sample.woff")),
    /TTF, OTF or WOFF2/,
  );
  await assert.rejects(
    readFontFile(new File(["fake"], "sample.ttf")),
    /contents do not match/,
  );
  await assert.rejects(readFontFile(new File([], "sample.ttf")), /empty/);
  await assert.rejects(
    readFontFile(
      new File([new Uint8Array(10 * 1024 * 1024 + 1)], "sample.ttf"),
    ),
    /10 MiB/,
  );
});
