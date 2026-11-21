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
import { homedir, tmpdir } from "node:os";
import { resolve } from "node:path";
import { once } from "node:events";
import { createHash } from "node:crypto";

test.describe("native GUI with real offline workers", () => {
  test.describe.configure({ mode: "parallel" });
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
  async function start(
    port = 0,
    extraEnv: Record<string, string | undefined> = {},
    extraArgs: string[] = [],
  ) {
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
        ...extraArgs,
      ],
      {
        env: {
          ...process.env,
          XDG_DATA_HOME: resolve(root, "data"),
          // Legacy fixtures have no project pin; their installed compiler is explicit.
          MEDEN_ATLAS_LEAN:
            process.env.MEDEN_ATLAS_LEAN ||
            resolve(homedir(), ".elan/toolchains/leanprover--lean4---v4.35.0"),
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
  async function openDependencyControls(page: Page) {
    if (
      !(await page
        .getByRole("region", { name: "Lean解析ツール", exact: true })
        .isVisible())
    )
      await page
        .getByRole("button", { name: "Lean解析ツール", exact: true })
        .click();
    const details = page
      .getByRole("region", { name: "Lean解析ツール", exact: true })
      .locator("details");
    if (!(await details.evaluate((e) => (e as HTMLDetailsElement).open)))
      await details.locator("summary").click();
  }
  async function setPaths(page: Page, value: string) {
    await openDependencyControls(page);
    await page
      .getByRole("textbox", { name: "グラフの対象ファイル（1行に1つ）" })
      .fill(value);
  }
  async function refreshReferences(page: Page) {
    await openDependencyControls(page);
    await page.getByRole("button", { name: "参照を更新", exact: true }).click();
  }
  async function compileDependencies(page: Page) {
    await openDependencyControls(page);
    const paths = await page
      .getByRole("textbox", { name: "グラフの対象ファイル（1行に1つ）" })
      .inputValue();
    await page
      .getByRole("button", {
        name: paths.trim()
          ? "コンパイル後の依存を抽出"
          : "Lakeターゲットを自動選択して表示",
        exact: true,
      })
      .click();
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
      if (request.url === "/v1/models") {
        response.writeHead(200, { "Content-Type": "application/json" });
        response.end(
          JSON.stringify({ data: [{ id: "scripted-gui-fixture" }] }),
        );
        return;
      }
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
        await expect(
          page.getByRole("button", { name: "アプリをプレビュー", exact: true }),
        ).toBeVisible();
        const before = await nativeState(page);
        expect(before.authorized).toBe(false);
        await page
          .getByRole("button", { name: "アプリをプレビュー", exact: true })
          .click();
        await expect(page.locator("iframe")).toHaveCount(0);
        const port = Number(new URL(url).port);
        await stop();
        await page.goto(await start(port));
      }
      await expect(
        page.getByRole("button", { name: "アプリをプレビュー", exact: true }),
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

  test("discover the project Lean without Atlas environment variables, recover after installation and extract in one action", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    const lean =
      process.env.MEDEN_ATLAS_LEAN ||
      resolve(homedir(), ".elan/toolchains/leanprover--lean4---v4.35.0");
    const elan = resolve(root, "elan");
    const installed = resolve(elan, "toolchains/leanprover--lean4---v4.35.0");
    await mkdir(resolve(elan, "toolchains"), { recursive: true });
    await writeFile(
      resolve(project, "lean-toolchain"),
      "leanprover/lean4:v4.35.0\n",
    );
    await writeFile(
      resolve(project, "Domain.lean"),
      "def price : Nat := 150\n",
    );
    await writeFile(
      resolve(project, "App.lean"),
      "import Domain\ndef invoice (n : Nat) := price * n\n",
    );
    const env = { MEDEN_ATLAS_LEAN: undefined, ELAN_HOME: elan };
    await page.goto(await start(0, env));
    await openDependencyControls(page);
    await expect(
      page.getByRole("button", {
        name: "コンパイル後の依存を抽出",
        exact: true,
      }),
    ).toBeDisabled();
    await expect(
      page.getByText("elan toolchain install leanprover/lean4:v4.35.0", {
        exact: false,
      }),
    ).toBeVisible();
    await page
      .getByRole("button", { name: "再検出", exact: true })
      .scrollIntoViewIfNeeded();
    await page.screenshot({
      path: "target/gui-checks/native-atlas-missing-installation.png",
    });
    // Installed resources are a test prerequisite, not a cold-download validation.
    await symlink(lean, installed, "dir");
    await page.getByRole("button", { name: "再検出", exact: true }).click();
    await expect(
      page.getByRole("button", { name: "依存抽出の準備", exact: true }),
    ).toBeEnabled();
    await setPaths(page, "App.lean");
    await compileDependencies(page);
    const graph = page.getByRole("region", { name: "ローカルソースのグラフ" });
    await expect(graph).toContainText("Lean Atlasによるコンパイル後の依存", {
      timeout: 120_000,
    });
    const first = await nativeState(page);
    expect(first.atlas.prepared).toBe(true);
    expect(Object.keys(first.source_snapshot.files).sort()).toEqual([
      "App.lean",
      "Domain.lean",
    ]);
    expect(first.source_snapshot.atlas.graph.edges).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ source: "invoice", target: "price" }),
      ]),
    );
    expect(requests).toHaveLength(0);
    await page.screenshot({
      path: "target/gui-checks/native-atlas-auto-discovered.png",
    });
    await stop();
    await page.goto(await start(0, env));
    await openDependencyControls(page);
    await expect(
      page.getByText("Lean Atlas: オフライン抽出の準備済み", { exact: false }),
    ).toBeVisible();
    expect((await nativeState(page)).authorized).toBe(false);
    expect(requests).toHaveLength(0);
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
    await openDependencyControls(page);
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
    await setPaths(page, "App.lean");
    const readyBox = await page
      .getByText("Lean Atlas: オフライン抽出の準備済み", { exact: false })
      .boundingBox();
    const labelBox = await page
      .locator("label")
      .filter({ hasText: "グラフの対象ファイル（1行に1つ）" })
      .boundingBox();
    expect(readyBox).not.toBeNull();
    expect(labelBox).not.toBeNull();
    expect(readyBox!.y).toBeGreaterThanOrEqual(labelBox!.y + labelBox!.height);
    await compileDependencies(page);
    const graph = page.getByRole("region", { name: "ローカルソースのグラフ" });
    await expect(graph).toContainText("Lean Atlasによるコンパイル後の依存", {
      timeout: 60_000,
    });
    const state = await nativeState(page);
    expect(Object.keys(state.source_snapshot.files).sort()).toEqual([
      "App.lean",
      "Domain.lean",
    ]);
    expect(state.source_snapshot.collection.roots).toEqual(["App.lean"]);
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
    await expect(
      page.locator(".ide-file-page:not([hidden]) [aria-current=location]"),
    ).toContainText("theorem permitted");
    await page
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    await page
      .getByRole("button", { name: "このモデルを使用", exact: true })
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
    await compileDependencies(page);
    await expect(
      page
        .getByRole("alert")
        .filter({ hasText: "Lean source compilation failed" }),
    ).toBeVisible({ timeout: 60_000 });
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
    await compileDependencies(page);
    await expect(graph).toContainText("Lean Atlasによるコンパイル後の依存", {
      timeout: 60_000,
    });
    const port = Number(new URL(url).port);
    await stop();
    await page.goto(await start(port));
    await openDependencyControls(page);
    await expect(
      page.getByText("Lean Atlas: オフライン抽出の準備済み", { exact: false }),
    ).toBeVisible();
    await expect(graph).not.toBeVisible();
    await setPaths(page, "App.lean\nDomain.lean");
    await compileDependencies(page);
    await expect(graph).toContainText("Lean Atlasによるコンパイル後の依存", {
      timeout: 60_000,
    });
    expect(await readFile(resolve(project, "Domain.lean"), "utf8")).toBe(
      domain,
    );
  });

  test("Lake discovers configured roots and dependency sources, follows their paths and recovers from a missing root", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    await writeFile(
      resolve(project, "lakefile.toml"),
      'name = "app"\nsrcDir = "src"\n[[lean_lib]]\nname = "App"\n[[require]]\nname = "dep"\npath = "vendor/dep"\n',
    );
    await mkdir(resolve(project, "src"), { recursive: true });
    await mkdir(resolve(project, "vendor/dep/source"), { recursive: true });
    await writeFile(
      resolve(project, "src/App.lean"),
      "import Domain\ndef app : Nat := Domain.answer\n",
    );
    await writeFile(
      resolve(project, "vendor/dep/lakefile.lean"),
      'import Lake\nopen Lake DSL\npackage dep where\n  srcDir := "source"\nlean_lib Domain\n',
    );
    await writeFile(
      resolve(project, "vendor/dep/source/Domain.lean"),
      "namespace Domain\ndef answer : Nat := 42\nend Domain\n",
    );
    await page.goto(await start());
    await openDependencyControls(page);
    await page
      .getByRole("button", { name: "依存抽出の準備", exact: true })
      .click();
    await expect(
      page.getByText("Lean Atlas: オフライン抽出の準備済み", { exact: false }),
    ).toBeVisible({ timeout: 120_000 });
    const extract = page.getByRole("button", {
      name: "Lakeターゲットを自動選択して表示",
      exact: true,
    });
    await setPaths(page, "");
    await expect(extract).toBeEnabled();
    await compileDependencies(page);
    const graph = page.getByRole("region", { name: "ローカルソースのグラフ" });
    await expect(graph).toContainText("Lake: app", { timeout: 60_000 });
    await expect(graph).toContainText("Lake: dep");
    const state = await nativeState(page);
    expect(Object.keys(state.source_snapshot.files).sort()).toEqual([
      "src/App.lean",
      "vendor/dep/source/Domain.lean",
    ]);
    await graph.getByRole("button", { name: /Domain.answer.*を選択/ }).click();
    await expect(
      page.locator(".ide-file-page:not([hidden]) [aria-current=location]"),
    ).toContainText("def answer");
    await setPaths(page, "missing.lean");
    await compileDependencies(page);
    await expect(
      page.getByRole("alert").filter({ hasText: "Source observation failed" }),
    ).toBeVisible({ timeout: 60_000 });
    expect((await nativeState(page)).source_snapshot).toBeNull();
    await setPaths(page, "");
    await compileDependencies(page);
    await expect(graph).toContainText("Lake: dep", { timeout: 60_000 });
    expect(requests).toHaveLength(0);
    await page.screenshot({
      path: "target/gui-checks/native-lake-project.png",
      fullPage: true,
    });
  });
  test("declaration kinds and highlighted source stay independent of chat until the user sends", async ({
    page,
  }) => {
    test.setTimeout(90000);
    await mkdir(resolve(project, ".meden"));
    await writeFile(
      resolve(project, "Spec.lean"),
      "namespace Demo\n-- comment for highlighting\ndef value : Nat := 1\nlemma helper : value = value := rfl\ntheorem result : value = value := helper\nstructure Container where\n  count : Nat\nend Demo\n",
    );
    await writeFile(
      resolve(project, ".meden/workspace.json"),
      JSON.stringify({
        version: 1,
        name: "Kinds",
        tasks: [
          {
            id: "main",
            title: "Identity",
            requirement: "Retain the original identity",
            module: "Spec",
            theorems: ["Demo.result"],
            next: "",
          },
        ],
        events: [],
        conversations: [],
      }),
    );
    await page.goto(await start());
    const map = page.getByRole("region", {
      name: "プロジェクトマップ",
      exact: true,
    });
    const main = map.getByRole("button", { name: /宣言: Demo.result ·/ });
    await expect(main).toHaveAttribute("data-declaration-role", "main_theorem");
    await expect(
      map.getByRole("button", { name: /宣言: Demo.helper ·/ }),
    ).toHaveAttribute("data-declaration-role", "lemma");
    await expect(
      map.getByRole("button", { name: /宣言: Demo.value ·/ }),
    ).toHaveAttribute("data-declaration-role", "definition");
    await expect(
      map.getByRole("button", { name: /宣言: Demo.Container ·/ }),
    ).toHaveAttribute("data-declaration-role", "structure");
    await page
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    await page
      .getByRole("button", { name: "このモデルを使用", exact: true })
      .click();
    await expect(page.getByRole("dialog")).not.toBeVisible();
    const chat = page.getByRole("complementary", {
      name: "作業チャット",
      exact: true,
    });
    const input = chat.getByRole("textbox", {
      name: "メッセージ",
      exact: true,
    });
    const send = chat.getByRole("button", { name: "送信", exact: true });
    await input.fill("Only a chat send may change this conversation");
    await expect(send).toBeEnabled();
    await page.getByRole("heading", { name: "Meden", exact: true }).click();
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    await page.waitForTimeout(400);
    let release = () => {};
    const released = new Promise<void>((resolve) => (release = resolve));
    let reading = false;
    await page.route("**/api/command", async (route) => {
      const body = route.request().postDataJSON();
      if (
        body.type !== "inspect" ||
        body.directory ||
        body.path !== "Spec.lean"
      )
        return route.continue();
      const response = await route.fetch();
      reading = true;
      await released;
      await route.fulfill({ response });
    });
    await chat.evaluate((element) => {
      (window as any).__chatChanges = [];
      new MutationObserver((records) =>
        (window as any).__chatChanges.push(
          ...records.map((r) => r.type + ":" + (r.target as Element).nodeName),
        ),
      ).observe(element, {
        attributes: true,
        childList: true,
        subtree: true,
        characterData: true,
      });
      (window as any).__chatScroll =
        element.querySelector(".transcript")?.scrollTop;
    });
    try {
      await main.click();
      await expect.poll(() => reading).toBe(true);
      const source = page
        .locator(".ide-file-page:not([hidden])")
        .getByLabel("ファイルの内容");
      await expect(
        source.locator(".syntax-token").filter({ hasText: /^theorem$/ }),
      ).toHaveCount(1);
      const color = await source
        .locator(".syntax-token")
        .filter({ hasText: /^theorem$/ })
        .evaluate((e) => getComputedStyle(e).color);
      expect(color).not.toBe(
        await source.evaluate((e) => getComputedStyle(e).color),
      );
      await expect(page.locator('[aria-current="location"]')).toContainText(
        "theorem result",
      );
      await expect(
        page.getByRole("combobox", { name: "中心の宣言" }),
      ).toHaveCount(0);
      await expect(
        page.getByRole("combobox", { name: "依存関係の範囲" }),
      ).toHaveCount(0);
      await expect(
        page.getByRole("button", { name: "Leanで解析", exact: true }),
      ).toHaveCount(0);
      await expect(send).toBeEnabled();
      await expect(input).toHaveValue(
        "Only a chat send may change this conversation",
      );
      expect(await page.evaluate(() => (window as any).__chatChanges)).toEqual(
        [],
      );
      expect(
        await chat.locator(".transcript").evaluate((e) => e.scrollTop),
      ).toBe(await page.evaluate(() => (window as any).__chatScroll));
      expect(requests).toHaveLength(0);
      responses.push((response) =>
        reply(response, {
          role: "assistant",
          content: "ONLY-CHAT-SEND-REACTS",
        }),
      );
      await send.click();
      await expect(chat.getByRole("status")).toContainText(
        "送信を待っています",
      );
      expect(requests).toHaveLength(0);
      release();
      await expect(
        chat.getByText("ONLY-CHAT-SEND-REACTS", { exact: true }),
      ).toBeVisible();
      expect(requests).toHaveLength(1);
      await page.screenshot({
        path: "target/gui-checks/quiet-chat-highlighted-source.png",
      });
    } finally {
      release();
    }
  });

  test("IDE map keeps one camera while revealing cached declarations, opens large source files and selects models from chat", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    page.setDefaultTimeout(15_000);
    await mkdir(resolve(project, "Networking"));
    await mkdir(resolve(project, "Other"));
    await writeFile(
      resolve(project, "Networking/Domain.lean"),
      "def price : Nat := 150\n",
    );
    const largeSource =
      "import Networking.Domain\n" +
      "-- a long original source comment with no declarations\n".repeat(4800) +
      "def invoice (n : Nat) := price * n\n";
    await writeFile(
      resolve(project, "Networking/Multistream.lean"),
      largeSource,
    );
    await writeFile(
      resolve(project, "Other/Helper.lean"),
      "def helper : Nat := 1\n",
    );
    const commands: string[] = [];
    page.on("request", (request) => {
      if (request.url().endsWith("/api/command"))
        commands.push(request.postDataJSON()?.type);
    });
    await page.goto(await start());
    const map = page.getByRole("region", {
      name: "プロジェクトマップ",
      exact: true,
      includeHidden: true,
    });
    await expect(
      map.getByRole("button", { name: "カテゴリ: Networking", exact: true }),
    ).toBeVisible();
    const header = page.getByRole("banner");
    await expect(header).not.toContainText("ローカルワークスペース");
    await expect(header.getByRole("alert")).toHaveCount(0);
    await expect(
      header.getByRole("button", { name: "接続先", exact: true }),
    ).toHaveCount(0);
    const chat = page.getByRole("complementary", {
      name: "作業チャット",
      exact: true,
    });
    await expect(
      chat
        .locator(".composer")
        .getByRole("button", { name: "モデルを選択", exact: true }),
    ).toBeVisible();
    const treeBox = await page
      .getByRole("complementary", { name: "ファイル", exact: true })
      .boundingBox();
    const mapBox = await map.boundingBox(),
      chatBox = await chat.boundingBox();
    expect(treeBox!.x + treeBox!.width).toBeLessThanOrEqual(mapBox!.x + 1);
    expect(mapBox!.x + mapBox!.width).toBeLessThanOrEqual(chatBox!.x + 1);
    expect(mapBox!.width).toBeGreaterThan(600);
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    await page
      .getByRole("tab", { name: "プロジェクトマップ", exact: true })
      .click();
    const beforeCommands = commands.length;
    await map.locator(".continuous-map-scene").evaluate((element) => {
      (window as any).__originalMap = element;
    });
    const category = map.getByRole("button", {
      name: "カテゴリ: Networking",
      exact: true,
    });
    const originalWorld = await category.getAttribute("data-world-x");
    const box = await category.boundingBox();
    await page.mouse.move(
      box!.x + box!.width * 0.5,
      box!.y + box!.height * 0.5,
    );
    await page.mouse.wheel(0, -700);
    const file = map.getByRole("button", {
      name: "依存関係: Networking/Multistream.lean",
      exact: true,
    });
    await expect(file).toBeVisible();
    await expect(category).toHaveAttribute("data-world-x", originalWorld!);
    expect(
      await map
        .locator(".continuous-map-scene")
        .evaluate((element) => element === (window as any).__originalMap),
    ).toBe(true);
    await file.click();
    const fileBox = await file.boundingBox();
    await page.mouse.move(
      fileBox!.x + fileBox!.width * 0.5,
      fileBox!.y + fileBox!.height * 0.5,
    );
    await page.mouse.wheel(0, -500);
    const invoice = map.getByRole("button", {
      name: /宣言: invoice · Networking\/Multistream.lean:/,
    });
    await expect(invoice).toBeVisible();
    expect(commands.slice(beforeCommands)).toEqual([]);
    expect((await nativeState(page)).source_snapshot).toBeNull();
    await page.screenshot({
      path: "target/gui-checks/ide-continuous-graph.png",
    });
    const beforeSelection = await map
      .locator(".react-flow__viewport")
      .getAttribute("style");
    await invoice.click();
    await expect(map).not.toBeVisible();
    await expect(map.locator(".react-flow__viewport")).toHaveAttribute(
      "style",
      beforeSelection!,
    );
    await expect(
      page.getByRole("tab", { name: "Multistream.lean", exact: true }),
    ).toHaveAttribute("aria-selected", "true");
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("ファイルの内容"),
    ).toContainText("def invoice");
    await expect(page.locator('[aria-current="location"]')).toContainText(
      "def invoice",
    );
    const references = page.getByRole("region", {
      name: "ファイルの参照グラフ",
      exact: true,
    });
    await expect(references).toBeVisible();
    const sourceBox = await page
      .getByRole("region", { name: "ソースコード", exact: true })
      .boundingBox();
    const graphBox = await references.boundingBox();
    expect(sourceBox!.x + sourceBox!.width).toBeLessThanOrEqual(
      graphBox!.x + 1,
    );
    await expect(chat.getByRole("status")).toHaveCount(0);
    await expect(page.getByRole("tablist", { name: "表示モード" })).toHaveCount(
      0,
    );
    await expect(
      page.getByRole("link", { name: "React Flow attribution" }),
    ).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "プロジェクト全体", exact: true }),
    ).toHaveCount(0);
    const close = page.getByTitle("閉じる: Networking/Multistream.lean", {
      exact: true,
    });
    expect(
      await close.evaluate(
        (e) => getComputedStyle(e.parentElement!).backgroundColor,
      ),
    ).toBe("rgb(255, 255, 255)");
    await page.screenshot({ path: "target/gui-checks/file-split-source.png" });
    await page
      .getByRole("tab", { name: "プロジェクトマップ", exact: true })
      .click();
    await expect(map).toBeVisible();
    await expect(map.locator(".react-flow__viewport")).toHaveAttribute(
      "style",
      beforeSelection!,
    );
    expect(
      await map
        .locator(".continuous-map-scene")
        .evaluate((e) => e === (window as any).__originalMap),
    ).toBe(true);
    await page
      .getByRole("tab", { name: "Multistream.lean", exact: true })
      .click();
    await compileDependencies(page);
    const compiled = page.getByRole("region", {
      name: "ローカルソースのグラフ",
      exact: true,
    });
    await expect(compiled).toBeVisible({ timeout: 120000 });
    expect(
      (await nativeState(page)).source_snapshot.files[
        "Networking/Multistream.lean"
      ],
    ).toBe(largeSource);
    expect((await nativeState(page)).source_snapshot.atlas.graph.edges).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ source: "invoice", target: "price" }),
      ]),
    );
    await expect(chat.getByRole("alert")).toHaveCount(0);
    await page
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    for (const name of [
      "モデル一覧を更新",
      "この接続を選ぶ",
      "このPCの接続設定を読み直す",
    ])
      await expect(page.getByRole("button", { name, exact: true })).toHaveCount(
        0,
      );
    await expect(
      page.getByRole("button", { name: "このモデルを使用", exact: true }),
    ).toBeVisible();
    await expect(page.getByRole("dialog")).not.toContainText(
      "作業ディレクトリ:",
    );
    await page.getByRole("button", { name: "閉じる", exact: true }).click();
    await page
      .getByRole("tab", { name: "プロジェクトマップ", exact: true })
      .click();
    await page.setViewportSize({ width: 390, height: 844 });
    await page.getByRole("button", { name: "チャット", exact: true }).click();
    await expect(
      chat
        .locator(".composer")
        .getByRole("button", { name: "モデルを選択", exact: true }),
    ).toBeInViewport();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    await page.screenshot({ path: "target/gui-checks/file-split-mobile.png" });
    expect(requests).toHaveLength(0);
    await stop();
    await page.setViewportSize({ width: 1440, height: 960 });
    await page.goto(await start());
    await expect(map).toBeVisible();
    expect((await nativeState(page)).authorized).toBe(false);
  });

  test("opening a file never compiles its import closure and an explicit analysis failure stays beside its readable source", async ({
    page,
  }) => {
    test.setTimeout(90000);
    for (let i = 0; i < 33; i++)
      await writeFile(
        resolve(project, "Part" + i + ".lean"),
        "def part" + i + " : Nat := " + i + "\n",
      );
    await writeFile(
      resolve(project, "Main.lean"),
      Array.from({ length: 33 }, (_, i) => "import Part" + i).join("\n") +
        "\ndef original : Nat := 1\n",
    );
    await page.goto(await start());
    const chat = page.getByRole("complementary", {
      name: "作業チャット",
      exact: true,
    });
    await page
      .getByRole("button", { name: "ファイル: Main.lean", exact: true })
      .click();
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("ファイルの内容"),
    ).toContainText("def original");
    await expect(
      page.getByRole("region", { name: "ファイルの参照グラフ", exact: true }),
    ).toBeVisible();
    expect((await nativeState(page)).source_snapshot).toBeNull();
    await expect(chat.getByRole("status")).toHaveCount(0);
    await compileDependencies(page);
    const pane = page.getByRole("region", {
      name: "ファイルの参照グラフ",
      exact: true,
    });
    await expect(pane.getByRole("alert")).toContainText(
      "コンパイル後の依存を抽出できませんでした",
      { timeout: 60000 },
    );
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("ファイルの内容"),
    ).toContainText("def original");
    await expect(chat.getByRole("alert")).toHaveCount(0);
    await expect(chat.getByRole("status")).toHaveCount(0);
    expect((await nativeState(page)).source_snapshot).toBeNull();
    await page.screenshot({
      path: "target/gui-checks/file-split-analysis-failure.png",
    });
  });

  test("missing Mathlib explains preparation and an existing-file picker recovers through source inspection without fetching", async ({
    page,
  }) => {
    test.setTimeout(180_000);
    await writeFile(
      resolve(project, "lakefile.toml"),
      'name = "app"\n[[lean_lib]]\nname = "App"\n[[require]]\nname = "mathlib"\npath = "vendor/mathlib"\n',
    );
    await writeFile(
      resolve(project, "App.lean"),
      "import Shared\ndef app : Nat := shared\n",
    );
    await page.goto(await start());
    await openDependencyControls(page);
    const picker = page.getByRole("combobox", {
      name: "ファイル",
      exact: true,
    });
    await openDependencyControls(page);
    await openDependencyControls(page);
    await picker.fill("App.lean");
    await page.getByRole("option", { name: "App.lean", exact: true }).click();
    await expect(
      page.getByRole("textbox", { name: "グラフの対象ファイル（1行に1つ）" }),
    ).toHaveValue("App.lean");
    await compileDependencies(page);
    const missing = page
      .getByRole("alert")
      .filter({ hasText: "Lake dependency mathlib" });
    await expect(missing).toBeVisible({ timeout: 120_000 });
    await expect(missing).toHaveCount(1);
    await expect(missing).toContainText("vendor/mathlib");
    await expect(missing).toContainText("lake exe cache get");
    expect((await nativeState(page)).source_snapshot).toBeNull();

    await missing.locator("summary").click();
    await missing.scrollIntoViewIfNeeded();
    await page.screenshot({
      path: "target/gui-checks/native-lake-missing-dependency-help.png",
    });
    await setPaths(page, "missing.lean");
    const source = page.getByRole("button", {
      name: "参照を更新",
      exact: true,
    });
    await source.click();
    await expect(
      page
        .getByRole("alert")
        .filter({ hasText: "Source observation failed for missing.lean" }),
    ).toBeVisible();
    await openDependencyControls(page);
    await picker.fill("App.lean");
    await page.getByRole("option", { name: "App.lean", exact: true }).click();
    await source.click();
    const graph = page.getByRole("region", { name: "ローカルソースのグラフ" });
    await expect(graph).toContainText("app");
    let state = await nativeState(page);
    expect(state.source_snapshot.atlas).toBeUndefined();
    expect(state.project.verified).toBe(false);
    await expect(
      readFile(resolve(project, "vendor/mathlib/lakefile.lean")),
    ).rejects.toThrow();
    expect(requests).toHaveLength(0);
    await stop();
    await page.goto(await start());
    await openDependencyControls(page);
    state = await nativeState(page);
    expect(state.authorized).toBe(false);
    expect(state.source_snapshot).toBeNull();
    await openDependencyControls(page);
    await picker.fill("App.lean");
    await page.getByRole("option", { name: "App.lean", exact: true }).click();
    await source.click();
    await expect(graph).toContainText("app");
    expect(requests).toHaveLength(0);
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
      await openDependencyControls(page);
      await openDependencyControls(page);
      await openDependencyControls(page);
      await openDependencyControls(page);
      await page
        .getByRole("button", { name: "依存抽出の準備", exact: true })
        .click();
      await expect(
        page.getByText("Lean Atlas: オフライン抽出の準備済み", {
          exact: false,
        }),
      ).toBeVisible({ timeout: 120_000 });
      await setPaths(page, "SandboxProbe.lean");
      const graph = page.getByRole("region", {
        name: "ローカルソースのグラフ",
      });
      await compileDependencies(page);
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
      await compileDependencies(page);
      await expect(
        page
          .getByRole("alert")
          .filter({ hasText: "Atlas compiled library changed" }),
      ).toBeVisible();
      await expect(graph).not.toBeVisible();
      await openDependencyControls(page);
      await page
        .getByRole("button", { name: "依存抽出の準備", exact: true })
        .click();
      await expect(
        page.getByText("Lean Atlas: オフライン抽出の準備済み", {
          exact: false,
        }),
      ).toBeVisible({ timeout: 120_000 });
      await compileDependencies(page);
      await expect(graph).toContainText("1宣言", { timeout: 60_000 });
      await writeFile(
        resolve(project, "SandboxProbe.lean"),
        'import Lean\n#eval (throw (IO.userError "EXPECTED-PROBE-FAILURE") : IO Unit)\ndef checked : Nat := 1\n',
      );
      await compileDependencies(page);
      await expect(
        page.getByRole("alert").filter({ hasText: "EXPECTED-PROBE-FAILURE" }),
      ).toBeVisible({ timeout: 60_000 });
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
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    await page.getByRole("button", { name: "このモデルを使用" }).click();
    await page
      .getByRole("textbox", { name: "メッセージ", exact: true })
      .fill(
        "Prepare the original payload for host review, then change the workspace file",
      );
    await page.getByRole("button", { name: "送信", exact: true }).click();
    await expect(
      page.getByText("DELIVERIES-QUEUED", { exact: true }),
    ).toBeVisible();
    await page.getByRole("button", { name: "送信提案を確認" }).click();
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
      await expect(
        page
          .getByRole("tablist", { name: "開いているファイル" })
          .getByRole("tab"),
      ).toHaveText(["プロジェクトマップ"]);
      await expect(
        page.getByRole("button", { name: "送信提案を確認" }),
      ).toHaveCount(0);
      expect((await nativeState(page)).exports).toEqual([]);
      expect((await nativeState(page)).authorized).toBe(false);
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
    await page
      .getByRole("button", { name: "アプリをプレビュー", exact: true })
      .click();
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
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    await page.getByRole("button", { name: "このモデルを使用" }).click();
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
    await page
      .getByRole("tab", { name: "プロジェクトマップ", exact: true })
      .click();
    await page
      .getByRole("button", { name: "アプリをプレビュー", exact: true })
      .click();
    await expect(frame.getByLabel("Count")).toHaveText("2");
    await page.screenshot({
      path: "target/gui-checks/native-preview.png",
      fullPage: true,
    });
    await page.setViewportSize({ width: 390, height: 844 });
    await page
      .getByRole("button", { name: "アプリをプレビュー", exact: true })
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
    await page
      .getByRole("button", { name: "アプリをプレビュー", exact: true })
      .click();
    await expect(frame.getByLabel("Count")).toHaveText("0");
    const port = Number(new URL(url).port);
    await stop();
    await page.goto(await start(port));
    await page
      .getByRole("button", { name: "アプリをプレビュー", exact: true })
      .click();
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
      // Let the actual missing-resource warning settle before interacting with the
      // frame below it; its appearance can move the form during pointer input.
      await expect(
        page
          .getByRole("alert")
          .filter({ hasText: "Missing preview asset: missing.js" }),
      ).toBeVisible();
      const formBlocked = page.waitForEvent("console", {
        predicate: (message) => message.text().includes("form-action 'none'"),
      });
      await frame.getByRole("button", { name: "Attempt upload" }).click();
      await formBlocked;
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
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    await expect(
      page.getByRole("combobox", { name: "model", exact: true }),
    ).toHaveValue("scripted-gui-fixture");
    await page.getByRole("button", { name: "このモデルを使用" }).click();
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
      .getByRole("button", { name: "ファイル: result.txt", exact: true })
      .click();
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("ファイルの内容"),
    ).toContainText("built by native worker");
    await expect(
      page.getByRole("alert").filter({ hasText: "429" }),
    ).toBeVisible();
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
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    await page.getByRole("button", { name: "このモデルを使用" }).click();
    await page.getByRole("button", { name: "送信", exact: true }).click();
    await expect(
      page.getByText("RESTART-CONTEXT-RETAINED", { exact: true }),
    ).toBeVisible();
    expect(JSON.stringify(requests[2])).toContain("built by native worker");
    expect(JSON.stringify(requests[2])).toContain("Build the fixture");
    await page
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    await page.getByRole("button", { name: "送信の許可を取り消す" }).click();
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "閉じる", exact: true })
      .click();
    await expect(
      page.getByRole("textbox", { name: "メッセージ", exact: true }),
    ).toBeDisabled();
    await page
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    await expect(
      page.getByRole("button", { name: "このモデルを使用" }),
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
            connection_revision: (await nativeState(page)).connection_revision,
            endpoint: "https://evil.invalid",
            model: (await nativeState(page)).model,
          }),
        })
      ).status,
    ).toBe(409);
    await page
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    await page.getByRole("button", { name: "このモデルを使用" }).click();
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
      page.getByRole("tab", { name: "プロジェクトマップ", exact: true }),
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
    await openDependencyControls(page);
    await setPaths(page, "Spec.lean");
    await refreshReferences(page);
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
    await expect(
      page.locator(".ide-file-page:not([hidden]) [aria-current=location]"),
    ).toContainText("theorem stock_bound");
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("ファイルの内容"),
    ).toContainText("sorry");
    await page.screenshot({
      path: "target/gui-checks/native-source-graph.png",
      fullPage: true,
    });
    await page
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    await page.getByRole("button", { name: "このモデルを使用" }).click();
    await page
      .getByRole("textbox", { name: "メッセージ", exact: true })
      .fill("Rename the property to inventory_bound");
    await page.getByRole("button", { name: "送信", exact: true }).click();
    await expect(
      page.getByText("SOURCE-UPDATED", { exact: true }),
    ).toBeVisible();
    await expect(graph).not.toBeVisible();
    await refreshReferences(page);
    await expect(
      graph.getByRole("button", { name: /Demo.inventory_bound.*を選択/ }),
    ).toBeVisible();
    await expect(
      graph.getByRole("button", { name: /Demo.stock_bound.*を選択/ }),
    ).not.toBeVisible();
    await setPaths(page, "missing.lean");
    await refreshReferences(page);
    await expect(
      page.getByRole("alert").filter({ hasText: "Source observation failed" }),
    ).toBeVisible();
    await expect(graph).not.toBeVisible();
    await setPaths(page, "Spec.lean");
    await refreshReferences(page);
    await expect(graph).toContainText("2宣言");
    await page.reload();
    await openDependencyControls(page);
    await expect(
      page.getByRole("textbox", { name: "グラフの対象ファイル（1行に1つ）" }),
    ).toHaveValue("Spec.lean");
    await expect(
      page.getByRole("button", { name: "参照を更新" }),
    ).toBeEnabled();
    await page.getByRole("button", { name: "閉じる", exact: true }).click();
    await expect(graph).toContainText("2宣言");
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
  async function openMeaning(
    page: Page,
    extraArgs: string[] = [],
    beforeApproval?: () => Promise<void>,
  ) {
    await writeFile(resolve(project, "Spec.lean"), formalSource);
    await page.goto(await start(0, {}, extraArgs));
    await openDependencyControls(page);
    await setPaths(page, "Spec.lean");
    await refreshReferences(page);
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
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    if (beforeApproval) await beforeApproval();
    await page.getByRole("button", { name: "このモデルを使用" }).click();
  }
  const stockSource = "pub fn stock(n: u64) -> u64 { n }\n";
  const stockRequirement = "Decrement positive stock and keep zero at zero.";
  const stockTranslation = (fixed = false) =>
    tool("record_informalization", {
      lemmaName: "stock",
      naturalLanguage: fixed
        ? "Positive stock decreases; zero remains zero."
        : "Stock remains unchanged.",
      preconditions: "Any u64 stock",
      postcondition: fixed ? "Saturating decrement" : "Output equals input",
      scope: "One transition",
      strength: "moderate",
      confidence: 0.9,
    });
  const stockComparison = (fixed = false) =>
    tool("record_comparison", {
      lemmaName: "stock",
      match: fixed,
      discrepancy: fixed ? "" : "No decrement is established.",
      weakeningType: fixed ? "none" : "wrong-property",
      explanation: fixed
        ? "The positive and zero cases agree."
        : "Unchanged stock does not satisfy the requested decrement.",
    });
  const stockRepair = () =>
    tool("propose_repair", {
      file: "stock.rs",
      old_text: "{ n }",
      new_text: "{ n.saturating_sub(1) }",
      explanation: "Decrement positive stock and retain zero.",
    });
  async function openStockFinding(page: Page) {
    await writeFile(resolve(project, "stock.rs"), stockSource);
    await page.goto(await start());
    await openDependencyControls(page);
    await setPaths(page, "stock.rs");
    await refreshReferences(page);
    await page
      .getByRole("region", { name: "ローカルソースのグラフ" })
      .getByRole("button", { name: /stock.*を選択/ })
      .click();
    await page
      .getByRole("textbox", { name: "比較する要件", exact: true })
      .fill(stockRequirement);
    await page
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    await page.getByRole("button", { name: "このモデルを使用" }).click();
    await page.getByRole("button", { name: "この宣言の意味を解析" }).click();
    await expect(
      page.getByLabel("意味解析の結果", { exact: true }),
    ).toContainText("意味の差異候補（モデル評価）");
  }
  test("review a Finding repair, reject it, refuse stale sources and reanalyze the applied exact diff", async ({
    page,
  }) => {
    responses.push(
      ...[
        stockTranslation(),
        stockComparison(),
        stockRepair(),
        stockRepair(),
        stockTranslation(),
        stockComparison(),
        stockRepair(),
        stockTranslation(true),
        stockComparison(true),
      ].map(
        (message) => (response: ServerResponse) => reply(response, message),
      ),
    );
    await openStockFinding(page);
    const prepare = page.getByRole("button", {
      name: "このFindingの修正差分を作る",
    });
    const dialog = page.getByRole("dialog", {
      name: "修正差分と元の要件を確認",
    });
    const apply = dialog.getByRole("button", {
      name: "この差分を一度適用して再解析",
    });
    await prepare.click();
    await expect(dialog).toBeVisible();
    await expect(dialog.getByLabel("適用する正確な修正差分")).toContainText(
      "+pub fn stock(n: u64) -> u64 { n.saturating_sub(1) }",
    );
    expect(await readFile(resolve(project, "stock.rs"), "utf8")).toBe(
      stockSource,
    );
    await dialog.getByRole("button", { name: "修正を拒否" }).click();
    await expect(dialog).not.toBeVisible();
    expect(requests).toHaveLength(3);
    await prepare.click();
    await expect(dialog).toBeVisible();
    const concurrent = stockSource + "// concurrent host edit\n";
    await writeFile(resolve(project, "stock.rs"), concurrent);
    await apply.click();
    await expect(
      page
        .getByRole("alert")
        .filter({ hasText: "Sources changed since the Finding" }),
    ).toBeVisible();
    expect(await readFile(resolve(project, "stock.rs"), "utf8")).toBe(
      concurrent,
    );
    expect(requests).toHaveLength(4);
    await refreshReferences(page);
    await page.getByRole("button", { name: "この宣言の意味を解析" }).click();
    await expect.poll(() => requests.length).toBe(6);
    await expect(prepare).toBeEnabled();
    await prepare.click();
    await expect(dialog).toBeVisible();
    const reference = (await nativeState(page)).repair_review.reference;
    await page.setViewportSize({ width: 300, height: 250 });
    await expect(apply).toBeDisabled();
    expect(await readFile(resolve(project, "stock.rs"), "utf8")).toBe(
      concurrent,
    );
    await page.setViewportSize({ width: 1440, height: 960 });
    await expect(apply).toBeEnabled();
    await page.screenshot({
      path: "target/gui-checks/native-repair-review.png",
      fullPage: true,
    });
    await apply.click();
    const result = page.getByLabel("意味解析の結果", { exact: true });
    await expect(result).toContainText("一致の候補（モデル評価）");
    expect(await readFile(resolve(project, "stock.rs"), "utf8")).toBe(
      concurrent.replace("{ n }", "{ n.saturating_sub(1) }"),
    );
    const state = await nativeState(page);
    expect(state.project.verified).toBe(false);
    expect(state.analysis_history).toHaveLength(3);
    expect(
      new Set(
        state.analysis_history.map(
          (entry: { requirement_id: string }) => entry.requirement_id,
        ),
      ).size,
    ).toBe(1);
    expect(requests).toHaveLength(9);
    expect(await hostCommand(page, { type: "apply_repair", reference })).toBe(
      202,
    );
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    expect((await nativeState(page)).stopped).toContain(
      "Repair review selection changed",
    );
    expect(requests).toHaveLength(9);
    const port = Number(new URL(url).port);
    await stop();
    await page.goto(await start(port));
    await expect(
      page.getByRole("tab", { name: "プロジェクトマップ", exact: true }),
    ).toBeVisible();
    expect((await nativeState(page)).repair_review).toBe(null);
    expect((await nativeState(page)).authorized).toBe(false);
    await page
      .getByRole("button", { name: "ファイル: stock.rs", exact: true })
      .click();
    await expect(result).toContainText("保存された解析です");
  });
  test("restarting discards a pending repair approval without changing source", async ({
    page,
  }) => {
    responses.push(
      ...[stockTranslation(), stockComparison(), stockRepair()].map(
        (message) => (response: ServerResponse) => reply(response, message),
      ),
    );
    await openStockFinding(page);
    await page
      .getByRole("button", { name: "このFindingの修正差分を作る" })
      .click();
    const dialog = page.getByRole("dialog", {
      name: "修正差分と元の要件を確認",
    });
    await expect(dialog).toBeVisible();
    const reference = (await nativeState(page)).repair_review.reference;
    const port = Number(new URL(url).port);
    await stop();
    await page.goto(await start(port));
    await expect(
      page.getByRole("tab", { name: "プロジェクトマップ", exact: true }),
    ).toBeVisible();
    await expect(dialog).not.toBeVisible();
    expect(await hostCommand(page, { type: "apply_repair", reference })).toBe(
      202,
    );
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    expect((await nativeState(page)).stopped).toContain(
      "Repair review selection changed",
    );
    expect(await readFile(resolve(project, "stock.rs"), "utf8")).toBe(
      stockSource,
    );
    expect(requests).toHaveLength(3);
  });
  test("local meaning history retains requirement and Finding identities across reruns and restart", async ({
    page,
  }) => {
    responses.push(
      ...Array.from({ length: 3 }, () => [
        (response: ServerResponse) => reply(response, informalization()),
        (response: ServerResponse) => reply(response, judgement()),
      ]).flat(),
    );
    await openMeaning(page);
    const result = page.getByLabel("意味解析の結果", { exact: true });
    const run = page.getByRole("button", { name: "この宣言の意味を解析" });
    const requirement = page.getByRole("textbox", {
      name: "比較する要件",
      exact: true,
    });
    await run.click();
    await expect(result).toContainText("意味の差異候補（モデル評価）");
    const first = (await nativeState(page)).analysis;
    expect(first.finding_id).toMatch(/^finding-/);
    await requirement.fill("A different requirement retained as its own input");
    await run.click();
    await expect
      .poll(async () => (await nativeState(page)).analysis_history.length)
      .toBe(2);
    const second = (await nativeState(page)).analysis;
    expect(second.id).not.toBe(first.id);
    expect(second.requirement_id).not.toBe(first.requirement_id);
    const history = page.getByRole("combobox", { name: "保存された解析履歴" });
    await history.selectOption(first.id);
    await expect(result).toContainText("保存された解析です");
    await expect(requirement).toHaveValue(first.input.requirement);
    expect(requests).toHaveLength(4);
    await run.click();
    await expect
      .poll(async () => (await nativeState(page)).analysis_history.length)
      .toBe(3);
    const state = await nativeState(page);
    expect(state.analysis.requirement_id).toBe(first.requirement_id);
    expect(state.analysis.finding_id).not.toBe(first.finding_id);
    expect(state.analysis_history[0].assessment).toEqual(first.assessment);
    expect(state.analysis_history[1].input).toEqual(second.input);
    await expect(result).toContainText("意味の差異候補（モデル評価）");
    const port = Number(new URL(url).port);
    await stop();
    await page.goto(await start(port));
    await page
      .getByRole("button", { name: "ファイル: Spec.lean", exact: true })
      .click();
    await history.selectOption(second.id);
    await expect(result).toContainText(second.input.requirement);
    expect((await nativeState(page)).authorized).toBe(false);
    expect(requests).toHaveLength(6);
    await page.setViewportSize({ width: 390, height: 844 });
    await result.getByText("解析対象と処理の詳細", { exact: true }).click();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    expect(
      (
        await new AxeBuilder({ page })
          .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
          .analyze()
      ).violations,
    ).toEqual([]);
    await page.screenshot({
      path: "target/gui-checks/native-meaning-history.png",
      fullPage: true,
    });
  });
  test("separate judge models are visible, retained and revoked together", async ({
    page,
  }) => {
    responses.push(
      (response) => reply(response, informalization()),
      (response) => reply(response, judgement()),
    );
    await openMeaning(
      page,
      ["--judge-model", "separate-judge-fixture"],
      async () => {
        const state = await nativeState(page);
        expect(
          await hostCommand(page, {
            type: "approve_processor",
            connection_revision: state.connection_revision,
            endpoint: state.endpoint,
            model: state.model,
          }),
        ).toBe(409);
        expect(
          await hostCommand(page, {
            type: "approve_processor",
            connection_revision: state.connection_revision,
            endpoint: state.endpoint,
            model: state.model,
            judge: { ...state.judge, model: "other-model" },
          }),
        ).toBe(409);
        expect((await nativeState(page)).authorized).toBe(false);
        expect(requests).toHaveLength(0);
        await page.setViewportSize({ width: 390, height: 250 });
        await expect(
          page.getByRole("button", { name: "このモデルを使用" }),
        ).toBeDisabled();
        await page.setViewportSize({ width: 1440, height: 960 });
      },
    );
    await page
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    await expect(page.getByRole("dialog")).toContainText(
      "separate-judge-fixture",
    );
    await page.getByRole("button", { name: "閉じる", exact: true }).click();
    await page.getByRole("button", { name: "この宣言の意味を解析" }).click();
    const result = page.getByLabel("意味解析の結果", { exact: true });
    await expect(result).toContainText("意味の差異候補");
    expect((requests[0] as { model: string }).model).toBe(
      "scripted-gui-fixture",
    );
    expect((requests[1] as { model: string }).model).toBe(
      "separate-judge-fixture",
    );
    const record = (await nativeState(page)).analysis;
    expect(record.judge.model).toBe("separate-judge-fixture");
    await result.getByText("解析対象と処理の詳細").click();
    await expect(result).toContainText("比較: separate-judge-fixture");
    let releaseMetadata = () => {};
    const metadataReleased = new Promise<void>(
      (resolve) => (releaseMetadata = resolve),
    );
    let metadataHeld = false;
    await page.route("**/api/command", async (route) => {
      if (route.request().postDataJSON().type !== "list_models")
        return route.continue();
      const response = await route.fetch();
      metadataHeld = true;
      await metadataReleased;
      await route.fulfill({ response });
    });
    await page
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    try {
      await expect.poll(() => metadataHeld).toBe(true);
      await page
        .getByRole("button", { name: "送信の許可を取り消す", exact: true })
        .click();
      await expect
        .poll(async () => (await nativeState(page)).authorized)
        .toBe(false);
    } finally {
      releaseMetadata();
    }
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "閉じる", exact: true })
      .click();
    await expect(
      page.getByRole("button", { name: "この宣言の意味を解析" }),
    ).toBeDisabled();
    await expect
      .poll(async () => (await nativeState(page)).authorized)
      .toBe(false);
    expect(requests).toHaveLength(2);
    await page.screenshot({
      path: "target/gui-checks/native-separate-judge.png",
      fullPage: true,
    });
  });
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
    await openDependencyControls(page);
    await expect(result).toContainText("保存された解析です");
    expect(requests).toHaveLength(2);
    await setPaths(page, "Spec.lean");
    await refreshReferences(page);
    await expect(input).toHaveValue(original);
    await expect(
      page.getByRole("button", { name: "この宣言の意味を解析" }),
    ).toBeDisabled();
    await page
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    await page.getByRole("button", { name: "このモデルを使用" }).click();
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
    await openDependencyControls(page);
    await setPaths(page, "Spec.lean");
    await refreshReferences(page);
    await page
      .getByRole("region", { name: "ローカルソースのグラフ" })
      .getByRole("button", { name: /Demo.stock_bound.*を選択/ })
      .click();
    await input.fill(original);
    await page
      .getByRole("button", { name: "モデルを選択", exact: true })
      .click();
    await page.getByRole("button", { name: "このモデルを使用" }).click();
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
          await page.getByRole("button", { name: "中止", exact: true }).click();
        else if (mode === "revoke") {
          await page
            .getByRole("button", { name: "モデルを選択", exact: true })
            .click();
          await page
            .getByRole("button", { name: "送信の許可を取り消す", exact: true })
            .click();
          await page
            .getByRole("dialog")
            .getByRole("button", { name: "閉じる", exact: true })
            .click();
        } else
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
