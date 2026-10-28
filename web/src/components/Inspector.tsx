import type { Scene } from "../../../renderer/types";
import { nodeName } from "../state";
import type { DesignNode } from "../types";
import { Icon } from "./Icon";

function Property({ name, value }: { name: string; value: unknown }) {
  const display =
    typeof value === "object" ? JSON.stringify(value, null, 2) : String(value);
  return (
    <div className="property">
      <dt>{name}</dt>
      <dd title={display}>{display}</dd>
    </div>
  );
}

export function Inspector({
  node,
  scene,
}: {
  node?: DesignNode;
  scene?: Scene;
}) {
  const computed = scene?.nodes.find((entry) => entry.id === node?.id);
  return (
    <aside className="inspector" aria-label="Read-only inspector">
      <div className="section-heading">
        Inspect<span className="readonly-label">Read only</span>
      </div>
      {node ? (
        <>
          <div className="inspector-identity">
            <span className="node-icon">
              <Icon name={node.kind} size={20} />
            </span>
            <div>
              <h2>{nodeName(node)}</h2>
              <span className="subtle">{node.kind}</span>
            </div>
          </div>
          <section className="inspect-section">
            <h3>Authored properties</h3>
            <dl>
              <Property name="ID" value={node.id} />
              {Object.entries(node.props).map(([name, value]) => (
                <Property key={name} name={name} value={value} />
              ))}
            </dl>
          </section>
          <section className="inspect-section">
            <h3>Computed bounds</h3>
            {computed ? (
              <>
                <p className="section-note">
                  Frame-local pixels · revision {scene!.provenance.revision}
                </p>
                <dl className="bounds-grid">
                  {Object.entries(computed.bounds).map(([name, value]) => (
                    <Property
                      key={name}
                      name={name}
                      value={Math.round(value * 100) / 100}
                    />
                  ))}
                </dl>
                {computed.kind === "text" && (
                  <p className="section-note">
                    {computed.lines.length} precomputed text{" "}
                    {computed.lines.length === 1 ? "line" : "lines"}
                  </p>
                )}
              </>
            ) : (
              <p className="section-note">
                Available when this frame's scene is ready.
              </p>
            )}
          </section>
          {scene && (
            <section className="inspect-section">
              <h3>Scene source</h3>
              <dl>
                <Property name="Revision" value={scene.provenance.revision} />
                <Property
                  name="Renderer"
                  value={scene.provenance.rendererVersion}
                />
                <Property name="Coordinates" value={scene.coordinateSpace} />
              </dl>
            </section>
          )}
        </>
      ) : (
        <p className="panel-empty">
          Select a layer on the canvas or in the layers panel to inspect it.
        </p>
      )}
    </aside>
  );
}
