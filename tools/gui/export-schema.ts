import { writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { validator } from "./source-schema.mjs";
import { format } from "prettier";
import { z } from "zod";
import { workspaceSchema } from "@meden/workspace-schema";

for (const [name, schema] of [["workspace-v1", workspaceSchema]] as const) {
  const json = z.toJSONSchema(schema);
  writeFileSync(
    "schemas/" + name + ".schema.json",
    JSON.stringify(json, null, 2) + "\n",
  );
}

const source = JSON.parse(
  execFileSync(
    "cargo",
    [
      "run",
      "--quiet",
      "--locked",
      "-p",
      "meden-app",
      "--example",
      "export-source-schema",
    ],
    { encoding: "utf8" },
  ),
);
writeFileSync(
  "schemas/native-source.schema.json",
  JSON.stringify(source, null, 2) + "\n",
);
writeFileSync(
  "packages/agent-protocol/src/source.generated.ts",
  await format(
    `// Generated from meden_app::view::SourcePage. Do not edit.\nimport { z } from "zod";\nexport const sourcePageSchema = ${validator(source)};\nexport type SourcePage = z.infer<typeof sourcePageSchema>;\n`,
    { parser: "typescript" },
  ),
);
