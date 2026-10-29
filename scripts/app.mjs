import { spawn } from "node:child_process";
import { access } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const workspace = fileURLToPath(new URL("../", import.meta.url));
const [action, profile, ...args] = process.argv.slice(2);
const target = path.resolve(
  workspace,
  process.env.CARGO_TARGET_DIR ?? "target",
);
const suffix = process.platform === "win32" ? ".exe" : "";
const binary = (name) => path.join(target, profile, `${name}${suffix}`);
const buildCommand = profile === "release" ? "build:release" : "build";

// Run from the repository even when invoked by npm --prefix from another directory.
// npm's CLI runs via Node, avoiding shell quoting and Windows .cmd differences.
async function run(command, argv, env = process.env) {
  const child = spawn(command, argv, { cwd: workspace, env, stdio: "inherit" });
  const interrupt = () => child.kill("SIGINT");
  process.on("SIGINT", interrupt);
  process.on("SIGTERM", interrupt);
  try {
    await new Promise((resolve, reject) => {
      child.once("error", reject);
      child.once("close", (code, signal) => {
        if (code === 0) resolve();
        else {
          const error = new Error(
            `${path.basename(command)} ${signal ?? `exited ${code}`}`,
          );
          error.exitCode = code ?? 1;
          reject(error);
        }
      });
    });
  } finally {
    process.off("SIGINT", interrupt);
    process.off("SIGTERM", interrupt);
  }
}

async function requireArtifact(file) {
  try {
    await access(file);
  } catch {
    throw new Error(
      `Missing built artifact: ${file}\nRun npm run ${buildCommand} first.`,
    );
  }
}

try {
  if (!["debug", "release"].includes(profile))
    throw new Error("Unknown app profile");
  if (action === "build") {
    const npmCli = process.env.npm_execpath;
    if (!npmCli) throw new Error("Use npm run build or npm run build:release.");
    await run(process.execPath, [npmCli, "run", "build:renderer"]);
    await run("cargo", [
      "build",
      "--locked",
      "-p",
      "opencanvas-layout",
      "-p",
      "opencanvas-server",
      ...(profile === "release" ? ["--release"] : []),
      ...args,
    ]);
    await run(process.execPath, [npmCli, "--prefix", "web", "run", "build"]);
  } else if (action === "run") {
    for (const artifact of [
      binary("opencanvas-server"),
      binary("layout-step"),
      path.join(workspace, "renderer/dist/index.js"),
      path.join(workspace, "web/dist/index.html"),
    ])
      await requireArtifact(artifact);
    await run(
      binary("opencanvas-server"),
      ["--workspace", workspace, ...args],
      {
        ...process.env,
        OPENCANVAS_LAYOUT_BIN: binary("layout-step"),
        PLAYWRIGHT_BROWSERS_PATH:
          process.env.PLAYWRIGHT_BROWSERS_PATH ??
          path.join(workspace, ".runtime/browsers"),
      },
    );
  } else throw new Error("Unknown app action");
} catch (error) {
  console.error(error.message);
  process.exitCode = error.exitCode ?? 1;
}
