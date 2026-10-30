// Part D markdown tables (README Phần D) from the folded results of report.mjs. "N / 1" cells are
// the Rust prototype with one thread per core (N) and with one thread.
const f0 = (v) => (v == null ? "–" : Math.round(v).toLocaleString("en-US").replace(/,/g, " "));
const f1 = (v) => (v == null ? "–" : v.toFixed(1));
const f2 = (v) => (v == null ? "–" : v < 10 ? v.toFixed(2) : v.toFixed(1));
const kb = (v) => (v == null ? "–" : `${f0(v / 1024)}`);
const x = (v) => (v >= 1 ? `${v.toFixed(1)}×` : `÷${(1 / v).toFixed(1)}`);
const pair = (a, b, f = f1) => `${f(a)} / ${f(b)}`;
const row = (cells) => `| ${cells.join(" | ")} |`;
const head = (cols) => [row(cols), row(cols.map(() => "---"))];
const p = (s) => s?.p50;
const P = (o) => Object.fromEntries(Object.entries(o).map(([k, v]) => [k, v?.p50 ?? v]));

export function tables(out) {
  const L = [];
  const C = Object.entries(out.configs);
  const n = C[0][1].rust.threads;
  L.push("**D1. Frame**", "", ...head(["Frame", "Viewport", "Node", "Text node", "Dòng text", "Pass cold", "Mục đo cold", "PNG Chrome 1×/2× (KB)"]));
  for (const [name, fr] of Object.entries(out.frames)) {
    const c1 = out.configs[`${name}-s1`], c2 = out.configs[`${name}-s2`];
    L.push(row([name, `${fr.viewport.width}×${fr.viewport.height}`, fr.sceneNodes, fr.textNodes, fr.textLines,
      c1.chromiumStages.cold.passes, c1.chromiumStages.cold.measured,
      `${kb(c1.chromiumServer.pngBytes)} / ${kb(c2.chromiumServer.pngBytes)}`]));
  }
  L.push("", `**D2. End-to-end (ms).** Chromium = server release, MCP \`render_frame\` tới khi có bytes PNG. Rust = prototype, painter bbox, N = ${n} thread / 1 thread (cold: cả tiến trình; warm: trong tiến trình). Tỷ lệ = Chromium ÷ (Rust + MCP enqueue + tải ảnh đo ở đường server).`, "",
    ...head(["Frame × scale", "Chromium cold", "Rust cold N / 1", "MCP", "Tỷ lệ cold N / 1", "Chromium warm p50 / p95", "Rust warm p50 N / 1", "Rust warm p95 N / 1", "Rust warm Fast p50 N / 1", "Tỷ lệ warm N / 1", "Tỷ lệ warm Fast N / 1"]));
  for (const [k, c] of C) {
    const s = c.chromiumServer, r = c.rust, q = c.ratio;
    L.push(row([k, f0(s.coldMs.p50), pair(r.cold.wallMs.p50, r.cold1.wallMs.p50), f1(c.mcpMs), pair(q.cold, q.cold1, x),
      pair(p(s.warm.totalMs), s.warm.totalMs.p95), pair(p(r.warm.totalMs), p(r.warm1.totalMs)),
      pair(r.warm.totalMs.p95, r.warm1.totalMs.p95), pair(p(r.warm.totalFastMs), p(r.warm1.totalFastMs)),
      pair(q.warm, q.warm1, x), pair(q.warmFast, q.warmFast1, x)]));
  }
  L.push("", "**D3. Chromium warm, p50 theo stage (ms)** (script stage, cùng input; `resources` = 2 lần `loadResources`).", "",
    ...head(["Frame × scale", "Job", "resources", "layout-step (pass, KB JSON)", "measure RTT (trong trang)", "draw RTT", "drawScene", "toDataURL", "IPC Playwright", "NDJSON worker + server", "MCP enqueue + tải ảnh", "Server p50", "Giải thích"]));
  for (const [k, c] of C) {
    const w = P(c.chromiumStages.warm), s = c.chromiumServer.warm;
    const nd = w.workerNdjsonMs + c.serverNdjson.msTotal, mcp = p(s.enqueueMs) + p(s.imageMs);
    L.push(row([k, f1(w.jobMs), f1(w.resourcesRttMs), `${f1(w.layoutSpawnMs)} (${w.passes}, ${kb(w.layoutBytes)})`,
      `${f2(w.measureRttMs)} (${f2(w.measurePageMs)})`, f1(w.drawRttMs), f2(w.drawPageMs), f2(w.toDataURLPageMs), f1(w.ipcMs),
      f2(nd), f1(mcp), f1(p(s.totalMs)), `${Math.round((100 * (w.jobMs + nd + mcp)) / p(s.totalMs))}%`]));
  }
  L.push("", "**D4. Rust warm, p50 theo stage (ms)**, painter bbox. CPU = user + sys của mọi thread trong phần `Tổng` (không gồm PNG Fast).", "",
    ...head(["Frame × scale", "Tổng N / 1", "CPU N / 1", "parse", "layout (pass)", "đo (mục)", "paint N / 1", "text raster N / 1", "text composite", "hình", "clip mask (số)", "PNG mặc định N / 1", "PNG Fast N / 1", "KB mặc định / Fast / Chrome"]));
  for (const [k, c] of C) {
    const w = P(c.rust.warm), w1 = P(c.rust.warm1), pr = P(c.rust.warm.profile), pr1 = P(c.rust.warm1.profile);
    L.push(row([k, pair(w.totalMs, w1.totalMs), pair(w.cpuMs, w1.cpuMs), f2(w.parseMs), `${f2(w.layoutMs)} (${w.passes})`,
      `${f2(w.measureMs)} (${w.measured})`, pair(w.paintMs, w1.paintMs), pair(pr.rasterMs, pr1.rasterMs), f1(pr.compositeMs),
      f1(pr.shapeMs), `${f1(pr.maskMs)} (${pr.masks})`, pair(w.encodeMs, w1.encodeMs), pair(w.encodeFastMs, w1.encodeFastMs),
      `${kb(w.pngBytes)} / ${kb(w.pngFastBytes)} / ${kb(c.chromiumServer.pngBytes)}`]));
  }
  L.push("", "**D5. Cold (ms).** Chromium: tiến trình Node mới (script stage); Rust: cả tiến trình, painter bbox.", "",
    ...head(["Frame × scale", "Node boot", "import playwright", "launch", "newPage + goto + import renderer", "Job đầu", "Cộng + MCP", "Server cold", "Rust: khởi (font đọc/SHA/parse/pool)", "Rust: run đầu N / 1", "Rust: wall N / 1"]));
  for (const [k, c] of C) {
    const st = c.chromiumStages.startup, r = c.rust.cold, r1 = c.rust.cold1, su = r.startup;
    L.push(row([k, f1(st.scriptStartMs), f1(st.importPlaywright), f1(st.launch), f1(st.newPage + st.goto + st.importRenderer),
      f1(c.chromiumStages.cold.firstJobMs), f1(st.readyMs + c.chromiumStages.cold.firstJobMs + c.mcpMs), f0(c.chromiumServer.coldMs.p50),
      `${f1(su.totalMs)} (${f2(su.fontReadMs)}/${f2(su.fontHashMs)}/${f2(su.fontParseMs)}/${f2(su.poolMs)})`,
      pair(r.run.totalMs, r1.run.totalMs), pair(r.wallMs.p50, r1.wallMs.p50)]));
  }
  L.push("", "**D6. Layout + đo khi cache đo trống (warm process, p50, ms)** — phần Task 10 thay.", "",
    ...head(["Frame × scale", "Mục đo", "Chromium: layout-step spawn", "Chromium: measure RTT", "Chromium: cộng", "Rust: layout", "Rust: đo", "Rust: cộng"]));
  for (const [k, c] of C.filter(([k]) => k.endsWith("s1"))) {
    const w = P(c.chromiumStages.warmNoCache), r = P(c.rust.warmNoCache);
    L.push(row([k.replace(/-s1$/, ""), w.measured, f1(w.layoutSpawnMs), f1(w.measureRttMs), f1(w.layoutSpawnMs + w.measureRttMs),
      f2(r.layoutMs), f2(r.measureMs), f2(r.layoutMs + r.measureMs)]));
  }
  L.push("", "**D7. Gate hình học và pixel** (Rust so Chromium, cùng input; pixel: Δ là chênh lớn nhất giữa kênh RGBA).", "",
    ...head(["Frame", "Viewport so", "Node so / lệch", "Dòng so / lệch", "Script = server", "1× Δ>0 / Δ>32 / max / mean", "2× Δ>0 / Δ>32 / max / mean"]));
  for (const [name, g] of Object.entries(out.gate)) {
    const rv = g.rustVsChrome, sv = g.stageVsServer;
    const px = (s) => `${f0(s.gt0)} / ${f0(s.gt32)} / ${s.maxDelta} / ${s.meanDelta.toFixed(2)} (${((100 * s.gt0) / s.pixels).toFixed(1)}% px)`;
    L.push(row([name, rv.widths, `${f0(rv.nodes)} / ${rv.nodeMismatches + rv.missing}`, `${f0(rv.lines)} / ${rv.lineMismatches}`,
      `${sv.nodeMismatches + sv.lineMismatches + sv.missing === 0 ? "trùng" : "LỆCH"}, PNG ${g.pixels.s1.stagePngIdenticalToServer && g.pixels.s2.stagePngIdenticalToServer ? "trùng byte" : "khác"}`,
      px(g.pixels.s1), px(g.pixels.s2)]));
  }
  L.push("", "**D8. Painter trước/sau, đo thực (warm p50, ms).** Cũ = painter toàn canvas (`--paint full`, 1 thread), đo lại cùng lượt; ước lượng cũ = công thức D8 trước đây áp lên lượt đo đó.", "",
    ...head(["Frame × scale", "Paint cũ", "Paint ước lượng cũ", "Paint mới 1 / N", "Cũ ÷ mới 1 / N", "Rust warm Fast: ước lượng cũ / đo N", "Rust warm cũ / mới N", "Chromium warm p50"]));
  for (const [k, c] of C) {
    const e = c.rust.estimate, old = P(c.rust.warmFull), w = P(c.rust.warm), w1 = P(c.rust.warm1);
    L.push(row([k, f1(old.paintMs), f1(e.paintMs), pair(w1.paintMs, w.paintMs), pair(old.paintMs / w1.paintMs, old.paintMs / w.paintMs, (v) => `${v.toFixed(0)}×`),
      pair(e.totalFastMs, w.totalFastMs), pair(old.totalMs, w.totalMs), f1(p(c.chromiumServer.warm.totalMs))]));
  }
  const pg = out.paintGate;
  L.push("", `**D9. Gate pixel của painter mới** (\`rust-spike paint-gate\`, so từng byte RGBA premultiplied; (b) so cả RGBA đã demultiply đưa vào PNG; thread ${pg.threads.join(", ")}).`, "",
    ...head(["Ca", "Lượt", "(a) bbox = full", "(b) bằng nhau mọi số thread", "Text node", "Mask fallback toàn canvas"]));
  for (const g of pg.groups)
    L.push(row([g.case, g.cases, `${g.fullEqualsBbox}/${g.cases}`, `${g.threadsEqual}/${g.cases}`, g.textNodes, g.fallbacks]));
  L.push("", "**D10. CPU mỗi job warm (ms).** Chromium: CPU của worker Node + mọi process Chromium (không gồm `layout-step` và server) chia cho 24 job; Rust: getrusage quanh phần `Tổng`.", "",
    ...head(["Frame × scale", "Chromium wall p50", "Chromium CPU/job", "Chromium process / thread", "Rust N wall / CPU", "Rust 1 wall / CPU"]));
  for (const [k, c] of C) {
    const s = c.chromiumServer, cpu = s.engineCpu, w = P(c.rust.warm), w1 = P(c.rust.warm1);
    L.push(row([k, f1(p(s.warm.totalMs)), f1(cpu.perJobMs), `${cpu.processes} / ${cpu.threads}`, pair(w.totalMs, w.cpuMs), pair(w1.totalMs, w1.cpuMs)]));
  }
  return L.join("\n");
}
