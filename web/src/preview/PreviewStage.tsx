import { useEffect, useRef, useState } from "react";
import type { Resources, Scene } from "../../../renderer/types";
import type { NodeMotion } from "./model";
import { wireMotion } from "./motion";
import { buildSceneDom } from "./sceneDom";
import { stateStyles } from "./stateStyles";
import type { TokenTable } from "./tokens";
import type { LinkDef, StateName } from "./types";

export interface StageProps {
  scene: Scene;
  states: Partial<Record<StateName, Scene>>;
  resources: Resources;
  motion: Map<string, NodeMotion>;
  tokens: TokenTable;
  reducedMotion: boolean;
  disabled: boolean;
  replayKey: number;
  onLink: (link: LinkDef, nodeId: string) => void;
  onWarnings: (warnings: string[]) => void;
}

/** Fit scale of a frame inside the stage (never above 1:1). */
function useFitScale(width: number, height: number) {
  const ref = useRef<HTMLDivElement>(null);
  const [scale, setScale] = useState(1);
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    const measure = () => {
      const padding = 32;
      const available = {
        width: Math.max(1, element.clientWidth - padding),
        height: Math.max(1, element.clientHeight - padding),
      };
      setScale(Math.min(1, available.width / width, available.height / height));
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [width, height]);
  return { ref, scale };
}

/** Mounts the scene DOM, the state stylesheet and the motion wiring; rebuilds on any input. */
export function PreviewStage(props: StageProps) {
  const { width, height } = props.scene.provenance.viewport;
  const { ref, scale } = useFitScale(width, height);
  const host = useRef<HTMLDivElement>(null);
  const callbacks = useRef(props);
  useEffect(() => {
    callbacks.current = props;
  });
  const { scene, states, resources, motion, tokens, reducedMotion, replayKey } =
    props;

  useEffect(() => {
    const target = host.current!;
    const styles = stateStyles(scene, states);
    const dom = buildSceneDom(scene, resources, styles.strokeIds);
    for (const id of styles.changedIds.get("focus") ?? [])
      dom.elements.get(id)?.node.setAttribute("tabindex", "0");
    const sheet = document.createElement("style");
    sheet.dataset.ocStates = "";
    sheet.textContent = styles.css;
    target.replaceChildren(sheet, dom.root);
    const controller = wireMotion(dom, scene, motion, tokens, {
      reducedMotion,
      onLink: (link, nodeId) => callbacks.current.onLink(link, nodeId),
    });
    callbacks.current.onWarnings(controller.warnings);
    return () => {
      controller.dispose();
      target.replaceChildren();
    };
  }, [scene, states, resources, motion, tokens, reducedMotion, replayKey]);

  // Disabled has no pointer trigger: the toolbar toggles it on every node that declares it.
  useEffect(() => {
    const ids = props.disabled
      ? props.states.disabled
        ? scene.nodes.map((n) => n.id)
        : []
      : [];
    const root = host.current;
    root
      ?.querySelectorAll("[data-oc-disabled]")
      .forEach((el) => el.removeAttribute("data-oc-disabled"));
    for (const id of ids)
      root
        ?.querySelector(`[data-oc-id="${CSS.escape(id)}"]`)
        ?.setAttribute("data-oc-disabled", "");
  }, [props.disabled, props.states.disabled, scene, replayKey, reducedMotion]);

  return (
    <div className="pv-stage" ref={ref}>
      <div
        className="pv-scaler"
        style={{ width: width * scale, height: height * scale }}
      >
        <div
          ref={host}
          className="pv-host"
          style={{ width, height, transform: `scale(${scale})` }}
          data-scale={scale}
        />
      </div>
    </div>
  );
}
