import assert from "node:assert/strict";
import { FIXTURE } from "./preview-fixture.mjs";
import { settle } from "./preview-settle.mjs";

const { source, first, second, animation } = FIXTURE.parentHover;
const ids = [
  `${first}:${animation}`,
  `${first}:${animation}:fill`,
  `${second}:${animation}`,
];
const selected = `${JSON.stringify(ids)}.map((id) => document.getAnimations().find((a) => a.id === id))`;
const selector = (id) => `[data-oc-id="${id}"]`;
const outside = { x: 1090, y: 750 };

async function points(page) {
  const left = await page.locator(selector(first)).boundingBox();
  const right = await page.locator(selector(second)).boundingBox();
  const centerY = (box) => box.y + box.height / 2;
  return {
    left: { x: left.x + left.width / 2, y: centerY(left) },
    right: { x: right.x + right.width / 2, y: centerY(right) },
    gap: { x: (left.x + left.width + right.x) / 2, y: centerY(left) },
  };
}

function firePair(group, pointerType) {
  group.dispatchEvent(new PointerEvent("pointerenter", { pointerType }));
  const entered = group.querySelectorAll(".oc-node, .oc-box");
  const active = [...entered].flatMap((element) =>
    element.getAnimations(),
  ).length;
  group.dispatchEvent(new PointerEvent("pointerleave", { pointerType }));
  const remaining = [...entered].flatMap((element) =>
    element.getAnimations(),
  ).length;
  return [active, remaining];
}

export async function checkParentHover(page) {
  const group = page.locator(selector(source));
  const hit = await points(page);
  assert.equal(
    await page.evaluate(
      ({ x, y }) => document.elementFromPoint(x, y).parentElement.dataset.ocId,
      hit.gap,
    ),
    source,
  );
  assert.deepEqual(await group.evaluate(firePair, "touch"), [0, 0]);
  assert.deepEqual(await group.evaluate(firePair, "mouse"), [3, 0]);

  await page.mouse.move(hit.gap.x, hit.gap.y);
  await page.waitForFunction(`${selected}.every(Boolean)`);
  assert.deepEqual(
    await page.evaluate(
      `${selected}.map((a) => [a.effect.target.classList.contains("oc-box"), a.effect.target.closest("[data-oc-id]").dataset.ocId])`,
    ),
    [
      [false, first],
      [true, first],
      [false, second],
    ],
  );

  // Pause at a non-endpoint time so any second enter/leave within the group is observable.
  const midpoint = 250;
  await page.evaluate(`${selected}.forEach((a) => a.pause())`);
  await settle(
    page,
    "pause parent tracks",
    `Promise.all(${selected}.map((a) => a.ready))`,
  );
  await page.evaluate(
    `${selected}.forEach((a) => { a.currentTime = ${midpoint}; })`,
  );
  for (const point of [hit.left, hit.gap, hit.right]) {
    await page.mouse.move(point.x, point.y);
    assert.deepEqual(
      await page.evaluate(
        `${selected}.map((a) => [a.playState, a.playbackRate, a.currentTime])`,
      ),
      ids.map(() => ["paused", 1, midpoint]),
    );
  }
  assert.deepEqual(await group.evaluate(firePair, "touch"), [3, 3]);
  assert.deepEqual(
    await page.evaluate(
      `${selected}.map((a) => [a.playState, a.playbackRate, a.currentTime])`,
    ),
    ids.map(() => ["paused", 1, midpoint]),
  );

  // Capture synchronously after preview handles the real leave, before the next frame.
  await group.evaluate((element, animationIds) => {
    element.addEventListener(
      "pointerleave",
      () => {
        window.parentHoverLeave = animationIds.map((id) => {
          const track = document
            .getAnimations()
            .find((entry) => entry.id === id);
          return [track.playbackRate, track.currentTime];
        });
      },
      { once: true },
    );
  }, ids);
  await page.mouse.move(outside.x, outside.y);
  assert.deepEqual(
    await page.evaluate(() => window.parentHoverLeave),
    ids.map(() => [-1, midpoint]),
  );
  await settle(
    page,
    "parent hover reverse",
    `Promise.all(${selected}.map((a) => a.finished))`,
  );
  assert.deepEqual(
    await page.evaluate(
      `${selected}.map((a) => { const style = getComputedStyle(a.effect.target); return a.id.endsWith(":fill") ? style.backgroundColor : style.opacity; })`,
    ),
    ["1", "rgb(34, 102, 204)", "1"],
  );
}

export async function checkReducedParentHover(page) {
  const { gap } = await points(page);
  await page.mouse.move(gap.x, gap.y);
  await page.waitForFunction(
    `${selected}.every((a) => a?.playState === "finished")`,
  );
  assert.deepEqual(
    await page.evaluate(
      `${selected}.map((a) => { const style = getComputedStyle(a.effect.target); return a.id.endsWith(":fill") ? style.backgroundColor : style.opacity; })`,
    ),
    ["0.25", "rgb(239, 68, 68)", "0.5"],
  );
  await page.mouse.move(outside.x, outside.y);
  assert.deepEqual(await page.evaluate(selected), [
    undefined,
    undefined,
    undefined,
  ]);
}

export async function checkMissingParentHover(page, projectId) {
  const missing = "missing-hover-source";
  // Storage disallows this. Simulate a preview whose snapshot and scene disagree.
  await page.route(
    `**/api/v1/projects/${encodeURIComponent(projectId)}/snapshot`,
    async (route) => {
      // Fetch a fresh body without the browser's conditional request headers.
      const response = await route.fetch({
        headers: { Accept: "application/json" },
      });
      assert.equal(response.status(), 200);
      const snapshot = await response.json();
      const target = snapshot.nodes.find((node) => node.id === first);
      target.props.animations[0].triggerNodeId = missing;
      await route.fulfill({ json: snapshot });
    },
    { times: 1 },
  );
  await page.getByRole("button", { name: "Reload" }).click();
  await page
    .getByText(
      `${ids[0]}: hover trigger "${missing}" is missing from the preview; animation skipped`,
    )
    .waitFor();
  await page.locator(selector(first)).hover();
  await page.waitForFunction(`!!${selected}[2]`);
  assert.deepEqual(await page.evaluate(`${selected}.map((a) => a?.id)`), [
    undefined,
    undefined,
    ids[2],
  ]);
}
