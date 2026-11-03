import { startEngine, stopEngine, readRuntime } from './runtime.mjs';
const [command, directory] = process.argv.slice(2);
if (command === 'start') {
  const runtime = await startEngine();
  console.log(JSON.stringify({ runtime: runtime.directory, manifest: runtime.manifest }));
} else if (command === 'cleanup' && directory) stopEngine(readRuntime(directory));
else throw new Error('Usage: node infra/engines.mjs start | cleanup RUNTIME_DIRECTORY');
