import {
  importWorkspace,
  createWorkspace,
  type Workspace,
} from "@meden/workspace-schema";
import { importAtlas } from "@meden/atlas-adapter";

const MAX_IMPORT_BYTES = 16 * 1024 * 1024;
export async function readWorkspaceFile(file: File): Promise<Workspace> {
  if (file.size > MAX_IMPORT_BYTES)
    throw new Error("Choose a JSON file no larger than 16 MB.");
  let data: unknown;
  try {
    data = JSON.parse(await file.text());
  } catch {
    throw new Error("Could not parse JSON. Check the file format.");
  }
  try {
    if (typeof data === "object" && data !== null && "schemaVersion" in data)
      return importWorkspace(data);
    const graph = importAtlas(data);
    return {
      ...createWorkspace(graph.name, {
        id: crypto.randomUUID(),
        updatedAt: new Date().toISOString(),
      }),
      ...graph,
      origin: "imported",
    };
  } catch {
    throw new Error(
      "Check the Meden workspace or Lean Atlas graph format, including requirement, declaration and evidence references.",
    );
  }
}
export function downloadJson(name: string, value: unknown): void {
  const url = URL.createObjectURL(
    new Blob([JSON.stringify(value, null, 2) + "\n"], {
      type: "application/json",
    }),
  );
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = name;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
