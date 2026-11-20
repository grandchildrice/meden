import { test, expect } from "@playwright/test";
import { spawn, execFile } from "node:child_process";
import { promisify } from "node:util";
import { once } from "node:events";
import { createServer } from "node:http";
import { mkdtemp, mkdir, readFile, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";

const exec = promisify(execFile);
const binary = process.env.MEDEN_BIN || resolve("target/debug/meden");
const original =
  "-- Source inspection must reach the proof beyond the default tool page.\n".repeat(
    500,
  ) +
  "theorem portable_identity (n : Nat) : n = n := rfl\n-- PORTABLE-SOURCE-END\n";

test("portable GUI history survives a real git clone and checks actual Lean without restoring authority", async ({
  page,
}, info) => {
  test.setTimeout(180_000);
  const root = await mkdtemp(resolve(tmpdir(), "meden-portable-gui-"));
  const project = resolve(root, "project"),
    clone = resolve(root, "clone");
  await mkdir(project);
  await writeFile(resolve(project, "Pilot.lean"), original);
  await writeFile(
    resolve(project, "lakefile.toml"),
    'name = "Portable"\n[[lean_lib]]\nname = "Pilot"\n',
  );
  await writeFile(
    resolve(project, "lean-toolchain"),
    "leanprover/lean4:v4.35.0\n",
  );
  await writeFile(resolve(project, ".gitignore"), ".lake/\n");
  const requests: unknown[] = [];
  let repair = false;
  const server = createServer(async (request, response) => {
    if (request.url === "/v1/models") {
      response.writeHead(200, { "Content-Type": "application/json" });
      response.end(
        JSON.stringify({ data: [{ id: "portable-scripted-fixture" }] }),
      );
      return;
    }
    let text = "";
    for await (const chunk of request) text += String(chunk);
    const body = JSON.parse(text);
    requests.push(body);
    let message: unknown = {
      role: "assistant",
      content: "The current work and its remaining obligations are retained.",
    };
    if (repair && body.messages.at(-1)?.role === "user")
      message = {
        role: "assistant",
        content: null,
        tool_calls: [
          {
            id: "repair_source",
            type: "function",
            function: {
              name: "write_file",
              arguments: JSON.stringify({
                path: "Pilot.lean",
                content: original,
              }),
            },
          },
        ],
      };
    response.writeHead(200, { "Content-Type": "application/json" });
    response.end(
      JSON.stringify({ choices: [{ message, finish_reason: "stop" }] }),
    );
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  let child: ReturnType<typeof spawn> | undefined;
  let sessionToken = "";
  async function stop() {
    if (child && child.exitCode === null) {
      const exited = once(child, "exit");
      child.kill("SIGTERM");
      await exited;
    }
    child = undefined;
  }
  async function start(workspace: string, data: string) {
    child = spawn(
      binary,
      [
        "gui",
        "--workspace",
        workspace,
        "--assets",
        resolve("apps/web/dist"),
        "--port",
        "0",
        "--base-url",
        `http://127.0.0.1:${(server.address() as { port: number }).port}/v1`,
        "--model",
        "portable-scripted-fixture",
        "--api-key",
        "PORTABLE_SECRET_MUST_STAY_LOCAL",
      ],
      {
        env: {
          ...process.env,
          XDG_DATA_HOME: data,
          ...(process.env.MEDEN_ATLAS_LEAN
            ? { MEDEN_PROJECT_LEAN: process.env.MEDEN_ATLAS_LEAN }
            : {}),
        },
      },
    );
    let stderr = "";
    child.stderr!.on("data", (data) => (stderr += String(data)));
    const url = await new Promise<string>((resolve, reject) => {
      const timer = setTimeout(
        () => reject(new Error(`GUI startup: ${stderr}`)),
        20_000,
      );
      child!.stdout!.on("data", (data) => {
        const found = String(data).match(/Meden GUI: (http:\/\/\S+)/);
        if (found) {
          clearTimeout(timer);
          resolve(found[1]!);
        }
      });
      child!.once("exit", (code) => {
        clearTimeout(timer);
        reject(new Error(`GUI exited ${code}: ${stderr}`));
      });
    });
    sessionToken = new URL(url).hash.slice("#session=".length);
    await page.goto(url);
    await page
      .getByRole("tab", { name: "プロジェクトマップ", exact: true })
      .click();
    await page.getByRole("button", { name: "作業・履歴", exact: true }).click();
  }
  async function idle() {
    await expect(
      page.getByRole("button", { name: "実行を中止", exact: true }),
    ).toHaveCount(0, { timeout: 90_000 });
  }
  async function approve() {
    const manager = page.getByRole("dialog", { name: /作業の管理/ });
    if (await manager.isVisible())
      await manager
        .getByRole("button", { name: "閉じる", exact: true })
        .click();
    await page
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    await page
      .getByRole("button", { name: "このモデルを使用", exact: true })
      .click();
  }
  try {
    await start(project, resolve(root, "pc-one"));
    await page
      .getByLabel("プロジェクト名", { exact: true })
      .fill("Portable verification");
    await page
      .getByRole("button", { name: "共有履歴を作成", exact: true })
      .click();
    await expect(
      page.getByRole("heading", { name: "Portable verification", exact: true }),
    ).toBeVisible();
    await page.getByRole("button", { name: "作業を追加", exact: true }).click();
    await page.getByLabel("作業ID", { exact: true }).fill("identity");
    await page
      .getByLabel("作業名", { exact: true })
      .fill("Natural-number identity");
    await page
      .getByLabel("作業の要件", { exact: true })
      .fill("Every natural number equals itself.");
    await page.getByLabel("Leanモジュール", { exact: true }).fill("Pilot");
    await page
      .getByLabel("定理名（1行に1件）", { exact: true })
      .fill("portable_identity");
    await page
      .getByLabel("再開時の作業", { exact: true })
      .fill("Read the theorem and preserve its statement.");
    await page.getByRole("button", { name: "作業を保存", exact: true }).click();
    await expect(
      page.getByRole("heading", {
        name: "Natural-number identity",
        exact: true,
      }),
    ).toBeVisible();
    await page
      .getByRole("button", { name: "Leanを再チェック", exact: true })
      .click();
    await expect(
      page.getByText("保存されたチェック記録：成功", { exact: true }),
    ).toBeVisible({ timeout: 90_000 });
    expect(requests).toHaveLength(0);
    await page
      .getByLabel("進捗メモ", { exact: true })
      .fill("The original theorem passed the real local checker.");
    await page
      .getByLabel("次の作業のメモ", { exact: true })
      .fill("Inspect the same theorem after cloning.");
    await page.getByRole("button", { name: "進捗を保存", exact: true }).click();
    await expect(
      page.getByText("Inspect the same theorem after cloning.", {
        exact: true,
      }),
    ).toBeVisible();
    await approve();
    await page
      .getByRole("textbox", { name: "メッセージ", exact: true })
      .fill("Retain this conversation for another computer.");
    await page.getByRole("button", { name: "送信", exact: true }).click();
    await expect(
      page.getByRole("region", { name: "会話", exact: true }),
    ).toContainText("remaining obligations are retained");
    await idle();
    await page.getByRole("button", { name: "作業・履歴", exact: true }).click();
    await page
      .getByRole("button", { name: "会話と作業を保存", exact: true })
      .click();
    await idle();
    await stop();
    const persisted = await readFile(
      resolve(project, ".meden/workspace.json"),
      "utf8",
    );
    expect(persisted).not.toContain("PORTABLE_SECRET_MUST_STAY_LOCAL");
    expect(persisted).not.toContain('"authorized"');
    await exec("git", ["init", "-q", project]);
    await exec("git", ["-C", project, "add", "."]);
    await exec("git", [
      "-C",
      project,
      "-c",
      "user.name=Fixture",
      "-c",
      "user.email=fixture@example.invalid",
      "commit",
      "-qm",
      "Retain portable work",
    ]);
    await exec("git", ["clone", "--quiet", project, clone]);
    const count = requests.length;
    await start(clone, resolve(root, "pc-two"));
    await expect(
      page.getByRole("heading", {
        name: "Natural-number identity",
        exact: true,
      }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "この作業を再開", exact: true }),
    ).toBeDisabled();
    await expect(
      page.getByText("保存されたチェック記録：成功", { exact: true }),
    ).toBeVisible();
    await expect(
      page.getByText("保存された結果は履歴です。", { exact: false }),
    ).toBeVisible();
    await expect(
      page
        .getByRole("region", { name: "会話", exact: true })
        .getByText("保存された会話", { exact: true }),
    ).toHaveCount(0);
    expect(requests).toHaveLength(count);
    await page.getByText(/保存された会話 ·/).click();
    await expect(
      page.getByRole("region", { name: "作業と共有履歴", exact: true }),
    ).toContainText("Retain this conversation for another computer.");
    await page
      .getByRole("button", { name: "証明コードを読む", exact: true })
      .click();
    await expect(
      page
        .locator(".ide-file-page:not([hidden])")
        .getByLabel("ファイルの内容", { exact: true }),
    ).toContainText("1: ");
    await page.getByRole("button", { name: "次のページ", exact: true }).click();
    await expect(
      page
        .locator(".ide-file-page:not([hidden])")
        .getByLabel("ファイルの内容", { exact: true }),
    ).toContainText("201: ");
    await page.getByRole("button", { name: "次のページ", exact: true }).click();
    await expect(
      page
        .locator(".ide-file-page:not([hidden])")
        .getByLabel("ファイルの内容", { exact: true }),
    ).toContainText("401: ");
    await expect(
      page
        .locator(".ide-file-page:not([hidden])")
        .getByLabel("ファイルの内容", { exact: true }),
    ).toContainText("PORTABLE-SOURCE-END");
    await page.getByRole("button", { name: "前のページ", exact: true }).click();
    await expect(
      page
        .locator(".ide-file-page:not([hidden])")
        .getByLabel("ファイルの内容", { exact: true }),
    ).toContainText("201: ");
    await page
      .getByRole("tab", { name: "プロジェクトマップ", exact: true })
      .click();
    await page.getByRole("button", { name: "作業・履歴", exact: true }).click();
    // Deliberately break the source: old transferred success must not admit a fresh check.
    await writeFile(
      resolve(clone, "Pilot.lean"),
      "axiom injected : False\ntheorem portable_identity : False := injected\n",
    );
    await page
      .getByRole("button", { name: "Leanを再チェック", exact: true })
      .click();
    await expect(
      page.getByText("保存されたチェック記録：失敗・中断", { exact: true }),
    ).toBeVisible({ timeout: 90_000 });
    await page
      .getByRole("dialog")
      .getByText(/Leanチェック失敗・中断の記録/)
      .first()
      .click();
    await expect(
      page.getByRole("dialog").getByLabel("定理の型と公理").first(),
    ).toContainText("Unexpected axiom");
    expect(requests).toHaveLength(count);
    await approve();
    repair = true;
    await page.getByRole("button", { name: "作業・履歴", exact: true }).click();
    await page
      .getByRole("button", { name: "この作業を再開", exact: true })
      .click();
    await expect
      .poll(async () => readFile(resolve(clone, "Pilot.lean"), "utf8"))
      .toBe(original);
    await idle();
    await page
      .getByRole("button", { name: "Leanを再チェック", exact: true })
      .click();
    await expect(
      page.getByText("保存されたチェック記録：成功", { exact: true }),
    ).toBeVisible({ timeout: 90_000 });
    await page.screenshot({
      path: resolve("target/gui-checks/portable-workspace.png"),
      fullPage: true,
    });
    const state = await page.evaluate(
      async (token) =>
        (
          await fetch("/api/state", {
            headers: { Authorization: `Bearer ${token}` },
          })
        ).json(),
      sessionToken,
    );
    expect(state.project.verified).toBe(false);
    expect(state.project.accepted).toBe(false);
    expect(JSON.stringify(requests.slice(count))).toContain(
      "repository-provided historical context",
    );
    expect(JSON.stringify(requests.slice(count))).toContain(
      "Retain this conversation for another computer.",
    );
    const continuation = JSON.stringify(requests.slice(count));
    expect(
      continuation.indexOf("Next step for the current turn:"),
    ).toBeLessThan(
      continuation.indexOf("repository-provided historical context"),
    );
    await info.attach("portable-workspace", {
      body: await readFile(resolve(clone, ".meden/workspace.json")),
      contentType: "application/json",
    });
  } finally {
    await stop();
    await new Promise<void>((resolve) => server.close(() => resolve()));
    await rm(root, { recursive: true, force: true });
  }
});
