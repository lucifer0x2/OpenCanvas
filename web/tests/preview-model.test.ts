import assert from "node:assert/strict";
import test from "node:test";
import type { Scene, SceneNode } from "../../renderer/types.ts";
import { declaredStates, rootFrames } from "../src/preview/model.ts";
import { stateStyles } from "../src/preview/stateStyles.ts";
import type { DesignNode } from "../src/types.ts";

const node = (
  id: string,
  kind: string,
  parentId: string | null,
  props: Record<string, unknown> = {},
  sortKey = 0,
): DesignNode => ({ id, kind, parentId, props, sortKey });

test("declared states include instance masters; root frames keep page order", () => {
  const nodes = [
    node("page", "page", null),
    node("home", "frame", "page", {}, 0),
    node("button", "frame", "page", { component: { name: "Button" } }, 1),
    node("label", "text", "button", { states: { focus: { fill: "#000" } } }),
    node("cta", "instance", "home", { componentId: "button" }),
    node("card", "rectangle", "home", {
      states: { hover: {}, disabled: { opacity: 0.4 } },
    }),
  ];
  assert.deepEqual(declaredStates(nodes, "home"), ["focus", "disabled"]);
  assert.deepEqual(
    rootFrames(nodes).map((frame) => frame.id),
    ["home", "button"],
  );
});

const sceneNode = (id: string, extra: Partial<SceneNode> = {}): SceneNode => ({
  id,
  parentId: null,
  kind: "rectangle",
  bounds: { x: 0, y: 0, width: 10, height: 10 },
  lines: [],
  fontId: null,
  fontSize: 0,
  lineHeight: 0,
  fill: "#eeeeee",
  assetId: null,
  fit: "fit",
  opacity: 1,
  clip: false,
  radius: 4,
  crop: null,
  ...extra,
});
const scene = (nodes: SceneNode[]) => ({ nodes }) as unknown as Scene;

test("state differences become !important rules on the state selector", () => {
  const base = scene([sceneNode("a"), sceneNode("b"), sceneNode('q"x')]);
  const hover = scene([
    sceneNode("a", { fill: "#ff0000" }),
    sceneNode("b"),
    sceneNode('q"x', { opacity: 0.5 }),
  ]);
  const focus = scene([
    sceneNode("a", {
      stroke: {
        color: ["#00f", "#00f", "#00f", "#00f"],
        width: [2, 2, 2, 2],
        align: "outside",
      },
    }),
    sceneNode("b"),
    sceneNode('q"x'),
  ]);
  const styles = stateStyles(base, { hover, focus });
  assert.deepEqual(styles.changedIds.get("hover"), ["a", 'q"x']);
  assert.deepEqual(styles.changedIds.get("focus"), ["a"]);
  assert.deepEqual([...styles.strokeIds], ["a"]);
  const rules = styles.css.split("\n");
  assert.ok(
    rules.includes(
      '[data-oc-id="a"]:hover>.oc-box{background-color:#ff0000 !important}',
    ),
  );
  assert.ok(
    rules.includes('[data-oc-id="q\\"x"]:hover{opacity:0.5 !important}'),
  );
  const stroke = rules.find((rule) =>
    rule.startsWith('[data-oc-id="a"]:focus-visible>.oc-stroke'),
  );
  assert.match(stroke!, /display:block !important/);
  assert.match(stroke!, /top:-2px !important/);
  assert.match(
    stroke!,
    /border-radius:6px 6px 6px 6px \/ 6px 6px 6px 6px !important/,
  );
  assert.ok(!styles.css.includes('"b"'));
});
