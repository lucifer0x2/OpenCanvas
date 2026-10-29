import assert from "node:assert/strict";

/** Public combobox behavior shared by viewer and preview. Uses fixture data only. */
export async function verifyCustomSelects(page) {
  assert.equal(await page.locator("select").count(), 0);
  for (const label of ["Project", "Page", "Frame", "Viewport"]) {
    const picker = page.getByRole("combobox", { name: label, exact: true });
    await picker.click();
    await page.getByRole("listbox", { name: label, exact: true }).waitFor();
    assert.equal(
      await picker.evaluate((el) => document.activeElement === el),
      true,
    );
    const activeId = await picker.getAttribute("aria-activedescendant");
    assert.ok(activeId && (await page.locator(`[id="${activeId}"]`).count()));
    await picker.press("Escape");
    assert.equal(await page.getByRole("listbox").count(), 0);
  }
  const viewport = page.getByRole("combobox", {
    name: "Viewport",
    exact: true,
  });
  await viewport.focus();
  await viewport.press("Enter");
  await viewport.press("End");
  await viewport.press("Enter");
  assert.equal(await viewport.getAttribute("value"), "mobile");
  await viewport.press(" ");
  await viewport.press("Home");
  await viewport.press("ArrowDown");
  await viewport.press(" ");
  assert.equal(await viewport.getAttribute("value"), "desktop");
  await viewport.press("ArrowUp");
  await viewport.press("ArrowDown");
  await viewport.press("Enter");
  assert.equal(await viewport.getAttribute("value"), "tablet");
  await viewport.press("m");
  await viewport.press("Enter");
  assert.equal(await viewport.getAttribute("value"), "mobile");
  await viewport.click();
  await viewport.press("Home");
  await viewport.press("Escape");
  assert.equal(
    await viewport.getAttribute("value"),
    "mobile",
    "Escape cancels the pending option",
  );
  await viewport.click();
  await page.locator(".statusbar > span.connection").click();
  assert.equal(
    await page.getByRole("listbox").count(),
    0,
    "outside pointer closes menu",
  );
  await viewport.focus();
  await viewport.press("Enter");
  await viewport.press("Tab");
  assert.equal(
    await page.getByRole("listbox").count(),
    0,
    "Tab closes without trapping focus",
  );
  await viewport.focus();
  const outline = await viewport.evaluate(
    (el) => getComputedStyle(el).outlineStyle,
  );
  assert.equal(outline, "solid", "keyboard focus remains visible");
  await viewport.press("Home");
  await viewport.press("Enter");
  assert.equal(await viewport.getAttribute("value"), "frame");

  await page.setViewportSize({ width: 390, height: 844 });
  await viewport.click();
  const menu = await page.getByRole("listbox").boundingBox();
  assert.ok(
    menu && menu.x >= 0 && menu.x + menu.width <= 390,
    "portal fits narrow viewport",
  );
  assert.ok(
    menu.y >= 0 && menu.y + menu.height <= 844,
    "portal stays on screen",
  );
  await viewport.press("Escape");
  await page.setViewportSize({ width: 1440, height: 1000 });
}

export async function verifyPreviewSelect(page, baseURL) {
  await page.goto(`${baseURL}/preview?project=fixture&frame=frame`);
  const frame = page.getByRole("combobox", { name: "Frame", exact: true });
  await frame.waitFor();
  await frame.click();
  await page.getByRole("option", { name: "Welcome", exact: true }).click();
  assert.equal(new URL(page.url()).searchParams.get("frame"), "frame");
  assert.equal(await frame.getAttribute("aria-expanded"), "false");
  assert.equal(await page.locator("select").count(), 0);
}

export async function verifyEmptySelects(browser, baseURL) {
  const empty = await browser.newPage();
  try {
    await empty.route("**/api/v1/projects", (route) =>
      route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({ projects: [] }),
      }),
    );
    await empty.goto(baseURL);
    await empty.getByText("No project selected", { exact: true }).waitFor();
    for (const label of ["Project", "Page", "Frame", "Viewport"]) {
      assert.equal(
        await empty
          .getByRole("combobox", { name: label, exact: true })
          .isDisabled(),
        true,
      );
    }
    assert.equal(
      await empty
        .getByRole("combobox", { name: "Project", exact: true })
        .textContent(),
      "No projects yet",
    );
    await empty.unroute("**/api/v1/projects");
    await empty.route("**/api/v1/projects", (route) =>
      route.fulfill({
        contentType: "application/json",
        body: JSON.stringify({
          projects: Array.from({ length: 30 }, (_, index) => ({
            projectId: index === 0 ? "fixture" : `project-${index}`,
            slug: `project-${index}`,
            name: `Project${String(index).padStart(2, "0")} with a long descriptive name`,
            revision: 7,
          })),
        }),
      }),
    );
    await empty.reload();
    await empty.getByText("Scene r7", { exact: true }).waitFor();
    const picker = empty.getByRole("combobox", {
      name: "Project",
      exact: true,
    });
    await picker.evaluate((el) =>
      Object.assign(el.style, {
        position: "fixed",
        bottom: "8px",
        left: "8px",
      }),
    );
    await picker.press("Enter");
    await picker.press("End");
    const list = empty.getByRole("listbox");
    const last = empty.getByRole("option").last();
    const bounds = await list.boundingBox();
    const trigger = await picker.boundingBox();
    const option = await last.boundingBox();
    assert.ok(
      bounds && trigger && bounds.y + bounds.height <= trigger.y,
      "menu flips above a low trigger",
    );
    assert.ok(
      option &&
        option.y >= bounds.y &&
        option.y + option.height <= bounds.y + bounds.height,
      "End scrolls the active option into view",
    );
    await empty.setViewportSize({ width: 390, height: 844 });
    const narrow = await list.boundingBox();
    assert.ok(
      narrow && narrow.x >= 0 && narrow.x + narrow.width <= 390,
      "open menu repositions after resize",
    );
    await picker.press("Escape");
    await picker.pressSequentially("Project29");
    await picker.press("Enter");
    assert.equal(
      await picker.getAttribute("value"),
      "project-29",
      "typeahead accumulates a prefix from the closed combobox",
    );
  } finally {
    await empty.close();
  }
}
