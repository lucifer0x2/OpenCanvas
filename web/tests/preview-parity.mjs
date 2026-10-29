// Compares the preview DOM of a frame at rest with the worker PNG of the same scene.
import { waitFor } from "../../spikes/e2e/protocol.mjs";

async function workerPng(base, projectId, frameId, width, height) {
  const snapshot = await (
    await fetch(`${base}/api/v1/projects/${projectId}/snapshot`)
  ).json();
  const query = new URLSearchParams({
    frameId,
    revision: String(snapshot.revision),
    width: String(width),
    height: String(height),
    theme: "default",
    state: "default",
  });
  let job = await (
    await fetch(`${base}/api/v1/projects/${projectId}/scene?${query}`)
  ).json();
  job = await waitFor(async () => {
    const next = await (await fetch(`${base}/api/v1/jobs/${job.jobId}`)).json();
    if (next.status === "failed") throw new Error(JSON.stringify(next.error));
    return next.status === "ready" && next;
  }, "worker PNG");
  const image = await fetch(`${base}/api/v1/jobs/${job.jobId}/image`);
  return Buffer.from(await image.arrayBuffer());
}

/**
 * Share of pixels whose largest channel difference exceeds `threshold`, between a screenshot
 * of the 1:1 `.oc-scene` and the worker PNG (both decoded by the page's own canvas).
 */
export async function frameParity(
  page,
  base,
  projectId,
  frameId,
  threshold = 48,
) {
  const frame = page.locator(".oc-scene");
  const box = await frame.boundingBox();
  const width = Math.round(box.width);
  const height = Math.round(box.height);
  const dom = await frame.screenshot();
  const png = await workerPng(base, projectId, frameId, width, height);
  return page.evaluate(
    async ({ dom, png, width, height, threshold }) => {
      const pixels = async (base64) => {
        const image = new Image();
        image.src = `data:image/png;base64,${base64}`;
        await image.decode();
        const canvas = document.createElement("canvas");
        canvas.width = width;
        canvas.height = height;
        const context = canvas.getContext("2d");
        context.fillStyle = "white";
        context.fillRect(0, 0, width, height);
        context.drawImage(image, 0, 0, width, height);
        return context.getImageData(0, 0, width, height).data;
      };
      const [a, b] = await Promise.all([pixels(dom), pixels(png)]);
      let different = 0;
      for (let i = 0; i < a.length; i += 4) {
        const delta = Math.max(
          Math.abs(a[i] - b[i]),
          Math.abs(a[i + 1] - b[i + 1]),
          Math.abs(a[i + 2] - b[i + 2]),
        );
        if (delta > threshold) different++;
      }
      return different / (width * height);
    },
    {
      dom: dom.toString("base64"),
      png: png.toString("base64"),
      width,
      height,
      threshold,
    },
  );
}
