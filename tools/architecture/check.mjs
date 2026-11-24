import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, extname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const pure = {
  "workspace-schema": ["zod"],
  "agent-protocol": ["zod", "@meden/workspace-schema"],
  "atlas-adapter": ["zod", "@meden/workspace-schema"],
  "expert-review": ["@meden/workspace-schema", "@meden/atlas-adapter"],
};
const layers = {
  "meden-core": [],
  "meden-agent": ["meden-core"],
  "meden-runtime": ["meden-core", "meden-agent"],
  "meden-app": ["meden-core", "meden-agent", "meden-runtime"],
  "meden-llm": ["meden-core", "meden-agent", "meden-runtime", "meden-app"],
  "meden-cli": [
    "meden-core",
    "meden-agent",
    "meden-runtime",
    "meden-app",
    "meden-llm",
  ],
};

export function cargoViolations(packages, members) {
  const names = new Set(
    packages.filter((p) => members.includes(p.id)).map((p) => p.name),
  );
  const errors = [];
  for (const p of packages.filter((p) => members.includes(p.id))) {
    if (!layers[p.name]) {
      errors.push(`${p.name}: assign the new crate an architectural role`);
      continue;
    }
    for (const d of p.dependencies.filter((d) => d.kind !== "dev")) {
      if (names.has(d.name) && !layers[p.name].includes(d.name))
        errors.push(`${p.name}: outward dependency on ${d.name}`);
      if (p.name === "meden-core" && !["serde", "thiserror"].includes(d.name))
        errors.push(`${p.name}: unreviewed kernel dependency ${d.name}`);
      if (
        p.name === "meden-app" &&
        ["ratatui", "crossterm", "clap", "tiny_http"].includes(d.name)
      )
        errors.push(
          `${p.name}: presentation implementation dependency ${d.name}`,
        );
    }
    if (
      p.name === "meden-core" &&
      p.targets.some((t) => t.kind.includes("custom-build"))
    )
      errors.push("meden-core: a host build script is outside the pure kernel");
  }
  return errors;
}

export function sourceViolations(source, checker, packageRoot, allowed) {
  const errors = [];
  const fail = (node, message) => {
    const { line } = source.getLineAndCharacterOfPosition(
      node.getStart(source),
    );
    errors.push(`${relative(root, source.fileName)}:${line + 1}: ${message}`);
  };
  const dependency = (node) => {
    if (!node) return;
    if (!ts.isStringLiteralLike(node)) {
      fail(node, "data packages require static module specifiers");
      return;
    }
    const name = node.text.replaceAll("\\", "/");
    if (name.startsWith(".")) {
      const path = relative(
        packageRoot,
        resolve(dirname(source.fileName), name),
      ).replaceAll("\\", "/");
      if (path === ".." || path.startsWith("../"))
        fail(node, "relative import leaves its package");
    } else {
      const packageName = name.startsWith("@")
        ? name.split("/").slice(0, 2).join("/")
        : name.split("/")[0];
      if (!allowed.includes(packageName))
        fail(node, `dependency ${name} is outside this data/port boundary`);
    }
  };
  const visit = (node) => {
    if (ts.isImportDeclaration(node) || ts.isExportDeclaration(node))
      dependency(node.moduleSpecifier);
    if (ts.isImportTypeNode(node) && ts.isLiteralTypeNode(node.argument))
      dependency(node.argument.literal);
    if (
      ts.isImportEqualsDeclaration(node) &&
      ts.isExternalModuleReference(node.moduleReference)
    )
      dependency(node.moduleReference.expression);
    if (ts.isCallExpression(node)) {
      if (
        node.expression.kind === ts.SyntaxKind.ImportKeyword ||
        (ts.isIdentifier(node.expression) && node.expression.text === "require")
      )
        dependency(node.arguments[0]);
      if (["Date.now", "Math.random"].includes(node.expression.getText(source)))
        fail(node, "pass clock/seed values explicitly");
    }
    if (
      ts.isNewExpression(node) &&
      node.expression.getText(source) === "Date" &&
      !node.arguments?.length
    )
      fail(node, "pass the timestamp explicitly");
    if (ts.isIdentifier(node)) {
      const symbol = checker.getSymbolAtLocation(node);
      if (
        symbol?.declarations?.some((d) =>
          /typescript\/lib\/lib\.(dom|webworker)[.]/.test(
            d.getSourceFile().fileName.replaceAll("\\", "/"),
          ),
        )
      )
        fail(node, `browser global/type ${node.text} belongs in an adapter`);
    }
    ts.forEachChild(node, visit);
  };
  visit(source);
  if (
    relative(root, source.fileName).replaceAll("\\", "/") ===
    "packages/agent-protocol/src/native.ts"
  )
    errors.push(...nativeBarrelViolations(source));
  return errors;
}

// A maintenance boundary, not an alias-proof security check. Host controllers
// remain authoritative regardless of any browser type or validator.
export function nativeBarrelViolations(source) {
  const aliases = new Set(["HistoryData", "PortableWorkspace", "WorkItem"]);
  return source.statements
    .filter((node) => {
      if (ts.isImportDeclaration(node) || ts.isExportDeclaration(node))
        return (
          !node.moduleSpecifier ||
          !ts.isStringLiteral(node.moduleSpecifier) ||
          !node.moduleSpecifier.text.startsWith("./") ||
          !node.moduleSpecifier.text.endsWith(".generated")
        );
      return !(ts.isTypeAliasDeclaration(node) && aliases.has(node.name.text));
    })
    .map((node) => {
      const { line } = source.getLineAndCharacterOfPosition(
        node.getStart(source),
      );
      return `${relative(root, source.fileName)}:${line + 1}: native wire shapes must be generated; keep the barrel to generated re-exports and derived history aliases`;
    });
}

// This entry module is captured in the compiler implementation identity.
// Wire metadata belongs in view modules; this is a maintenance rule, not a
// general Rust parser or semantic-equivalence proof.
export function compilerEntryViolations(source) {
  return /\bJsonSchema\b/.test(source)
    ? [
        "compiler-captured atlas entry must not own wire schemas; project availability in view instead",
      ]
    : [];
}

export function programFor(files, virtual = new Map()) {
  const options = {
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.NodeNext,
    moduleResolution: ts.ModuleResolutionKind.NodeNext,
    noEmit: true,
    allowJs: true,
    types: [],
    skipLibCheck: true,
  };
  const host = ts.createCompilerHost(options);
  const get = host.getSourceFile.bind(host);
  host.getSourceFile = (name, language, ...rest) =>
    virtual.has(name)
      ? ts.createSourceFile(name, virtual.get(name), language, true)
      : get(name, language, ...rest);
  return ts.createProgram(files, options, host);
}

function sources(path) {
  return readdirSync(path, { withFileTypes: true }).flatMap((entry) => {
    if (["node_modules", "dist"].includes(entry.name)) return [];
    const file = resolve(path, entry.name);
    return entry.isDirectory()
      ? sources(file)
      : [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts"].includes(
            extname(file),
          ) && !/\.(test|spec)\.[cm]?[jt]sx?$/.test(file)
        ? [file]
        : [];
  });
}

export function check() {
  const metadata = JSON.parse(
    execFileSync(
      "cargo",
      [
        "metadata",
        "--format-version",
        "1",
        "--no-deps",
        "--locked",
        "--offline",
      ],
      { cwd: root, encoding: "utf8", maxBuffer: 4 * 1024 * 1024 },
    ),
  );
  const errors = cargoViolations(metadata.packages, metadata.workspace_members);
  errors.push(
    ...compilerEntryViolations(
      readFileSync(resolve(root, "crates/meden-app/src/atlas/mod.rs"), "utf8"),
    ),
  );
  const packages = Object.entries(pure).map(([name, allowed]) => ({
    root: resolve(root, "packages", name),
    allowed,
  }));
  const files = packages.flatMap((p) => sources(p.root));
  const program = programFor(files),
    checker = program.getTypeChecker();
  for (const p of packages) {
    const manifest = JSON.parse(
      readFileSync(resolve(p.root, "package.json"), "utf8"),
    );
    for (const dependency of Object.keys(manifest.dependencies || {}))
      if (!p.allowed.includes(dependency))
        errors.push(`${manifest.name}: unreviewed dependency ${dependency}`);
    for (const file of files.filter(
      (f) => !relative(p.root, f).startsWith(".."),
    ))
      errors.push(
        ...sourceViolations(
          program.getSourceFile(file),
          checker,
          p.root,
          p.allowed,
        ),
      );
  }
  if (errors.length) throw new Error(errors.join("\n"));
  console.log(
    `Architecture boundaries: ${metadata.workspace_members.length} Rust crates and ${files.length} data/port sources checked.`,
  );
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  check();
