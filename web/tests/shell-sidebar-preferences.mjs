import assert from "node:assert/strict";

const toggle = (page, panel, open) => page.getByRole("button", {
  name: `${open ? "Hide" : "Show"} ${panel} panel`, exact: true,
});

/** The user's persisted panel preferences must survive App/cache changes. */
export async function checkSidebarPreferences(browser, base) {
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  try {
    await page.goto(`${base}/?scene=server`);
    for (const panel of ["layers", "inspector"]) await toggle(page, panel, true).click();
    assert.deepEqual(await page.evaluate(() => [
      localStorage.getItem("left-open"), localStorage.getItem("right-open"),
    ]), ["false", "false"]);
    await page.reload();
    for (const panel of ["layers", "inspector"]) {
      await toggle(page, panel, false).waitFor();
      await toggle(page, panel, false).click();
    }
    await page.reload();
    for (const panel of ["layers", "inspector"]) await toggle(page, panel, true).waitFor();
  } finally {
    await page.close();
  }
  for (const stored of ["{invalid", '"false"', "null", "0", "{}", "[]"]) {
    const malformed = await browser.newPage();
    try {
      await malformed.addInitScript((value) => {
        localStorage.setItem("left-open", value);
        localStorage.setItem("right-open", value);
      }, stored);
      await malformed.goto(`${base}/?scene=server`);
      for (const panel of ["layers", "inspector"]) await toggle(malformed, panel, true).waitFor();
    } finally {
      await malformed.close();
    }
  }
  const denied = await browser.newPage();
  try {
    await denied.addInitScript(() => {
      Storage.prototype.getItem = Storage.prototype.setItem = () => {
        throw new DOMException("Storage denied", "SecurityError");
      };
    });
    await denied.goto(`${base}/?scene=server`);
    for (const panel of ["layers", "inspector"]) {
      await toggle(denied, panel, true).click();
      await toggle(denied, panel, false).click();
      await toggle(denied, panel, true).waitFor();
    }
  } finally {
    await denied.close();
  }
  console.log("ok - sidebar close/reopen survives reload, malformed/denied storage stays usable");
}
