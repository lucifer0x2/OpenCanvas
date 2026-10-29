// Browser smoke test of /preview against a temporary production server and data root.
// Requires target/debug/opencanvas-server + layout-step, renderer/dist and web/dist.
import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { connectMcp } from "../../spikes/e2e/protocol.mjs";
import { startServer } from "../../spikes/e2e/server.mjs";
import { createFixture, FIXTURE } from "./preview-fixture.mjs";
import { frameParity } from "./preview-parity.mjs";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(
  new URL("../../.runtime/browsers", import.meta.url),
);
const { chromium } = await import("playwright");
const artifacts = fileURLToPath(
  new URL("../../artifacts/preview/", import.meta.url),
);
const dataRoot = await mkdtemp(join(tmpdir(), "opencanvas-preview-smoke-"));
const server = await startServer(dataRoot);
let browser;
const step = (name) => console.log(`ok - ${name}`);
const sel = (id) => `[data-oc-id="${id}"]`;

try {
  const mcp = await connectMcp(server.base);
  const projectId = await createFixture(mcp);
  await mcp.close();
  browser = await chromium.launch();
  const page = await browser.newPage({
    viewport: { width: 1100, height: 760 },
  });
  const methods = new Set();
  const errors = [];
  page.on("request", (request) => methods.add(request.method()));
  page.on("pageerror", (error) => errors.push(error.message));
  const anim = (id) =>
    `document.getAnimations().find((a) => a.id === ${JSON.stringify(id)})`;
  const style = (selector, property) =>
    page.$eval(selector, (el, p) => getComputedStyle(el)[p], property);

  await page.goto(`${server.base}/preview?project=${projectId}&frame=home`);
  const loadId = `${FIXTURE.load.id}:${FIXTURE.load.name}`;
  await page.waitForFunction(`!!${anim(loadId)}`, null, { timeout: 60_000 });

  // Load: fill both shows the offset-0 keyframe during the delay, then ends at rest.
  const load = await page.evaluate(`(() => {
    const a = ${anim(loadId)};
    const timing = a.effect.getComputedTiming();
    a.pause();
    a.currentTime = 100;
    const el = document.querySelector('${sel(FIXTURE.load.id)}');
    const during = { opacity: getComputedStyle(el).opacity, translate: getComputedStyle(el).translate };
    a.play();
    return { fill: timing.fill, delay: timing.delay, keyframes: a.effect.getKeyframes().map((k) => [k.offset, k.opacity, k.translate]), during };
  })()`);
  assert.equal(load.fill, "both");
  assert.equal(load.delay, FIXTURE.load.delay);
  assert.deepEqual(load.during, { opacity: "0", translate: "0px 24px" });
  assert.deepEqual(load.keyframes, [
    [0, "0", "0px 24px"],
    [1, "1", "0px"],
  ]);
  await page.evaluate(`${anim(loadId)}.finished`);
  assert.equal(await style(sel(FIXTURE.load.id), "opacity"), "1");
  assert.match(
    await style(sel(FIXTURE.load.id), "translate"),
    /^(none|0px|0px 0px)$/,
  );
  const tint = await page.evaluate(
    `${anim("title:tint:fill")}.effect.getKeyframes().map((k) => k.color)`,
  );
  assert.deepEqual(tint, ["rgb(34, 102, 204)", "rgb(21, 38, 59)"]);
  step(
    "load animation plays from its offset-0 keyframe with fill both and ends at rest",
  );

  // At rest the DOM matches the worker PNG of the same scene (text antialiasing aside).
  await page.evaluate(
    `Promise.all(document.getAnimations().map((a) => a.finished))`,
  );
  const mismatch = await frameParity(page, server.base, projectId, "home");
  assert.ok(mismatch < 0.01, `DOM vs worker PNG mismatch ${mismatch}`);
  step(
    `resting DOM matches the worker PNG (${(mismatch * 100).toFixed(3)}% pixels differ > 48/255)`,
  );

  // Hover: transition to the keyframe and stay; the state fill wins over the resting fill.
  const card = sel(FIXTURE.hover.id);
  const hoverId = `${FIXTURE.hover.id}:${FIXTURE.hover.name}`;
  await page.hover(card);
  await page.waitForFunction(`!!${anim(hoverId)}`);
  await page.evaluate(`${anim(hoverId)}.finished`);
  await page.waitForTimeout(FIXTURE.hover.duration + 150);
  assert.equal(await style(card, "scale"), String(FIXTURE.hover.scale));
  assert.equal(
    await style(`${card} > .oc-box`, "backgroundColor"),
    FIXTURE.hover.fill,
  );
  await mkdir(artifacts, { recursive: true });
  await page.screenshot({ path: join(artifacts, "hover.png") });
  await page.mouse.move(1090, 750);
  const leaving = await page.evaluate(
    `(() => { const a = ${anim(hoverId)}; return [a.playbackRate, a.playState]; })()`,
  );
  assert.deepEqual(leaving, [-1, "running"]);
  await page.evaluate(`${anim(hoverId)}.finished`);
  assert.match(await style(card, "scale"), /^(none|1)$/);
  assert.equal(
    await style(`${card} > .oc-box`, "backgroundColor"),
    FIXTURE.hover.rest,
  );
  step(
    "hover holds the end value and state fill, then animates back to rest on leave",
  );

  // Pressed state (:active) wins over the resting inline opacity; the click restarts every time.
  const clickId = `${FIXTURE.click.id}:${FIXTURE.click.name}`;
  const pulse = await page.locator(sel(FIXTURE.click.id)).boundingBox();
  await page.mouse.move(pulse.x + pulse.width / 2, pulse.y + pulse.height / 2);
  await page.mouse.down();
  assert.equal(await style(sel(FIXTURE.click.id), "opacity"), "0.6");
  await page.mouse.up();
  assert.equal(await style(sel(FIXTURE.click.id), "opacity"), "1");
  await page.waitForFunction(`!!${anim(clickId)}`);
  await page.evaluate(`${anim(clickId)}.finished`);
  const done = await page.evaluate(`${anim(clickId)}.playState`);
  await page.click(sel(FIXTURE.click.id));
  const again = await page.evaluate(
    `(() => { const a = ${anim(clickId)}; return [a.playState, a.currentTime]; })()`,
  );
  assert.equal(done, "finished");
  assert.equal(again[0], "running");
  assert.ok(again[1] < FIXTURE.click.duration, `restarted at ${again[1]}`);
  step(
    "pressed state applies while the pointer is down; click restarts its animation on every click",
  );

  // Links: navigate updates the URL frame; back returns.
  await page.click(sel(FIXTURE.link.id));
  await page.waitForSelector(sel(FIXTURE.back.id));
  assert.equal(new URL(page.url()).searchParams.get("frame"), FIXTURE.link.to);
  await page.click(sel(FIXTURE.back.id));
  await page.waitForSelector(sel(FIXTURE.hover.id));
  assert.equal(new URL(page.url()).searchParams.get("frame"), "home");
  step(
    "navigate link opens the target frame and updates the URL; back returns",
  );

  // Reduced motion: Replay jumps every load animation to its end.
  await page.getByLabel("Reduced motion").setChecked(true);
  await page.waitForFunction(`${anim(loadId)}?.playState === "finished"`);
  assert.equal(await style(sel(FIXTURE.load.id), "opacity"), "1");
  step("reduced motion settles load animations without playing them");

  assert.deepEqual([...methods], ["GET"]);
  assert.deepEqual(errors, []);
  step("preview sends only GET requests and raised no page errors");

  // The viewer header opens the preview of the selected frame in a new tab.
  await page.goto(`${server.base}/`);
  const button = page.getByRole("button", { name: /motion preview/i });
  await page.waitForFunction(() =>
    [...document.querySelectorAll("button")].some(
      (b) => b.textContent === "Preview" && !b.disabled,
    ),
  );
  const [opened] = await Promise.all([
    page.context().waitForEvent("page"),
    button.click(),
  ]);
  await opened.waitForURL(/\/preview\?/);
  const params = new URL(opened.url()).searchParams;
  assert.deepEqual(
    [params.get("project"), params.get("frame")],
    [projectId, "home"],
  );
  step("viewer Preview button opens /preview of the selected frame");
} catch (error) {
  console.error(server.output().slice(-4000));
  throw error;
} finally {
  await browser?.close();
  await server.stop();
  await rm(dataRoot, { recursive: true, force: true });
}
