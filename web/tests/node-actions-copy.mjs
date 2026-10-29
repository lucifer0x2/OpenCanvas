import assert from "node:assert/strict";
import { waitFor } from "../../spikes/e2e/protocol.mjs";

export async function verifyCopy(page, address) {
  await page.getByRole("button", { name: "Export box", exact: true }).click();
  for (const key of ["Control+c", "Meta+c"]) {
    await page.keyboard.press(key);
    assert.equal(
      await page.evaluate(() => navigator.clipboard.readText()),
      address("box"),
    );
  }
  const nativeCopy = process.platform === "darwin" ? "Meta+c" : "Control+c";
  await page.evaluate(() => {
    const range = document.createRange();
    range.selectNodeContents(document.querySelector(".inspector h2"));
    window.getSelection().removeAllRanges();
    window.getSelection().addRange(range);
  });
  await page.keyboard.press(nativeCopy);
  await waitFor(
    async () =>
      (await page.evaluate(() => navigator.clipboard.readText())) ===
      "Export box",
    "native selected page text copy",
  );
  await page.evaluate(() => {
    window.getSelection().removeAllRanges();
    window.__writeClipboard = navigator.clipboard.writeText;
    navigator.clipboard.writeText = async () => {
      throw new Error("Permission denied");
    };
  });
  await page.getByRole("button", { name: "Copy link", exact: true }).click();
  await page
    .getByText(
      "Could not copy the link. Allow clipboard access and try again.",
      { exact: true },
    )
    .waitFor();
  await page.evaluate(async () => {
    navigator.clipboard.writeText = window.__writeClipboard;
    await navigator.clipboard.writeText("native");
    const input = document.createElement("input");
    input.id = "copy-test";
    input.value = "Native text";
    document.body.append(input);
    input.focus();
    input.select();
  });
  await page.keyboard.press(nativeCopy);
  await waitFor(
    async () =>
      (await page.evaluate(() => navigator.clipboard.readText())) ===
      "Native text",
    "native input copy",
  );
}
