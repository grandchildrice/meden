// Test-only native builds. No production flag overrides embedded resources.
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { promisify } from "node:util";
import { cp, mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const exec = promisify(execFile);
const directory = resolve("target/gui-metadata-variants");
await mkdir(directory, { recursive: true });
const source = await mkdtemp(resolve(directory, "source-"));
const { stdout } = await exec("git", [
  "ls-files",
  "-z",
  "--cached",
  "--others",
  "--exclude-standard",
]);
const roots = new Set([
  "Cargo.toml",
  "Cargo.lock",
  "crates",
  "vendor",
  "verification",
  "lean",
  "tools",
  ".github",
]);
for (const file of new Set(stdout.split("\0").filter(Boolean))) {
  if (!roots.has(file.split("/")[0])) continue;
  const target = resolve(source, file);
  await mkdir(dirname(target), { recursive: true });
  await cp(resolve(file), target, { dereference: false });
}
const helper = resolve(source, "crates/meden-app/src/atlas/project-atlas.py");
const original = await readFile(helper, "utf8");
const target = resolve(directory, "build");
const result = {};
for (const name of ["original", "updated"]) {
  if (name === "updated")
    await writeFile(
      helper,
      original + "\n# Metadata-only implementation update.\n",
    );
  // Each snapshot has new absolute source paths. Never trust a shared target's
  // prior workspace dep-info to notice those paths or a helper-only edit.
  await exec(
    "cargo",
    [
      "clean",
      "--profile",
      "profiling",
      "--target-dir",
      target,
      ...[
        "meden-core",
        "meden-runtime",
        "meden-agent",
        "meden-llm",
        "meden-app",
        "meden-cli",
      ].flatMap((crate) => ["-p", crate]),
    ],
    { cwd: source },
  );
  const log = await exec(
    "cargo",
    [
      "build",
      "--locked",
      "--offline",
      "--profile",
      "profiling",
      "-p",
      "meden-cli",
      "--target-dir",
      target,
    ],
    { cwd: source, maxBuffer: 8 * 1024 * 1024 },
  );
  await writeFile(
    resolve(source, name + "-build.log"),
    log.stdout + log.stderr,
  );
  const bytes = await readFile(resolve(target, "profiling/meden"));
  if (!bytes.includes(await readFile(helper)))
    throw new Error(
      "Built executable does not embed the selected metadata helper",
    );
  const digest = createHash("sha256").update(bytes).digest("hex");
  const binary = resolve(directory, digest, "meden");
  await mkdir(dirname(binary), { recursive: true });
  await writeFile(binary, bytes, { mode: 0o555, flag: "wx" }).catch(
    async (error) => {
      if (error.code !== "EEXIST" || !(await readFile(binary)).equals(bytes))
        throw error;
    },
  );
  result[name] = {
    binary,
    sha256: digest,
    helper_sha256: createHash("sha256")
      .update(await readFile(helper))
      .digest("hex"),
  };
}
await writeFile(
  resolve(source, "builds.json"),
  JSON.stringify(result, null, 2),
);
console.log(JSON.stringify({ ...result, source }));
