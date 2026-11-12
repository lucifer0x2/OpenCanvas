import { memo, useState } from "react";
import { nodeName } from "../state";
import { EMPTY_NODES, type NodeIndex } from "../nodeIndex";
import type { DesignNode } from "../types";
import type { LayerSelectionPath } from "../layerSelection";
import { Icon, type IconName } from "./Icon";

export const LayerRow = memo(function LayerRow({
  node,
  index,
  selection,
  select,
  openActions,
  depth = 0,
}: {
  node: DesignNode;
  index: NodeIndex;
  selection?: LayerSelectionPath;
  select: (id: string) => void;
  openActions?: (id: string, x: number, y: number) => void;
  depth?: number;
}) {
  const [expanded, setExpanded] = useState(true);
  const nested = index.childrenByParent.get(node.id) ?? EMPTY_NODES;
  const selected = selection?.id === node.id && !selection.child;
  return (
    <li>
      <div
        className={`layer-row ${selected ? "is-selected" : ""}`}
        style={{ paddingLeft: 10 + depth * 14 }}
        onContextMenu={(event) => {
          if (!openActions) return;
          event.preventDefault();
          openActions(node.id, event.clientX, event.clientY);
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
              openActions?.(node.id, rect.left, rect.bottom);
            }
          }}
          aria-pressed={selected}
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
              selection={
                selection?.child?.id === child.id ? selection.child : undefined
              }
              select={select}
              openActions={openActions}
              depth={depth + 1}
            />
          ))}
        </ul>
      )}
    </li>
  );
});
