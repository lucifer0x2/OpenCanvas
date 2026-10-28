import { useState } from "react";
import { children, nodeName } from "../state";
import type { DesignNode } from "../types";
import { Icon } from "./Icon";

function LayerRow({
  node,
  nodes,
  selectedId,
  select,
  depth = 0,
}: {
  node: DesignNode;
  nodes: DesignNode[];
  selectedId: string;
  select: (id: string) => void;
  depth?: number;
}) {
  const [expanded, setExpanded] = useState(true);
  const nested = children(nodes, node.id);
  return (
    <li>
      <div
        className={`layer-row ${selectedId === node.id ? "is-selected" : ""}`}
        style={{ paddingLeft: 10 + depth * 14 }}
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
          aria-pressed={selectedId === node.id}
          title={nodeName(node)}
        >
          <Icon name={node.kind} />
          <span>{nodeName(node)}</span>
        </button>
      </div>
      {!!nested.length && expanded && (
        <ul>
          {nested.map((child) => (
            <LayerRow
              key={child.id}
              node={child}
              nodes={nodes}
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

export function Layers({
  nodes,
  frameId,
  selectedId,
  select,
}: {
  nodes: DesignNode[];
  frameId: string;
  selectedId: string;
  select: (id: string) => void;
}) {
  const root = nodes.find((node) => node.id === frameId);
  return (
    <section className="layers-content" aria-label="Layers">
      <div className="section-heading">
        <span>Layers</span>
        <span className="subtle">Read only</span>
      </div>
      {root ? (
        <ul className="layers-list">
          <LayerRow
            node={root}
            nodes={nodes}
            selectedId={selectedId}
            select={select}
          />
        </ul>
      ) : (
        <p className="panel-empty">Choose a frame to see its layers.</p>
      )}
      <div className="panel-footer">
        Changes from your agent appear here automatically.
      </div>
    </section>
  );
}
