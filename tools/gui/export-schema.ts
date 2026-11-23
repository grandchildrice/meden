import { writeFileSync } from "node:fs";
import { z } from "zod";
import { workspaceSchema } from "@meden/workspace-schema";

for (const [name, schema] of [["workspace-v1", workspaceSchema]] as const) {
  const json = z.toJSONSchema(schema);
  writeFileSync(
    "schemas/" + name + ".schema.json",
    JSON.stringify(json, null, 2) + "\n",
  );
}
