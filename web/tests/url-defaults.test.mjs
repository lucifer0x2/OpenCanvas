import assert from "node:assert/strict";
import test from "node:test";
import {
  listPath,
  navigationHarness,
  projects,
  snapshot,
  snapshotPath,
} from "./url-navigation-support.mjs";

test("partial, missing, invalid and empty addresses normalize only against authoritative data", async (t) => {
  for (const scenario of [
    { query: "?project=q&scene=server", projectId: "q" },
    { query: "?scene=server", projectId: "p" },
    {
      query: "?project=missing&page=gone&frame=gone",
      projectId: "p",
      notice: /project is unavailable/,
    },
    {
      query: "?project=p&page=gone&frame=gone",
      projectId: "p",
      notice: /page was removed/,
    },
    {
      query: "?project=p&page=page&frame=gone",
      projectId: "p",
      notice: /frame was removed/,
    },
    {
      query: "?project=p&page=page&frame=frame&node=gone",
      projectId: "p",
      frame: "frame",
      node: "frame",
      notice: /layer was removed/,
    },
    {
      query: "?project=p&page=page&frame=frame&node=instance%2Flabel",
      projectId: "p",
      frame: "frame",
      node: "instance/label",
    },
  ]) {
    await t.test(scenario.query, async (t) => {
      const h = await navigationHarness(t, scenario.query);
      const initial = await h.address();
      assert.equal(
        initial.href.endsWith(scenario.query),
        true,
        "no guessed defaults before authoritative list",
      );
      await h.reply(listPath, { projects });
      await h.reply(
        snapshotPath(scenario.projectId),
        snapshot(scenario.projectId, 3),
        '"v3"',
      );
      await h.loaded(scenario.projectId, 3);
      await h.selected({
        projectId: scenario.projectId,
        pageId: "page",
        frameId: scenario.frame ?? ":all:",
        nodeId: scenario.node ?? "",
      });
      const address = await h.address();
      const params = new URL(address.href).searchParams;
      assert.equal(params.get("project"), scenario.projectId);
      assert.equal(params.get("page"), "page");
      assert.equal(params.get("frame"), scenario.frame ?? ":all:");
      assert.equal(params.get("node"), scenario.node ?? null);
      if (scenario.notice)
        assert.match((await h.value()).notice, scenario.notice);
      assert.equal(address.length, initial.length);
      assert.deepEqual(h.errors, []);
    });
  }
  await t.test("empty workspace", async (t) => {
    const h = await navigationHarness(
      t,
      "?project=missing&page=gone&node=gone&scene=server",
    );
    const initial = await h.address();
    await h.reply(listPath, { projects: [] });
    await h.selected({ projectId: "", pageId: "", nodeId: "" });
    const address = await h.address();
    assert.equal(new URL(address.href).searchParams.has("project"), false);
    assert.equal(new URL(address.href).searchParams.has("page"), false);
    assert.equal(new URL(address.href).searchParams.has("node"), false);
    assert.equal(new URL(address.href).searchParams.has("frame"), false);
    assert.equal(address.length, initial.length);
    assert.deepEqual(h.errors, []);
  });
});

test("accepted snapshots reconcile partial and stale addresses while the project list is unavailable", async (t) => {
  for (const scenario of [
    { query: "?project=p", page: "page" },
    {
      query: "?project=p&page=gone&frame=gone",
      page: "page",
      notice: /page was removed/,
    },
    {
      query: "?project=p&page=page&frame=gone",
      page: "page",
      notice: /frame was removed/,
    },
    { query: "?project=p&frame=frame2", page: "page2", frame: "frame2" },
  ]) {
    await t.test(scenario.query, async (t) => {
      const h = await navigationHarness(t, scenario.query);
      const initial = await h.address();
      await h.reply(
        listPath,
        { error: { code: "UNAVAILABLE", message: "List failed." } },
        undefined,
        503,
      );
      await h.page.waitForFunction(() => window.projectProbe.value().error);
      await h.reply(snapshotPath("p"), snapshot("p", 3), '"v3"');
      await h.loaded("p", 3);
      const frame = scenario.frame ?? ":all:";
      await h.selected({
        projectId: "p",
        pageId: scenario.page,
        frameId: frame,
      });
      const address = await h.address();
      const params = new URL(address.href).searchParams;
      assert.equal(params.get("project"), "p");
      assert.equal(params.get("page"), scenario.page);
      assert.equal(params.get("frame"), frame);
      assert.equal(params.has("node"), false);
      assert.equal(address.length, initial.length, "defaults use replaceState");
      if (scenario.notice)
        assert.match((await h.value()).notice, scenario.notice);
      else assert.equal((await h.value()).notice, "");
      assert.deepEqual(h.errors, []);
    });
  }
});
