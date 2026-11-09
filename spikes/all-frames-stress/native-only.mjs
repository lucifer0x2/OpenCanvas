// Optional continuation when a human-operated browser is not available yet.
import { readFile } from "node:fs/promises";
import { OUT, save, startBackend } from "./runtime.mjs";
import { nativePage } from "./native.mjs";
import { cloneDataRoot } from "./clone-data.mjs";

const report = JSON.parse(await readFile(`${OUT}/measurements.json`, "utf8"));
const session = JSON.parse(await readFile(`${OUT}/session.json`, "utf8"));
let backend;
const rows = await readFile(`${OUT}/native.json`, "utf8")
  .then(JSON.parse)
  .catch((error) => {
    if (error.code === "ENOENT") return [];
    throw error;
  });
const stop = async () => {
  if (backend) await backend.stop();
};
process.on("SIGINT", async () => {
  await stop();
  process.exit(0);
});
process.on("SIGTERM", async () => {
  await stop();
  process.exit(0);
});
try {
  for (const fixture of report.rows) {
    if (fixture.native || rows.some((row) => row.profile === fixture.profile))
      continue;
    backend = await startBackend(await cloneDataRoot(session.dataRoot));
    console.log(`NATIVE ${fixture.profile} ${backend.base}`);
    const result = await nativePage(backend.base, fixture, backend.pid);
    rows.push({
      profile: fixture.profile,
      projectId: fixture.projectId,
      ...result,
      seedPrerendersExcluded: true,
      dataRoot: backend.dataRoot,
    });
    await save("native.json", rows);
    console.log(
      `NATIVE_RESULT ${fixture.profile} elapsedMs=${result.elapsedMs.toFixed(1)} nodes=${result.jobs.reduce((sum, job) => sum + (job.nodes ?? 0), 0)} errors=${JSON.stringify(result.errors)}`,
    );
    await stop();
    backend = undefined;
  }
} finally {
  await stop();
}
