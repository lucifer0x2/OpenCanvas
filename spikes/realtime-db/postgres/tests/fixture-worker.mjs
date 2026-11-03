// Test-owned process: claim once and wait to be killed; no renderer is run.
import { request } from '../../harness/client.mjs';

try {
  const response = await request(process.argv[2], '/jobs/claim', { workerId: 'worker-dead', leaseMs: 300 });
  if (response.status !== 200 || !response.body.job) throw new Error(JSON.stringify(response));
  process.send({ job: response.body.job });
  setInterval(() => {}, 1000);
} catch (error) {
  process.send({ error: error.message });
  process.exitCode = 1;
}
