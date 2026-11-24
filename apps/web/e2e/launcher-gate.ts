import {
  access,
  chmod,
  readFile,
  realpath,
  rm,
  writeFile,
} from "node:fs/promises";
import { constants } from "node:fs";
import { delimiter, resolve } from "node:path";

async function publicTool(name: string) {
  for (const entry of (process.env.PATH ?? "").split(delimiter)) {
    const directory = await realpath(entry || ".").catch(() => "");
    if (
      !directory.startsWith("/nix/store/") &&
      !directory.startsWith("/usr/") &&
      directory !== "/bin"
    )
      continue;
    const candidate = resolve(entry, name);
    const target = await realpath(candidate).catch(() => "");
    if (
      !target.startsWith("/nix/store/") &&
      !target.startsWith("/usr/") &&
      !target.startsWith("/bin/")
    )
      continue;
    if (
      await access(candidate, constants.X_OK).then(
        () => true,
        () => false,
      )
    )
      return candidate;
  }
  throw new Error(
    `Installed public ${name} is required for the verification gate`,
  );
}
const quote = (value: string) => "'" + value.replaceAll("'", "'\\''") + "'";

/** Trusted host launcher fixture, outside the writable project. Only the fixed
 * Z3 or recorded-work Lake build dispatch is held. Release executes Bubblewrap with unchanged
 * arguments; no checker result, wrapper solver or unconfined fallback exists.
 */
export async function verificationGate(
  root: string,
  kind: "z3" | "recorded" = "z3",
) {
  const bash = await publicTool("bash"),
    sleep = await publicTool("sleep");
  const bwrap =
    process.env.MEDEN_BWRAP === undefined
      ? await publicTool("bwrap")
      : resolve(process.env.MEDEN_BWRAP);
  await access(bwrap, constants.X_OK);
  const launcher = resolve(root, "held-bwrap"),
    started = resolve(root, "verification-started"),
    release = resolve(root, "verification-release");
  await writeFile(
    launcher,
    `#!${bash}
set -eu
hold=false
${kind === "z3" ? 'if [[ "$#" -ge 2 && "\${@: -2:1}" == "-in" && "\${@: -1}" == "-smt2" ]]; then hold=true; fi' : 'previous=""; for argument in "$@"; do if [[ "$previous" == "/analysis/library/project.sh" && "$argument" == "build" ]]; then hold=true; fi; previous="$argument"; done'}
if $hold; then
  printf started > ${quote(started)}
  while [[ ! -e ${quote(release)} ]]; do ${quote(sleep)} 0.02; done
fi
exec ${quote(bwrap)} "$@"
`,
  );
  await chmod(launcher, 0o755);
  return {
    environment: { MEDEN_BWRAP: launcher },
    started: () => readFile(started, "utf8").catch(() => ""),
    reset: () => rm(started, { force: true }),
    release: () => writeFile(release, "release"),
  };
}
