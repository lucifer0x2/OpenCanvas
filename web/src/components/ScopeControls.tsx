import { ALL_FRAMES } from "../frameScope";
import { nodeName } from "../state";
import type { DesignNode, Selection } from "../types";
import { CustomSelect } from "./CustomSelect";
import { Icon } from "./Icon";

export const PRESETS = [
  { key: "frame", name: "Frame size", width: 0, height: 0 },
  { key: "desktop", name: "Desktop", width: 1440, height: 900 },
  { key: "tablet", name: "Tablet", width: 768, height: 1024 },
  { key: "mobile", name: "Mobile", width: 390, height: 844 },
];

export function ScopeControls({
  pages,
  frames,
  selection,
  select,
  preset,
  setPreset,
}: {
  pages: DesignNode[];
  frames: DesignNode[];
  selection: Selection;
  select: (patch: Partial<Selection>) => void;
  preset: string;
  setPreset: (value: string) => void;
}) {
  return (
    <div className="viewbar">
      <div className="scope-controls">
        <label>
          <span>Page</span>
          <CustomSelect
            label="Page"
            disabled={!pages.length}
            value={selection.pageId}
            placeholder="No pages"
            options={pages.map((page) => ({
              value: page.id,
              label: nodeName(page),
            }))}
            onChange={(pageId) =>
              select({
                pageId,
                frameId: ALL_FRAMES,
                nodeId: "",
              })
            }
          />
        </label>
        <span className="scope-slash">/</span>
        <label>
          <span>Frame</span>
          <CustomSelect
            label="Frame"
            disabled={!pages.length}
            value={selection.frameId}
            placeholder="No frames"
            options={[
              { value: ALL_FRAMES, label: "All" },
              ...frames.map((frame) => ({
                value: frame.id,
                label: nodeName(frame),
              })),
            ]}
            onChange={(frameId) =>
              select({ frameId, nodeId: frameId === ALL_FRAMES ? "" : frameId })
            }
          />
        </label>
      </div>
      <label className="viewport-picker">
        <Icon name="frame" />
        <span className="sr-only">Viewport</span>
        <CustomSelect
          label="Viewport"
          value={preset}
          disabled={!frames.length}
          onChange={setPreset}
          options={PRESETS.map((entry) => ({
            value: entry.key,
            label: entry.name,
            description: entry.width
              ? `${entry.width} × ${entry.height}`
              : undefined,
          }))}
        />
      </label>
    </div>
  );
}
