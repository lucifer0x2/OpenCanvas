// Browser tab for the live workspace group: API request log, EventSource count and event log.
// Every EventSource request of the tab, counted by Playwright's request listener.
function trackStreams(tab) {
  const open = new Map();
  const state = { max: 0, total: 0, paths: [] };
  tab.on("request", (request) => {
    if (request.resourceType() !== "eventsource") return;
    const path = new URL(request.url()).pathname;
    open.set(request, path);
    state.total++;
    state.paths.push(path);
    state.max = Math.max(state.max, open.size);
  });
  const end = (request) => open.delete(request);
  tab.on("requestfinished", end);
  tab.on("requestfailed", end);
  state.current = () => [...open.values()];
  return state;
}

export async function openTab(browser, base) {
  const tab = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const view = { page: tab, requests: [], errors: [] };
  tab.on("pageerror", (error) => view.errors.push(error.message));
  tab.on("request", (request) => {
    const url = new URL(request.url());
    if (url.pathname.startsWith("/api/"))
      view.requests.push({ method: request.method(), path: url.pathname });
  });
  // Observation only: log the named events the viewer's EventSource receives, with wall time.
  await tab.addInitScript(() => {
    const Native = EventSource;
    const types = [
      "commit",
      "project_created",
      "project_activity",
      "reset_required",
    ];
    globalThis.__sse = [];
    globalThis.EventSource = class extends Native {
      constructor(url, init) {
        super(url, init);
        const path = new URL(url, location.href).pathname;
        for (const type of types)
          this.addEventListener(type, (event) =>
            globalThis.__sse.push({
              at: Date.now(),
              path,
              type,
              data: JSON.parse(event.data),
            }),
          );
      }
    };
  });
  view.streams = trackStreams(tab);
  await tab.goto(base);
  return view;
}
