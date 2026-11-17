import { writeFileSync } from "node:fs";
import { z } from "zod";
import { workspaceSchema } from "@meden/workspace-schema";
import { eventSchema } from "@meden/agent-protocol";

for (const [name, schema] of [
  ["workspace-v1", workspaceSchema],
  ["agent-event-v1", eventSchema],
] as const) {
  const json = z.toJSONSchema(schema);
  writeFileSync(
    "schemas/" + name + ".schema.json",
    JSON.stringify(json, null, 2) + "\n",
  );
}
