import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";

const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");

export async function verifyFontUi(
  page,
  base,
  projectId,
  fixture,
  artifactDir,
) {
  await page.goto(`${base}/?project=${projectId}`);
  await page.getByRole("combobox", { name: "Project", exact: true }).click();
  await page
    .getByRole("option", { name: "Font portability", exact: true })
    .click();
  const trigger = page.getByRole("button", { name: "Fonts", exact: true });
  await trigger.click();
  const dialog = page.getByRole("dialog", { name: "Project fonts" });
  await dialog.getByText("OpenCanvas Noto Sans", { exact: true }).waitFor();
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    dialog
      .getByRole("button", {
        name: "Download OpenCanvas Noto Sans, 400 normal",
      })
      .click(),
  ]);
  assert.equal(hash(await readFile(await download.path())), hash(fixture));
  // Explicit focus containment including the custom listbox on the native dialog layer.
  await dialog.getByRole("button", { name: "Close fonts" }).focus();
  for (let i = 0; i < 16; i++) {
    await page.keyboard.press(i < 8 ? "Tab" : "Shift+Tab");
    assert.equal(
      await page.evaluate(() => !!document.activeElement.closest("dialog")),
      true,
    );
  }
  const style = dialog.getByRole("combobox", {
    name: "Font style",
    exact: true,
  });
  await style.click();
  assert.equal(
    await page
      .getByRole("listbox", { name: "Font style" })
      .evaluate((el) => !!el.closest("dialog")),
    true,
  );
  await style.press("End");
  await style.press("Enter");
  assert.equal(await style.getAttribute("value"), "italic");
  await style.press("Home");
  await style.press("Enter");
  await style.press("Enter");
  await style.press("Escape");
  assert.equal(await dialog.isVisible(), true);
  await style.press("Escape");
  await dialog.waitFor({ state: "hidden" });
  assert.equal(
    await trigger.evaluate((el) => document.activeElement === el),
    true,
  );
  await trigger.click();
  const file = dialog.getByLabel("Font file", { exact: true });
  for (const [name, buffer, message] of [
    ["bad.woff", fixture, "Choose a TTF, OTF or WOFF2 font file."],
    [
      "bad.ttf",
      Buffer.from("not a font"),
      "The file contents do not match its font format.",
    ],
    [
      "huge.ttf",
      Buffer.alloc(10 * 1024 * 1024 + 1),
      "Font files must be 10 MiB or smaller.",
    ],
  ]) {
    await file.setInputFiles({ name, mimeType: "font/ttf", buffer });
    await dialog.getByText(message, { exact: true }).waitFor();
    assert.equal(
      await dialog
        .getByRole("button", { name: "Add font", exact: true })
        .isDisabled(),
      true,
    );
  }
  await file.setInputFiles({
    name: "Portable.ttf",
    mimeType: "font/ttf",
    buffer: fixture,
  });
  await dialog.getByLabel("Font family", { exact: true }).fill("Portable Noto");
  await dialog.getByLabel("License (optional)").fill("OFL-1.1");
  await page.screenshot({ path: `${artifactDir}/font-desktop.png` });
  return { dialog, file, trigger, hash };
}
