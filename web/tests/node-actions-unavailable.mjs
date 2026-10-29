import assert from "node:assert/strict";

/** A scene that cannot mount a Stage must still let background clicks dismiss selection. */
export async function verifyUnavailableCanvas(page, address, projectId) {
  for (const status of ["pending", "failed"]) {
    const scene = async (route) =>
      route.fulfill({
        json: {
          jobId: "unavailable",
          projectId,
          revision: 2,
          frameId: "frame",
          status,
          error: { code: "WORKER_FAILED", message: "Unavailable fixture" },
        },
      });
    await page.route("**/scene?**", scene);
    await page.goto(address("box"));
    await page
      .getByRole("heading", { name: "Export box updated", exact: true })
      .waitFor();
    await page
      .getByRole("heading", {
        name:
          status === "pending" ? "Rendering this revision" : "Render failed",
        exact: true,
      })
      .waitFor();
    await page.evaluate(() => {
      const input = document.createElement("input");
      input.id = "pending-focus";
      document.body.append(input);
      input.focus();
    });
    const host = await page.locator(".canvas-host").boundingBox();
    await page.mouse.click(host.x + 10, host.y + 10);
    assert.equal(await page.locator(".inspector h2").count(), 0);
    assert.notEqual(
      await page.evaluate(() => document.activeElement.id),
      "pending-focus",
    );
    await page.unroute("**/scene?**", scene);
  }
  console.log(
    "ok - pending/failed canvas backgrounds deselect and blur without a Stage",
  );
}
