export function fixtureData(hash, getState) {
  const page = {
    id: "page",
    parentId: null,
    kind: "page",
    sortKey: 0,
    props: { name: "Website" },
  };
  const frame = {
    id: "frame",
    parentId: "page",
    kind: "frame",
    sortKey: 0,
    props: { name: "Welcome", width: 800, height: 600, fill: "#ffffff" },
  };
  const title = {
    id: "title",
    parentId: "frame",
    kind: "text",
    sortKey: 1,
    props: {
      name: "Headline",
      text: "A place for your next idea.",
      fontId: "font_bundled",
      fontSize: 42,
      lineHeight: 54,
      width: "fill",
      height: "hug",
      fill: "#26364a",
    },
  };
  const nodes = () => [
    page,
    frame,
    ...(getState().deleted ? [] : [title]),
    {
      id: "body",
      parentId: "frame",
      kind: "text",
      sortKey: 2,
      props: {
        name: "Description",
        text: "Thoughtful tools for a more creative everyday.",
        fontId: "font_bundled",
        fontSize: 18,
        lineHeight: 28,
      },
    },
  ];
  const baseNode = (id, kind, bounds, fill, extras = {}) => ({
    id,
    kind,
    parentId: id === "frame" ? null : "frame",
    bounds,
    fill,
    lines: [],
    fontId: null,
    fontSize: 18,
    lineHeight: 28,
    assetId: null,
    fit: "contain",
    opacity: 1,
    clip: false,
    radius: 0,
    crop: null,
    ...extras,
  });
  const scene = (rev, width, height) => ({
    schemaVersion: 1,
    coordinateSpace: "frame-local",
    diagnostics: [],
    provenance: {
      projectId: "fixture",
      revision: rev,
      frameId: "frame",
      viewport: { width, height },
      theme: "default",
      state: "default",
      sceneVersion: "scene-2",
      rendererVersion: "web-2",
      sourceSceneKey: `key-${rev}-${width}`,
      fontManifestHash: hash,
      assetManifestHash: "none",
    },
    fonts: [
      {
        id: "font_bundled",
        family: "OpenCanvas Noto Sans",
        hash,
        url:
          getState().mode === "missing"
            ? "/api/v1/missing-font"
            : "/api/v1/font",
        weight: 400,
        style: "normal",
      },
    ],
    assets:
      getState().mode === "missing"
        ? [
            {
              id: "missing",
              hash,
              url: "/api/v1/missing-image",
              mediaType: "image/png",
            },
          ]
        : [],
    nodes: [
      baseNode("frame", "frame", { x: 0, y: 0, width, height }, "#fff"),
      baseNode(
        "accent",
        "rectangle",
        { x: 56, y: 60, width: 44, height: 5 },
        "#355f52",
        { radius: 2 },
      ),
      ...(getState().deleted
        ? []
        : [
            baseNode(
              "title",
              "text",
              { x: 56, y: 102, width: 650, height: 108 },
              "#26364a",
              {
                fontId: "font_bundled",
                fontSize: 42,
                lineHeight: 54,
                lines: [
                  { text: "A place for your", x: 0, y: 43, width: 410 },
                  { text: "next idea.", x: 0, y: 97, width: 300 },
                ],
              },
            ),
          ]),
      baseNode(
        "body",
        "text",
        { x: 56, y: 240, width: 680, height: 56 },
        "#627083",
        {
          fontId: "font_bundled",
          lines: [
            {
              text: "Thoughtful tools for a more creative everyday.",
              x: 0,
              y: 21,
              width: 600,
            },
          ],
        },
      ),
      baseNode(
        "art",
        "rectangle",
        { x: 56, y: 337, width: 688, height: 195 },
        "#e7eee9",
        { radius: 12 },
      ),
      baseNode(
        "art-inner",
        "rectangle",
        { x: 92, y: 371, width: 320, height: 127 },
        "#cadbd0",
        { radius: 8 },
      ),
      ...(getState().mode === "missing"
        ? [
            baseNode(
              "image",
              "image",
              { x: 490, y: 370, width: 210, height: 120 },
              "transparent",
              { assetId: "missing" },
            ),
          ]
        : []),
    ],
  });
  return { nodes, scene };
}
