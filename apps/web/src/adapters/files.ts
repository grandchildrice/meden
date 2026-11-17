import {
  importWorkspace,
  createWorkspace,
  type Workspace,
} from "@meden/workspace-schema";
import { importAtlas } from "@meden/atlas-adapter";

const MAX_IMPORT_BYTES = 16 * 1024 * 1024;
export async function readWorkspaceFile(file: File): Promise<Workspace> {
  if (file.size > MAX_IMPORT_BYTES)
    throw new Error("16 MB以下のJSONファイルを選んでください。");
  let data: unknown;
  try {
    data = JSON.parse(await file.text());
  } catch {
    throw new Error("JSONを読み込めません。ファイルの形式を確認してください。");
  }
  try {
    if (typeof data === "object" && data !== null && "schemaVersion" in data)
      return importWorkspace(data);
    const graph = importAtlas(data);
    return { ...createWorkspace(graph.name), ...graph, origin: "imported" };
  } catch {
    throw new Error(
      "MedenワークスペースまたはLean Atlasのグラフ形式を確認してください。要件・宣言・根拠の参照先も必要です。",
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
