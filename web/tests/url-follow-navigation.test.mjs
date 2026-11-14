import assert from "node:assert/strict";
import test from "node:test";
import {
  listPath,
  navigationHarness,
  projects,
  snapshot,
  snapshotPath,
} from "./url-navigation-support.mjs";

test("follow validates unlisted hints and cancels obsolete asynchronous navigation", async (t) => {
  const h = await navigationHarness(t, "?project=p&page=page&frame=frame");
  await h.reply(listPath, { projects });
  await h.reply(snapshotPath("p"), snapshot(), '"p3"');
  await h.loaded("p", 3);
  await h.probe("follow", true);
  await h.settle();
  const created = (id) =>
    h.probe("emit", hSource, "project_created", { projectId: id, name: id });
  const hSource = await h.source();
  const initial = await h.address();
  await created("hidden");
  await h.reply(listPath, { projects: structuredClone(projects) });
  await h.page.waitForFunction(() =>
    /unavailable/.test(window.projectProbe.value().notice),
  );
  assert.equal(
    (await h.address()).href,
    initial.href,
    "an unchanged authoritative list settles a rejected hint without navigating",
  );

  await created("r");
  await h.pending(listPath);
  await h.probe("scope", { nodeId: "node" });
  await h.selected({ nodeId: "node" });
  const r = { projectId: "r", slug: "r", name: "r", revision: 1 };
  await h.reply(listPath, { projects: [...projects, r] });
  await h.settle();
  assert.equal(
    (await h.value()).selection.projectId,
    "p",
    "user scope navigation cancels a pending follow",
  );
  assert.equal((await h.value()).selection.nodeId, "node");

  await created("s");
  await h.pending(listPath);
  await h.probe("follow", false);
  await h.settle();
  const s = { projectId: "s", slug: "s", name: "s", revision: 1 };
  await h.reply(listPath, { projects: [...projects, r, s] });
  await h.settle();
  assert.equal(
    (await h.value()).selection.projectId,
    "p",
    "turning follow off cancels the pending intent",
  );
  await h.probe("follow", true);
  await h.settle();

  await created("new");
  const added = { projectId: "new", slug: "new", name: "new", revision: 1 };
  await h.reply(listPath, { projects: [...projects, r, s, added] });
  await h.reply(
    snapshotPath("new"),
    { ...snapshot("p", 1), ...added },
    '"new1"',
  );
  await h.loaded("new", 1);
  await h.selected({
    projectId: "new",
    pageId: "page",
    frameId: ":all:",
    nodeId: "",
  });
  assert.match((await h.value()).notice, /Following the agent/);
  assert.equal(
    (await h.address()).length,
    initial.length + 2,
    "only the explicit layer choice and accepted follow push",
  );

  await h.probe("emit", await h.source(), "project_created", {
    projectId: "after-unmount",
    name: "after-unmount",
  });
  const pending = await h.pending(listPath);
  const beforeUnmount = await h.address();
  await h.probe("unmount");
  await h.probe("reply", pending, {
    projects: [
      ...projects,
      { projectId: "after-unmount", name: "late", slug: "late", revision: 1 },
    ],
  });
  await h.settle();
  assert.equal(
    (await h.address()).href,
    beforeUnmount.href,
    "an unmounted stream cannot complete a follow navigation",
  );
  assert.deepEqual(h.errors, []);
});
