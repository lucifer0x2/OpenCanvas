// S5 — get_outline latency for one project when the data root holds 1 project
// and when it holds 20. Each set gets a fresh backend process.
import { withEnvironment, seedThenRestart } from "../env.mjs";
import { latency, recorder } from "../meter.mjs";
import { seedSmall, seedStacks } from "../seed.mjs";
import { summarize } from "./common.mjs";

export const id = "S5";
export const title =
  "get_outline latency with 1 vs 20 projects in the data root";

async function sample(rec, client, phase, args, samples) {
  rec.phase(`${phase}:warmup`);
  await client.call("get_outline", args);
  rec.phase(phase);
  for (let i = 0; i < samples; i++) await client.call("get_outline", args);
  return latency(rec.calls.filter((c) => c.phase === phase));
}

export async function run(meter, options) {
  const opts = options.s5;
  const { result, cleanup } = await withEnvironment(async (env) => {
    const project = await seedThenRestart(env, (mcp) => seedStacks(mcp));
    // EXTENSION POINT: s5.outlineArgs is merged into every get_outline call.
    const args = { projectId: project.projectId, ...opts.outlineArgs };
    const rec = recorder(meter);
    const one = await sample(
      rec,
      rec.client(await env.connect()),
      "1-project",
      args,
      opts.samples,
    );
    // Filler projects are small: the cost under test is opening every project DB, not their size.
    await seedThenRestart(env, async (mcp) => {
      for (let i = 1; i <= opts.extraProjects; i++)
        await seedSmall(mcp, `filler-${String(i).padStart(2, "0")}`);
    });
    const many = await sample(
      rec,
      rec.client(await env.connect()),
      `${opts.extraProjects + 1}-projects`,
      args,
      opts.samples,
    );
    return summarize(id, title, rec, {
      samplesPerSet: opts.samples,
      projects: { one: 1, many: opts.extraProjects + 1 },
      latency: { one, many },
      p50Ratio: Math.round((many.p50 / one.p50) * 100) / 100,
    });
  });
  return { ...result, cleanup };
}
