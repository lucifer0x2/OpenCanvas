/** A minimal scene that passes the browser boundary checks for `request`. */
export const sceneFor = ({ projectId, revision, frameId, width, height }) => ({
  schemaVersion: 1,
  coordinateSpace: "frame-local",
  provenance: {
    projectId,
    revision,
    frameId,
    viewport: { width, height },
    theme: "default",
    state: "default",
    sceneVersion: "scene-3",
    rendererVersion: "web-4",
    sourceSceneKey: `${frameId}@${revision}`,
  },
  nodes: [],
  fonts: [],
  assets: [],
  diagnostics: [],
});
