import { fileURLToPath } from "node:url";
import { createServer } from "vite";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(
  new URL("../../.runtime/browsers", import.meta.url),
);
const { chromium } = await import("playwright");
export const projects = [
  { projectId: "p", slug: "alpha", name: "Alpha", revision: 3 },
  { projectId: "q", slug: "beta", name: "Beta", revision: 2 },
];
export function snapshot(projectId = "p", revision = 3) {
  const node = (id, parentId, kind, props = {}) => ({
    id,
    parentId,
    kind,
    sortKey: 0,
    props,
  });
  return {
    ...projects.find((project) => project.projectId === projectId),
    revision,
    schemaVersion: 1,
    nodes: [
      node("page", null, "page"),
      node("page2", null, "page"),
      node("frame", "page", "frame"),
      node("frame2", "page2", "frame"),
      node("node", "frame", "rect"),
      node("master", "page", "frame"),
      node("label", "master", "text"),
      node("instance", "frame", "instance", { componentId: "master" }),
    ],
    tokens: [],
    styles: [],
    fonts: [],
    assets: [],
    truncated: false,
    retention: { oldestCommitRevision: 1, idempotencyPolicy: "bounded" },
  };
}

export async function navigationHarness(t, query) {
  const vite = await createServer({
    root: fileURLToPath(new URL("../", import.meta.url)),
    server: { host: "127.0.0.1", port: 0 },
    logLevel: "error",
  });
  await vite.listen();
  const browser = await chromium.launch({ headless: true });
  t.after(async () => {
    await browser.close();
    await vite.close();
  });
  const page = await browser.newPage();
  page.setDefaultTimeout(5000);
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.addInitScript(() => {
    localStorage.setItem(
      "opencanvas.selection",
      JSON.stringify({
        projectId: "wrong",
        pageId: "wrong",
        frameId: "wrong",
        nodeId: "wrong",
      }),
    );
    history.replaceState({ otherOwner: "retained" }, "");
  });
  await page.route("**/url-navigation-test**", (route) =>
    route.fulfill({
      contentType: "text/html",
      body: '<div id="root"></div><script type="module" src="/tests/project-hooks-fixture.tsx"></script>',
    }),
  );
  await page.goto(`${vite.resolvedUrls.local[0]}url-navigation-test${query}`);
  const probe = (method, ...args) =>
    page.evaluate(
      ([method, args]) => window.projectProbe[method](...args),
      [method, args],
    );
  const pending = async (path) => {
    await page.waitForFunction(
      (path) =>
        window.projectProbe
          ?.requests()
          .some(
            (request) =>
              request.path === path && !request.done && !request.aborted,
          ),
      path,
    );
    return (await probe("requests")).findLast(
      (request) => request.path === path && !request.done && !request.aborted,
    ).id;
  };
  const reply = async (path, body, etag, status) =>
    probe("reply", await pending(path), body, etag, status);
  const settle = () =>
    page.evaluate(
      () =>
        new Promise((resolve) =>
          requestAnimationFrame(() => requestAnimationFrame(resolve)),
        ),
    );
  const value = () => probe("value");
  const selected = async (selection) => {
    await page.waitForFunction(
      (selection) =>
        Object.entries(selection).every(
          ([key, value]) =>
            window.projectProbe.value().selection[key] === value,
        ),
      selection,
    );
    await settle();
  };
  const loaded = async (projectId, revision) => {
    await page.waitForFunction(
      ([projectId, revision]) =>
        window.projectProbe.value().snapshot?.projectId === projectId &&
        window.projectProbe.value().snapshot?.revision === revision,
      [projectId, revision],
    );
    await settle();
  };
  const source = async () =>
    (await probe("sources")).findLastIndex((entry) => !entry.closed);
  const count = async (path) =>
    (await probe("requests")).filter((request) => request.path === path).length;
  const address = () => probe("address");
  return {
    page,
    probe,
    pending,
    reply,
    settle,
    value,
    selected,
    loaded,
    source,
    count,
    address,
    errors,
  };
}

export const listPath = "/api/v1/projects";
export const snapshotPath = (id) => `/api/v1/projects/${id}/snapshot`;
