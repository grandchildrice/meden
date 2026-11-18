import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import {
  createServer,
  request as httpRequest,
  type ServerResponse,
} from "node:http";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { once } from "node:events";

test.describe("native GUI with real offline workers", () => {
  test.describe.configure({ mode: "serial" });
  let root: string,
    project: string,
    child: ChildProcess | undefined,
    url: string;
  let requests: unknown[], responses: Array<(response: ServerResponse) => void>;
  let server: ReturnType<typeof createServer>;
  const reply = (response: ServerResponse, message: unknown) => {
    response.writeHead(200, { "Content-Type": "application/json" });
    response.end(
      JSON.stringify({ choices: [{ message, finish_reason: "stop" }] }),
    );
  };
  const tool = (name: string, args: unknown) => ({
    role: "assistant",
    content: null,
    tool_calls: [
      {
        id: "call",
        type: "function",
        function: { name, arguments: JSON.stringify(args) },
      },
    ],
  });
  async function start(port = 0) {
    let stderr = "";
    child = spawn(
      process.env.MEDEN_BIN || resolve("target/debug/meden"),
      [
        "gui",
        "--workspace",
        project,
        "--assets",
        resolve("apps/web/dist"),
        "--port",
        String(port),
        "--base-url",
        `http://127.0.0.1:${(server.address() as { port: number }).port}/v1`,
        "--model",
        "scripted-gui-fixture",
        "--api-key",
        "NATIVE-FIXTURE-NOT-SAVED",
      ],
      { env: { ...process.env, XDG_DATA_HOME: resolve(root, "data") } },
    );
    child.stderr!.on("data", (data) => {
      stderr += String(data);
    });
    url = await new Promise<string>((resolve, reject) => {
      let output = "";
      const timer = setTimeout(
        () => reject(new Error(`Gateway start timed out: ${stderr}`)),
        15_000,
      );
      child!.stdout!.on("data", (data) => {
        output += String(data);
        const match = output.match(/Meden GUI: (http:\/\/[^\s]+)/);
        if (match) {
          clearTimeout(timer);
          resolve(match[1]!);
        }
      });
      child!.on("error", (error) => {
        clearTimeout(timer);
        reject(error);
      });
      child!.once("exit", (code) => {
        clearTimeout(timer);
        reject(new Error(`Gateway exited ${code}: ${stderr}`));
      });
    });
    return url;
  }
  async function stop() {
    if (child?.exitCode === null) {
      const exited = once(child, "exit");
      child.kill("SIGTERM");
      await exited;
    }
    child = undefined;
  }
  test.beforeEach(async () => {
    root = await mkdtemp(resolve(tmpdir(), "meden-native-browser-"));
    project = resolve(root, "project");
    await mkdir(project);
    await writeFile(
      resolve(project, "original-test.txt"),
      "keep this original test\n",
    );
    requests = [];
    responses = [];
    server = createServer(async (request, response) => {
      let body = "";
      for await (const chunk of request) body += String(chunk);
      requests.push(JSON.parse(body));
      const respond = responses.shift();
      if (!respond) {
        response.writeHead(500);
        response.end("unexpected fixture request");
        return;
      }
      respond(response);
    });
    server.listen(0, "127.0.0.1");
    await once(server, "listening");
  });
  test.afterEach(async () => {
    server.closeAllConnections();
    await stop();
    await new Promise<void>((resolve) => server.close(() => resolve()));
    await rm(root, { recursive: true, force: true });
  });

  test("develop, recover from provider failure, inspect files and restart without restoring grants", async ({
    page,
  }) => {
    responses.push(
      (response) =>
        reply(
          response,
          tool("write_file", {
            path: "result.txt",
            content: "built by native worker",
          }),
        ),
      (response) => {
        response.writeHead(429);
        response.end();
      },
      (response) => reply(response, tool("read_file", { path: "result.txt" })),
      (response) =>
        reply(response, {
          role: "assistant",
          content: "RESTART-CONTEXT-RETAINED",
        }),
    );
    await page.goto(await start());
    await expect(
      page.getByRole("textbox", { name: "メッセージ", exact: true }),
    ).toBeDisabled();
    expect(requests).toHaveLength(0);
    await page
      .getByRole("button", { name: "接続先を確認", exact: true })
      .click();
    await expect(page.getByRole("dialog")).toContainText(
      "scripted-gui-fixture",
    );
    await page.getByRole("button", { name: "この送信先を承認" }).click();
    await page
      .getByRole("textbox", { name: "メッセージ", exact: true })
      .fill("Build the fixture and keep the original tests");
    await page.getByRole("button", { name: "送信", exact: true }).click();
    await expect(
      page.getByRole("alert").filter({ hasText: "429" }),
    ).toBeVisible();
    expect(await readFile(resolve(project, "result.txt"), "utf8")).toBe(
      "built by native worker",
    );
    expect(await readFile(resolve(project, "original-test.txt"), "utf8")).toBe(
      "keep this original test\n",
    );
    await page
      .getByRole("textbox", { name: "ワークスペース内のパス" })
      .fill("result.txt");
    await page.getByRole("button", { name: "ファイルを読む" }).click();
    await expect(page.getByLabel("ファイルの内容")).toContainText(
      "built by native worker",
    );
    await page
      .getByRole("textbox", { name: "メッセージ", exact: true })
      .fill("Continue from the retained files");
    await page.reload();
    await expect(
      page.getByRole("textbox", { name: "メッセージ", exact: true }),
    ).toHaveValue("Continue from the retained files");
    const port = Number(new URL(url).port);
    await stop();
    await page.goto(await start(port));
    await expect(
      page.getByRole("textbox", { name: "メッセージ", exact: true }),
    ).toBeDisabled();
    await expect(
      page.getByText("Build the fixture and keep the original tests", {
        exact: true,
      }),
    ).toBeVisible();
    await page
      .getByRole("button", { name: "接続先を確認", exact: true })
      .click();
    await page.getByRole("button", { name: "この送信先を承認" }).click();
    await page.getByRole("button", { name: "送信", exact: true }).click();
    await expect(
      page.getByText("RESTART-CONTEXT-RETAINED", { exact: true }),
    ).toBeVisible();
    expect(JSON.stringify(requests[2])).toContain("built by native worker");
    expect(JSON.stringify(requests[2])).toContain("Build the fixture");
    await page.getByRole("button", { name: "送信の許可を取り消す" }).click();
    await expect(
      page.getByRole("textbox", { name: "メッセージ", exact: true }),
    ).toBeDisabled();
    await page
      .getByRole("button", { name: "接続先を確認", exact: true })
      .click();
    await expect(
      page.getByRole("button", { name: "この送信先を承認" }),
    ).toBeDisabled();
    await page.getByRole("button", { name: "閉じる", exact: true }).click();
    await page.screenshot({
      path: "target/gui-checks/native-restart.png",
      fullPage: true,
    });
    const files = await readFile(resolve(project, "result.txt"), "utf8");
    expect(files).not.toContain("NATIVE-FIXTURE");
  });

  test("visible silent wait, cancellation before model reply and guarded local API", async ({
    page,
  }) => {
    let held: ServerResponse | undefined;
    responses.push(
      (response) => {
        held = response;
      },
      (response) =>
        reply(response, { role: "assistant", content: "EXPLICIT-NEW-TURN" }),
    );
    await page.goto(await start());
    const origin = new URL(url).origin;
    const token = new URLSearchParams(new URL(url).hash.slice(1)).get(
      "session",
    )!;
    const headers = {
      Authorization: `Bearer ${token}`,
      Origin: origin,
      "Content-Type": "application/json",
    };
    expect((await fetch(origin + "/api/state")).status).toBe(401);
    expect(
      (
        await fetch(origin + "/api/state", {
          headers: { ...headers, Origin: "https://evil.invalid" },
        })
      ).status,
    ).toBe(403);
    const wrongHost = await new Promise<number | undefined>(
      (resolve, reject) => {
        const request = httpRequest(
          origin + "/api/state",
          { headers: { ...headers, Host: "evil.invalid" } },
          (response) => {
            response.resume();
            resolve(response.statusCode);
          },
        );
        request.on("error", reject);
        request.end();
      },
    );
    expect(wrongHost).toBe(403);
    expect(
      (
        await fetch(origin + "/api/command", {
          method: "POST",
          headers,
          body: JSON.stringify({
            type: "approve_processor",
            endpoint: "https://evil.invalid",
          }),
        })
      ).status,
    ).toBe(409);
    await page
      .getByRole("button", { name: "接続先を確認", exact: true })
      .click();
    await page.getByRole("button", { name: "この送信先を承認" }).click();
    await page
      .getByRole("textbox", { name: "メッセージ", exact: true })
      .fill("Wait for cancellation");
    await page.getByRole("button", { name: "送信", exact: true }).click();
    await expect(page.getByRole("status")).toContainText("Waiting for model");
    await expect.poll(() => held !== undefined).toBe(true);
    expect(
      (
        await fetch(origin + "/api/command", {
          method: "POST",
          headers,
          body: JSON.stringify({ type: "instruction", text: "must not queue" }),
        })
      ).status,
    ).toBe(409);
    await page.getByRole("button", { name: "実行を中止" }).click();
    reply(
      held!,
      tool("write_file", { path: "must-not-exist.txt", content: "bad" }),
    );
    await expect(
      page.getByRole("button", { name: "実行を中止" }),
    ).not.toBeVisible();
    await expect(
      readFile(resolve(project, "must-not-exist.txt")),
    ).rejects.toThrow();
    await page
      .getByRole("textbox", { name: "メッセージ", exact: true })
      .fill("Start an explicit new turn");
    await page.getByRole("button", { name: "送信", exact: true }).click();
    await expect(
      page.getByText("EXPLICIT-NEW-TURN", { exact: true }),
    ).toBeVisible();
    await page.setViewportSize({ width: 390, height: 844 });
    await expect(
      page.getByRole("tab", { name: "コード", exact: true }),
    ).toBeVisible();
    expect(
      (
        await new AxeBuilder({ page })
          .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
          .analyze()
      ).violations,
    ).toEqual([]);
    await page.screenshot({
      path: "target/gui-checks/native-cancel.png",
      fullPage: true,
    });
  });
  test("local source graph follows exact code, refreshes after development and rejects unavailable inputs", async ({
    page,
  }) => {
    const source =
      "namespace Demo\ndef transition (stock : Nat) : Nat :=\n  if stock > 0 then stock - 1 else stock\n\ntheorem stock_bound (stock : Nat) : transition stock ≤ stock := by\n  sorry\nend Demo\n";
    await writeFile(resolve(project, "Spec.lean"), source);
    responses.push(
      (response) =>
        reply(
          response,
          tool("write_file", {
            path: "Spec.lean",
            content: source.replace("stock_bound", "inventory_bound"),
          }),
        ),
      (response) =>
        reply(response, { role: "assistant", content: "SOURCE-UPDATED" }),
    );
    await page.goto(await start());
    await page.getByRole("tab", { name: "グラフ", exact: true }).click();
    await page
      .getByRole("textbox", { name: "グラフの対象ファイル（1行に1つ）" })
      .fill("Spec.lean");
    await page
      .getByRole("button", { name: "ソースを読み直してグラフを表示" })
      .click();
    const graph = page.getByRole("region", { name: "ローカルソースのグラフ" });
    await expect(graph).toContainText("1ファイル · 2宣言 · 1参照候補");
    expect(requests).toHaveLength(0);
    await graph
      .getByRole("button", { name: /Demo.stock_bound.*を選択/ })
      .click();
    await expect(page.getByLabel("選択した宣言")).toContainText("証明は未確認");
    await page.screenshot({
      path: "target/gui-checks/native-live-graph.png",
      fullPage: true,
    });
    await page
      .getByRole("button", { name: "Spec.lean:5 のソースを開く", exact: true })
      .click();
    await expect(
      page.getByRole("dialog").locator('[aria-current="location"]'),
    ).toContainText("theorem stock_bound");
    await expect(
      page.getByRole("dialog").getByLabel("ファイルの内容"),
    ).toContainText("sorry");
    await page.screenshot({
      path: "target/gui-checks/native-source-graph.png",
      fullPage: true,
    });
    await page.getByRole("button", { name: "閉じる", exact: true }).click();
    await page
      .getByRole("button", { name: "接続先を確認", exact: true })
      .click();
    await page.getByRole("button", { name: "この送信先を承認" }).click();
    await page
      .getByRole("textbox", { name: "メッセージ", exact: true })
      .fill("Rename the property to inventory_bound");
    await page.getByRole("button", { name: "送信", exact: true }).click();
    await expect(
      page.getByText("SOURCE-UPDATED", { exact: true }),
    ).toBeVisible();
    await expect(graph).not.toBeVisible();
    await page
      .getByRole("button", { name: "ソースを読み直してグラフを表示" })
      .click();
    await expect(
      graph.getByRole("button", { name: /Demo.inventory_bound.*を選択/ }),
    ).toBeVisible();
    await expect(
      graph.getByRole("button", { name: /Demo.stock_bound.*を選択/ }),
    ).not.toBeVisible();
    await page
      .getByRole("textbox", { name: "グラフの対象ファイル（1行に1つ）" })
      .fill("missing.lean");
    await page
      .getByRole("button", { name: "ソースを読み直してグラフを表示" })
      .click();
    await expect(
      page.getByRole("alert").filter({ hasText: "Source observation failed" }),
    ).toBeVisible();
    await expect(graph).not.toBeVisible();
    await page
      .getByRole("textbox", { name: "グラフの対象ファイル（1行に1つ）" })
      .fill("Spec.lean");
    await page
      .getByRole("button", { name: "ソースを読み直してグラフを表示" })
      .click();
    await expect(graph).toContainText("2宣言");
    await page.reload();
    await page.getByRole("tab", { name: "グラフ", exact: true }).click();
    await expect(graph).toContainText("2宣言");
    await expect(
      page.getByRole("textbox", { name: "グラフの対象ファイル（1行に1つ）" }),
    ).toHaveValue("Spec.lean");
    await expect(
      page.getByRole("button", { name: "ソースを読み直してグラフを表示" }),
    ).toBeEnabled();
    await page.setViewportSize({ width: 390, height: 844 });
    expect(
      (
        await new AxeBuilder({ page })
          .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
          .analyze()
      ).violations,
    ).toEqual([]);
  });
});
