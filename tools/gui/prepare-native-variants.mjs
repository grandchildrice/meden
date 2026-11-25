// Test-only native builds. No production flag overrides embedded resources.
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { promisify } from "node:util";
import { cp, mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const exec = promisify(execFile);
const kind = process.argv[2];
const changes =
  kind === "metadata"
    ? [
        [
          "crates/meden-app/src/atlas/project-atlas.py",
          "\n# Metadata-only implementation update.\n",
        ],
      ]
    : kind === "repair"
      ? [
          [
            "crates/meden-app/src/adapters/lean/repair.rs",
            "\n// Repair-only implementation update.\n",
          ],
          [
            "crates/meden-app/src/adapters/lean.rs",
            "\n// Composition-only implementation update.\n",
          ],
        ]
      : kind === "pipeline"
        ? [
            [
              "crates/meden-app/src/adapters/lean/pipeline.rs",
              "\n// Orchestration-only implementation update.\n",
            ],
          ]
        : null;
if (!changes)
  throw new Error("Choose the fixed metadata, repair or pipeline test variant");
const directory = resolve("target/gui-native-variants/" + kind);
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
const helper = resolve(source, changes[0][0]);
const originals = await Promise.all(
  changes.map(async ([path, suffix]) => ({
    path,
    suffix,
    text: await readFile(resolve(source, path), "utf8"),
  })),
);
const target = resolve(directory, "build");
const result = {};
for (const name of ["original", "updated"]) {
  if (name === "updated")
    for (const change of originals)
      await writeFile(
        resolve(source, change.path),
        (kind === "pipeline"
          ? change.text.replace(
              "Preparing explanations",
              "Preparing explanations (composition update)",
            )
          : change.text) + change.suffix,
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
  if (
    kind === "pipeline"
      ? !bytes.includes(
          Buffer.from(
            name === "updated"
              ? "Preparing explanations (composition update)"
              : "Preparing explanations",
          ),
        )
      : !bytes.includes(await readFile(helper))
  )
    throw new Error(
      "Built executable does not embed the selected implementation",
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
    changed_sources: await Promise.all(
      changes.map(async ([path]) => ({
        path,
        sha256: createHash("sha256")
          .update(await readFile(resolve(source, path)))
          .digest("hex"),
      })),
    ),
    implementation_sha256: createHash("sha256")
      .update(await readFile(helper))
      .digest("hex"),
  };
}
await writeFile(
  resolve(source, "builds.json"),
  JSON.stringify(result, null, 2),
);
console.log(JSON.stringify({ ...result, source, kind }));
