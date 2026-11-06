export interface FrameSize {
  id: string;
  width: number;
  height: number;
}
export interface FramePlacement extends FrameSize {
  x: number;
  y: number;
  labelSpace?: number;
}
export const FRAME_GAP = 64;
export const FRAME_LABEL = 32;

function shelves(frames: FrameSize[], limit: number, labelHeight: number) {
  const rows: { frames: FramePlacement[]; width: number; height: number }[] =
    [];
  let row = { frames: [] as FramePlacement[], width: 0, height: 0 };
  for (const frame of frames) {
    const nextWidth =
      row.width + (row.frames.length ? FRAME_GAP : 0) + frame.width;
    if (row.frames.length && nextWidth > limit + 0.001) {
      rows.push(row);
      row = { frames: [], width: 0, height: 0 };
    }
    const x = row.width + (row.frames.length ? FRAME_GAP : 0);
    row.frames.push({ ...frame, x, y: 0 });
    row.width = x + frame.width;
    row.height = Math.max(row.height, frame.height + labelHeight);
  }
  if (row.frames.length) rows.push(row);
  const width = rows.reduce((max, entry) => Math.max(max, entry.width), 0);
  let y = 0;
  const placements: FramePlacement[] = [];
  for (const entry of rows) {
    for (const frame of entry.frames)
      placements.push({
        ...frame,
        x: frame.x + (width - entry.width) / 2,
        y: y + labelHeight,
        labelSpace: labelHeight + (y ? FRAME_GAP : 0),
      });
    y += entry.height + FRAME_GAP;
  }
  return { frames: placements, width, height: Math.max(0, y - FRAME_GAP) };
}

/** Stable authored order, bounded candidate search, real dimensions and centered rows. */
export function packFrames(
  frames: FrameSize[],
  aspect = 1.5,
  labelHeight = FRAME_LABEL,
) {
  if (!frames.length) return { frames: [], width: 0, height: 0 };
  const ratio = Math.max(0.1, Math.min(10, aspect || 1.5));
  const min = frames.reduce((max, frame) => Math.max(max, frame.width), 0);
  const max = frames.reduce(
    (sum, frame) => sum + frame.width + FRAME_GAP,
    -FRAME_GAP,
  );
  const candidates = new Set([min, max]);
  // Width changes are logarithmic: also cover very long/tall mixed-size pages.
  for (let i = 1; i < 64; i++)
    candidates.add(min * Math.pow(max / min, i / 64));
  const area = frames.reduce(
    (sum, frame) => sum + frame.width * (frame.height + labelHeight),
    0,
  );
  let best = shelves(frames, max, labelHeight),
    score = Infinity;
  for (const width of candidates) {
    const layout = shelves(frames, width, labelHeight);
    const envelope = Math.max(layout.width / ratio, layout.height);
    const wasted = 1 - area / (layout.width * layout.height);
    const smallRowPenalty =
      frames.length <= 3 &&
      ratio >= 1 &&
      layout.frames.some((frame) => frame.y !== layout.frames[0].y)
        ? 0.3
        : 0;
    const next = envelope * (1 + wasted * 0.12 + smallRowPenalty);
    if (next < score) {
      best = layout;
      score = next;
    }
  }
  return best;
}

/** Reserve readable label clearance at Fit without changing frame positions during live zoom. */
export function fitFrameBoard(
  frames: FrameSize[],
  size: { width: number; height: number },
) {
  const width = Math.max(1, size.width - 96),
    height = Math.max(1, size.height - 120);
  let labelHeight = FRAME_LABEL;
  let layout = packFrames(frames, width / height, labelHeight);
  for (let i = 0; i < 16 && frames.length; i++) {
    const scale = Math.max(
      0.0001,
      Math.min(width / layout.width, height / layout.height, 1),
    );
    const desired = Math.max(FRAME_LABEL, 28 / scale);
    if (Math.abs(desired - labelHeight) < 0.01) break;
    // Monotonic clearance avoids oscillation when the best shelf arrangement changes.
    labelHeight = Math.max(labelHeight, desired);
    layout = packFrames(frames, width / height, labelHeight);
  }
  return layout;
}
