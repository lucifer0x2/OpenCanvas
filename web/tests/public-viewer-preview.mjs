import assert from "node:assert/strict";
import { join } from "node:path";

export async function verifyPublicPreview({
  viewer,
  preview,
  mcp,
  projectId,
  artifacts,
}) {
  const viewerUrl = viewer.url();
  const url = new URL(viewerUrl);
  url.pathname = "/preview";
  await preview.goto(url.href);
  const project = viewer.getByRole("combobox", {
    name: "Project",
    exact: true,
  });
  const text = preview
    .locator('.oc-node[data-oc-id="text"]')
    .getByText("Public live Việt", { exact: true });
  await text.waitFor();
  const image = preview.locator("img.oc-image");
  await image.waitFor();
  assert.equal(
    await image.evaluate(
      (element) => element.complete && element.naturalWidth > 0,
    ),
    true,
  );
  assert.ok((await image.getAttribute("src")).startsWith(url.origin));
  await preview.screenshot({ path: join(artifacts, "preview.png") });
  const navigations = [];
  for (const tab of [viewer, preview])
    tab.on("domcontentloaded", () => navigations.push(tab.url()));

  await mcp.call("update_project", { projectId, privacy_status: "private" });
  await project.getByText("No projects yet", { exact: true }).waitFor();
  await preview
    .locator(".pv-stage")
    .getByText("Project unavailable.")
    .waitFor();
  assert.equal(await project.isDisabled(), true);
  assert.equal(await viewer.locator("canvas").count(), 0);
  assert.equal(
    await viewer.getByRole("button", { name: "Public label" }).count(),
    0,
  );
  assert.equal(await preview.locator(".oc-node").count(), 0);

  await mcp.call("update_project", { projectId, privacy_status: "public" });
  await project.getByText("Public smoke", { exact: true }).waitFor();
  await viewer.getByText("Scene r4", { exact: true }).waitFor();
  await text.waitFor();
  assert.equal(viewer.url(), viewerUrl);
  assert.equal(preview.url(), url.href);
  assert.deepEqual(navigations, []);
}
