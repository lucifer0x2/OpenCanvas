import { memo, useState } from "react";
import { nodeName } from "../state";
import { EMPTY_NODES, nodeIndex, type NodeIndex } from "../nodeIndex";
import type { DesignNode } from "../types";
import { Icon, type IconName } from "./Icon";
import { useNodeActions } from "./NodeActionsProvider";

function LayerRow({
  node,
  index,
  selectedId,
  select,
  depth = 0,
}: {
  node: DesignNode;
  index: NodeIndex;
  selectedId: string;
  select: (id: string) => void;
  depth?: number;
}) {
  const [expanded, setExpanded] = useState(true);
  const actions = useNodeActions();
  const nested = index.childrenByParent.get(node.id) ?? EMPTY_NODES;
  return (
    <li>
      <div
        className={`layer-row ${selectedId === node.id ? "is-selected" : ""}`}
        style={{ paddingLeft: 10 + depth * 14 }}
        onContextMenu={(event) => {
          if (!actions) return;
          event.preventDefault();
          actions.open(node.id, event.clientX, event.clientY);
        }}
      >
        {nested.length ? (
          <button
            className={`disclosure ${expanded ? "is-open" : ""}`}
            aria-label={`${expanded ? "Collapse" : "Expand"} ${nodeName(node)}`}
            aria-expanded={expanded}
            onClick={() => setExpanded(!expanded)}
          >
            <Icon name="chevron" size={12} />
          </button>
        ) : (
          <span className="disclosure-spacer" />
        )}
        <button
          className="layer-select"
          onClick={() => select(node.id)}
          onKeyDown={(event) => {
            if (
              event.key === "ContextMenu" ||
              (event.shiftKey && event.key === "F10")
            ) {
              event.preventDefault();
              const rect = event.currentTarget.getBoundingClientRect();
              actions?.open(node.id, rect.left, rect.bottom);
            }
          }}
          aria-pressed={selectedId === node.id}
          title={nodeName(node)}
        >
          <Icon name={node.kind as IconName} />
          <span>{nodeName(node)}</span>
        </button>
      </div>
      {!!nested.length && expanded && (
        <ul>
          {nested.map((child) => (
            <LayerRow
              key={child.id}
              node={child}
              index={index}
              selectedId={selectedId}
              select={select}
              depth={depth + 1}
            />
          ))}
        </ul>
      )}
    </li>
  );
}

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
              selectedId={selectedId}
              select={select}
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
