import { cp, mkdtemp, readdir } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";

/** SQLite backup API gives native probes an independent, consistent data root. */
export async function cloneDataRoot(source) {
  if (!basename(source).startsWith("opencanvas-all-stress-"))
    throw new Error("Only this harness's temporary data root may be cloned");
  const target = await mkdtemp(join(tmpdir(), "opencanvas-all-stress-native-"));
  for (const entry of await readdir(source, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const from = join(source, entry.name),
      to = join(target, entry.name);
    await cp(from, to, {
      recursive: true,
      filter: (path) => !/^db\.sql(?:-wal|-shm)?$/.test(basename(path)),
    });
    // tempdir paths are generated locally; SQLite shell quoting is still explicit.
    const destination = join(to, "db.sql").replaceAll("'", "''");
    execFileSync("sqlite3", [join(from, "db.sql"), `.backup '${destination}'`]);
  }
  return target;
}
