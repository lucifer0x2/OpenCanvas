import { memo, useMemo } from "react";
import { nodeIndex } from "../nodeIndex";
import type { DesignNode } from "../types";
import { layerSelectionPath } from "../layerSelection";
import { LayerRow } from "./LayerRow";
import { useNodeActions } from "./NodeActionsProvider";

export const Layers = memo(function Layers({
  nodes,
  frameId,
  frameIds,
  selectedId,
  select,
}: {
  nodes: DesignNode[];
  frameId: string;
  frameIds?: string[];
  selectedId: string;
  select: (id: string) => void;
}) {
  const index = nodeIndex(nodes);
  const actions = useNodeActions();
  const selection = useMemo(
    () => layerSelectionPath(nodes, selectedId),
    [nodes, selectedId],
  );
  const roots = (frameIds ?? [frameId]).flatMap((id) => index.byId.get(id) ?? []);
  return (
    <section className="layers-content" aria-label="Layers">
      <div className="section-heading">
        <span>Layers</span>
        <span className="subtle">Read only</span>
      </div>
      {roots.length ? (
        <ul className="layers-list">
          {roots.map((root) => (
            <LayerRow
              key={root.id}
              node={root}
              index={index}
              selection={selection?.id === root.id ? selection : undefined}
              select={select}
              openActions={actions?.open}
            />
          ))}
        </ul>
      ) : (
        <p className="panel-empty">Choose a frame to see its layers.</p>
      )}
      <div className="panel-footer">
        Changes from your agent appear here automatically.
      </div>
    </section>
  );
});
