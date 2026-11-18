/** Real-browser/live-provider journey. No routes, replies or checker installation are mocked. */
import { chromium, expect, type Page } from "@playwright/test";
import {
  nativeStateSchema,
  type NativeState,
} from "@meden/agent-protocol/native";
import { createHash } from "node:crypto";
import { parseArgs } from "node:util";
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { mkdtemp, mkdir, readFile, writeFile, rm } from "node:fs/promises";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { tmpdir } from "node:os";
import { once } from "node:events";

const { values } = parseArgs({
  options: {
    binary: { type: "string", default: "target/debug/meden" },
    provider: { type: "string" },
    model: { type: "string" },
    out: { type: "string" },
    assets: { type: "string", default: "apps/web/dist" },
    chrome: { type: "string" },
    timeout: { type: "string", default: "180000" },
  },
});
if (!values.provider || !values.model || !values.out)
  throw new Error("Supply --provider, --model and --out explicitly");
const deadline = Number(values.timeout);
if (!Number.isFinite(deadline) || deadline < 1000)
  throw new Error("Invalid timeout");
const binary = resolve(values.binary!);
const out = resolve(values.out);
await mkdir(out, { recursive: true });
const root = await mkdtemp(resolve(tmpdir(), "meden-live-gui-"));
const project = resolve(root, "project");
await mkdir(project);
const originalTests =
  'include!("kernel.rs");\n#[test] fn boundary() { assert!(!can_purchase(149, 1)); assert!(can_purchase(150, 1)); assert!(!can_purchase(150, 0)); }\n';
await writeFile(
  resolve(project, "kernel.rs"),
  "pub fn can_purchase(balance: u64, stock: u64) -> bool { balance >= 100 && stock > 0 }\n",
);
await writeFile(resolve(project, "tests.rs"), originalTests);
const report: Record<string, unknown> = {
  success: false,
  binarySha256: createHash("sha256")
    .update(await readFile(binary))
    .digest("hex"),
  provider: values.provider,
  model: values.model,
  inference: "live",
  originalTestsPreserved: false,
  cases: [],
};
let child: ChildProcess | undefined;
const browser = await chromium.launch({
  executablePath:
    values.chrome ||
    process.env.MEDEN_GUI_CHROME ||
    (existsSync("/run/current-system/sw/bin/google-chrome-stable")
      ? "/run/current-system/sw/bin/google-chrome-stable"
      : undefined),
  args: ["--no-sandbox"],
});
const page = await browser.newPage({
  viewport: { width: 1440, height: 1000 },
  reducedMotion: "reduce",
});
page.setDefaultTimeout(deadline);
async function state(page: Page): Promise<NativeState> {
  // Read-only host observation; all user actions, including approval, use the GUI.
  const value = await page.evaluate(async () => {
    const token = sessionStorage.getItem("meden-native-session");
    const response = await fetch("/api/state", {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!response.ok) throw new Error("Native state unavailable");
    return response.json();
  });
  return nativeStateSchema.parse(value);
}
async function launch(port = 0) {
  let stderr = "";
  child = spawn(
    binary,
    [
      "gui",
      "--workspace",
      project,
      "--assets",
      resolve(values.assets!),
      "--provider",
      values.provider!,
      "--model",
      values.model!,
      "--port",
      String(port),
    ],
    { env: { ...process.env, XDG_DATA_HOME: resolve(root, "data") } },
  );
  child.stderr!.on("data", (data) => {
    stderr += String(data);
  });
  const url = await new Promise<string>((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new Error(`Gateway URL timeout: ${stderr}`)),
      15_000,
    );
    let output = "";
    child!.stdout!.on("data", (data) => {
      output += String(data);
      const match = output.match(/Meden GUI: (http:\/\/\S+)/);
      if (match) {
        clearTimeout(timer);
        resolve(match[1]!);
      }
    });
    child!.once("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child!.once("exit", (code) => {
      clearTimeout(timer);
      reject(new Error(`Gateway exited ${code}: ${stderr}`));
    });
  });
  await page.goto(url);
  await expect(
    page.getByRole("button", { name: "接続先を確認", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "接続先を確認", exact: true }).click();
  await expect(page.getByRole("dialog")).toContainText(values.model!);
  await page.screenshot({
    path: resolve(out, "processor-review.png"),
    fullPage: true,
  });
  await page
    .getByRole("button", { name: "この送信先を承認", exact: true })
    .click();
  console.log("Processor reviewed through the browser");
  return Number(new URL(url).port);
}
async function stop() {
  if (child?.exitCode === null) {
    const process = child;
    const exited = once(process, "exit");
    const timer = setTimeout(() => process.kill("SIGKILL"), 10_000);
    process.kill("SIGTERM");
    await exited;
    clearTimeout(timer);
  }
  child = undefined;
}
async function waitIdle() {
  await expect
    .poll(async () => (await state(page)).running, {
      timeout: deadline,
      intervals: [350, 1000],
    })
    .toBe(false);
}
try {
  const port = await launch();
  const instruction =
    "Fix kernel.rs so can_purchase(balance: u64, stock: u64) returns true exactly when balance is at least 150 and stock is positive. Keep the existing tests.rs unchanged. Read the files first. Run rustc --edition=2021 --test tests.rs -o tests && ./tests. This task needs ordinary code and tests, not a new formal contract.";
  await page
    .getByRole("textbox", { name: "メッセージ", exact: true })
    .fill(instruction);
  await page.getByRole("button", { name: "送信", exact: true }).click();
  await expect
    .poll(
      async () =>
        (await state(page)).messages.some(
          (message) => message.text === instruction,
        ),
      { timeout: deadline },
    )
    .toBe(true);
  await waitIdle();
  const developed = await state(page);
  if (developed.stopped) throw new Error(developed.stopped);
  console.log("Native development turn completed");
  report.developmentLogs = developed.logs;
  report.source = await readFile(resolve(project, "kernel.rs"), "utf8");
  report.originalTestsPreserved =
    (await readFile(resolve(project, "tests.rs"), "utf8")) === originalTests;
  const independent = spawnSync(binary, ["exec", "--workspace", project], {
    input:
      "rustc --edition=2021 --test tests.rs -o /workspace/independent-tests && /workspace/independent-tests",
    encoding: "utf8",
    timeout: 30_000,
    env: { ...process.env, XDG_DATA_HOME: resolve(root, "data") },
  });
  report.independentExit = independent.status;
  const independentResult = JSON.parse(independent.stdout) as {
    result?: { exit_code?: number; stdout?: string; stderr?: string };
  };
  report.independentResult = independentResult.result;
  if (
    !report.originalTestsPreserved ||
    independent.status !== 0 ||
    independentResult.result?.exit_code !== 0
  )
    throw new Error("Independent original tests did not pass");
  await page.getByRole("tab", { name: "グラフ", exact: true }).click();
  await page
    .getByRole("textbox", { name: "グラフの対象ファイル（1行に1つ）" })
    .fill("kernel.rs");
  await page
    .getByRole("button", {
      name: "ソースを読み直してグラフを表示",
      exact: true,
    })
    .click();
  await page
    .getByRole("region", { name: "ローカルソースのグラフ" })
    .getByRole("button", { name: /can_purchase.*を選択/ })
    .click();
  for (const [name, requirement, expected] of [
    [
      "missing-threshold",
      "A purchase is permitted exactly when balance is at least 100 and stock is positive.",
      "mismatch",
    ],
    [
      "matching-threshold",
      "A purchase is permitted exactly when balance is at least 150 and stock is positive.",
      "match",
    ],
  ] as const) {
    await page
      .getByRole("textbox", { name: "比較する要件", exact: true })
      .fill(requirement);
    await page
      .getByRole("button", { name: "この宣言の意味を解析", exact: true })
      .click();
    await expect
      .poll(
        async () => {
          const current = await state(page);
          return (
            current.analysis?.input.requirement === requirement &&
            !current.running &&
            current.analysis?.state
          );
        },
        { timeout: deadline, intervals: [350, 1000] },
      )
      .toBe("complete");
    const candidate = (await state(page)).analysis;
    console.log(
      `Meaning case completed: ${name} (${candidate?.assessment?.verdict})`,
    );
    (report.cases as unknown[]).push({
      name,
      requirement,
      analysis: candidate,
    });
    await page
      .getByLabel("意味解析の結果", { exact: true })
      .scrollIntoViewIfNeeded();
    await page.screenshot({
      path: resolve(out, name + ".png"),
      fullPage: true,
    });
    if (candidate?.assessment?.verdict !== expected)
      throw new Error(
        `Live meaning case ${name}: expected ${expected}, observed ${candidate?.assessment?.verdict}`,
      );
    await expect(page.getByLabel("選択した宣言")).toContainText("証明は未確認");
  }
  await stop();
  await launch(port);
  await expect(page.getByText(instruction, { exact: true })).toBeVisible();
  await page.getByRole("tab", { name: "グラフ", exact: true }).click();
  await expect(
    page.getByLabel("意味解析の結果", { exact: true }),
  ).toContainText("保存された解析です");
  report.restartedWithHistoricalAnalysis = true;
  report.success = true;
} catch (error) {
  report.error = String(error);
  report.lastState = await state(page).catch(() => undefined);
  await page
    .screenshot({ path: resolve(out, "failure.png"), fullPage: true })
    .catch(() => {});
} finally {
  await stop();
  await browser.close();
  report.finishedAt = new Date().toISOString();
  await writeFile(
    resolve(out, "report.json"),
    JSON.stringify(report, null, 2) + "\n",
  );
  await rm(root, { recursive: true, force: true });
}
console.log(
  JSON.stringify(
    { success: report.success, out, error: report.error },
    null,
    2,
  ),
);
if (!report.success) process.exitCode = 1;
