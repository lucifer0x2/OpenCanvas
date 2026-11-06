export function serverFrameJob(hash, frameId, width, height, rev) {
  const key = `${frameId}-${width}-${height}-${rev}`;
  const sceneNode = (id, kind, bounds, fill) => ({
    id,
    kind,
    parentId: kind === "frame" ? null : frameId,
    bounds,
    fill,
    lines: [],
    fontId: null,
    fontSize: 16,
    lineHeight: 24,
    assetId: null,
    fit: "contain",
    opacity: 1,
    clip: false,
    radius: 0,
    crop: null,
  });
  return {
    jobId: key,
    projectId: "all-fixture",
    revision: rev,
    frameId,
    status: "ready",
    sceneKey: key,
    scene: {
      schemaVersion: 1,
      coordinateSpace: "frame-local",
      fonts: [],
      assets: [],
      diagnostics: [],
      provenance: {
        projectId: "all-fixture",
        revision: rev,
        frameId,
        viewport: { width, height },
        theme: "default",
        state: "default",
        sourceSceneKey: key,
        sceneVersion: "scene-3",
        rendererVersion: "web-4",
        fontManifestHash: hash,
        assetManifestHash: "none",
      },
      nodes: [
        sceneNode(frameId, "frame", { x: 0, y: 0, width, height }, "#fff"),
        sceneNode(
          `${frameId}-shape`,
          "rectangle",
          { x: 24, y: 32, width: 220, height: 160 },
          "#dae8f0",
        ),
      ],
    },
  };
}
