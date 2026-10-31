import assert from "node:assert/strict";
import {
  camera,
  settled,
  withContinuityViewer,
} from "./canvas-continuity-harness.mjs";

await withContinuityViewer(async (page, fixture, errors) => {
  const ready = async (rev) => {
    await page.getByText(`Scene r${rev}`, { exact: true }).waitFor();
    await settled(page);
  };
  const pending = async (rev) => {
    await page.getByText(`Authored r${rev}`, { exact: true }).waitFor();
    await page.getByText("Scene pending", { exact: true }).waitFor();
    await settled(page);
  };
  const sameCamera = async (expected, message) =>
    assert.deepEqual(await camera(page), expected, message);
  const choose = async (label, name) => {
    await page.getByRole("combobox", { name: label, exact: true }).click();
    await page.getByRole("option", { name, exact: true }).click();
  };
  const manual = async () => {
    await page.getByRole("button", { name: "Zoom in", exact: true }).click();
    await page.locator(".canvas-host").focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("ArrowDown");
    await settled(page);
    return camera(page);
  };
  await settled(page);
  await page.getByRole("button", { name: "Welcome", exact: true }).click();
  assert.equal(
    await page.locator(".inspector .bounds-grid").count(),
    1,
    "computed geometry is present before refresh",
  );
  await page.evaluate(() => {
    window.__originalStage = window.Konva.stages[0];
    window.__originalCanvas = document.querySelector("canvas");
  });
  let expected = await manual();
  fixture.update(8, "pending");
  await pending(8);
  assert.ok(
    await page.evaluate(
      () =>
        window.Konva.stages[0] === window.__originalStage &&
        document.querySelector("canvas") === window.__originalCanvas,
    ),
    "revision pending retains the mounted Stage and canvas",
  );
  await sameCamera(expected, "pending keeps camera");
  await page.getByText(/Showing scene r7/).waitFor();
  assert.equal(
    await page.locator(".inspector .bounds-grid").count(),
    0,
    "old computed geometry is suppressed",
  );
  assert.equal(
    await page
      .locator(".canvas-refresh")
      .evaluate((el) => getComputedStyle(el).pointerEvents),
    "none",
  );
  // Zoom remains available over the retained scene while the new job is pending.
  expected = await manual();
  fixture.update(9, "pending");
  await pending(9);
  fixture.update(10);
  await ready(10);
  await sameCamera(expected, "rapid revisions preserve manual camera");
  fixture.complete(8);
  fixture.complete(9);
  await page.waitForTimeout(650);
  await ready(10);
  await sameCamera(
    expected,
    "obsolete jobs cannot reset camera or replace newer scene",
  );

  // A new scene arrives while the pointer stays down. The same native drag must continue.
  const host = await page.locator(".canvas-host").boundingBox();
  const point = { x: host.x + host.width / 2, y: host.y + host.height / 2 };
  await page.mouse.move(point.x, point.y);
  await page.mouse.down();
  await page.mouse.move(point.x + 35, point.y + 25, { steps: 6 });
  assert.equal(
    await page.evaluate(() => window.Konva.stages[0].isDragging()),
    true,
  );
  expected = await camera(page);
  fixture.update(11, "pending");
  await pending(11);
  await sameCamera(expected, "pending does not interrupt active drag");
  fixture.complete(11);
  await ready(11);
  assert.ok(
    await page.evaluate(
      () =>
        window.Konva.stages[0] === window.__originalStage &&
        window.Konva.stages[0].isDragging(),
    ),
    "ready swap retains active drag on the same Stage",
  );
  await sameCamera(expected, "ready swap does not rewind the drag");
  await page.mouse.move(point.x + 85, point.y + 60, { steps: 6 });
  const moved = await camera(page);
  assert.ok(
    Math.abs(moved.x - expected.x - 50) <= 1 &&
      Math.abs(moved.y - expected.y - 35) <= 1,
    "active drag continues from its pre-refresh position",
  );
  await page.mouse.up();
  expected = await camera(page);

  fixture.update(12, "ready", 1100);
  await ready(12);
  await sameCamera(
    expected,
    "authored frame dimension changes preserve camera",
  );
  await page.getByRole("button", { name: "Hide layers panel" }).click();
  await settled(page);
  await sameCamera(expected, "panel toggle preserves camera");
  await page.setViewportSize({ width: 1200, height: 850 });
  await settled(page);
  await sameCamera(expected, "window resize preserves camera");

  fixture.update(13, "failed");
  await page
    .getByRole("heading", { name: "Render failed", exact: true })
    .waitFor();
  await page.getByText(/Showing scene r12/).waitFor();
  await sameCamera(expected, "render failure retains camera");
  fixture.setMode("ready");
  await page.getByRole("button", { name: "Retry render" }).click();
  await ready(13);
  await sameCamera(expected, "retry retains camera");
  fixture.update(14, "mismatch");
  await page
    .getByRole("heading", { name: "Render failed", exact: true })
    .waitFor();
  await page.getByText(/does not match this revision or viewport/).waitFor();
  await page.getByText(/Showing scene r13/).waitFor();
  fixture.setMode("ready");
  await page.getByRole("button", { name: "Retry render" }).click();
  await ready(14);

  await page.getByRole("button", { name: "Fit frame", exact: true }).click();
  await settled(page);
  assert.notDeepEqual(
    await camera(page),
    expected,
    "explicit Fit uses updated frame and pane dimensions",
  );
  const fit = await camera(page);
  await manual();
  await page.keyboard.press("f");
  await settled(page);
  await sameCamera(fit, "keyboard F deliberately refits");

  // Navigation clears old content immediately, including preset changes with an old job.
  await manual();
  fixture.setMode("pending");
  await choose("Viewport", /^Mobile/);
  await page.getByText("Scene pending", { exact: true }).waitFor();
  assert.equal(
    await page.locator("canvas").count(),
    0,
    "preset navigation hides incompatible scene",
  );
  fixture.complete(14);
  await ready(14);
  const mobileFit = await camera(page);
  assert.notDeepEqual(mobileFit, fit, "new preset deliberately fits");
  await manual();
  fixture.setMode("pending");
  await choose("Frame", "Other frame");
  await page.getByText("Scene pending", { exact: true }).waitFor();
  assert.equal(
    await page.locator("canvas").count(),
    0,
    "frame navigation hides old frame",
  );
  fixture.complete(14);
  await ready(14);
  await sameCamera(
    mobileFit,
    "frame identity resets camera even with identical viewport dimensions",
  );
  await manual();
  fixture.setMode("pending");
  await choose("Project", "second");
  await page.getByText("Scene pending", { exact: true }).waitFor();
  assert.equal(
    await page.locator("canvas").count(),
    0,
    "project navigation hides old content",
  );
  fixture.complete(14);
  await ready(14);
  await sameCamera(
    mobileFit,
    "project identity resets camera even with colliding frame IDs",
  );
  assert.deepEqual(errors, []);
  assert.ok(fixture.requests.every((request) => request.method === "GET"));
  console.log(
    "canvas continuity: revision/drag/resize/failure/retry/fit/scope/stale-job/GET checks passed",
  );
});
