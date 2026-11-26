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
        : kind === "navigation" || kind === "saved-map"
          ? [
              ["crates/meden-app/src/adapters/source.rs", ""],
              ...(kind === "saved-map"
                ? [["crates/meden-cli/src/gui/overview.rs", ""]]
                : []),
            ]
          : null;
if (!changes)
  throw new Error(
    "Choose the fixed metadata, repair, pipeline, navigation or saved-map test variant",
  );
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
for (const name of [
  "original",
  "updated",
  ...(kind === "saved-map" ? ["rejected"] : []),
]) {
  if ((kind === "navigation" || kind === "saved-map") && name === "original") {
    // Private counterfactual only: the original independent two-pass algorithm
    // implements the current typed port. Production has only the combined pass.
    const current = originals[0].text;
    const boundary = current.indexOf("fn is_declaration(");
    if (boundary < 0) throw Error("Lexical declaration boundary missing");
    const prefix = current
      .slice(0, boundary)
      .replace("use std::borrow::Cow;\n", "")
      .replace("use std::collections::BTreeSet;\n", "");
    const oracle = (
      await readFile(
        resolve(source, "crates/meden-app/src/adapters/source/oracle.rs"),
        "utf8",
      )
    )
      .split("\n")
      .filter(
        (line) => !line.startsWith("//!") && line !== "use super::Symbol;",
      )
      .join("\n");
    await writeFile(
      helper,
      prefix +
        oracle +
        `
pub fn index(source: &str, lean: bool) -> Indexed<Symbol> {
    let (declarations, theorems, imports) = headers(source, lean);
    Indexed { declarations, theorems, imports, symbols: symbols(source, lean) }
}
`,
    );
  }
  if (name !== "original")
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
  if (name === "rejected") {
    const overview = resolve(source, "crates/meden-cli/src/gui/overview.rs");
    const current = await readFile(overview, "utf8");
    const original = "saved(workspace).and_then(|s| s.load())";
    if (!current.includes(original))
      throw Error("Saved-map restore boundary missing");
    // Counterfactual only: reintroduce the former method coupling in the
    // original restore location, with the same current storage/schema.
    await writeFile(
      overview,
      current.replace(
        original,
        `saved(workspace).and_then(|s| {
            let saved: Option<Captured> = s.load()?;
            anyhow::ensure!(saved.as_ref().is_none_or(|c| c.method == crate::adapters::navigation::identity()), "Saved navigation method changed");
            Ok(saved)
        })`,
      ),
    );
  }
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
  if (
    name === "rejected" &&
    !bytes.includes(Buffer.from("Saved navigation method changed"))
  )
    throw Error("Former saved-map coupling was not compiled");
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
