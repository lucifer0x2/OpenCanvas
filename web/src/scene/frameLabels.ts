/** Labels stay 12 CSS pixels; hide only when zoomed so far out they collide or have no width. */
export function frameLabelMetrics(
  width: number,
  scale: number,
  clearance: number,
) {
  return {
    visible: width * scale >= 8 && clearance * scale >= 24,
    fontSize: 12 / scale,
    y: -24 / scale,
    height: 20 / scale,
  };
}
