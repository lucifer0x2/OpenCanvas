// Self-contained: Playwright serializes this function into the page. Never resolve an error payload.
export function runWorker({ timeoutMs, ...message }) {
  return new Promise((resolve, reject) => {
    const worker = new Worker("/worker.js", { type: "module" });
    const transfers = new Map();
    const finish = (error, result) => {
      clearTimeout(timer);
      worker.terminate();
      if (error) reject(error);
      else resolve(result);
    };
    const timer = setTimeout(() => finish(new Error("Worker benchmark timeout")), timeoutMs);
    worker.onerror = (e) => finish(new Error(`Worker error: ${e.message}`));
    worker.onmessageerror = () => finish(new Error("Worker messageerror"));
    worker.onmessage = ({ data }) => {
      if (data?.transferProbe) {
        const p = data.transferProbe;
        transfers.set(p.case, { bytes: p.buffer.byteLength,
          workerTransferMs: performance.timeOrigin + performance.now() - p.sentAt });
        return;
      }
      if (data?.error) finish(new Error(data.error));
      else {
        for (const c of data?.cases ?? []) {
          const transfer = transfers.get(c.case);
          if (transfer?.bytes !== c.size[0] * c.size[1] * 4) return finish(new Error(`Missing RGBA transfer: ${c.case}`));
          c.workerTransferMs = transfer.workerTransferMs;
        }
        finish(null, data);
      }
    };
    try { worker.postMessage(message); }
    catch (error) { finish(error); }
  });
}
