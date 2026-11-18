import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import {
  createServer,
  request as httpRequest,
  type ServerResponse,
} from "node:http";
import { spawn, type ChildProcess } from "node:child_process";
import {
  mkdtemp,
  mkdir,
  readFile,
  readdir,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { once } from "node:events";
import { createHash } from "node:crypto";

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
  async function start(port = 0, extraEnv: Record<string, string> = {}) {
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
      {
        env: {
          ...process.env,
          XDG_DATA_HOME: resolve(root, "data"),
          ...extraEnv,
        },
      },
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

  test("warm browser assets survive repeated host restart with fresh session review", async ({
    page,
  }, testInfo) => {
    test.setTimeout(120_000);
    const failures: string[] = [];
    page.on("pageerror", (error) => failures.push(error.message));
    page.on("requestfailed", (request) => {
      const location = new URL(request.url());
      failures.push(`${location.pathname}: ${request.failure()?.errorText}`);
    });
    try {
      await page.goto(await start());
      for (let restart = 0; restart < 6; restart++) {
        const launch = new URL(url).searchParams.get("launch");
        expect(launch).toMatch(/^[0-9a-f]{32}$/);
        await expect(
          page.getByRole("tab", { name: "プレビュー", exact: true }),
        ).toBeVisible();
        const before = await nativeState(page);
        expect(before.authorized).toBe(false);
        await page
          .getByRole("tab", { name: "プレビュー", exact: true })
          .click();
        await expect(page.locator("iframe")).toHaveCount(0);
        const port = Number(new URL(url).port);
        await stop();
        await page.goto(await start(port));
        expect(new URL(url).searchParams.get("launch")).not.toBe(launch);
      }
      await expect(
        page.getByRole("tab", { name: "プレビュー", exact: true }),
      ).toBeVisible();
      expect((await nativeState(page)).authorized).toBe(false);
      expect(requests).toHaveLength(0);
    } finally {
      await testInfo.attach("restart-browser-failures", {
        body: JSON.stringify(failures),
        contentType: "application/json",
      });
    }
  });

  test("prepare real Atlas offline, extract elaborated cross-module scope, analyze it and recover from compilation failure", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    page.setDefaultTimeout(15_000);
    const domain =
      "def eligibility (balance stock : Nat) : Prop := balance ≥ 150 ∧ stock > 0\ntheorem support (b s : Nat) (h : eligibility b s) : eligibility b s := h\n";
    const app =
      "import Domain\n-- ORIGINAL_COMMENT_MUST_NOT_REACH_TRANSLATOR\nnamespace Store\ntheorem permitted (b s : Nat) (h : eligibility b s) : eligibility b s := support b s h\ndef price : Nat := 150\ndef invoice (quantity : Nat) : Nat := quantity * price\ntheorem unfinished : True := by sorry\nend Store\n";
    await writeFile(resolve(project, "Domain.lean"), domain);
    await writeFile(resolve(project, "App.lean"), app);
    await page.goto(await start());
    await page.getByRole("tab", { name: "グラフ", exact: true }).click();
    await expect(
      page.getByRole("button", { name: "依存抽出の準備", exact: true }),
    ).toBeEnabled();
    await page
      .getByRole("button", { name: "依存抽出の準備", exact: true })
      .click();
    await expect(
      page.getByText("Lean Atlas: オフライン抽出の準備済み", { exact: false }),
    ).toBeVisible({ timeout: 120_000 });
    expect(requests).toHaveLength(0);
    const paths = page.getByRole("textbox", {
      name: "グラフの対象ファイル（1行に1つ）",
    });
    await paths.fill("App.lean\nDomain.lean");
    const readyBox = await page
      .getByText("Lean Atlas: オフライン抽出の準備済み", { exact: false })
      .boundingBox();
    const labelBox = await page
      .locator("label")
      .filter({ hasText: "グラフの対象ファイル（1行に1つ）" })
      .boundingBox();
    expect(readyBox).not.toBeNull();
    expect(labelBox).not.toBeNull();
    expect(labelBox!.y).toBeGreaterThanOrEqual(readyBox!.y + readyBox!.height);
    const extract = page.getByRole("button", {
      name: "Leanでコンパイルして依存を抽出",
      exact: true,
    });
    await extract.click();
    const graph = page.getByRole("region", { name: "ローカルソースのグラフ" });
    await expect(graph).toContainText("Lean Atlasによるコンパイル後の依存", {
      timeout: 60_000,
    });
    const state = await nativeState(page);
    const atlas = state.source_snapshot.atlas;
    expect(atlas.graph.edges).toEqual(
      expect.arrayContaining([
        {
          source: "Store.permitted",
          target: "eligibility",
          kind: "theorem_type_to_definition",
        },
        {
          source: "Store.permitted",
          target: "support",
          kind: "theorem_value_to_theorem",
        },
        {
          source: "Store.invoice",
          target: "Store.price",
          kind: "definition_value_to_definition",
        },
      ]),
    );
    await graph.getByRole("combobox", { name: "依存関係の範囲" }).click();
    await page
      .getByRole("option", { name: "プロジェクト全体", exact: true })
      .click();
    await graph
      .getByRole("button", { name: /Store.unfinished.*を選択/ })
      .click();
    await expect(page.getByLabel("選択した宣言")).toContainText(
      "未完成の証明を検出",
    );
    await graph
      .getByRole("button", { name: /Store.permitted.*を選択/ })
      .click();
    await expect(page.getByLabel("選択した宣言")).toContainText("eligibility");
    await page
      .getByRole("button", { name: "App.lean:4 のソースを開く", exact: true })
      .click();
    await expect(
      page.getByRole("dialog").locator('[aria-current="location"]'),
    ).toContainText("theorem permitted");
    await page.getByRole("button", { name: "閉じる", exact: true }).click();
    await page
      .getByRole("button", { name: "接続先を確認", exact: true })
      .click();
    await page
      .getByRole("button", { name: "この送信先を承認", exact: true })
      .click();
    responses.push((response) =>
      reply(
        response,
        tool("record_informalization", {
          lemmaName: "Store.permitted",
          naturalLanguage: "Eligibility implies itself.",
          preconditions: "eligibility b s already holds",
          postcondition: "eligibility b s",
          scope: "natural balance and stock",
          strength: "trivial",
          confidence: 0.95,
        }),
      ),
    );
    responses.push((response) =>
      reply(
        response,
        tool("record_comparison", {
          lemmaName: "Store.permitted",
          match: false,
          discrepancy:
            "The theorem assumes eligibility instead of establishing it for all requested inputs.",
          weakeningType: "tautology",
          explanation: "It repeats its premise.",
        }),
      ),
    );
    await page
      .getByRole("textbox", { name: "比較する要件", exact: true })
      .fill(
        "ORIGINAL_SEPARATE_REQUIREMENT: all balances of at least 100 with positive stock permit purchase.",
      );
    await page
      .getByRole("button", { name: "この宣言の意味を解析", exact: true })
      .click();
    await expect(
      page.getByLabel("意味解析の結果", { exact: true }),
    ).toContainText("意味の差異候補");
    const first = JSON.stringify(requests[0]);
    const translationRequest = requests[0] as {
      messages: Array<{ content: string }>;
    };
    const context = JSON.parse(translationRequest.messages.at(-1)!.content);
    expect(context.files).toBeUndefined();
    expect(
      context.elaborated.declarations
        .map((node: { id: string }) => node.id)
        .sort(),
    ).toEqual(["Store.permitted", "eligibility"]);
    expect(first).toContain("eligibility");
    expect(first).not.toContain("ORIGINAL_COMMENT_MUST_NOT_REACH_TRANSLATOR");
    expect(first).not.toContain("ORIGINAL_SEPARATE_REQUIREMENT");
    expect(first).not.toContain("by sorry");
    expect((await nativeState(page)).project.verified).toBe(false);
    await page
      .getByLabel("意味解析の結果", { exact: true })
      .scrollIntoViewIfNeeded();
    await page.screenshot({
      path: "target/gui-checks/native-atlas-meaning.png",
      fullPage: true,
    });
    responses.push((response) =>
      reply(
        response,
        tool("write_file", {
          path: "App.lean",
          content: app + "invalid lean syntax }\n",
        }),
      ),
    );
    responses.push((response) =>
      reply(response, { role: "assistant", content: "ATLAS-SOURCE-CHANGED" }),
    );
    await page
      .getByRole("textbox", { name: "メッセージ", exact: true })
      .fill("Change the selected source to exercise compilation failure");
    await page.getByRole("button", { name: "送信", exact: true }).click();
    await expect(
      page.getByText("ATLAS-SOURCE-CHANGED", { exact: true }),
    ).toBeVisible();
    await extract.click();
    await expect(
      page
        .getByRole("alert")
        .filter({ hasText: "Lean source compilation failed" }),
    ).toBeVisible();
    await expect(graph).not.toBeVisible();
    await page.screenshot({
      path: "target/gui-checks/native-atlas-compile-failed.png",
      fullPage: true,
    });
    responses.push((response) =>
      reply(response, tool("write_file", { path: "App.lean", content: app })),
    );
    responses.push((response) =>
      reply(response, { role: "assistant", content: "ATLAS-SOURCE-REPAIRED" }),
    );
    await page
      .getByRole("textbox", { name: "メッセージ", exact: true })
      .fill(
        "Restore the valid Lean implementation and keep Domain.lean unchanged",
      );
    await page.getByRole("button", { name: "送信", exact: true }).click();
    await expect(
      page.getByText("ATLAS-SOURCE-REPAIRED", { exact: true }),
    ).toBeVisible();
    await extract.click();
    await expect(graph).toContainText("Lean Atlasによるコンパイル後の依存", {
      timeout: 60_000,
    });
    const port = Number(new URL(url).port);
    await stop();
    await page.goto(await start(port));
    await page.getByRole("tab", { name: "グラフ", exact: true }).click();
    await expect(
      page.getByText("Lean Atlas: オフライン抽出の準備済み", { exact: false }),
    ).toBeVisible();
    await expect(graph).not.toBeVisible();
    await paths.fill("App.lean\nDomain.lean");
    await extract.click();
    await expect(graph).toContainText("Lean Atlasによるコンパイル後の依存", {
      timeout: 60_000,
    });
    expect(await readFile(resolve(project, "Domain.lean"), "utf8")).toBe(
      domain,
    );
  });

  test("Atlas code stays offline with read-only sources and libraries, and corrupt resources recover through the GUI", async ({
    page,
  }) => {
    test.setTimeout(240_000);
    page.setDefaultTimeout(15_000);
    await writeFile(resolve(root, "host-private.txt"), "HOST-CANARY");
    await writeFile(resolve(project, "unselected.txt"), "UNSELECTED-CANARY");
    let connections = 0;
    const sink = createServer((_request, response) => {
      connections++;
      response.end();
    });
    sink.listen(0, "127.0.0.1");
    await once(sink, "listening");
    try {
      const probe = `import os, socket\nfrom pathlib import Path\nassert os.getenv('MEDEN_ATLAS_TEST_SECRET') is None\nassert not Path(${JSON.stringify(resolve(root, "host-private.txt"))}).exists()\nassert not Path('/workspace/unselected.txt').exists()\nfor path in ['/analysis/sources/SandboxProbe.lean','/analysis/library/MedenAtlas.lean']:\n try: Path(path).write_text('tamper')\n except OSError: pass\n else: raise AssertionError('analysis resource was writable')\ntry: socket.create_connection(('127.0.0.1',${(sink.address() as { port: number }).port}),timeout=0.3)\nexcept OSError: pass\nelse: raise AssertionError('host network was reachable')\nPath('/workspace/job-write').write_text('only private job')\nprint('ISOLATION-CHECKED')\n`;
      const source = `import Lean\n#eval do\n  let output ← IO.Process.output { cmd := "/tools/python3", args := #["-c", ${JSON.stringify(probe)}] }\n  if output.exitCode != 0 then throw <| IO.userError output.stderr\n  IO.println output.stdout\ndef checked : Nat := 1\n`;
      await writeFile(resolve(project, "SandboxProbe.lean"), source);
      await page.goto(
        await start(0, { MEDEN_ATLAS_TEST_SECRET: "SYNTHETIC-HOST-SECRET" }),
      );
      await page.getByRole("tab", { name: "グラフ", exact: true }).click();
      await page
        .getByRole("button", { name: "依存抽出の準備", exact: true })
        .click();
      await expect(
        page.getByText("Lean Atlas: オフライン抽出の準備済み", {
          exact: false,
        }),
      ).toBeVisible({ timeout: 120_000 });
      await page
        .getByRole("textbox", { name: "グラフの対象ファイル（1行に1つ）" })
        .fill("SandboxProbe.lean");
      const extract = page.getByRole("button", {
        name: "Leanでコンパイルして依存を抽出",
        exact: true,
      });
      const graph = page.getByRole("region", {
        name: "ローカルソースのグラフ",
      });
      await extract.click();
      await expect(graph).toContainText("1宣言", { timeout: 60_000 });
      expect(connections).toBe(0);
      expect(requests).toHaveLength(0);
      expect(
        await readFile(resolve(project, "SandboxProbe.lean"), "utf8"),
      ).toBe(source);
      await expect(readFile(resolve(project, "job-write"))).rejects.toThrow();
      const cache = resolve(root, "data/meden/atlas");
      const prepared = (await readdir(cache)).filter(
        (name) => !name.startsWith("."),
      );
      expect(prepared).toHaveLength(1);
      await writeFile(
        resolve(cache, prepared[0]!, "LeanAtlas/Deps/Core.olean"),
        "HOST-CORRUPTION",
      );
      await extract.click();
      await expect(
        page
          .getByRole("alert")
          .filter({ hasText: "Atlas compiled library changed" }),
      ).toBeVisible();
      await expect(graph).not.toBeVisible();
      await page
        .getByRole("button", { name: "依存抽出の準備", exact: true })
        .click();
      await expect(
        page.getByText("Lean Atlas: オフライン抽出の準備済み", {
          exact: false,
        }),
      ).toBeVisible({ timeout: 120_000 });
      await extract.click();
      await expect(graph).toContainText("1宣言", { timeout: 60_000 });
      await writeFile(
        resolve(project, "SandboxProbe.lean"),
        'import Lean\n#eval (throw (IO.userError "EXPECTED-PROBE-FAILURE") : IO Unit)\ndef checked : Nat := 1\n',
      );
      await extract.click();
      await expect(
        page.getByRole("alert").filter({ hasText: "EXPECTED-PROBE-FAILURE" }),
      ).toBeVisible();
      await expect(graph).not.toBeVisible();
      expect(connections).toBe(0);
      expect(requests).toHaveLength(0);
      await page.screenshot({
        path: "target/gui-checks/native-atlas-isolation.png",
        fullPage: true,
      });
    } finally {
      sink.closeAllConnections();
      await new Promise<void>((resolve) => sink.close(() => resolve()));
    }
  });

  async function nativeState(page: Page) {
    return page.evaluate(async () => {
      const response = await fetch("/api/state", {
        headers: {
          Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
        },
      });
      return response.json();
    });
  }
  async function hostCommand(page: Page, command: unknown) {
    return page.evaluate(async (command) => {
      const response = await fetch("/api/command", {
        method: "POST",
        headers: {
          Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify(command),
      });
      return response.status;
    }, command);
  }
  async function queueExports(page: Page, targets: string[]) {
    for (const target of targets)
      responses.push((response) =>
        reply(
          response,
          tool("propose_export", { path: "payload.bin", target }),
        ),
      );
    responses.push((response) =>
      reply(
        response,
        tool("write_file", {
          path: "payload.bin",
          content: "CHANGED-AFTER-PREPARATION",
        }),
      ),
    );
    responses.push((response) =>
      reply(response, { role: "assistant", content: "DELIVERIES-QUEUED" }),
    );
    await page.goto(await start());
    await page
      .getByRole("button", { name: "接続先を確認", exact: true })
      .click();
    await page.getByRole("button", { name: "この送信先を承認" }).click();
    await page
      .getByRole("textbox", { name: "メッセージ", exact: true })
      .fill(
        "Prepare the original payload for host review, then change the workspace file",
      );
    await page.getByRole("button", { name: "送信", exact: true }).click();
    await expect(
      page.getByText("DELIVERIES-QUEUED", { exact: true }),
    ).toBeVisible();
    await page.getByRole("tab", { name: /^配送/ }).click();
  }

  test("review frozen delivery bytes, reject another target, send once and restart without a queue", async ({
    page,
  }) => {
    test.setTimeout(60_000);
    const payload = Buffer.concat([
      Buffer.from([0, 255]),
      Buffer.from("FROZEN-PAYLOAD\n".repeat(170)),
    ]);
    await writeFile(resolve(project, "payload.bin"), payload);
    const received: Array<{ path: string; bytes: Buffer }> = [];
    const receiver = createServer(async (request, response) => {
      const chunks: Buffer[] = [];
      for await (const chunk of request) chunks.push(Buffer.from(chunk));
      received.push({ path: request.url!, bytes: Buffer.concat(chunks) });
      response.writeHead(201);
      response.end();
    });
    receiver.listen(0, "127.0.0.1");
    await once(receiver, "listening");
    const target = `http://127.0.0.1:${(receiver.address() as { port: number }).port}/approved?exact=1`;
    try {
      await queueExports(page, [
        target,
        target.replace("approved", "rejected"),
      ]);
      expect(received).toHaveLength(0);
      expect(await readFile(resolve(project, "payload.bin"), "utf8")).toBe(
        "CHANGED-AFTER-PREPARATION",
      );
      await page
        .getByRole("button", {
          name: "配送 #0 の内容と宛先を確認",
          exact: true,
        })
        .click();
      const dialog = page.getByRole("dialog");
      await expect(dialog).toContainText(target);
      await expect(dialog).toContainText(
        createHash("sha256").update(payload).digest("hex"),
      );
      await expect(dialog.getByLabel("送信する正確なバイト列")).toContainText(
        "00 ff 46 52",
      );
      await expect(
        dialog.getByRole("button", { name: "この宛先へ1回送信" }),
      ).toBeEnabled();
      const first = (await nativeState(page)).export_review;
      await page.screenshot({
        path: "target/gui-checks/native-delivery-review.png",
        fullPage: true,
      });
      await page.setViewportSize({ width: 390, height: 240 });
      await expect(
        dialog.getByRole("button", { name: "この宛先へ1回送信" }),
      ).toBeDisabled();
      await page.setViewportSize({ width: 390, height: 844 });
      await expect(
        dialog.getByRole("button", { name: "この宛先へ1回送信" }),
      ).toBeEnabled();
      await page.screenshot({
        path: "target/gui-checks/native-delivery-mobile.png",
        fullPage: true,
      });
      await page.setViewportSize({ width: 1440, height: 960 });
      await dialog.getByRole("button", { name: "次の内容" }).click();
      await expect(dialog).toContainText("バイト位置 2048");
      await expect(dialog).toContainText(target);
      await dialog.getByRole("button", { name: "この宛先へ1回送信" }).click();
      await expect(dialog).not.toBeVisible();
      await expect(
        page.getByRole("alert").filter({ hasText: "HTTP 201" }),
      ).toBeVisible();
      expect(received).toHaveLength(1);
      expect(received[0]!.bytes.equals(payload)).toBe(true);
      expect(received[0]!.path).toBe("/approved?exact=1");
      await hostCommand(page, {
        type: "send_export",
        reference: { session: first.summary.session, id: 0 },
        serial: first.serial,
      });
      await expect(
        page
          .getByRole("alert")
          .filter({ hasText: "Open the exact export review" }),
      ).toBeVisible();
      expect(received).toHaveLength(1);
      await page
        .getByRole("button", {
          name: "配送 #1 の内容と宛先を確認",
          exact: true,
        })
        .click();
      await dialog.getByRole("button", { name: "この配送を拒否" }).click();
      await expect(dialog).not.toBeVisible();
      await page
        .getByRole("button", {
          name: "配送 #1 の内容と宛先を確認",
          exact: true,
        })
        .click();
      await expect(
        dialog.getByRole("button", { name: "この宛先へ1回送信" }),
      ).toBeDisabled();
      await dialog.getByRole("button", { name: "閉じる", exact: true }).click();
      const port = Number(new URL(url).port);
      await stop();
      await page.goto(await start(port));
      await page.getByRole("tab", { name: /^配送/ }).click();
      await expect(
        page.getByText("配送の提案はありません。", { exact: false }),
      ).toBeVisible();
      expect(received).toHaveLength(1);
    } finally {
      receiver.closeAllConnections();
      await new Promise<void>((resolve) => receiver.close(() => resolve()));
    }
  });

  test("delivery refuses stale reviews, does not follow redirects and leaves uncertain attempts terminal", async ({
    page,
  }) => {
    test.setTimeout(60_000);
    await writeFile(resolve(project, "payload.bin"), Buffer.from([0, 255, 10]));
    const paths: string[] = [];
    const receiver = createServer(async (request, response) => {
      for await (const _chunk of request) {
      }
      paths.push(request.url!);
      if (request.url === "/uncertain") {
        request.socket.destroy();
        return;
      }
      response.writeHead(307, { Location: "/must-not-follow" });
      response.end();
    });
    receiver.listen(0, "127.0.0.1");
    await once(receiver, "listening");
    const target = `http://127.0.0.1:${(receiver.address() as { port: number }).port}`;
    try {
      await queueExports(page, [target + "/redirect", target + "/uncertain"]);
      await page
        .getByRole("button", {
          name: "配送 #0 の内容と宛先を確認",
          exact: true,
        })
        .click();
      const old = (await nativeState(page)).export_review;
      await hostCommand(page, {
        type: "review_export",
        reference: { session: old.summary.session, id: 1 },
        offset: 0,
      });
      await expect(page.getByRole("dialog")).toContainText(
        target + "/uncertain",
      );
      await hostCommand(page, {
        type: "send_export",
        reference: { session: old.summary.session, id: 0 },
        serial: old.serial,
      });
      await expect(page.getByRole("dialog")).not.toBeVisible();
      await expect(
        page
          .getByRole("alert")
          .filter({ hasText: "selected export review changed" }),
      ).toBeVisible();
      expect(paths).toEqual([]);
      await page
        .getByRole("button", {
          name: "配送 #0 の内容と宛先を確認",
          exact: true,
        })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", { name: "この宛先へ1回送信" })
        .click();
      await expect(
        page.getByRole("alert").filter({ hasText: "HTTP 307" }),
      ).toBeVisible();
      expect(paths).toEqual(["/redirect"]);
      await page
        .getByRole("button", {
          name: "配送 #1 の内容と宛先を確認",
          exact: true,
        })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", { name: "この宛先へ1回送信" })
        .click();
      await expect(
        page.getByRole("alert").filter({ hasText: "配送結果不明・再送不可" }),
      ).toBeVisible();
      expect(paths).toEqual(["/redirect", "/uncertain"]);
      await page
        .getByRole("button", {
          name: "配送 #1 の内容と宛先を確認",
          exact: true,
        })
        .click();
      await expect(
        page
          .getByRole("dialog")
          .getByRole("button", { name: "この宛先へ1回送信" }),
      ).toBeDisabled();
      await page.screenshot({
        path: "target/gui-checks/native-delivery-uncertain.png",
        fullPage: true,
      });
    } finally {
      receiver.closeAllConnections();
      await new Promise<void>((resolve) => receiver.close(() => resolve()));
    }
  });

  async function preview(page: Page, folder = ".", entry = "index.html") {
    await page.getByRole("tab", { name: "プレビュー", exact: true }).click();
    await page
      .getByRole("textbox", { name: "公開フォルダ（ワークスペース内）" })
      .fill(folder);
    await page
      .getByRole("textbox", { name: "開始するHTMLファイル" })
      .fill(entry);
    await page
      .getByRole("button", { name: "ファイルを読み直してプレビュー" })
      .click();
  }
  async function previewRequest(
    address: string,
    path: string,
    method = "GET",
    host?: string,
  ) {
    const parsed = new URL(address);
    return new Promise<{ status: number | undefined; body: string }>(
      (resolve, reject) => {
        const request = httpRequest(
          {
            hostname: "127.0.0.1",
            port: parsed.port,
            path,
            method,
            headers: { Host: host ?? parsed.host },
          },
          (response) => {
            let body = "";
            response.on("data", (chunk) => {
              body += String(chunk);
            });
            response.on("end", () =>
              resolve({ status: response.statusCode, body }),
            );
          },
        );
        request.on("error", reject);
        request.end();
      },
    );
  }

  test("develop and interact with local module assets, refresh edits, recover and restart", async ({
    page,
  }) => {
    test.setTimeout(60_000);
    await mkdir(resolve(project, "public"));
    const html =
      '<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Counter</title><link rel="stylesheet" href="/style.css"><script type="module" src="/counter.js"></script></head><body><h1>Local counter</h1><output aria-label="Count">0</output><form><button type="submit">Increment</button></form></body></html>';
    const script =
      'const {step} = await (await fetch("/settings.json")).json(); let count=0; document.querySelector("form").addEventListener("submit",event=>{event.preventDefault();document.querySelector("output").textContent=String(count+=step);});';
    for (const [path, content] of Object.entries({
      "public/index.html": html,
      "public/counter.js": script,
      "public/settings.json": '{"step":2}',
      "public/style.css":
        "body { background: rgb(232, 244, 255); color: #14202c; font: 18px sans-serif; padding: 20px; } button { padding: 12px; }",
    }))
      responses.push((response) =>
        reply(response, tool("write_file", { path, content })),
      );
    responses.push((response) =>
      reply(response, { role: "assistant", content: "COUNTER-BUILT" }),
    );
    responses.push((response) =>
      reply(
        response,
        tool("write_file", {
          path: "public/settings.json",
          content: '{"step":3}',
        }),
      ),
    );
    responses.push((response) =>
      reply(response, { role: "assistant", content: "COUNTER-UPDATED" }),
    );
    await page.goto(await start());
    await page
      .getByRole("button", { name: "接続先を確認", exact: true })
      .click();
    await page.getByRole("button", { name: "この送信先を承認" }).click();
    await page
      .getByRole("textbox", { name: "メッセージ", exact: true })
      .fill("Build a local counter with module assets");
    await page.getByRole("button", { name: "送信", exact: true }).click();
    await expect(
      page.getByText("COUNTER-BUILT", { exact: true }),
    ).toBeVisible();
    await preview(page, "public");
    const frame = page.frameLocator(
      'iframe[title="ローカルアプリのプレビュー"]',
    );
    await expect(
      frame.getByRole("heading", { name: "Local counter" }),
    ).toBeVisible();
    await expect(frame.locator("body")).toHaveCSS(
      "background-color",
      "rgb(232, 244, 255)",
    );
    await frame.getByRole("button", { name: "Increment" }).click();
    await expect(frame.getByLabel("Count")).toHaveText("2");
    const originalUrl = (await page.locator("iframe").getAttribute("src"))!;
    await page.getByRole("tab", { name: "コード", exact: true }).click();
    await page.getByRole("tab", { name: "プレビュー", exact: true }).click();
    await expect(frame.getByLabel("Count")).toHaveText("2");
    await page.screenshot({
      path: "target/gui-checks/native-preview.png",
      fullPage: true,
    });
    await page.setViewportSize({ width: 390, height: 844 });
    await page
      .getByRole("tab", { name: "プレビュー", exact: true })
      .scrollIntoViewIfNeeded();
    await expect(
      page.getByRole("button", { name: "ファイルを読み直してプレビュー" }),
    ).toBeVisible();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    await frame.getByRole("button", { name: "Increment" }).click();
    await expect(frame.getByLabel("Count")).toHaveText("4");
    await page.screenshot({
      path: "target/gui-checks/native-preview-mobile.png",
      fullPage: true,
    });
    await page.setViewportSize({ width: 1440, height: 960 });
    await page
      .getByRole("textbox", { name: "メッセージ", exact: true })
      .fill("Change the increment to three");
    await page.getByRole("button", { name: "送信", exact: true }).click();
    await expect(
      page.getByText("COUNTER-UPDATED", { exact: true }),
    ).toBeVisible();
    await expect(page.locator("iframe")).toHaveCount(0);
    expect((await previewRequest(originalUrl, "/index.html")).status).toBe(403);
    await preview(page, "public");
    await frame.getByRole("button", { name: "Increment" }).click();
    await expect(frame.getByLabel("Count")).toHaveText("3");
    await preview(page, "public", "missing.html");
    await expect(
      page.getByRole("alert").filter({ hasText: "preview entry is missing" }),
    ).toBeVisible();
    await expect(page.locator("iframe")).toHaveCount(0);
    await page.screenshot({
      path: "target/gui-checks/native-preview-failed.png",
      fullPage: true,
    });
    await preview(page, "public");
    await frame.getByRole("button", { name: "Increment" }).click();
    await expect(frame.getByLabel("Count")).toHaveText("3");
    await page.reload();
    await page.getByRole("tab", { name: "プレビュー", exact: true }).click();
    await expect(frame.getByLabel("Count")).toHaveText("0");
    const port = Number(new URL(url).port);
    await stop();
    await page.goto(await start(port));
    await page.getByRole("tab", { name: "プレビュー", exact: true }).click();
    await expect(page.locator("iframe")).toHaveCount(0);
    await preview(page, "public");
    await frame.getByRole("button", { name: "Increment" }).click();
    await expect(frame.getByLabel("Count")).toHaveText("3");
    expect(await readFile(resolve(project, "original-test.txt"), "utf8")).toBe(
      "keep this original test\n",
    );
    expect(requests).toHaveLength(7);
  });

  test("preview separates browser authority, rejects missing assets and bounds captured files", async ({
    page,
  }) => {
    test.setTimeout(60_000);
    let outbound = 0;
    const sink = createServer((_request, response) => {
      outbound++;
      response.writeHead(200, { "Access-Control-Allow-Origin": "*" });
      response.end("unapproved");
    });
    sink.listen(0, "127.0.0.1");
    await once(sink, "listening");
    try {
      const sinkUrl = `http://127.0.0.1:${(sink.address() as { port: number }).port}`;
      await page.goto(await start());
      const parentOrigin = new URL(url).origin;
      const script = `let results=[]; for(const read of [()=>parent.document.body,()=>parent.sessionStorage,()=>localStorage]){try{read();results.push("leaked")}catch{results.push("denied")}}; Promise.all([fetch(${JSON.stringify(sinkUrl + "/fetch")}).then(()=>"leaked",()=>"blocked"),fetch(${JSON.stringify(parentOrigin + "/api/state")}).then(()=>"leaked",()=>"blocked")]).then(values=>{document.querySelector("output").textContent=results.concat(values).join(",")}); const img=new Image(); img.src=${JSON.stringify(sinkUrl + "/image")}; const s=document.createElement("script");s.src=${JSON.stringify(sinkUrl + "/script")};document.body.append(s);`;
      await writeFile(
        resolve(project, "index.html"),
        `<!doctype html><html lang="en"><head><title>Boundaries</title><script type="module" src="/missing.js"></script></head><body><h1>Boundaries</h1><output aria-label="Boundary results">waiting</output><form action="${sinkUrl}/upload" method="post"><input name="secret" value="synthetic-private-data"><button>Attempt upload</button></form><script>${script}</script></body></html>`,
      );
      await preview(page);
      const frame = page.frameLocator(
        'iframe[title="ローカルアプリのプレビュー"]',
      );
      await expect(frame.getByLabel("Boundary results")).toHaveText(
        "denied,denied,denied,blocked,blocked",
      );
      const formBlocked = page.waitForEvent("console", {
        predicate: (message) => message.text().includes("form-action 'none'"),
      });
      await frame.getByRole("button", { name: "Attempt upload" }).click();
      await formBlocked;
      await expect(
        page
          .getByRole("alert")
          .filter({ hasText: "Missing preview asset: missing.js" }),
      ).toBeVisible();
      expect(outbound).toBe(0);
      expect(requests).toHaveLength(0);
      const previewUrl = (await page.locator("iframe").getAttribute("src"))!;
      expect((await previewRequest(previewUrl, "/api/state")).status).toBe(404);
      expect(
        (await previewRequest(previewUrl, "/index.html", "POST")).status,
      ).toBe(405);
      expect(
        (await previewRequest(previewUrl, "/index.html", "GET", "evil.invalid"))
          .status,
      ).toBe(403);
      expect(
        (await previewRequest(previewUrl, "/../original-test.txt")).status,
      ).toBe(404);
      const body = (await previewRequest(previewUrl, "/index.html")).body;
      expect(body).not.toContain(new URL(url).hash.slice(1));
      await preview(page, "../");
      await expect(
        page.getByRole("alert").filter({ hasText: "preview root leaves" }),
      ).toBeVisible();
      await expect(page.locator("iframe")).toHaveCount(0);
      await mkdir(resolve(project, "private"));
      await writeFile(
        resolve(project, "private/index.html"),
        "must not be served through alias",
      );
      await symlink("private", resolve(project, "public-alias"));
      await preview(page, "public-alias");
      await expect(
        page
          .getByRole("alert")
          .filter({ hasText: "Preview observation failed" }),
      ).toBeVisible();
      await expect(page.locator("iframe")).toHaveCount(0);
      await writeFile(resolve(project, "oversize.js"), Buffer.alloc(4_194_305));
      await preview(page);
      await expect(
        page
          .getByRole("alert")
          .filter({ hasText: "snapshot exceeds selected byte limit" }),
      ).toBeVisible();
      await expect(page.locator("iframe")).toHaveCount(0);
      expect((await previewRequest(previewUrl, "/index.html")).status).toBe(
        403,
      );
    } finally {
      sink.closeAllConnections();
      await new Promise<void>((resolve) => sink.close(() => resolve()));
    }
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
    let releaseCancel = () => {};
    const cancelResponse = new Promise<void>((resolve) => {
      releaseCancel = resolve;
    });
    let cancellationReachedHost = false;
    await page.route("**/api/command", async (route) => {
      if (route.request().postDataJSON()?.type !== "cancel")
        return route.continue();
      const response = await route.fetch();
      cancellationReachedHost = true;
      await cancelResponse;
      await route.fulfill({ response });
    });
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
    await expect.poll(() => cancellationReachedHost).toBe(true);
    await expect(
      page.getByRole("button", { name: "送信", exact: true }),
    ).toBeDisabled();
    releaseCancel();
    await expect(
      page.getByRole("button", { name: "送信", exact: true }),
    ).toBeEnabled();
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
  const formalSource =
    "namespace Demo\n-- FORMAL_CONTEXT_SENTINEL\ndef purchase (balance stock : Nat) := if balance ≥ 150 then stock - 1 else stock\ntheorem stock_bound (balance stock : Nat) : purchase balance stock ≤ stock := by sorry\nend Demo\n";
  const informalization = () =>
    tool("record_informalization", {
      lemmaName: "Demo.stock_bound",
      naturalLanguage:
        "Stock never increases on a single purchase; it may stay unchanged.",
      preconditions: "Natural-number balance and stock",
      postcondition: "The output stock is at most the input stock",
      scope: "A single transition, not every reachable execution",
      strength: "moderate",
      confidence: 0.9,
    });
  const judgement = (matches = false) =>
    tool("record_comparison", {
      lemmaName: "Demo.stock_bound",
      match: matches,
      discrepancy: matches
        ? ""
        : "The requirement asks for a decrement in every case, but the theorem allows unchanged stock.",
      weakeningType: matches ? "none" : "missing-case",
      explanation:
        "The all-cases decrement condition is not established by non-increase alone.",
    });
  async function openMeaning(page: Page) {
    await writeFile(resolve(project, "Spec.lean"), formalSource);
    await page.goto(await start());
    await page.getByRole("tab", { name: "グラフ", exact: true }).click();
    await page
      .getByRole("textbox", { name: "グラフの対象ファイル（1行に1つ）" })
      .fill("Spec.lean");
    await page
      .getByRole("button", { name: "ソースを読み直してグラフを表示" })
      .click();
    await page
      .getByRole("region", { name: "ローカルソースのグラフ" })
      .getByRole("button", { name: /Demo.stock_bound.*を選択/ })
      .click();
    await page
      .getByRole("textbox", { name: "比較する要件", exact: true })
      .fill(
        "ORIGINAL_REQUIREMENT_SENTINEL: A purchase always decrements stock regardless of balance.",
      );
    await expect(
      page.getByRole("button", { name: "この宣言の意味を解析" }),
    ).toBeDisabled();
    await page
      .getByRole("button", { name: "接続先を確認", exact: true })
      .click();
    await page.getByRole("button", { name: "この送信先を承認" }).click();
  }
  test("meaning analysis runs separated real requests, keeps proof unknown and restores a historical result", async ({
    page,
  }) => {
    responses.push(
      (response) => reply(response, informalization()),
      (response) => reply(response, judgement()),
      (response) => reply(response, informalization()),
      (response) => reply(response, judgement()),
    );
    await openMeaning(page);
    await page.getByRole("button", { name: "この宣言の意味を解析" }).click();
    const result = page.getByLabel("意味解析の結果", { exact: true });
    await expect(result).toContainText("意味の差異候補（モデル評価）");
    await expect(result).toContainText("Stock never increases");
    await expect(page.getByLabel("選択した宣言")).toContainText("証明は未確認");
    expect(requests).toHaveLength(2);
    expect(JSON.stringify(requests[0])).toContain("FORMAL_CONTEXT_SENTINEL");
    expect(JSON.stringify(requests[0])).not.toContain(
      "ORIGINAL_REQUIREMENT_SENTINEL",
    );
    expect(JSON.stringify(requests[1])).toContain(
      "ORIGINAL_REQUIREMENT_SENTINEL",
    );
    expect(JSON.stringify(requests[1])).not.toContain(
      "FORMAL_CONTEXT_SENTINEL",
    );
    await result.scrollIntoViewIfNeeded();
    await page.screenshot({
      path: "target/gui-checks/native-meaning-mismatch.png",
      fullPage: true,
    });
    const input = page.getByRole("textbox", {
      name: "比較する要件",
      exact: true,
    });
    const original = await input.inputValue();
    await input.fill("Changed requirement");
    await expect(result).toContainText(
      "表示中の要件・宣言・ソース版と異なる解析",
    );
    expect(requests).toHaveLength(2);
    const port = Number(new URL(url).port);
    await stop();
    await page.goto(await start(port));
    await page.getByRole("tab", { name: "グラフ", exact: true }).click();
    await expect(result).toContainText("保存された解析です");
    expect(requests).toHaveLength(2);
    await page
      .getByRole("textbox", { name: "グラフの対象ファイル（1行に1つ）" })
      .fill("Spec.lean");
    await page
      .getByRole("button", { name: "ソースを読み直してグラフを表示" })
      .click();
    await expect(input).toHaveValue(original);
    await expect(
      page.getByRole("button", { name: "この宣言の意味を解析" }),
    ).toBeDisabled();
    await page
      .getByRole("button", { name: "接続先を確認", exact: true })
      .click();
    await page.getByRole("button", { name: "この送信先を承認" }).click();
    await page.getByRole("button", { name: "この宣言の意味を解析" }).click();
    await expect(result).toContainText("意味の差異候補（モデル評価）");
    await expect(result).not.toContainText("保存された解析です");
    expect(requests).toHaveLength(4);
    await stop();
    const journals = resolve(root, "data/meden/projects");
    const [projectKey] = await readdir(journals);
    const checkpoint = resolve(journals, projectKey!, "meaning-analysis.json");
    await writeFile(checkpoint, "invalid analysis checkpoint");
    await page.goto(await start(port));
    await expect(
      page
        .getByRole("alert")
        .filter({ hasText: "Meaning-analysis checkpoint unavailable" }),
    ).toBeVisible();
    expect(await readFile(checkpoint, "utf8")).toBe(
      "invalid analysis checkpoint",
    );
    responses.push(
      (response) => reply(response, informalization()),
      (response) => reply(response, judgement()),
    );
    await page.getByRole("tab", { name: "グラフ", exact: true }).click();
    await page
      .getByRole("textbox", { name: "グラフの対象ファイル（1行に1つ）" })
      .fill("Spec.lean");
    await page
      .getByRole("button", { name: "ソースを読み直してグラフを表示" })
      .click();
    await page
      .getByRole("region", { name: "ローカルソースのグラフ" })
      .getByRole("button", { name: /Demo.stock_bound.*を選択/ })
      .click();
    await input.fill(original);
    await page
      .getByRole("button", { name: "接続先を確認", exact: true })
      .click();
    await page.getByRole("button", { name: "この送信先を承認" }).click();
    await page.getByRole("button", { name: "この宣言の意味を解析" }).click();
    await expect(result).toContainText("意味の差異候補（モデル評価）");
    expect(requests).toHaveLength(6);
    await page.setViewportSize({ width: 390, height: 844 });
    expect(
      (
        await new AxeBuilder({ page })
          .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
          .analyze()
      ).violations,
    ).toEqual([]);
  });
  for (const mode of [
    "cancel",
    "revoke",
    "stale",
    "provider",
    "invalid",
  ] as const) {
    test(`meaning analysis refuses a current result after ${mode}`, async ({
      page,
    }) => {
      let held: ServerResponse | undefined;
      if (mode === "cancel" || mode === "revoke")
        responses.push((response) => {
          held = response;
        });
      if (mode === "stale")
        responses.push(
          (response) => reply(response, informalization()),
          (response) => {
            held = response;
          },
        );
      if (mode === "provider")
        responses.push((response) => {
          response.writeHead(429);
          response.end();
        });
      if (mode === "invalid")
        responses.push((response) =>
          reply(
            response,
            tool("write_file", {
              path: "must-not-exist.txt",
              content: "not admitted",
            }),
          ),
        );
      await openMeaning(page);
      await page.getByRole("button", { name: "この宣言の意味を解析" }).click();
      const result = page.getByLabel("意味解析の結果", { exact: true });
      if (mode === "cancel" || mode === "revoke" || mode === "stale") {
        await expect.poll(() => held !== undefined).toBe(true);
        await expect(result).toContainText(
          mode === "cancel" || mode === "revoke"
            ? "自然言語化しています"
            : "元の要件と比較しています",
        );
        if (mode === "cancel")
          await page
            .getByRole("button", { name: "実行を中止", exact: true })
            .click();
        else if (mode === "revoke")
          await page
            .getByRole("button", { name: "送信の許可を取り消す", exact: true })
            .click();
        else
          await writeFile(
            resolve(project, "Spec.lean"),
            formalSource + "-- concurrent host change\n",
          );
        reply(
          held!,
          mode === "cancel" || mode === "revoke"
            ? informalization()
            : judgement(true),
        );
      }
      await expect(result).toContainText(
        mode === "cancel" || mode === "revoke"
          ? "意味解析を中止しました"
          : mode === "stale"
            ? "対象のソースが変更されました"
            : "意味解析を完了できませんでした",
      );
      await expect(result).not.toContainText("一致の候補");
      expect(requests).toHaveLength(mode === "stale" ? 2 : 1);
      if (mode === "revoke")
        await expect(
          page.getByRole("button", { name: "この宣言の意味を解析" }),
        ).toBeDisabled();
      if (mode === "invalid")
        await expect(
          readFile(resolve(project, "must-not-exist.txt")),
        ).rejects.toThrow();
      await result.scrollIntoViewIfNeeded();
      await page.screenshot({
        path: `target/gui-checks/native-meaning-${mode}.png`,
        fullPage: true,
      });
    });
  }
});
