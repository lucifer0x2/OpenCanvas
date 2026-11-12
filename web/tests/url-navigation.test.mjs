import assert from "node:assert/strict";
import test from "node:test";
import {
  listPath,
  navigationHarness,
  projects,
  snapshot,
  snapshotPath,
} from "./url-navigation-support.mjs";

test("URL owns scope, optional/virtual nodes, history, extras and warm project cache in StrictMode", async (t) => {
  const h = await navigationHarness(
    t,
    "?project=p&page=page&frame=frame&scene=server#kept",
  );
  const initial = await h.address();
  await h.reply(snapshotPath("p"), snapshot(), '"p3"');
  await h.reply(listPath, { projects });
  await h.loaded("p", 3);
  await h.selected({
    projectId: "p",
    pageId: "page",
    frameId: "frame",
    nodeId: "",
  });
  assert.equal(
    (await h.address()).length,
    initial.length,
    "normalization never pushes history",
  );
  assert.equal(
    new URL((await h.address()).href).searchParams.has("node"),
    false,
  );
  assert.equal(
    await h.page.evaluate(
      () => JSON.parse(localStorage.getItem("opencanvas.selection")).projectId,
    ),
    "wrong",
    "legacy storage is neither read nor written",
  );

  const expectAddress = async (expected) => {
    const address = await h.address();
    const url = new URL(address.href);
    for (const [key, value] of Object.entries(expected))
      assert.equal(url.searchParams.get(key), value);
    assert.equal(url.searchParams.get("scene"), "server");
    assert.equal(url.hash, "#kept");
    assert.deepEqual(address.state, { otherOwner: "retained" });
  };
  const oldAcceptance = await h.probe("captureAcceptance");
  await h.probe("scope", { nodeId: "instance/label" });
  await h.selected({ nodeId: "instance/label" });
  await expectAddress({
    project: "p",
    page: "page",
    frame: "frame",
    node: "instance/label",
  });
  assert.match((await h.address()).href, /node=instance%2Flabel/);
  const virtual = await h.address();
  const obsoleteSnapshot = snapshot();
  obsoleteSnapshot.nodes = obsoleteSnapshot.nodes.filter(
    (node) => node.id !== "label",
  );
  await h.probe("acceptCaptured", oldAcceptance, obsoleteSnapshot);
  await h.settle();
  assert.equal(
    (await h.address()).href,
    virtual.href,
    "an old same-project acceptance cannot rewrite a newer URL",
  );
  assert.equal((await h.value()).selection.nodeId, "instance/label");
  await h.probe("scope", { nodeId: "instance/label" });
  await h.probe("select", "p");
  await h.settle();
  assert.equal(
    (await h.address()).length,
    virtual.length,
    "no-op node/project changes do not push",
  );
  await h.probe("scope", { nodeId: "" });
  await h.selected({ nodeId: "" });
  await expectAddress({ node: null });
  const deselected = await h.address();
  await h.page.goBack();
  await h.selected({ nodeId: "instance/label", frameId: "frame" });
  await h.page.goForward();
  await h.selected({ nodeId: "" });
  assert.equal((await h.address()).href, deselected.href);

  await h.probe("scope", { pageId: "page2", frameId: ":all:", nodeId: "" });
  await h.selected({ pageId: "page2", frameId: ":all:" });
  await h.probe("scope", { frameId: "frame2", nodeId: "frame2" });
  await h.selected({ frameId: "frame2", nodeId: "frame2" });
  await h.probe("select", "q");
  assert.equal(
    (await h.value()).snapshot,
    null,
    "a project change cannot expose the previous snapshot",
  );
  await h.reply(snapshotPath("q"), snapshot("q", 2), '"q2"');
  await h.loaded("q", 2);
  await h.selected({
    projectId: "q",
    pageId: "page",
    frameId: ":all:",
    nodeId: "",
  });
  await expectAddress({
    project: "q",
    page: "page",
    frame: ":all:",
    node: null,
  });
  await h.page.goBack();
  await h.loaded("p", 3);
  await h.selected({ pageId: "page2", frameId: "frame2", nodeId: "frame2" });
  await h.page.goBack();
  await h.selected({ pageId: "page2", frameId: ":all:", nodeId: "" });
  await h.page.goBack();
  await h.selected({ pageId: "page", frameId: "frame", nodeId: "" });
  await h.page.goForward();
  await h.page.goForward();
  await h.page.goForward();
  await h.loaded("q", 2);
  await h.selected({ projectId: "q", frameId: ":all:" });
  assert.equal(await h.count(snapshotPath("p")), 1);
  assert.equal(
    await h.count(snapshotPath("q")),
    1,
    "scope/history navigation never refetches warm snapshots",
  );
  assert.equal(
    (await h.probe("sources")).filter((source) => !source.closed).length,
    1,
  );

  // A delayed old-project response cannot reconcile the current history address.
  await h.page.goBack();
  await h.loaded("p", 3);
  await h.probe("emit", await h.source(), "commit", {
    projectId: "p",
    revision: 4,
  });
  const obsolete = await h.pending(snapshotPath("p"));
  await h.page.goForward();
  await h.loaded("q", 2);
  const before = await h.address();
  await h.probe("reply", obsolete, snapshot("p", 4), '"p4"');
  await h.settle();
  assert.equal((await h.value()).snapshot.projectId, "q");
  assert.equal((await h.address()).href, before.href);
  assert.equal((await h.address()).length, before.length);
  await h.probe("emit", await h.source(), "commit", {
    projectId: "q",
    revision: 3,
  });
  await h.reply(snapshotPath("q"), snapshot("q", 3), '"q3"');
  await h.loaded("q", 3);
  assert.equal(
    (await h.address()).length,
    before.length,
    "accepted revisions never push history",
  );
  assert.deepEqual(h.errors, []);
});

test("four-key virtual links and explicit deselection survive reload", async (t) => {
  const h = await navigationHarness(
    t,
    "?project=p&page=page&frame=frame&node=instance%2Flabel",
  );
  const load = async () => {
    await h.reply(listPath, { projects });
    await h.reply(snapshotPath("p"), snapshot(), '"p3"');
    await h.loaded("p", 3);
  };
  await load();
  await h.selected({ nodeId: "instance/label" });
  await h.page.reload();
  await load();
  await h.selected({ frameId: "frame", nodeId: "instance/label" });
  await h.probe("scope", { nodeId: "" });
  await h.selected({ nodeId: "" });
  await h.page.reload();
  await load();
  await h.selected({ frameId: "frame", nodeId: "" });
  assert.equal(
    new URL((await h.address()).href).searchParams.has("node"),
    false,
  );
  assert.deepEqual(h.errors, []);
});
