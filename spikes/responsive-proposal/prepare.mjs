import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { cp, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { root } from "../../worker/runtime.mjs";

// Proposal experiment only. Patch a disposable COPY; never build over live binaries.
export async function prepare(directory) {
  const crate = path.join(directory, "layout");
  await cp(path.join(root, "crates/layout"), crate, { recursive: true });
  await cp(path.join(root, "Cargo.lock"), path.join(crate, "Cargo.lock"));
  const manifest = path.join(crate, "Cargo.toml");
  await writeFile(
    manifest,
    (await readFile(manifest, "utf8")) + "\n[workspace]\n",
  );
  const edits = [
    [
      "model.rs",
      "pub struct LayoutRule {",
      "pub struct LayoutRule {\n    #[serde(default)]\n    pub wrap: bool,",
    ],
    [
      "tree.rs",
      "        align_items: AlignItems::START,",
      `        flex_wrap: if p.layout.as_ref().is_some_and(|l| l.wrap) {
            FlexWrap::Wrap
        } else {
            FlexWrap::NoWrap
        },
        align_content: if p.layout.as_ref().is_some_and(|l| l.wrap) {
            AlignContent::START
        } else {
            AlignContent::NORMAL
        },
        align_items: AlignItems::START,`,
    ],
  ];
  const native = (
    await readFile(path.join(crate, "src/model.rs"), "utf8")
  ).includes("mod layout;");
  for (const [file, before, after] of native ? [] : edits) {
    const target = path.join(crate, "src", file);
    const original = await readFile(target, "utf8");
    assert.equal(original.split(before).length, 2, `unique anchor: ${file}`);
    await writeFile(target, original.replace(before, after));
  }
  const target = path.join(directory, "target");
  execFileSync(
    "cargo",
    [
      "build",
      "--offline",
      "--manifest-path",
      manifest,
      "--target-dir",
      target,
      "--bin",
      "layout-step",
      "-j",
      "2",
    ],
    { cwd: root, stdio: ["ignore", "ignore", "inherit"], timeout: 120_000 },
  );
  return { binary: path.join(target, "debug/layout-step"), native };
}
