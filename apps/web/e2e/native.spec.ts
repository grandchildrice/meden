import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import {
  createServer,
  request as httpRequest,
  type ServerResponse,
} from "node:http";
import { spawn, type ChildProcess } from "node:child_process";
import {
  cp,
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
import { projectEvidence, hasCurrentEvidence } from "./evidence";
import {
  analysisCurrentSchema,
  analysisHistorySchema,
} from "@meden/agent-protocol/native";

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
        .getByRole("region", { name: "Lean tools", exact: true })
        .isVisible())
    )
      await page
        .getByRole("button", { name: "Lean tools", exact: true })
        .click();
    const details = page
      .getByRole("region", { name: "Lean tools", exact: true })
      .locator("details");
    if (!(await details.evaluate((e) => (e as HTMLDetailsElement).open)))
      await details.locator("summary").click();
  }
  async function setPaths(page: Page, value: string) {
    await openDependencyControls(page);
    await page
      .getByRole("textbox", { name: "Graph source files (one per line)" })
      .fill(value);
  }
  async function refreshReferences(page: Page) {
    await openDependencyControls(page);
    await page
      .getByRole("button", { name: "Refresh references", exact: true })
      .click();
  }
  async function compileDependencies(page: Page) {
    await openDependencyControls(page);
    const paths = await page
      .getByRole("textbox", { name: "Graph source files (one per line)" })
      .inputValue();
    const extract = page.getByRole("button", {
      name: paths.trim()
        ? "Extract compiled dependencies"
        : "Select Lake targets automatically",
      exact: true,
    });
    await expect(extract).toBeEnabled({ timeout: 120000 });
    const accepted = page.waitForResponse(
      (response) =>
        response.url().endsWith("/api/command") &&
        response.request().postDataJSON()?.type === "atlas_snapshot",
    );
    await extract.click();
    await accepted;
  }
  async function stop() {
    if (child?.exitCode === null && !child.signalCode) {
      const exited = once(child, "exit");
      child.kill("SIGTERM");
      const timer = setTimeout(() => child?.kill("SIGKILL"), 15000);
      const [code, signal] = await exited;
      clearTimeout(timer);
      if (signal === "SIGKILL")
        throw new Error(`Gateway did not stop normally: ${code}`);
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

  test("registered finite verifier reviews, refutes, repairs and reopens with bounded evidence", async ({
    page,
  }) => {
    test.setTimeout(60000);
    const evidenceReads: string[] = [];
    let failProposal = true;
    let commands = 0;
    page.on("request", (request) => {
      if (new URL(request.url()).pathname === "/api/command") commands++;
    });
    await page.route("**/api/evidence?*", async (route) => {
      const response = await route.fetch();
      const body = await response.json();
      evidenceReads.push(body.revision);
      if (failProposal && body.data?.state === "proposed") {
        failProposal = false;
        await route.fulfill({
          status: 503,
          contentType: "application/json",
          body: JSON.stringify({
            error: "Temporary evidence transfer failure",
          }),
        });
      } else await route.fulfill({ response });
    });
    const contract = JSON.parse(
      await readFile(
        "verification/examples/finite-counter.contract.json",
        "utf8",
      ),
    );
    const correct = await readFile(
      "verification/examples/finite-counter.model.json",
      "utf8",
    );
    const broken = await readFile(
      "verification/examples/finite-counter-broken.model.json",
      "utf8",
    );
    for (const [name, args] of [
      ["propose_contract", { contract }],
      ["write_file", { path: contract.source_path, content: broken }],
      ["verify_project", {}],
      ["write_file", { path: contract.source_path, content: correct }],
      ["verify_project", {}],
    ] as const)
      responses.push((response) => reply(response, tool(name, args)));
    responses.push((response) =>
      reply(response, { role: "assistant", content: "FINITE-GUI-COMPLETE" }),
    );
    await page.goto(
      await start(0, { MEDEN_VERIFIER_BUNDLE: undefined }, [
        "--verifier",
        "z3.finite.v1",
      ]),
    );
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Use this model", exact: true })
      .click();
    await page
      .getByRole("textbox", { name: "Message", exact: true })
      .fill("Check every four-bit counter input; retain state on overflow.");
    await page.getByRole("button", { name: "Send", exact: true }).click();
    const retryEvidence = page.getByRole("button", {
      name: "Retry evidence",
      exact: true,
    });
    await expect(retryEvidence).toBeVisible();
    const beforeRetry = commands;
    await retryEvidence.click();
    await expect(page.locator(".chat-contract .MuiChip-label")).toHaveText(
      "Proposal",
    );
    expect(commands).toBe(beforeRetry);
    const proposed = await projectEvidence(page);
    expect(proposed.state).toBe("proposed");
    if (proposed.state !== "proposed")
      throw new Error("Expected actual finite proposal");
    expect(typeof proposed.contract.reference.revision).toBe("string");
    await page
      .getByRole("button", { name: "Review proposed contract", exact: true })
      .click();
    const review = page.getByRole("dialog", {
      name: "Review contract meaning",
    });
    await expect(review).toContainText("256 assignments");
    await expect(review).toContainText(
      "No reachability, unbounded time, Rust correspondence",
    );
    await review
      .getByRole("button", {
        name: "Accept this contract and continue",
        exact: true,
      })
      .click();
    await expect(
      page.getByText("FINITE-GUI-COMPLETE", { exact: true }),
    ).toBeVisible();
    const evidence = page.locator(".chat-contract");
    await expect(evidence.locator(".MuiChip-label")).toHaveText(
      "CHECKED · complete within finite bounds",
    );
    const current = await projectEvidence(page);
    expect(hasCurrentEvidence(current)).toBe(true);
    const status = await page.evaluate(async () =>
      (
        await fetch("/api/state", {
          headers: {
            Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
          },
        })
      ).json(),
    );
    expect(Object.keys(status.project).sort()).toEqual(["revision", "state"]);
    // Several real status polls do not transfer the complete review again.
    const readCount = evidenceReads.length;
    await page.waitForTimeout(1200);
    expect(evidenceReads.length).toBe(readCount);
    // Fault injection is complete; measure direct loopback reads without routing.
    await page.unroute("**/api/evidence?*");
    const timings = await page.evaluate(async () => {
      const headers = {
        Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
      };
      const before = await (await fetch("/api/metrics", { headers })).json();
      const state = await (await fetch("/api/state", { headers })).json();
      const samples = [];
      let bodyBytes = 0;
      for (let i = 0; i < 20; i++) {
        const start = performance.now();
        const response = await fetch(
          "/api/evidence?revision=" +
            encodeURIComponent(state.project.revision),
          { headers },
        );
        if (!response.ok) throw new Error("Exact evidence read failed");
        const body = await response.arrayBuffer();
        bodyBytes = body.byteLength;
        samples.push(performance.now() - start);
      }
      const after = await (await fetch("/api/metrics", { headers })).json();
      const materializations = (metrics: any) =>
        metrics.records.filter(
          (r: any) => r.name === "view.evidence.materialize",
        ).length;
      return {
        samples_ms: samples,
        body_bytes: bodyBytes,
        status_bytes: new TextEncoder().encode(JSON.stringify(state.project))
          .byteLength,
        materializations_before: materializations(before),
        materializations_after: materializations(after),
        metrics: after,
      };
    });
    expect(timings.materializations_after).toBe(
      timings.materializations_before,
    );
    expect(timings.body_bytes).toBeGreaterThan(timings.status_bytes);
    await test.info().attach("finite-evidence-timings.json", {
      body: JSON.stringify(timings, null, 2),
      contentType: "application/json",
    });
    await evidence.hover();
    await page.mouse.wheel(0, -2000);
    await expect(evidence.locator(".MuiChip-label")).toBeInViewport();
    expect(JSON.stringify(requests)).toContain("Finite property failed");
    expect(await readFile(resolve(project, contract.source_path), "utf8")).toBe(
      correct,
    );
    await page.screenshot({
      path: test.info().outputPath("finite-bounded-evidence.png"),
    });
    await stop();
    responses.length = 0;
    // The host's original finite choice survives without another selector.
    await page.goto(await start());
    await expect(page.locator(".chat-contract .MuiChip-label")).toHaveText(
      "Contract accepted",
    );
    const reopened = await projectEvidence(page);
    expect(hasCurrentEvidence(reopened)).toBe(false);
    if (current.state !== "accepted" || reopened.state !== "accepted")
      throw new Error("Original accepted agreement must survive restart");
    expect(reopened.contract.canonical).toBe(current.contract.canonical);
    expect(reopened.contract.reference.contract).toBe(
      current.contract.reference.contract,
    );
    const before = requests.length;
    await expect(
      page.getByRole("button", { name: "Choose model", exact: true }),
    ).toBeVisible();
    expect(requests.length).toBe(before);
  });

  test("chat stays flat, folds long messages and remembers draggable sidebar widths", async ({
    page,
  }) => {
    await mkdir(resolve(project, ".meden"));
    const user =
      "USER-LONG-START\n" +
      "Original user context stays available.\n".repeat(45) +
      "USER-LONG-END";
    const assistant =
      "ASSISTANT-LONG-START\n\n```lean\n" +
      "example : True := True.intro\n".repeat(45) +
      "```\nASSISTANT-LONG-END";
    await writeFile(
      resolve(project, "Spec.lean"),
      "theorem identity : True := True.intro\n",
    );
    await writeFile(
      resolve(project, ".meden/workspace.json"),
      JSON.stringify({
        version: 1,
        name: "Clean chat",
        tasks: [
          {
            id: "work",
            title: "Archived work",
            requirement: "Preserve original intent",
            module: "Spec",
            theorems: ["identity"],
            next: "Continue carefully",
          },
        ],
        events: [],
        conversations: [
          {
            id: "saved",
            messages: [
              { speaker: "user", text: user },
              { speaker: "assistant", text: assistant },
            ],
          },
        ],
      }),
    );
    await page.goto(await start());
    const chat = page.getByRole("complementary", {
      name: "Workspace chat",
      exact: true,
    });
    await expect(page.locator(".ide-header h1")).toHaveText("Meden");
    await expect(page.locator(".ide-project-name")).toHaveCount(0);
    await expect(
      chat.locator(".chat-work-card, .chat-work-history"),
    ).toHaveCount(0);
    const userMessage = chat.locator(".user-turn"),
      assistantMessage = chat.locator(".assistant-message");
    for (const message of [userMessage, assistantMessage]) {
      await expect(
        message.getByRole("button", { name: "Show more", exact: true }),
      ).toHaveAttribute("aria-expanded", "false");
      expect(
        await message.locator(".message-body").evaluate((e) => e.clientHeight),
      ).toBeLessThanOrEqual(240);
      await message
        .getByRole("button", { name: "Show more", exact: true })
        .click();
      expect(
        await message.locator(".message-body").evaluate((e) => e.clientHeight),
      ).toBeGreaterThan(500);
      await message
        .getByRole("button", { name: "Show less", exact: true })
        .click();
    }
    const draft = chat.getByRole("textbox", { name: "Message", exact: true });
    // The model need not be approved to resize, inspect history or expand text.
    const resize = async (name: string, dx: number) => {
      const handle = page.getByRole("separator", { name });
      const box = (await handle.boundingBox())!;
      await page.mouse.move(box.x + box.width / 2, box.y + 150);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width / 2 + dx, box.y + 150, {
        steps: 8,
      });
      await page.mouse.up();
    };
    await resize("Resize chat sidebar", -180);
    await expect
      .poll(async () => Math.round((await chat.boundingBox())!.width))
      .toBe(530);
    await resize("Resize file sidebar", 70);
    await expect
      .poll(async () =>
        Math.round(
          (await page
            .getByRole("complementary", { name: "Files", exact: true })
            .boundingBox())!.width,
        ),
      )
      .toBe(290);
    await page.reload();
    await expect
      .poll(async () => Math.round((await chat.boundingBox())!.width))
      .toBe(530);
    const handle = page.getByRole("separator", { name: "Resize chat sidebar" });
    await handle.focus();
    await handle.press("ArrowLeft");
    await expect
      .poll(async () => Math.round((await chat.boundingBox())!.width))
      .toBe(546);
    await handle.press("Home");
    await expect
      .poll(async () => Math.round((await chat.boundingBox())!.width))
      .toBe(350);
    await chat
      .getByRole("button", { name: "Work history", exact: true })
      .click();
    await expect(page.getByRole("dialog")).toContainText("Archived work");
    await page.getByRole("button", { name: "Close", exact: true }).click();
    await expect(draft).toBeVisible();
    expect(requests).toHaveLength(0);
    await page.screenshot({ path: "target/gui-checks/flat-chat-resize.png" });
    await page.setViewportSize({ width: 390, height: 844 });
    await expect(page.getByRole("separator")).toHaveCount(0);
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
  });

  test("large history is revisioned outside status polling, retries visibly and rejects late snapshots across restart", async ({
    page,
  }, testInfo) => {
    test.setTimeout(120000);
    await mkdir(resolve(project, ".meden"));
    const note = "LOG-SENTINEL:".repeat(4000);
    const saved = [
      { speaker: "user", text: "SAVED-REVISIONED-HISTORY" },
      { speaker: "assistant", text: "SAVED-ANSWER" },
    ];
    await writeFile(resolve(project, "Spec.rs"), "fn saved_target() {}\n");
    await writeFile(
      resolve(project, ".meden/workspace.json"),
      JSON.stringify({
        version: 1,
        name: "Large saved history",
        tasks: [
          {
            id: "work",
            title: "Saved work",
            requirement: "Preserve the complete original records",
            module: "Spec",
            theorems: ["saved_target"],
            next: "Continue from history",
          },
        ],
        events: Array.from({ length: 60 }, (_, i) => ({
          id: "event_" + i,
          task: "work",
          time: 1,
          note,
          check: null,
        })),
        conversations: [
          { id: "prefix", messages: saved.slice(0, 1) },
          { id: "saved", messages: saved },
        ],
      }),
    );
    const stateSizes: number[] = [],
      historyUrls: string[] = [];
    page.on("request", (request) => {
      if (new URL(request.url()).pathname === "/api/history")
        historyUrls.push(request.url());
    });
    page.on("requestfinished", async (request) => {
      if (new URL(request.url()).pathname === "/api/state")
        try {
          stateSizes.push((await request.sizes()).responseBodySize);
        } catch {}
    });
    let fail = true,
      conflict = true,
      hold = false,
      held = false,
      releaseOld = () => {};
    const oldReleased = new Promise<void>((resolve) => (releaseOld = resolve));
    await page.route("**/api/history?*", async (route) => {
      if (fail) {
        fail = false;
        return route.fulfill({
          status: 500,
          contentType: "application/json",
          body: JSON.stringify({ error: "History response interrupted" }),
        });
      }
      if (conflict) {
        conflict = false;
        return route.fulfill({
          status: 409,
          contentType: "application/json",
          body: JSON.stringify({ error: "Reference temporarily superseded" }),
        });
      }
      const response = await route.fetch();
      if (hold && !held && response.ok()) {
        held = true;
        await oldReleased;
      }
      await route.fulfill({ response }).catch(() => {});
    });
    await page.goto(await start());
    const chat = page.getByRole("complementary", {
      name: "Workspace chat",
      exact: true,
    });
    await expect(chat.getByRole("alert")).toContainText(
      "History response interrupted",
    );
    expect((await nativeState(page)).authorized).toBe(false);
    await chat
      .getByRole("button", { name: "Retry history", exact: true })
      .click();
    await expect(
      chat.getByText("SAVED-REVISIONED-HISTORY", { exact: true }),
    ).toHaveCount(1);
    await expect(chat.getByText("SAVED-ANSWER", { exact: true })).toBeVisible();
    await expect.poll(() => stateSizes.length).toBeGreaterThanOrEqual(6);
    expect(Math.max(...stateSizes)).toBeLessThan(16384);
    expect(historyUrls).toHaveLength(3);
    const status = await nativeState(page);
    expect(status).not.toHaveProperty("portable");
    expect(status).not.toHaveProperty("messages");
    expect(status).not.toHaveProperty("logs");
    const origin = new URL(url).origin;
    const historyPath =
      "/api/history?revision=" + encodeURIComponent(status.history.revision);
    expect((await fetch(origin + historyPath)).status).toBe(401);
    expect(
      (
        await fetch(origin + historyPath, {
          headers: {
            Authorization: "Bearer " + new URL(url).hash.split("=")[1],
            Origin: "https://evil.invalid",
          },
        })
      ).status,
    ).toBe(403);
    const snapshot = await page.evaluate(
      async (revision) =>
        (
          await fetch("/api/history?revision=" + encodeURIComponent(revision), {
            headers: {
              Authorization:
                "Bearer " + sessionStorage.getItem("meden-native-session"),
            },
          })
        ).json(),
      status.history.revision,
    );
    expect(snapshot.revision).toBe(status.history.revision);
    expect(snapshot.data.portable.document.events).toHaveLength(60);
    expect(snapshot.data.portable.document.events[59].note).toBe(note);
    expect(JSON.stringify(snapshot).length).toBeGreaterThan(3_000_000);
    const historyCount = historyUrls.length;
    const before = await chat.innerHTML();
    await page
      .getByRole("button", { name: "File: Spec.rs", exact: true })
      .click();
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("File contents"),
    ).toContainText("fn saved_target");
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    await page.getByRole("tab", { name: "Spec.rs", exact: true }).click();
    expect(historyUrls).toHaveLength(historyCount);
    expect(await chat.innerHTML()).toBe(before);
    await page
      .getByRole("button", { name: "Work history", exact: true })
      .click();
    const dialog = page.getByRole("dialog");
    await expect(
      dialog.getByRole("heading", {
        name: "Work and shared history",
        exact: true,
      }),
    ).toBeVisible();
    await expect(
      dialog.locator("details").filter({ hasText: note }),
    ).toHaveCount(60);
    await dialog.getByRole("button", { name: "Close", exact: true }).click();
    expect(requests).toHaveLength(0);
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Use this model", exact: true })
      .click();
    let model: ServerResponse | undefined;
    responses.push((response) => {
      model = response;
    });
    hold = true;
    try {
      await chat
        .getByRole("textbox", { name: "Message", exact: true })
        .fill("Continue the saved history");
      await chat.getByRole("button", { name: "Send", exact: true }).click();
      // A real metadata/approval job may still be settling on a cold CI host.
      // Observe the actual admitted model request; this is a stale-read test,
      // not a five-second metadata/preparation performance budget.
      await expect.poll(() => Boolean(model), { timeout: 30000 }).toBe(true);
      await expect.poll(() => held).toBe(true);
      reply(model!, { role: "assistant", content: "LATEST-HISTORY-ANSWER" });
      await expect(
        chat.getByText("LATEST-HISTORY-ANSWER", { exact: true }),
      ).toBeVisible({ timeout: 30000 });
      releaseOld();
      await page.waitForTimeout(500);
      await expect(
        chat.getByText("LATEST-HISTORY-ANSWER", { exact: true }),
      ).toHaveCount(1);
      await expect(chat.getByRole("alert")).toHaveCount(0);
      expect(requests).toHaveLength(1);
      await expect
        .poll(async () => (await nativeState(page)).running)
        .toBe(false);
      expect(
        await page.evaluate(async (revision) => {
          const response = await fetch(
            "/api/history?revision=" + encodeURIComponent(revision),
            {
              headers: {
                Authorization:
                  "Bearer " + sessionStorage.getItem("meden-native-session"),
              },
            },
          );
          return response.status;
        }, status.history.revision),
      ).toBe(409);
      await page.screenshot({
        path: "target/gui-checks/revisioned-history.png",
      });
      await testInfo.attach("history-transfer.json", {
        body: JSON.stringify({
          status_bytes: stateSizes,
          initial_history_bytes: JSON.stringify(snapshot).length,
          initial_history_fetches: 3,
          scope:
            "One intentionally failed initial response, explicit retry, a transient conflict under the same ID and automatic recovery after fresh status; unchanged navigation does not refetch history.",
        }),
        contentType: "application/json",
      });
      // Retire intercepted reads before the real host goes away. A poll that
      // entered route.fetch must not race this intentional restart.
      await page.unrouteAll({ behavior: "wait" });
      await stop();
      await page.goto(await start());
      await expect(
        page.getByText("LATEST-HISTORY-ANSWER", { exact: true }),
      ).toBeVisible();
      expect((await nativeState(page)).authorized).toBe(false);
      expect(requests).toHaveLength(1);
    } finally {
      releaseOld();
      await page.unrouteAll({ behavior: "wait" });
    }
  });

  test("typed folder listings retain every entry beyond 500, literal names and local retry without poll payloads", async ({
    page,
  }) => {
    test.setTimeout(120000);
    await mkdir(resolve(project, "Many"));
    for (let batch = 0; batch < 1200; batch += 100)
      await Promise.all(
        Array.from({ length: 100 }, (_, i) =>
          writeFile(
            resolve(project, "Many", `File${batch + i}.txt`),
            `literal row ${batch + i}\n`,
          ),
        ),
      );
    await writeFile(
      resolve(project, "Many", "line\n日本語.txt"),
      "literal newline filename",
    );
    await mkdir(resolve(project, "Missing"));
    let reads = 0;
    page.on("request", (r) => {
      if (new URL(r.url()).pathname === "/api/directory") reads++;
    });
    await page.goto(await start());
    const explorer = page.getByRole("region", {
      name: "File explorer",
      exact: true,
    });
    await explorer
      .getByRole("button", { name: "Folder: Many", exact: true })
      .click();
    await expect(
      explorer.getByRole("button", {
        name: "File: Many/File1199.txt",
        exact: true,
      }),
    ).toBeVisible();
    const status = await nativeState(page);
    expect(status).not.toHaveProperty("inspection");
    expect(JSON.stringify(status).length).toBeLessThan(16384);
    const listing = await page.evaluate(async (revision) => {
      const response = await fetch(
        "/api/directory?revision=" + encodeURIComponent(revision),
        {
          headers: {
            Authorization:
              "Bearer " + sessionStorage.getItem("meden-native-session"),
          },
        },
      );
      return response.json();
    }, status.directories.paths.Many.revision);
    expect(listing.data.reading.entries).toHaveLength(1201);
    const before = reads;
    await explorer
      .getByRole("button", { name: "Folder: Many", exact: true })
      .click();
    await explorer
      .getByRole("button", { name: "Folder: Many", exact: true })
      .click();
    await expect(
      explorer.getByRole("button", {
        name: "File: Many/File1199.txt",
        exact: true,
      }),
    ).toBeVisible();
    expect(reads).toBe(before);
    await explorer
      .getByRole("button", { name: "File: Many/File1199.txt", exact: true })
      .click();
    await expect(page.getByLabel("File contents")).toContainText(
      "literal row 1199",
    );
    await explorer
      .getByRole("button", { name: "File: Many/line\n日本語.txt", exact: true })
      .click();
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("File contents"),
    ).toContainText("literal newline filename");
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    await rm(resolve(project, "Missing"), { recursive: true });
    await explorer
      .getByRole("button", { name: "Folder: Missing", exact: true })
      .click();
    await expect(explorer.getByRole("alert")).toContainText(
      "Could not open the folder.",
    );
    await mkdir(resolve(project, "Missing"));
    await writeFile(resolve(project, "Missing", "Recovered.txt"), "recovered");
    await explorer
      .getByRole("button", { name: "Retry folder", exact: true })
      .click();
    await expect(
      explorer.getByRole("button", {
        name: "File: Missing/Recovered.txt",
        exact: true,
      }),
    ).toBeVisible();
    expect(requests).toHaveLength(0);
    await page.screenshot({
      path: "target/gui-checks/typed-directory-complete.png",
    });
  });

  test("inventory publications retry without recapture and reject older bodies across refresh and restart", async ({
    page,
  }) => {
    await writeFile(resolve(project, "Nav.rs"), "pub fn version_one() {}\n");
    let fail = true,
      hold = false,
      held = false,
      releaseOld = () => {};
    const released = new Promise<void>((resolve) => (releaseOld = resolve));
    let reads = 0;
    const commands: string[] = [];
    page.on("request", (request) => {
      if (new URL(request.url()).pathname === "/api/inventory") reads++;
      if (request.url().endsWith("/api/command"))
        commands.push(request.postDataJSON().type);
    });
    await page.route("**/api/inventory?revision=*", async (route) => {
      if (fail) {
        fail = false;
        return route.fulfill({
          status: 503,
          json: { error: "Prepared inventory unavailable" },
        });
      }
      const response = await route.fetch();
      if (hold && !held && response.ok()) {
        held = true;
        await released;
      }
      await route.fulfill({ response }).catch(() => {});
    });
    await page.goto(await start());
    const map = page.getByRole("region", { name: "Project map", exact: true });
    await expect(map.getByRole("alert")).toContainText(
      "Prepared inventory unavailable",
    );
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    const beforeRetry = commands.length;
    await map.getByRole("button", { name: "Retry map", exact: true }).click();
    await expect(
      map.getByRole("button", { name: /Declaration: version_one/ }),
    ).toBeVisible();
    expect(commands).toHaveLength(beforeRetry);
    const initial = (await nativeState(page)).overview.revision;
    const beforeNavigation = reads;
    await page
      .getByRole("button", { name: "File: Nav.rs", exact: true })
      .click();
    await expect(page.getByLabel("File contents")).toContainText("version_one");
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    await page.getByRole("tab", { name: "Nav.rs", exact: true }).click();
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    await page.waitForTimeout(450);
    expect(reads).toBe(beforeNavigation);
    const chat = page.getByRole("complementary", {
      name: "Workspace chat",
      exact: true,
    });
    const oldChat = await chat.innerHTML();
    hold = true;
    try {
      await page.reload();
      await expect.poll(() => held).toBe(true);
      await expect
        .poll(async () => (await nativeState(page)).running)
        .toBe(false);
      await writeFile(resolve(project, "Nav.rs"), "pub fn version_two() {}\n");
      await page
        .getByRole("button", { name: "Refresh files and map", exact: true })
        .click();
      await expect(
        map.getByRole("button", { name: /Declaration: version_two/ }),
      ).toBeVisible();
      releaseOld();
      await page.waitForTimeout(500);
      await expect(
        map.getByRole("button", { name: /Declaration: version_one/ }),
      ).toHaveCount(0);
      await expect(
        map.getByRole("button", { name: /Declaration: version_two/ }),
      ).toBeVisible();
      const result = await page.evaluate(async (revision) => {
        const headers = {
          Authorization:
            "Bearer " + sessionStorage.getItem("meden-native-session"),
        };
        return {
          original: (
            await fetch(
              "/api/inventory?revision=" + encodeURIComponent(revision),
              { headers },
            )
          ).status,
          retired: (await fetch("/api/overview", { headers })).status,
          chunk: (
            await fetch(
              "/api/map?inventory=" +
                encodeURIComponent(revision) +
                "&path=Nav.rs",
              { headers },
            )
          ).status,
        };
      }, initial);
      expect(result).toEqual({ original: 409, retired: 404, chunk: 409 });
      expect(await chat.innerHTML()).toBe(oldChat);
      await page.screenshot({
        path: "target/gui-checks/inventory-publication-latest.png",
      });
    } finally {
      releaseOld();
      await page.unrouteAll({ behavior: "wait" });
    }
    await stop();
    await page.goto(await start());
    await expect(
      map.getByRole("button", { name: /Declaration: version_two/ }),
    ).toBeVisible();
    expect((await nativeState(page)).authorized).toBe(false);
    expect(requests).toHaveLength(0);
  });

  test("prepared source publications retry locally, transfer once and refuse delayed older views across restart", async ({
    page,
  }, testInfo) => {
    test.setTimeout(120000);
    const text = (name: string) =>
      `pub fn ${name}() {}\n// ${"original content ".repeat(24000)}\n`;
    await writeFile(resolve(project, "Large.rs"), text("version_one"));
    await writeFile(resolve(project, "Tiny.txt"), "Independent reader\n");
    let fail = true,
      conflicts = 3,
      hold = false,
      held: string | undefined,
      releaseOld = () => {};
    const released = new Promise<void>((resolve) => (releaseOld = resolve));
    let reads = 0;
    page.on("request", (r) => {
      if (new URL(r.url()).pathname === "/api/source") reads++;
    });
    await page.route("**/api/source?*", async (route) => {
      if (fail) {
        fail = false;
        return route.fulfill({
          status: 503,
          contentType: "application/json",
          body: JSON.stringify({ error: "Prepared view unavailable" }),
        });
      }
      if (conflicts) {
        conflicts--;
        return route.fulfill({
          status: 409,
          contentType: "application/json",
          body: JSON.stringify({ error: "Reference temporarily superseded" }),
        });
      }
      const response = await route.fetch();
      if (hold && !held && response.ok()) {
        held = new URL(route.request().url()).searchParams.get("revision")!;
        await released;
      }
      await route.fulfill({ response }).catch(() => {});
    });
    await page.goto(await start());
    const explorer = page.getByRole("region", {
      name: "File explorer",
      exact: true,
    });
    const chat = page.getByRole("complementary", {
      name: "Workspace chat",
      exact: true,
    });
    await explorer
      .getByRole("button", { name: "File: Tiny.txt", exact: true })
      .click();
    await expect(page.getByLabel("File contents")).toContainText(
      "Independent reader",
    );
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    const before = await chat.innerHTML();
    await setPaths(page, "Large.rs");
    await refreshReferences(page);
    const pane = page.locator(".ide-file-page:not([hidden])");
    await expect(pane.getByRole("alert")).toContainText(
      "Could not load the prepared graph.",
    );
    await expect(pane.getByRole("progressbar")).toHaveCount(0);
    await expect(pane.getByRole("status")).toHaveText(
      "Source could not be loaded.",
    );
    await expect(chat.getByRole("alert")).toHaveCount(0);
    await page.screenshot({
      path: "target/gui-checks/source-publication-failure.png",
    });
    await pane
      .getByRole("button", { name: "Retry graph", exact: true })
      .click();
    await expect(pane.getByRole("alert")).toContainText(
      "The view changed while loading.",
    );
    expect(reads).toBe(4);
    await page.waitForTimeout(700);
    expect(reads).toBe(4);
    await pane
      .getByRole("button", { name: "Retry graph", exact: true })
      .click();
    await expect(pane.getByLabel("File contents")).toContainText("version_one");
    const first = await nativeState(page);
    const firstId = first.sources.current.revision;
    expect(first).not.toHaveProperty("source_snapshot");
    expect(first).not.toHaveProperty("source_views");
    expect(JSON.stringify(first).length).toBeLessThan(16384);
    expect(reads).toBe(5);
    const sourceQuery = await page.evaluate(async (revision) => {
      const response = await fetch(
        "/api/source?revision=" + encodeURIComponent(revision),
        {
          headers: {
            Authorization:
              "Bearer " + sessionStorage.getItem("meden-native-session"),
          },
        },
      );
      const body = await response.text();
      return {
        status: response.status,
        bytes: new TextEncoder().encode(body).length,
        body: JSON.parse(body),
      };
    }, firstId);
    expect(sourceQuery.status).toBe(200);
    expect(sourceQuery.bytes).toBeGreaterThan(350000);
    expect(sourceQuery.body.revision).toBe(firstId);
    expect(sourceQuery.body.data.files["Large.rs"]).toBe(text("version_one"));
    expect(sourceQuery.body.data).not.toHaveProperty("atlas");
    const origin = new URL(url).origin;
    const query = "/api/source?revision=" + encodeURIComponent(firstId);
    expect((await fetch(origin + query)).status).toBe(401);
    expect(
      (
        await fetch(origin + query, {
          headers: {
            Authorization: "Bearer " + new URL(url).hash.split("=")[1],
            Origin: "https://evil.invalid",
          },
        })
      ).status,
    ).toBe(403);
    const unchangedReads = reads;
    for (let i = 0; i < 3; i++) {
      await page.getByRole("tab", { name: "Project map", exact: true }).click();
      await page.getByRole("tab", { name: "Large.rs", exact: true }).click();
    }
    await page.waitForTimeout(1200);
    expect(reads).toBe(unchangedReads);
    expect(await chat.innerHTML()).toBe(before);
    await writeFile(resolve(project, "Large.rs"), text("version_two"));
    hold = true;
    try {
      await refreshReferences(page);
      await expect.poll(() => Boolean(held)).toBe(true);
      expect(held).not.toBe(firstId);
      await writeFile(resolve(project, "Large.rs"), text("version_three"));
      await refreshReferences(page);
      await expect(pane.getByLabel("File contents")).toContainText(
        "version_three",
      );
      const newest = await nativeState(page);
      expect(newest.sources.current.revision).not.toBe(held);
      releaseOld();
      await page.waitForTimeout(700);
      await expect(pane.getByLabel("File contents")).toContainText(
        "version_three",
      );
      await expect(pane.getByLabel("File contents")).not.toContainText(
        "version_two",
      );
      expect(await chat.innerHTML()).toBe(before);
      const stale = await page.evaluate(
        async (revision) =>
          (
            await fetch(
              "/api/source?revision=" + encodeURIComponent(revision),
              {
                headers: {
                  Authorization:
                    "Bearer " + sessionStorage.getItem("meden-native-session"),
                },
              },
            )
          ).status,
        held!,
      );
      expect(stale).toBe(409);
      expect(requests).toHaveLength(0);
      await page.screenshot({
        path: "target/gui-checks/source-publication-latest.png",
      });
      await testInfo.attach("source-transfer.json", {
        body: JSON.stringify({
          status_bytes: JSON.stringify(newest).length,
          source_bytes: sourceQuery.bytes,
          initial_reads_before_manual_probe: 5,
          unchanged_navigation_reads: 0,
          stale_response_status: stale,
          chat_dom_unchanged: true,
        }),
        contentType: "application/json",
      });
      await stop();
      await page.goto(await start());
      const reopened = await nativeState(page);
      expect(reopened.authorized).toBe(false);
      expect(reopened.sources).toEqual({ current: null, files: {} });
      await setPaths(page, "Large.rs");
      await refreshReferences(page);
      await expect(
        page
          .locator(".ide-file-page:not([hidden])")
          .getByLabel("File contents"),
      ).toContainText("version_three");
      expect(requests).toHaveLength(0);
    } finally {
      releaseOld();
    }
  });

  test("open file tabs restore source, graph camera and explanations without another inspection", async ({
    page,
  }) => {
    test.setTimeout(180000);
    await writeFile(
      resolve(project, "Alpha.lean"),
      "axiom N_DD_AA : Nat\ntheorem alpha (h : N_DD_AA = 0) : N_DD_AA = 0 := h\n" +
        Array.from({ length: 100 }, (_, i) => `-- retained line ${i}`).join(
          "\n",
        ) +
        "\ntheorem later (h : N_DD_AA = 0) : N_DD_AA = 0 := alpha h\n",
    );
    await writeFile(
      resolve(project, "Beta.lean"),
      "theorem beta : True := True.intro\n",
    );
    await writeFile(
      resolve(project, "lakefile.toml"),
      'name = "retained"\n[[lean_lib]]\nname = "Tabs"\nroots = ["Alpha", "Beta"]\n',
    );
    // This journey starts from an installed, built Lake project, as Riemann does.
    const build = spawn(
      process.env.MEDEN_BIN || resolve("target/debug/meden"),
      [
        "lake",
        "--workspace",
        project,
        "--lean",
        process.env.MEDEN_ATLAS_LEAN ||
          resolve(homedir(), ".elan/toolchains/leanprover--lean4---v4.35.0"),
        "--timeout",
        "120",
        "--",
        "build",
        "+Alpha",
        "+Beta",
      ],
    );
    let buildOutput = "";
    build.stdout.on("data", (data) => (buildOutput += String(data)));
    build.stderr.on("data", (data) => (buildOutput += String(data)));
    expect((await once(build, "exit"))[0], buildOutput).toBe(0);
    const commands: string[] = [];
    page.on("request", (r) => {
      if (r.url().endsWith("/api/command"))
        commands.push(r.postDataJSON().type);
    });
    await page.goto(await start(0, { MEDEN_ATLAS_BACKEND: "lake" }));
    const coldStart = performance.now();
    await page
      .getByRole("button", { name: "File: Alpha.lean", exact: true })
      .click();
    const alpha = page.locator(".ide-file-page").filter({
      has: page.getByRole("region", {
        name: "File: Alpha.lean",
        exact: true,
      }),
    });
    const source = alpha.getByLabel("File contents");
    await expect
      .poll(
        async () =>
          (await nativeState(page)).source_preparation["Alpha.lean"]?.outcome
            .state,
        { timeout: 120000 },
      )
      .toBe("ready");
    await expect(source).toContainText("theorem alpha", { timeout: 120000 });
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    const coldPreparation = performance.now() - coldStart;
    const button = alpha.locator(".root-node > button");
    const margins = await button.evaluate((b) => {
      const outer = b.getBoundingClientRect(),
        inner = b.querySelector("strong")!.getBoundingClientRect();
      const scale = outer.height / (b as HTMLElement).offsetHeight;
      return [
        (inner.top - outer.top) / scale,
        (outer.bottom - inner.bottom) / scale,
      ];
    });
    expect(Math.min(...margins)).toBeGreaterThan(10);
    await source.evaluate((e) => {
      e.scrollTop = 160;
    });
    const scroll = await source.evaluate((e) => e.scrollTop);
    expect(scroll).toBeGreaterThan(0);
    const graph = alpha.locator("[data-layout=l4sb]");
    const box = await graph.boundingBox();
    await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
    await page.mouse.wheel(0, -240);
    await expect
      .poll(() => graph.locator(".react-flow__viewport").getAttribute("style"))
      .toContain("scale");
    await page.waitForTimeout(300);
    const camera = await graph
      .locator(".react-flow__viewport")
      .getAttribute("style");
    for (let i = 0; i < 3; i++) {
      await graph.locator(".root-node > button").hover();
      await page.mouse.move(box!.x + box!.width - 8, box!.y + 8);
      await expect(graph.locator(".react-flow__viewport")).toHaveAttribute(
        "style",
        camera!,
      );
    }
    await page.setViewportSize({ width: 1380, height: 960 });
    await expect
      .poll(async () => (await graph.boundingBox())!.width)
      .not.toBe(box!.width);
    await graph.evaluate(async () => {
      for (let i = 0; i < 8; i++) await new Promise(requestAnimationFrame);
    });
    await expect(graph.locator(".react-flow__viewport")).toHaveAttribute(
      "style",
      camera!,
    );
    await page.setViewportSize({ width: 1440, height: 960 });
    await expect
      .poll(async () => (await graph.boundingBox())!.width)
      .toBe(box!.width);
    await graph.evaluate((e) => {
      (window as any).__retainedAlpha = e;
    });
    await page
      .getByRole("button", { name: "File: Beta.lean", exact: true })
      .click();
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("File contents"),
    ).toContainText("theorem beta", { timeout: 120000 });
    await expect
      .poll(
        async () =>
          (await nativeState(page)).source_preparation["Beta.lean"]?.outcome
            .state,
        { timeout: 120000 },
      )
      .toBe("ready");
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    expect(Object.keys((await nativeSource(page)).files)).toEqual([
      "Beta.lean",
    ]);
    expect((await nativeSource(page)).inputs_revision).toMatch(
      /^[a-f0-9]{64}$/,
    );
    await page.evaluate(() => {
      const samples: number[] = [];
      (window as any).__tabTimings = samples;
      document.addEventListener(
        "click",
        (event) => {
          if (!(event.target as Element)?.closest('[role="tab"]')) return;
          const start = performance.now();
          requestAnimationFrame(() =>
            requestAnimationFrame(() =>
              samples.push(performance.now() - start),
            ),
          );
        },
        { capture: true },
      );
    });
    const before = commands.length;
    for (let i = 0; i < 3; i++) {
      await page.getByRole("tab", { name: "Project map", exact: true }).click();
      await page.getByRole("tab", { name: "Alpha.lean", exact: true }).click();
      await expect(source).toBeVisible({ timeout: 1000 });
      await expect(graph).toBeVisible({ timeout: 1000 });
      await expect(
        alpha.getByRole("region", {
          name: "Preparing Alpha.lean",
          exact: true,
        }),
      ).toHaveCount(0);
      await page.getByRole("tab", { name: "Beta.lean", exact: true }).click();
    }
    await page.getByRole("tab", { name: "Alpha.lean", exact: true }).click();
    await page
      .getByRole("button", { name: "File: Alpha.lean", exact: true })
      .click();
    expect(
      await graph.evaluate((e) => e === (window as any).__retainedAlpha),
    ).toBe(true);
    await expect(graph.locator(".react-flow__viewport")).toHaveAttribute(
      "style",
      camera!,
    );
    expect(await source.evaluate((e) => e.scrollTop)).toBe(scroll);
    expect(commands.slice(before)).toEqual([]);
    expect(requests).toHaveLength(0);
    await page.waitForTimeout(600);
    const tabTimings: number[] = await page.evaluate(
      () => (window as any).__tabTimings,
    );
    expect(tabTimings.length).toBeGreaterThanOrEqual(9);
    const p95 = [...tabTimings].sort((a, b) => a - b)[
      Math.floor(tabTimings.length * 0.95)
    ]!;
    await test.info().attach("navigation-performance", {
      body: Buffer.from(
        JSON.stringify(
          {
            browser: page.context().browser()?.version(),
            motion: process.env.MEDEN_GUI_MOTION || "reduce",
            cold_analysis_store_preparation_ms: coldPreparation,
            warm_tab_two_raf_ms: tabTimings,
            p95_ms: p95,
            maximum_p95_ms: 250,
            scope:
              "Installed Lean/built Lake fixture, empty analysis store then retained tabs. Two-rAF presentation proxy, not physical paint, INP, live inference or cold installation.",
          },
          null,
          2,
        ),
      ),
      contentType: "application/json",
    });
    if (process.env.MEDEN_GUI_PERFORMANCE === "1")
      expect(p95).toBeLessThan(250);
    const timings = await page.evaluate(async () => {
      const denied = await fetch("/api/metrics");
      const response = await fetch("/api/metrics", {
        headers: {
          Authorization:
            "Bearer " + sessionStorage.getItem("meden-native-session"),
        },
      });
      return { denied: denied.status, data: await response.json() };
    });
    expect(timings.denied).toBe(401);
    expect(timings.data.capacity).toBe(4096);
    expect(timings.data.records.length).toBeLessThanOrEqual(4096);
    const extraction = timings.data.records.find(
      (row: { name: string }) => row.name === "analysis.extract",
    );
    expect(extraction).toBeDefined();
    expect(extraction.thread_cpu_us).toBeGreaterThanOrEqual(0);
    expect(
      timings.data.records.some(
        (row: { name: string; context: { trace: number } }) =>
          row.name === "atlas.project.prepare" &&
          row.context.trace === extraction.context.trace,
      ),
    ).toBe(true);
    expect(JSON.stringify(timings)).not.toContain("Alpha.lean");
    expect(JSON.stringify(timings)).not.toContain("NATIVE-FIXTURE-NOT-SAVED");
    await source.evaluate((e) => {
      e.scrollTop = 0;
    });
    await alpha
      .getByRole("button", { name: "Explain alpha", exact: true })
      .click();
    const meaning = page.getByRole("region", {
      name: "Meaning: alpha",
      exact: true,
    });
    await expect(meaning).toBeVisible();
    const font = await meaning
      .locator(".semantic-heading code")
      .evaluate((e) => getComputedStyle(e).fontFamily);
    expect(font).toContain("monospace");
    await page.screenshot({
      path: "target/gui-checks/retained-tabs-and-identifiers.png",
    });
    await page.mouse.move(1300, 700);
    await graph
      .getByRole("button", { name: /alpha.*: select/ })
      .first()
      .click();
    await alpha
      .getByRole("textbox", { name: "Requirement to compare", exact: true })
      .fill("Review alpha from its retained original graph");
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Use this model", exact: true })
      .click();
    responses.push(
      (r) =>
        reply(
          r,
          tool("record_informalization", {
            lemmaName: "alpha",
            naturalLanguage: "The supplied equality is returned.",
            preconditions: "The equality holds.",
            postcondition: "The same equality holds.",
            scope: "One theorem",
            strength: "trivial",
            confidence: 1,
          }),
        ),
      (r) =>
        reply(
          r,
          tool("record_comparison", {
            lemmaName: "alpha",
            match: true,
            discrepancy: "",
            weakeningType: "none",
            explanation: "The supplied equality is retained.",
          }),
        ),
    );
    await alpha
      .getByRole("button", { name: "Analyze declaration meaning", exact: true })
      .click();
    await expect(
      alpha.getByLabel("Meaning analysis result", { exact: true }),
    ).toContainText("Candidate match", { timeout: 60000 });
    expect(requests).toHaveLength(2);
    expect((await nativeAnalysis(page))?.input.target.file).toBe("Alpha.lean");
    expect(hasCurrentEvidence(await projectEvidence(page))).toBe(false);
    await graph.getByRole("button", { name: "Fit View", exact: true }).click();
    await graph.getByRole("button", { name: /later.*: select/ }).click();
    await expect(source.locator('[aria-current="location"]')).toContainText(
      "theorem later",
    );
    // Selecting a line already in this page must not replace/re-highlight it.
    await expect(source.locator(".editor-line").first()).toContainText(
      "axiom N_DD_AA",
    );
    expect(requests).toHaveLength(2);
  });

  test("bounded file availability preserves literal pages, unreadable inventory and retry across restart without analysis", async ({
    page,
  }) => {
    test.setTimeout(120000);
    const text = Array.from(
      { length: 400 },
      (_, i) => `${i + 1}: λ日本語`,
    ).join("\n");
    for (const [file, content] of [
      ["notes.txt", text],
      ["Empty.txt", ""],
      ["Trailing.txt", "line\r\n".repeat(200)],
      ["literal.json", '"literal JSON string"'],
      ["Missing.txt", "original"],
      ["Valid.rs", "fn indexed() {}\n"],
      ["Binary.lean", Buffer.from([65, 0, 66])],
      ["Encoding.rs", Buffer.from([0xff])],
      ["Huge.lean", Buffer.alloc(1_048_577, 65)],
    ] as const)
      await writeFile(resolve(project, file), content);
    const commands: string[] = [];
    page.on("request", (request) => {
      if (request.url().endsWith("/api/command"))
        commands.push(request.postDataJSON()?.type);
    });
    await page.goto(await start());
    await expect
      .poll(async () => (await nativeState(page)).overview.state)
      .toBe("ready");
    expect((await nativeState(page)).overview.total).toBe(4);
    const map = page.getByRole("region", { name: "Project map", exact: true });
    await expect(map).toBeVisible();
    const chat = page.getByRole("complementary", {
      name: "Workspace chat",
      exact: true,
    });
    const before = await chat.innerHTML();
    const open = async (file: string) => {
      await page
        .getByRole("button", { name: "File: " + file, exact: true })
        .click();
      await expect
        .poll(async () => {
          const s = await nativeState(page);
          return !s.running && Boolean(s.readers.files[file]);
        })
        .toBe(true);
    };
    const source = page.locator(".ide-file-page:not([hidden])");
    await open("notes.txt");
    await expect(
      source.getByLabel("File contents").locator("code").first(),
    ).toHaveText("1: λ日本語");
    await expect(
      source.getByRole("button", { name: "Previous page", exact: true }),
    ).toBeDisabled();
    const beforePaging = commands.length;
    await source
      .getByRole("button", { name: "Next page", exact: true })
      .click();
    await expect(
      source.getByLabel("File contents").locator("code").first(),
    ).toHaveText("201: λ日本語");
    await expect(
      source.getByRole("button", { name: "Next page", exact: true }),
    ).toBeDisabled();
    await source
      .getByRole("button", { name: "Previous page", exact: true })
      .click();
    await expect(
      source.getByLabel("File contents").locator("code").first(),
    ).toHaveText("1: λ日本語");
    expect(commands).toHaveLength(beforePaging);
    await open("Empty.txt");
    await expect(source.getByLabel("File contents").locator("code")).toHaveText(
      "",
    );
    await expect(
      source.getByRole("button", { name: "Next page", exact: true }),
    ).toBeDisabled();
    await open("Trailing.txt");
    await source
      .getByRole("button", { name: "Next page", exact: true })
      .click();
    await expect(
      source.getByLabel("File contents").locator(".editor-line"),
    ).toHaveCount(1);
    await expect(
      source.getByLabel("File contents").locator(".editor-line"),
    ).toContainText("201:");
    await expect(source.getByLabel("File contents").locator("code")).toHaveText(
      "",
    );
    await expect(
      source.getByRole("button", { name: "Next page", exact: true }),
    ).toBeDisabled();
    await open("literal.json");
    await expect(source.getByLabel("File contents").locator("code")).toHaveText(
      '"literal JSON string"',
    );
    for (const [file, reason] of [
      ["Binary.lean", "This is a binary file."],
      ["Encoding.rs", "This file is not supported UTF-8 text."],
      ["Huge.lean", "This file exceeds the 1 MiB reader limit."],
    ]) {
      await open(file!);
      await expect(source.getByRole("status")).toHaveText(reason!);
      await expect(source.getByLabel("File contents")).toHaveCount(0);
      await expect(source.locator(".cone-graph")).toHaveCount(0);
    }
    await rm(resolve(project, "Missing.txt"));
    await open("Missing.txt");
    await expect(source.getByRole("alert")).toContainText(
      "This file could not be read.",
    );
    expect((await nativeReader(page, "Missing.txt")).reading.state).toBe(
      "unavailable",
    );
    await writeFile(resolve(project, "Missing.txt"), "recovered source");
    await source.getByRole("button", { name: "Retry", exact: true }).click();
    await expect(source.getByLabel("File contents")).toContainText(
      "recovered source",
    );
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    const beforeReturn = commands.length;
    await page.getByRole("tab", { name: "notes.txt", exact: true }).click();
    await expect(
      source.getByLabel("File contents").locator("code").first(),
    ).toHaveText("1: λ日本語");
    expect(commands).toHaveLength(beforeReturn);
    expect((await nativeState(page)).source_preparation).toEqual({});
    expect(await nativeSource(page)).toBeNull();
    expect(await chat.innerHTML()).toBe(before);
    expect(requests).toHaveLength(0);
    await page.screenshot({
      path: "target/gui-checks/file-reader-availability.png",
    });
    await stop();
    await page.goto(await start());
    await open("notes.txt");
    await expect(
      source.getByLabel("File contents").locator("code").first(),
    ).toHaveText("1: λ日本語");
    expect((await nativeState(page)).authorized).toBe(false);
    expect(requests).toHaveLength(0);
  });

  test("reader publications recover locally and delayed old bodies cannot replace a refreshed file", async ({
    page,
  }) => {
    await writeFile(resolve(project, "pages.txt"), "original source");
    await page.goto(await start());
    let held = false,
      releaseOld = () => {};
    const release = new Promise<void>((resolve) => (releaseOld = resolve));
    await page.route("**/api/reader?revision=*", async (route) => {
      const response = await route.fetch();
      if (!held) {
        held = true;
        await release;
      }
      await route.fulfill({ response });
    });
    const open = () =>
      page
        .getByRole("button", { name: "File: pages.txt", exact: true })
        .click();
    const source = page.locator(".ide-file-page:not([hidden])");
    try {
      await open();
      await expect.poll(() => held).toBe(true);
      const old = (await nativeState(page)).readers.files["pages.txt"].revision;
      await writeFile(resolve(project, "pages.txt"), "changed source");
      await page
        .getByRole("button", { name: "Refresh files and map", exact: true })
        .click();
      await expect
        .poll(async () => (await nativeState(page)).running)
        .toBe(false);
      await open();
      await expect(source.getByLabel("File contents")).toContainText(
        "changed source",
      );
      releaseOld();
      await page.waitForTimeout(500);
      await expect(source.getByLabel("File contents")).toContainText(
        "changed source",
      );
      const obsolete = await page.request.get(
        new URL("/api/reader?revision=" + encodeURIComponent(old), url).href,
        {
          headers: {
            Authorization: "Bearer " + new URL(url).hash.split("=")[1],
          },
        },
      );
      expect(obsolete.status()).toBe(409);
      expect(requests).toHaveLength(0);
    } finally {
      releaseOld();
      await page.unrouteAll({ behavior: "wait" });
    }
    await stop();
    await page.goto(await start());
    let failures = 0;
    await page.route("**/api/reader?revision=*", async (route) => {
      if (failures++ === 0)
        return route.fulfill({
          status: 503,
          json: { error: "Reader temporarily unavailable" },
        });
      return route.continue();
    });
    await open();
    await expect(source.getByRole("alert")).toContainText(
      "Could not load the source.",
    );
    await page.screenshot({
      path: "target/gui-checks/reader-publication-failure.png",
    });
    await source
      .getByRole("button", { name: "Retry source", exact: true })
      .click();
    await expect(source.getByLabel("File contents")).toContainText(
      "changed source",
    );
    expect((await nativeState(page)).authorized).toBe(false);
    expect(requests).toHaveLength(0);
    await page.screenshot({
      path: "target/gui-checks/reader-publication-latest.png",
    });
  });

  test("long Unicode compiler failures stay file-local and status remains readable across restart", async ({
    page,
  }) => {
    test.setTimeout(180000);
    const path = "LongDiagnostic.lean";
    const source =
      'import Lean\n#eval (throw (IO.userError (String.join (List.replicate 6000 "𝕏"))) : IO Unit)\ntheorem readable : True := True.intro\n';
    await writeFile(resolve(project, path), source);
    await page.goto(await start());
    const chat = page.getByRole("complementary", {
      name: "Workspace chat",
      exact: true,
    });
    const before = await chat.innerHTML();
    const inspect = async () => {
      await page
        .getByRole("button", { name: "File: " + path, exact: true })
        .click();
      await expect(
        page
          .locator(".ide-file-page:not([hidden])")
          .getByLabel("File contents"),
      ).toContainText("theorem readable");
      await expect
        .poll(
          async () =>
            (await nativeState(page)).source_preparation[path]?.outcome.state,
          { timeout: 120000 },
        )
        .toBe("failed");
      const state = await nativeState(page);
      const progress = state.source_preparation[path];
      expect(progress.outcome.kind).toBe("operation");
      expect(progress.outcome.error).toContain("𝕏");
      expect(progress.outcome.error).toContain("diagnostic shortened");
      expect(Buffer.byteLength(progress.outcome.error)).toBeLessThanOrEqual(
        8000,
      );
      expect(progress.outcome.error.length).toBeLessThanOrEqual(8000);
      await expect(
        page
          .getByRole("region", { name: "File reference graph", exact: true })
          .getByRole("alert"),
      ).toContainText("stopped");
      await expect(page.getByText(/Connection unavailable\./)).toHaveCount(0);
      expect(state.authorized).toBe(false);
      expect(await chat.innerHTML()).toBe(before);
      expect(requests).toHaveLength(0);
      return progress;
    };
    const original = await inspect();
    expect(await readFile(resolve(project, path), "utf8")).toBe(source);
    await page.screenshot({
      path: "target/gui-checks/unicode-compiler-failure.png",
    });
    await stop();
    await page.goto(await start());
    const reopened = await inspect();
    expect(reopened.job).not.toBe(original.job);
  });

  test("build logs stream before completion, retain timeout and cancellation output, and retry the real compiler", async ({
    page,
  }, testInfo) => {
    test.setTimeout(300000);
    await writeFile(
      resolve(project, "lakefile.toml"),
      'name = "streamed"\n[[lean_lib]]\nname = "Entry"\nroots = ["Fast", "Slow", "Entry"]\n',
    );
    await writeFile(
      resolve(project, "Fast.lean"),
      "theorem fast : True := True.intro\n",
    );
    await writeFile(
      resolve(project, "Slow.lean"),
      `import Lean
import Fast
#eval do
  IO.FS.writeFile "/workspace/.lake/compiler-started" "started"
  while !(← System.FilePath.pathExists "/workspace/.lake/release-compiler") do
    IO.sleep 20
  IO.println (String.join (List.replicate 25000 "λ"))
theorem slow : True := fast
`,
    );
    await writeFile(
      resolve(project, "Entry.lean"),
      "import Slow\npartial def forever (n : Nat) : Nat := forever (n + 1)\naxiom longSearch : ∀ n : Nat, forever n = 0\naxiom supplied : False\ntheorem tracked : False := supplied\n#print axioms tracked\n" +
        Array.from(
          { length: 400 },
          (_, i) => `-- reader row ${i} λ日本語`,
        ).join("\n"),
    );
    const environment = {
      MEDEN_ATLAS_BACKEND: "lake",
      MEDEN_ATLAS_BUILD_TIMEOUT_SECONDS: "10",
    };
    let failLogTransfer = true;
    await page.route("**/api/preparation-log?revision=**", async (route) => {
      const response = await route.fetch();
      const body = await response.json();
      if (failLogTransfer && body.data?.text.includes("Built Fast")) {
        failLogTransfer = false;
        await route.fulfill({
          status: 503,
          contentType: "application/json",
          body: JSON.stringify({ error: "diagnostic transfer fixture" }),
        });
      } else await route.fulfill({ response });
    });
    await page.goto(await start(0, environment));
    const chat = page.getByRole("complementary", {
      name: "Workspace chat",
      exact: true,
    });
    const before = await chat.innerHTML();
    const sourceRequestedAt = performance.now();
    await page
      .getByRole("button", { name: "File: Entry.lean", exact: true })
      .click();
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("File contents"),
    ).toContainText("theorem tracked", { timeout: 5000 });
    const sourceVisibleMs = performance.now() - sourceRequestedAt;
    await testInfo.attach("source-before-analysis.json", {
      body: JSON.stringify({
        source_visible_ms: sourceVisibleMs,
        budget_ms: 5000,
        methodology:
          "Real offline reader and a deliberately held installed Lean compiler; no complete semantic snapshot is available yet.",
      }),
      contentType: "application/json",
    });
    await expect
      .poll(
        () =>
          readFile(resolve(project, ".lake/compiler-started"), "utf8").catch(
            () => "",
          ),
        { timeout: 120000 },
      )
      .toBe("started");
    const loading = page.getByRole("region", {
      name: "Preparing Entry.lean",
      exact: true,
    });
    await expect(
      loading.getByRole("button", { name: "Retry log", exact: true }),
    ).toBeVisible();
    await page.screenshot({
      path: "target/gui-checks/build-log-transfer-failure.png",
    });
    const logRetryCommands: string[] = [];
    const observeLogRetry = (request: import("@playwright/test").Request) => {
      if (request.url().endsWith("/api/command"))
        logRetryCommands.push(request.postDataJSON().type);
    };
    page.on("request", observeLogRetry);
    await loading
      .getByRole("button", { name: "Retry log", exact: true })
      .click();
    await expect(loading.getByRole("log", { name: "Build log" })).toContainText(
      "Built Fast",
    );
    page.off("request", observeLogRetry);
    expect(logRetryCommands).toEqual([]);
    const preparingStatus = await nativeState(page);
    expect(typeof preparingStatus.source_preparation["Entry.lean"].log).toBe(
      "object",
    );
    expect(preparingStatus.source_preparation["Entry.lean"]).not.toHaveProperty(
      "log_truncated",
    );
    expect(JSON.stringify(preparingStatus.source_preparation)).not.toContain(
      "Built Fast",
    );
    const retiredLog =
      preparingStatus.source_preparation["Entry.lean"].log.revision;
    const originalJob = preparingStatus.source_preparation["Entry.lean"].job;
    // Reading the original source must not wait for the held compiler or search.
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("File contents"),
    ).toContainText("theorem tracked");
    expect(
      (await nativeState(page)).source_preparation["Entry.lean"].outcome.state,
    ).toBe("preparing");
    expect(await nativeSource(page)).toBeNull();
    const pageCommands: string[] = [];
    const pageBodies: string[] = [];
    const observePage = (request: import("@playwright/test").Request) => {
      if (request.url().endsWith("/api/command"))
        pageCommands.push(request.postDataJSON().type);
      if (/\/api\/(reader|source)\?/.test(request.url()))
        pageBodies.push(request.url());
    };
    page.on("request", observePage);
    const pageTimes: number[] = [];
    for (const direction of [
      "Next page",
      "Previous page",
      "Next page",
      "Previous page",
    ]) {
      const observation = await page.evaluate(async (direction) => {
        const section = document.querySelector(".ide-file-page:not([hidden])")!;
        const button = [...section.querySelectorAll("button")].find(
          (b) => b.textContent?.trim() === direction,
        )!;
        const start = performance.now();
        button.click();
        await new Promise<void>((resolve) =>
          requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
        );
        return {
          elapsed_ms: performance.now() - start,
          first_line: section.querySelector(".editor-line")?.textContent,
        };
      }, direction);
      expect(observation.first_line).toContain(
        direction === "Next page" ? "201:" : "1:",
      );
      pageTimes.push(observation.elapsed_ms);
      await expect(
        page.locator(".ide-file-page:not([hidden]) .editor-line").first(),
      ).toContainText(direction === "Next page" ? "201:" : "1:");
    }
    page.off("request", observePage);
    if (process.env.MEDEN_GUI_PERFORMANCE === "1")
      expect(Math.max(...pageTimes)).toBeLessThan(250);
    expect(pageCommands).toEqual([]);
    expect(pageBodies).toEqual([]);
    expect(
      (await nativeState(page)).source_preparation["Entry.lean"].outcome.state,
    ).toBe("preparing");
    await testInfo.attach("paging-during-compilation.json", {
      body: JSON.stringify({
        page_to_two_raf_ms: pageTimes,
        commands: pageCommands,
        body_reads: pageBodies,
        scope:
          "Actual held Lean compiler; DOM page and two animation callbacks, not physical presentation.",
      }),
      contentType: "application/json",
    });
    expect(await chat.innerHTML()).toBe(before);
    await page.screenshot({
      path: "target/gui-checks/build-log-streaming.png",
    });
    const failed = async () => {
      await expect
        .poll(
          async () =>
            (await nativeState(page)).source_preparation["Entry.lean"]?.outcome
              .state,
          { timeout: 30000 },
        )
        .toBe("failed");
      await expect
        .poll(async () => (await nativeState(page)).running)
        .toBe(false);
    };
    await failed();
    expect(
      (await nativeState(page)).source_preparation["Entry.lean"].outcome.kind,
    ).toBe("operation");
    const pane = page.getByRole("region", {
      name: "File reference graph",
      exact: true,
    });
    await expect(pane.getByRole("alert")).toContainText(
      "Building Lean dependencies reached the 10-second time limit",
    );
    await expect(pane.getByRole("log", { name: "Build log" })).toContainText(
      "Built Fast",
    );
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("File contents"),
    ).toContainText("theorem tracked");
    expect(await chat.innerHTML()).toBe(before);
    await page.screenshot({ path: "target/gui-checks/build-log-timeout.png" });
    await pane
      .getByRole("button", { name: "Retry preparation", exact: true })
      .click();
    await expect(loading.getByRole("log", { name: "Build log" })).toContainText(
      "$ lake --json query +Entry +Entry:lean +Entry:olean +Entry:setup",
      { timeout: 120000 },
    );
    const replacement = await nativeState(page);
    expect(replacement.source_preparation["Entry.lean"].job).not.toBe(
      originalJob,
    );
    expect(
      await page.evaluate(
        async (id) =>
          (
            await fetch(
              "/api/preparation-log?revision=" + encodeURIComponent(id),
              {
                headers: {
                  Authorization:
                    "Bearer " + sessionStorage.getItem("meden-native-session"),
                },
              },
            )
          ).status,
        retiredLog,
      ),
    ).toBe(409);
    await loading
      .getByRole("button", { name: "Stop preparation", exact: true })
      .click();
    await failed();
    expect(
      (await nativeState(page)).source_preparation["Entry.lean"].outcome.kind,
    ).toBe("cancelled");
    await expect(pane.getByRole("alert")).toContainText("was cancelled");
    expect(await chat.innerHTML()).toBe(before);
    // Release the explicit compiler gate; no source/configuration repair is needed.
    await writeFile(resolve(project, ".lake/release-compiler"), "release");
    await pane
      .getByRole("button", { name: "Retry preparation", exact: true })
      .click();
    const ready = async () => {
      await expect
        .poll(
          async () =>
            (await nativeState(page)).source_preparation["Entry.lean"]?.outcome
              .state,
          { timeout: 120000 },
        )
        .toBe("ready");
      await expect
        .poll(async () => (await nativeState(page)).running)
        .toBe(false);
      return nativeState(page);
    };
    const state = await ready();
    const stateSource = await nativeSource(page, state);
    const diagnostic = await nativePreparationLog(page, state, "Entry.lean");
    const statusBytes = Buffer.byteLength(
      JSON.stringify(state.source_preparation),
    );
    expect(statusBytes).toBeLessThan(2000);
    expect(Buffer.byteLength(diagnostic.text)).toBeLessThanOrEqual(65536);
    expect(Buffer.byteLength(diagnostic.text)).toBeGreaterThan(60000);
    await testInfo.attach("preparation-transfer.json", {
      body: JSON.stringify({
        progress_projection_bytes: statusBytes,
        diagnostic_tail_bytes: Buffer.byteLength(diagnostic.text),
        job: state.source_preparation["Entry.lean"].job,
        resource: diagnostic.resource,
        log_revision: state.source_preparation["Entry.lean"].log.revision,
        truncated: diagnostic.truncated,
        retry_commands: logRetryCommands,
        scope:
          "Actual completed Lean/Lake fixture; JSON-encoded status projection bytes and original diagnostic UTF-8 bytes. Not an old/new latency comparison.",
      }),
      contentType: "application/json",
    });

    expect(
      (await nativePreparationLog(page, state, "Entry.lean")).truncated,
    ).toBe(true);
    expect(
      stateSource.graph.declarations.find((n: any) => n.id === "tracked")
        .axioms,
    ).toEqual(["supplied"]);
    expect(
      stateSource.graph.declarations.find((n: any) => n.id === "longSearch")
        .semantic.checker.status,
    ).toBe("budget_exhausted");
    expect(await chat.innerHTML()).toBe(before);
    expect(requests).toHaveLength(0);
    await page.unrouteAll({ behavior: "wait" });
    await stop();
    await page.goto(await start(0, environment));
    await page
      .getByRole("button", { name: "File: Entry.lean", exact: true })
      .click();
    const reopened = await ready();
    expect(reopened.source_preparation["Entry.lean"].outcome.cache_hit).toBe(
      true,
    );
    expect(reopened.authorized).toBe(false);
  });

  test("prepared Lake dependencies bypass source-copy limits, refresh exact artifacts and survive restart", async ({
    page,
  }) => {
    test.setTimeout(300000);
    const lean =
      process.env.MEDEN_ATLAS_LEAN ||
      resolve(homedir(), ".elan/toolchains/leanprover--lean4---v4.35.0");
    const configuration =
      'name = "prepared"\n[[lean_lib]]\nname = "App"\nroots = ' +
      JSON.stringify([
        "Entry",
        ...Array.from({ length: 34 }, (_, i) => `Part${i}`),
      ]) +
      "\n";
    await writeFile(resolve(project, "lakefile.toml"), configuration);
    for (let i = 0; i < 34; i++)
      await writeFile(
        resolve(project, `Part${i}.lean`),
        `def part${i} : Nat := 1\n`,
      );
    const source =
      Array.from({ length: 34 }, (_, i) => `import Part${i}`).join("\n") +
      "\ntheorem fromLibrary : part0 = part0 := rfl\ntheorem sampleCounterexample (n : Nat) : n + 1 ≤ n := by sorry\n";
    await writeFile(resolve(project, "Entry.lean"), source);
    const build = async () => {
      const worker = spawn(
        process.env.MEDEN_BIN || resolve("target/debug/meden"),
        [
          "lake",
          "--workspace",
          project,
          "--lean",
          lean,
          "--timeout",
          "120",
          "--",
          "build",
          "+Entry",
        ],
      );
      let output = "",
        error = "";
      worker.stdout.on("data", (data) => (output += String(data)));
      worker.stderr.on("data", (data) => (error += String(data)));
      const [code] = await once(worker, "exit");
      expect(code, error + output).toBe(0);
      expect(JSON.parse(output).success).toBe(true);
    };
    await build();
    await page.goto(await start(0, { MEDEN_ATLAS_BACKEND: "lake" }));
    const open = () =>
      page
        .getByRole("button", { name: "File: Entry.lean", exact: true })
        .click();
    const prepared = async () => {
      await expect
        .poll(
          async () =>
            (await nativeState(page)).source_preparation["Entry.lean"]?.outcome
              .state,
          { timeout: 120000 },
        )
        .toBe("ready");
      await expect
        .poll(async () => (await nativeState(page)).running)
        .toBe(false);
      return nativeState(page);
    };
    const extractionStages = async () => {
      const metrics = await page.evaluate(async () =>
        (
          await fetch("/api/metrics", {
            headers: {
              Authorization:
                "Bearer " + sessionStorage.getItem("meden-native-session"),
            },
          })
        ).json(),
      );
      const trace = metrics.records.findLast(
        (r: any) => r.name === "analysis.extract",
      ).context.trace;
      return metrics.records
        .filter((r: any) => r.context.trace === trace)
        .map((r: any) => r.name) as string[];
    };
    await open();
    let state = await prepared();
    let stateSource = await nativeSource(page, state);
    expect(await extractionStages()).toContain("atlas.lake_configuration");
    expect(Object.keys(stateSource.files)).toEqual(["Entry.lean"]);
    expect(stateSource.inputs_revision).toMatch(/^[a-f0-9]{64}$/);
    expect(stateSource.external).toContain("Part33");
    expect(
      stateSource.graph.declarations
        .find((n: any) => n.id === "sampleCounterexample")
        .semantic.checker.candidates.join(" "),
    ).toContain("n := 0");
    const first = stateSource;
    const firstViews = state.sources.files;
    let inspections = 0;
    page.on("request", (request) => {
      if (
        !request.url().endsWith("/api/command") ||
        request.method() !== "POST"
      )
        return;
      const input = request.postDataJSON();
      if (input.type === "inspect" && input.path === "Entry.lean")
        inspections++;
    });
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    const reread = async () => {
      const before = inspections;
      await page
        .getByRole("button", { name: "Refresh files and map", exact: true })
        .click();
      await expect
        .poll(async () => (await nativeState(page)).running)
        .toBe(false);
      await open();
      await expect.poll(() => inspections).toBe(before + 1);
    };
    // Reproduce a late status view after an explicit refresh. The backend,
    // compilation, source contents and input fingerprints remain real.
    await page.route("**/api/state", async (route) => {
      const response = await route.fetch();
      const status = await response.json();
      await route.fulfill({
        response,
        json: { ...status, sources: { ...status.sources, files: firstViews } },
      });
    });
    await reread();
    await expect.poll(() => inspections).toBe(1);
    await expect
      .poll(
        async () =>
          (await nativeState(page)).source_preparation["Entry.lean"]?.outcome
            .cache_hit,
        { timeout: 120000 },
      )
      .toBe(true);
    await page.unrouteAll({ behavior: "wait" });
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    await expect(
      page.locator(".ide-file-page:not([hidden]) .native-source-graph"),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Retry graph", exact: true }),
    ).toHaveCount(0);
    let stages = await extractionStages();
    expect(stages).toContain("atlas.lake_configuration.reuse");
    expect(stages).not.toContain("atlas.lake_configuration");
    expect(stages).toContain("atlas.project.prepare");
    const cachedLog = (
      await nativePreparationLog(page, await nativeState(page), "Entry.lean")
    ).text;
    expect(cachedLog.match(/\$ lake --json query /g)).toHaveLength(1);
    expect(cachedLog).toContain("Build/setup and input resolution completed");
    expect(cachedLog).toMatch(/Validated \d+ files \/ \d+ bytes in /);

    // A cached successful configuration cannot hide a new parser failure.
    // Failed evaluations must not become reusable cache entries on retry.
    await writeFile(resolve(project, "lakefile.toml"), "name =\n");
    await reread();
    const pane = page.getByRole("region", {
      name: "File reference graph",
      exact: true,
    });
    for (let attempt = 0; attempt < 2; attempt++) {
      if (attempt)
        await pane
          .getByRole("button", { name: "Retry preparation", exact: true })
          .click();
      await expect
        .poll(
          async () =>
            (await nativeState(page)).source_preparation["Entry.lean"]?.outcome
              .state,
          { timeout: 120000 },
        )
        .toBe("failed");
      await expect
        .poll(async () => (await nativeState(page)).running)
        .toBe(false);
      expect(await nativeSource(page)).toBeNull();
      expect(await extractionStages()).toContain("atlas.lake_configuration");
      await expect(pane.getByRole("alert")).toContainText(
        "Offline Lake configuration failed",
      );
    }
    await writeFile(resolve(project, "lakefile.toml"), configuration);
    await pane
      .getByRole("button", { name: "Retry preparation", exact: true })
      .click();
    state = await prepared();
    stateSource = await nativeSource(page, state);
    expect(state.source_preparation["Entry.lean"].outcome.cache_hit).toBe(true);
    expect(await extractionStages()).not.toContain("atlas.lake_configuration");

    await writeFile(resolve(project, "Part0.lean"), "def part0 : Nat := 2\n");
    await reread();
    await expect
      .poll(async () => (await nativeSource(page))?.graph?.revision, {
        timeout: 120000,
      })
      .toBeTruthy();
    await expect
      .poll(async () => (await nativeSource(page))?.graph?.revision, {
        timeout: 120000,
      })
      .not.toBe(first.graph.revision);
    state = await prepared();
    stateSource = await nativeSource(page, state);
    expect(stateSource.inputs_revision).not.toBe(first.inputs_revision);
    expect(state.source_preparation["Entry.lean"].outcome.cache_hit).toBe(
      false,
    );
    stages = await extractionStages();
    expect(stages).not.toContain("atlas.lake_configuration");
    expect(stages).toContain("atlas.project.prepare");
    expect(stages).toContain("atlas.project.extract");
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    await writeFile(resolve(project, "Part0.lean"), "invalid lean syntax }\n");
    await reread();
    await expect
      .poll(
        async () =>
          (await nativeState(page)).source_preparation["Entry.lean"]?.outcome
            .state,
        { timeout: 120000 },
      )
      .toBe("failed");
    expect(await nativeSource(page)).toBeNull();
    await expect(pane.getByRole("alert")).toContainText(
      "Prepared Lake compilation failed",
    );
    await expect(
      page
        .getByRole("complementary", { name: "Workspace chat", exact: true })
        .getByRole("alert"),
    ).toHaveCount(0);
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    await writeFile(resolve(project, "Part0.lean"), "def part0 : Nat := 2\n");
    await pane
      .getByRole("button", { name: "Retry preparation", exact: true })
      .click();
    await prepared();
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    await stop();
    await page.goto(await start(0, { MEDEN_ATLAS_BACKEND: "lake" }));
    await open();
    state = await prepared();
    stateSource = await nativeSource(page, state);
    expect(state.source_preparation["Entry.lean"].outcome.cache_hit).toBe(true);
    stages = await extractionStages();
    expect(stages).toContain("atlas.lake_configuration.reuse");
    expect(stages).not.toContain("atlas.lake_configuration");
    expect(stages).toContain("atlas.project.prepare");
    await page.screenshot({
      path: test
        .info()
        .outputPath("lake-configuration-reused-after-restart.png"),
    });
    expect(requests).toHaveLength(0);
    expect((await nativeState(page)).authorized).toBe(false);
  });

  test("large project map loads declaration nodes on zoom without opening files or changing chat", async ({
    page,
  }) => {
    test.setTimeout(180000);
    for (let group = 0; group <= 10; group++)
      await mkdir(resolve(project, "Many/Group" + group), { recursive: true });
    for (let start = 0; start < 10001; start += 200)
      await Promise.all(
        Array.from({ length: Math.min(200, 10001 - start) }, (_, i) =>
          writeFile(
            resolve(
              project,
              "Many/Group" + Math.floor((start + i) / 1000),
              "File" + (start + i) + ".lean",
            ),
            "theorem identity" + (start + i) + " : True := True.intro\n",
          ),
        ),
      );
    await writeFile(
      resolve(project, "Many/Group0/File0.lean"),
      "import Many.Group0.File2\ntheorem identity0 : True := identity1\n",
    );
    await writeFile(
      resolve(project, "Many/Group0/File2.lean"),
      "import Many.Group0.File1\n",
    );
    await writeFile(
      resolve(project, "Many/Group0/File3.lean"),
      Buffer.from([255]),
    );
    const commands: string[] = [];
    const indexedPaths: string[] = [];
    const mapReadTimes: number[] = [];
    page.on("request", (request) => {
      const requestUrl = new URL(request.url());
      if (requestUrl.pathname === "/api/map")
        indexedPaths.push(requestUrl.searchParams.get("path")!);
      if (request.url().endsWith("/api/command")) {
        commands.push(request.postDataJSON().type);
      }
    });
    page.on("requestfinished", (request) => {
      if (new URL(request.url()).pathname === "/api/map")
        mapReadTimes.push(request.timing().responseEnd);
    });
    await page.goto(await start());
    await expect
      .poll(async () => (await nativeState(page)).overview.state, {
        timeout: 60000,
      })
      .toBe("ready");
    const inventory = await page.evaluate(async () =>
      (
        await fetch(
          "/api/inventory?revision=" +
            encodeURIComponent(
              (
                await fetch("/api/state", {
                  headers: {
                    Authorization:
                      "Bearer " +
                      sessionStorage.getItem("meden-native-session"),
                  },
                }).then((r) => r.json())
              ).overview.revision,
            ),
          {
            headers: {
              Authorization:
                "Bearer " + sessionStorage.getItem("meden-native-session"),
            },
          },
        )
      )
        .json()
        .then((publication) => publication.data),
    );
    expect(inventory.files).toHaveLength(10001);
    expect(inventory.inventory_only).toBe(true);
    expect(inventory.files.every((f: { indexed: boolean }) => !f.indexed)).toBe(
      true,
    );
    const map = page.getByRole("region", { name: "Project map", exact: true });
    const chat = page.getByRole("complementary", {
      name: "Workspace chat",
      exact: true,
    });
    const chatBefore = await chat.innerHTML();
    await expect(
      map.getByLabel("Declaration types", { exact: true }),
    ).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Folder: Many", exact: true }),
    ).toBeVisible();
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    const beforeCommands = commands.length;
    const beforeInspection = (await nativeState(page)).directories;
    const originalCanvas = await map
      .locator(".workspace-overview-canvas")
      .boundingBox();
    const zoomFile = async (name: string) => {
      await map
        .getByRole("button", { name: "Category: Many/Group0", exact: true })
        .click();
      const file = map.locator(`[data-map-path="Many/Group0/${name}.lean"]`);
      await expect(file).toBeVisible();
      const box = await file.boundingBox();
      await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
      await page.mouse.wheel(0, -600);
      await file.click();
    };
    await zoomFile("File0");
    const declaration = map.getByRole("button", {
      name: /Declaration: identity0 · Many\/Group0\/File0.lean:/,
    });
    await expect(declaration).toBeVisible({ timeout: 30000 });
    await expect(
      map.getByLabel("Declaration types", { exact: true }),
    ).toBeVisible();
    await expect(
      map.getByRole("navigation", { name: "Map location" }),
    ).toContainText("File0.lean");
    // Normal-motion focus animates deliberately; retain the settled camera,
    // rather than treating an intermediate animation frame as the final view.
    const camera = await map
      .locator(".react-flow__viewport")
      .evaluate(async (element) => {
        let previous = element.getAttribute("style"),
          stable = 0;
        while (stable < 4) {
          await new Promise(requestAnimationFrame);
          const current = element.getAttribute("style");
          stable = current === previous ? stable + 1 : 0;
          previous = current;
        }
        return previous;
      });
    expect(
      (await map.locator(".workspace-overview-canvas").boundingBox())!.height,
    ).toBe(originalCanvas!.height);
    await expect(
      page.getByRole("tab", { name: "Project map", exact: true }),
    ).toHaveAttribute("aria-selected", "true");
    await expect(page.locator(".ide-file-tab")).toHaveCount(0);
    await expect(map.locator(".react-flow__viewport")).toHaveAttribute(
      "style",
      camera!,
    );
    expect(await chat.innerHTML()).toBe(chatBefore);
    expect(indexedPaths.length).toBeGreaterThan(0);
    expect(commands.slice(beforeCommands)).toEqual([]);
    expect(mapReadTimes.length).toBeGreaterThan(0);
    expect(Math.max(...mapReadTimes)).toBeLessThan(2000);
    expect(new Set(indexedPaths).size).toBe(indexedPaths.length);
    expect((await nativeState(page)).directories).toEqual(beforeInspection);
    await page.screenshot({
      path: "target/gui-checks/large-project-map-declarations.png",
    });
    await declaration.click();
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("File contents"),
    ).toContainText("theorem identity0", { timeout: 120000 });
    await expect
      .poll(
        async () =>
          (await nativeState(page)).source_preparation["Many/Group0/File0.lean"]
            ?.outcome.state,
        { timeout: 120000 },
      )
      .toBe("ready");
    await expect(
      page.locator(".ide-file-page:not([hidden]) [data-layout=l4sb]"),
    ).toBeVisible();
    await expect(
      page
        .locator(".ide-file-page:not([hidden]) .root-node")
        .getByRole("button", { name: /identity0.*: select/ }),
    ).toContainText("true");
    await expect(
      page.locator(".ide-file-page:not([hidden]) .radar-node"),
    ).toHaveCount(2);
    expect(
      (await nativeState(page)).source_preparation["Many/Group0/File0.lean"]
        .outcome.state,
    ).toBe("ready");
    expect(requests).toHaveLength(0);
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    await expect(declaration).toBeVisible();
    await expect(map.locator(".react-flow__viewport")).toHaveAttribute(
      "style",
      camera!,
    );
    expect(
      indexedPaths.filter((path) => path === "Many/Group0/File0.lean"),
    ).toHaveLength(1);
    await stop();
    await page.goto(await start());
    await expect(map.locator(".continuous-map-scene")).toBeVisible();
    await zoomFile("File3");
    await expect(
      map.getByRole("button", {
        name: "File: Many/Group0/File3.lean",
        exact: true,
      }),
    ).toBeVisible();
    await expect(map.getByRole("alert")).toHaveCount(0);
    await expect(chat.getByRole("alert")).toHaveCount(0);
    await expect(chat.getByRole("status")).toHaveCount(0);
    await expect(page.locator(".ide-file-tab")).toHaveCount(0);
    const failedCamera = await map
      .locator(".react-flow__viewport")
      .evaluate(async (element) => {
        let previous: string | null = null,
          stable = 0;
        for (let frame = 0; frame < 120; frame++) {
          await new Promise(requestAnimationFrame);
          const current = element.getAttribute("style");
          stable = current === previous ? stable + 1 : 0;
          if (stable >= 8) return current;
          previous = current;
        }
        throw new Error("The focused camera did not settle");
      });
    await page
      .getByRole("button", { name: "Folder: Many", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Folder: Many/Group0", exact: true })
      .click();
    await page
      .getByRole("region", { name: "File explorer", exact: true })
      .getByRole("button", {
        name: "File: Many/Group0/File3.lean",
        exact: true,
      })
      .click();
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByRole("status"),
    ).toHaveText("This file is not supported UTF-8 text.");
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    await page.screenshot({
      path: "target/gui-checks/large-project-map-unreadable-file.png",
    });
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    await expect(map.locator(".react-flow__viewport")).toHaveAttribute(
      "style",
      failedCamera!,
    );
    // An external edit is discovered through the public refresh control.
    await writeFile(
      resolve(project, "Many/Group0/File3.lean"),
      "theorem identity3 : True := True.intro\n",
    );
    await page
      .getByRole("button", { name: "Refresh files and map", exact: true })
      .click();
    await expect(
      map.getByRole("button", { name: /Declaration: identity3 ·/ }),
    ).toBeVisible({ timeout: 30000 });
    await expect(map.getByRole("alert")).toHaveCount(0);
    await expect(map.locator(".react-flow__viewport")).toHaveAttribute(
      "style",
      failedCamera!,
    );
    expect((await nativeState(page)).authorized).toBe(false);
    expect(requests).toHaveLength(0);
  });

  test("search settings reuse compiled modules and keep separately bound results across restart", async ({
    page,
  }) => {
    test.setTimeout(240000);
    await writeFile(
      resolve(project, "Claim.lean"),
      "theorem bad (n : Nat) : n + 1 ≤ n := by sorry\ndef twice (n : Nat) : Nat := n + n\n",
    );
    const open = async (seed: string) => {
      await page.goto(await start(0, { MEDEN_SEARCH_SEED: seed }));
      await page
        .getByRole("button", { name: "File: Claim.lean", exact: true })
        .click();
      await expect
        .poll(
          async () =>
            (await nativeState(page)).source_preparation["Claim.lean"]?.outcome
              .state,
          { timeout: 120000 },
        )
        .toBe("ready");
      await expect(page.getByLabel("File contents")).toContainText(
        "theorem bad",
      );
      return nativeState(page);
    };
    const index = async (role: string) =>
      JSON.parse(
        await readFile(
          resolve(
            root,
            "data/meden/projects",
            createHash("sha256").update(project).digest("hex"),
            "compiled-views",
            `${role}-index.json`,
          ),
          "utf8",
        ),
      );
    const metrics = () =>
      page.evaluate(async () =>
        (
          await fetch("/api/metrics", {
            headers: {
              Authorization:
                "Bearer " + sessionStorage.getItem("meden-native-session"),
            },
          })
        ).json(),
      );
    const first = await open("0");
    const firstSource = await nativeSource(page, first);
    const compilation = await index("observations");
    const originalSearch = await index("searches");
    expect(Object.values(compilation)).toHaveLength(1);
    expect(Object.values(originalSearch)).toHaveLength(1);
    const bad = (s: any) =>
      s.graph.declarations.find((n: any) => n.id === "bad");
    expect(bad(firstSource).semantic.checker).toMatchObject({
      status: "counterexample",
      seed: 0,
    });
    expect(
      firstSource.graph.declarations.find((n: any) => n.id === "twice").semantic
        .checker.status,
    ).toBe("not_proposition");
    await stop();
    const changed = await open("19");
    const changedSource = await nativeSource(page, changed);
    expect(changed.source_preparation["Claim.lean"].outcome.cache_hit).toBe(
      false,
    );
    expect(await index("observations")).toEqual(compilation);
    expect(Object.values(await index("searches"))).toHaveLength(2);
    expect(bad(changedSource).semantic.checker).toMatchObject({
      status: "counterexample",
      seed: 19,
    });
    expect(changedSource.revision).toBe(firstSource.revision);
    expect(changedSource.graph.revision).not.toBe(firstSource.graph.revision);
    const stages = (await metrics()).records.map((r: any) => r.name);
    expect(stages).toContain("atlas.search");
    expect(stages).not.toContain("atlas.compile");
    expect(stages).not.toContain("atlas.project.extract");
    expect(stages).not.toContain("atlas.execute");
    expect(stages).not.toContain("analysis.render");
    await page
      .getByRole("button", { name: "Explain bad", exact: true })
      .hover();
    await expect(
      page.getByRole("region", { name: "Meaning: bad", exact: true }),
    ).toContainText("Counterexample candidate");
    await page.screenshot({ path: "target/gui-checks/search-seed-update.png" });
    expect(changed.authorized).toBe(false);
    expect(requests).toHaveLength(0);
    await stop();
    const warm = await open("19");
    const warmSource = await nativeSource(page, warm);
    expect(warm.source_preparation["Claim.lean"].outcome.cache_hit).toBe(true);
    expect(bad(warmSource).semantic.checker).toEqual(
      bad(changedSource).semantic.checker,
    );
    expect((await metrics()).records.map((r: any) => r.name)).not.toContain(
      "atlas.search",
    );
    expect(warm.authorized).toBe(false);
    expect(requests).toHaveLength(0);
    await stop();
    // Corrupt host cache bytes, then use the ordinary file-open journey. The
    // compiler must repair its retained modules before a new search can use them.
    const raw = JSON.parse(
      await readFile(
        resolve(
          root,
          "data/meden/projects",
          createHash("sha256").update(project).digest("hex"),
          "compiled-views/observations",
          `${Object.values(compilation)[0]}.json`,
        ),
        "utf8",
      ),
    );
    const retained = raw.snapshot.atlas.compiled_modules;
    const module = Object.keys(retained.files).find((name) =>
      name.endsWith(".olean"),
    )!;
    await writeFile(
      resolve(
        root,
        "data/meden/atlas/compiled-modules",
        retained.id,
        "modules",
        module,
      ),
      "corrupt host cache",
    );
    const repaired = await open("20");
    const repairedSource = await nativeSource(page, repaired);
    expect(bad(repairedSource).semantic.checker).toMatchObject({
      status: "counterexample",
      seed: 20,
    });
    expect((await metrics()).records.map((r: any) => r.name)).toContain(
      "atlas.compile",
    );
    expect(repairedSource.revision).toBe(firstSource.revision);
    expect(requests).toHaveLength(0);
  });

  test("renderer updates reuse compiled observations, preserve counterexamples and reject changed sources", async ({
    page,
  }) => {
    test.setTimeout(300000);
    const rendererRoot = resolve(root, "renderer");
    await cp(
      process.env.MEDEN_INFORMATH_ROOT ||
        resolve(homedir(), ".local/share/meden/informath/0.4"),
      rendererRoot,
      { recursive: true },
    );
    const env = { MEDEN_INFORMATH_ROOT: rendererRoot };
    const original = "theorem bad (n : Nat) : n + 1 ≤ n := by sorry\n";
    await writeFile(resolve(project, "Claim.lean"), original);
    const preparationMs: number[] = [];
    const open = async () => {
      await page.goto(await start(0, env));
      const requestedAt = performance.now();
      await page
        .getByRole("button", { name: "File: Claim.lean", exact: true })
        .click();
      await expect
        .poll(
          async () =>
            (await nativeState(page)).source_preparation["Claim.lean"]?.outcome
              .state,
          { timeout: 120000 },
        )
        .toBe("ready");
      await expect
        .poll(async () => (await nativeState(page)).running)
        .toBe(false);
      await expect(page.getByLabel("File contents")).toContainText(
        "theorem bad",
      );
      await expect(
        page.locator(".ide-file-page:not([hidden]) .cone-graph"),
      ).toBeVisible();
      preparationMs.push(performance.now() - requestedAt);
      return nativeState(page);
    };
    const first = await open();
    const firstSource = await nativeSource(page, first);
    const firstNode = firstSource.graph.declarations.find(
      (n: any) => n.id === "bad",
    );
    expect(firstNode.semantic.checker.status).toBe("counterexample");
    expect(firstNode.semantic.renderer).toContain("Informath 0.4");
    const cacheRoot = resolve(
      root,
      "data/meden/projects",
      createHash("sha256").update(project).digest("hex"),
      "compiled-views",
    );
    const index = async (role: string) =>
      JSON.parse(
        await readFile(resolve(cacheRoot, `${role}-index.json`), "utf8"),
      );
    const originalIndex = await index("observations");
    const originalId = Object.values(originalIndex)[0];
    expect(Object.values(originalIndex)).toHaveLength(1);
    const originalArtifact = JSON.parse(
      await readFile(
        resolve(cacheRoot, "observations", `${originalId}.json`),
        "utf8",
      ),
    );
    expect(originalArtifact.compiler).toMatchObject({
      id: "lean.frozen",
      role: "compiler",
      version: "1",
    });
    expect(originalArtifact.compiler.output_schema_sha256).toMatch(
      /^[a-f0-9]{64}$/,
    );
    expect(
      originalArtifact.snapshot.atlas.graph.nodes[0].semantic.checker,
    ).toBeNull();
    const originalSearches = await index("searches");
    expect(Object.values(originalSearches)).toHaveLength(1);
    const searchId = Object.values(originalSearches)[0];
    const originalSearch = JSON.parse(
      await readFile(
        resolve(cacheRoot, "searches", `${searchId}.json`),
        "utf8",
      ),
    );
    expect(originalSearch.observation).toBe(originalId);
    expect(originalSearch.searcher).toMatchObject({
      id: "lean.plausible",
      role: "counterexample_search",
      version: "1",
    });
    expect(originalSearch.report.checks.bad).toEqual(
      firstNode.semantic.checker,
    );
    const firstReadings = await index("renderings");
    expect(Object.values(firstReadings)).toHaveLength(1);
    const firstReading = JSON.parse(
      await readFile(
        resolve(
          cacheRoot,
          "renderings",
          `${Object.values(firstReadings)[0]}.json`,
        ),
        "utf8",
      ),
    );
    expect(firstReading.renderer).toMatchObject({
      id: "informath.gf",
      role: "meaning_renderer",
      version: "1",
    });
    expect(firstReading.observation).toBe(originalId);
    await stop();

    // This is a host resource update, outside the model-writable project. The
    // actual pinned GF binary/grammar still run; no scripted renderer is used.
    const resource = resolve(rendererRoot, "share/baseconstants.dk");
    const bytes = Buffer.concat([await readFile(resource), Buffer.from("\n")]);
    await writeFile(resource, bytes);
    const manifestPath = resolve(rendererRoot, "manifest.json");
    const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
    manifest.files["share/baseconstants.dk"] = createHash("sha256")
      .update(bytes)
      .digest("hex");
    await writeFile(manifestPath, JSON.stringify(manifest));

    const rerendered = await open();
    const rerenderedSource = await nativeSource(page, rerendered);
    expect(rerendered.sources.current.revision).not.toBe(
      first.sources.current.revision,
    );
    const node = rerenderedSource.graph.declarations.find(
      (n: any) => n.id === "bad",
    );
    expect(rerenderedSource.revision).toBe(firstSource.revision);
    expect(rerenderedSource.graph.revision).not.toBe(
      firstSource.graph.revision,
    );
    expect(node.semantic.renderer).not.toBe(firstNode.semantic.renderer);
    expect(node.semantic.checker).toEqual(firstNode.semantic.checker);
    expect(node.semantic.intermediate).toBe(firstNode.semantic.intermediate);
    expect(rerendered.source_preparation["Claim.lean"].outcome.cache_hit).toBe(
      false,
    );
    expect(await index("observations")).toEqual(originalIndex);
    expect(await index("searches")).toEqual(originalSearches);
    const updatedReadings = await index("renderings");
    expect(Object.values(updatedReadings)).toHaveLength(2);
    const updated = await Promise.all(
      Object.values(updatedReadings).map(async (id) =>
        JSON.parse(
          await readFile(
            resolve(cacheRoot, "renderings", `${id}.json`),
            "utf8",
          ),
        ),
      ),
    );
    expect(updated.every((reading) => reading.observation === originalId)).toBe(
      true,
    );
    expect(
      new Set(updated.map((reading) => reading.renderer.configuration_sha256))
        .size,
    ).toBe(2);
    const metrics = await page.evaluate(async () =>
      (
        await fetch("/api/metrics", {
          headers: {
            Authorization:
              "Bearer " + sessionStorage.getItem("meden-native-session"),
          },
        })
      ).json(),
    );
    const stages = metrics.records.map((r: any) => r.name);
    expect(stages).toContain("analysis.render");
    expect(stages).not.toContain("atlas.compile");
    expect(stages).not.toContain("atlas.project.extract");
    expect(stages).not.toContain("atlas.execute");
    expect(stages).not.toContain("atlas.search");
    await page
      .getByRole("button", { name: "Explain bad", exact: true })
      .hover();
    await expect(
      page.getByRole("region", { name: "Meaning: bad", exact: true }),
    ).toContainText("Counterexample candidate");
    await page.screenshot({
      path: "target/gui-checks/renderer-update-reused-observation.png",
    });
    await page.mouse.move(20, 20);
    expect(rerendered.authorized).toBe(false);
    expect(requests).toHaveLength(0);
    await stop();

    // A complete reading cache also survives a new host without inference approval.
    const warm = await open();
    const warmSource = await nativeSource(page, warm);
    expect(warm.sources.current.revision).toBe(
      rerendered.sources.current.revision,
    );
    expect(warm.source_preparation["Claim.lean"].outcome.cache_hit).toBe(true);
    expect(warmSource.graph.revision).toBe(rerenderedSource.graph.revision);
    expect(warm.authorized).toBe(false);
    await test.info().attach("renderer-cache-preparation", {
      body: Buffer.from(
        JSON.stringify(
          {
            browser: page.context().browser()?.version(),
            cold_ms: preparationMs[0],
            renderer_update_ms: preparationMs[1],
            complete_cache_ms: preparationMs[2],
            scope:
              "File request to prepared DOM in a real installed-Lean/GF fixture. Diagnostic timing; cache reuse is established independently by native spans and preserved observations. Not cold installation, live inference or physical paint.",
          },
          null,
          2,
        ),
      ),
      contentType: "application/json",
    });
    // An actual edit cannot use that old compiler/search result to hide a failure.
    await writeFile(
      resolve(project, "Claim.lean"),
      original + "invalid Lean syntax }\n",
    );
    await setPaths(page, "Claim.lean");
    await compileDependencies(page);
    await expect
      .poll(
        async () =>
          (await nativeState(page)).source_preparation["Claim.lean"]?.outcome
            .state,
        { timeout: 120000 },
      )
      .toBe("failed");
    await expect(
      page.getByRole("button", { name: "Retry preparation", exact: true }),
    ).toBeVisible();
    expect(await nativeSource(page)).toBeNull();
    await page.screenshot({
      path: "target/gui-checks/renderer-cache-source-failure.png",
    });
    await writeFile(
      resolve(project, "Claim.lean"),
      "theorem bad (n : Nat) : n ≤ n := by omega\n",
    );
    await page
      .getByRole("button", { name: "Retry preparation", exact: true })
      .click();
    await expect
      .poll(
        async () =>
          (await nativeState(page)).source_preparation["Claim.lean"]?.outcome
            .state,
        { timeout: 120000 },
      )
      .toBe("ready");
    const repaired = await nativeState(page);
    const repairedSource = await nativeSource(page, repaired);
    expect(repairedSource.revision).not.toBe(firstSource.revision);
    expect(
      repairedSource.graph.declarations.find((n: any) => n.id === "bad")
        .semantic.checker.status,
    ).toBe("not_found");
    expect(requests).toHaveLength(0);
  });

  test("main targets stay visible at a distance and compiled English/counterexamples annotate source without inference", async ({
    page,
  }) => {
    test.setTimeout(360000);
    const original =
      [
        "import Shared",
        "def twice (n : Nat) : Nat := n + n",
        "def allowed (n : Nat) : Prop := n ≥ 2",
        "theorem aa_bad (n : Nat) : n + 1 ≤ n := by sorry",
        "theorem bb_good (n : Nat) (h : allowed n) : n ≥ 2 := h",
        "theorem cc_vacuous (n : Nat) (h : n < 0) : False := by omega",
        "opaque Token : Type := Nat",
        "axiom dd_opaque (x : Token) : False",
        "axiom zz_anyprop (p : Prop) : p",
        "theorem member_order (xs : List Nat) (x : Nat) (h : x ∈ xs) : x ∈ xs := h",
      ].join("\n") + "\n";
    await writeFile(resolve(project, "Main.lean"), original);
    await writeFile(resolve(project, "Shared.lean"), "def shared : Nat := 1\n");
    await mkdir(resolve(project, "Noise"));
    await mkdir(resolve(project, "Extra"));
    for (let start = 0; start < 10002; start += 200)
      await Promise.all(
        Array.from({ length: Math.min(200, 10002 - start) }, (_, offset) => {
          const i = start + offset;
          return writeFile(
            resolve(project, i < 5001 ? "Noise" : "Extra", `File${i}.lean`),
            `def value${i} : Nat := ${i}\n`,
          );
        }),
      );
    const commands: string[] = [];
    page.on("request", (request) => {
      if (request.url().endsWith("/api/command"))
        commands.push(request.postDataJSON().type);
    });
    await page.goto(await start());
    const map = page.getByRole("region", { name: "Project map", exact: true });
    const pin = map.getByRole("button", {
      name: "Main theorems: Main.lean (4)",
      exact: true,
    });
    await expect(pin).toBeVisible({ timeout: 120000 });
    await page.screenshot({
      path: "target/gui-checks/main-theorem-distant-map.png",
    });
    await pin.click();
    await map.getByRole("button", { name: /Declaration: aa_bad ·/ }).click();
    const source = page.getByRole("region", {
      name: "Source code",
      exact: true,
    });
    const meaning = page.getByRole("region", {
      name: "Meaning: aa_bad",
      exact: true,
    });
    const graph = page.getByRole("region", {
      name: "File reference graph",
      exact: true,
    });
    await expect(
      graph.getByRole("button", { name: /aa_bad.*: select/ }),
    ).toContainText(/for all natural numbers/i);
    await expect(source.getByLabel("File contents")).toContainText(
      "theorem aa_bad",
    );
    expect(commands).not.toContain("atlas_snapshot");
    await expect
      .poll(async () => (await nativeState(page)).running, { timeout: 120000 })
      .toBe(false);
    const first = await nativeState(page);
    const firstSource = await nativeSource(page, first);
    expect(first.stopped).toBeNull();
    const compiled = firstSource.graph.declarations;
    const observations = Object.fromEntries(
      compiled.map((n: any) => [n.id, n.semantic]),
    );
    expect(observations.aa_bad.checker.candidates.join(" ")).toContain(
      "n := 0",
    );
    expect(observations.aa_bad.checker.status).toBe("counterexample");
    expect(observations.member_order.sentence).toMatch(/element|belongs/);
    expect(observations.aa_bad.renderer).toContain("Informath 0.4");
    expect(observations.bb_good.checker.status).toBe("not_found");
    expect(observations.cc_vacuous.checker.status).toBe("gave_up");
    expect(observations.dd_opaque.checker.status).toBe("unsupported");
    expect(observations.zz_anyprop.checker.status).toBe("counterexample");
    expect(observations.twice.checker.status).toBe("not_proposition");
    expect(observations.allowed.checker.status).toBe("predicate_false_example");
    await source
      .getByRole("button", { name: "Explain aa_bad", exact: true })
      .hover({ timeout: 15000 });
    await expect(meaning).toContainText("Counterexample candidate");
    await expect(
      meaning.getByRole("row", { name: "n 0", exact: true }),
    ).toBeVisible();
    const sourceBounds = (await source.boundingBox())!;
    const meaningBounds = (await meaning.boundingBox())!;
    expect(meaningBounds.x).toBeGreaterThanOrEqual(sourceBounds.x);
    expect(meaningBounds.x + meaningBounds.width).toBeLessThanOrEqual(
      sourceBounds.x + sourceBounds.width,
    );
    await page.screenshot({
      path: "target/gui-checks/compiled-meaning-counterexample.png",
    });
    await page.mouse.move(20, 20);
    await setPaths(page, "Main.lean");
    await compileDependencies(page);
    await expect
      .poll(async () => (await nativeState(page)).running, { timeout: 120000 })
      .toBe(false);
    const repeated = (await nativeSource(page)).graph.declarations;
    expect(
      Object.fromEntries(repeated.map((n: any) => [n.id, n.semantic])),
    ).toEqual(observations);
    expect(requests).toHaveLength(0);
    await stop();
    commands.length = 0;
    await page.goto(await start());
    await expect(pin).toBeVisible({ timeout: 120000 });
    await pin.click();
    await map.getByRole("button", { name: /Declaration: aa_bad ·/ }).click();
    await source
      .getByRole("button", { name: "Explain aa_bad", exact: true })
      .hover({ timeout: 15000 });
    await expect(
      meaning.getByRole("row", { name: "n 0", exact: true }),
    ).toBeVisible();
    expect(commands).not.toContain("atlas_snapshot");
    expect(
      (await nativeState(page)).source_preparation["Main.lean"].outcome
        .cache_hit,
    ).toBe(true);
    expect((await nativeState(page)).authorized).toBe(false);
    await page.mouse.move(20, 20);
    await writeFile(resolve(project, "Shared.lean"), "def shared : Nat := 2\n");
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    await page
      .getByRole("button", { name: "Refresh files and map", exact: true })
      .click();
    await expect
      .poll(async () => (await nativeState(page)).running, { timeout: 120000 })
      .toBe(false);
    await map.getByRole("button", { name: /Declaration: aa_bad ·/ }).click();
    await source
      .getByRole("button", { name: "Explain aa_bad", exact: true })
      .hover({ timeout: 15000 });
    await expect(meaning).toContainText("Compiled expression");
    expect((await nativeSource(page)).revision).not.toBe(firstSource.revision);
    await expect(
      meaning.getByRole("row", { name: "n 0", exact: true }),
    ).toBeVisible();
    expect(await readFile(resolve(project, "Main.lean"), "utf8")).toBe(
      original,
    );
    await page.mouse.move(20, 20);
    await writeFile(resolve(project, "Shared.lean"), "def shared : Nat := 1\n");
    await writeFile(
      resolve(project, "Main.lean"),
      original.replace("n + 1 ≤ n := by sorry", "n + 1 ≤ n + 1 := by omega"),
    );
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    await page
      .getByRole("button", { name: "Refresh files and map", exact: true })
      .click();
    await expect
      .poll(async () => (await nativeState(page)).running, { timeout: 120000 })
      .toBe(false);
    await map.getByRole("button", { name: /Declaration: aa_bad ·/ }).click();
    await source
      .getByRole("button", { name: "Explain aa_bad", exact: true })
      .hover({ timeout: 15000 });
    await expect(meaning).toContainText("Compiled expression");
    await expect(meaning).not.toContainText("n := 0");
    await expect(source.getByLabel("File contents")).toContainText(
      "n + 1 ≤ n + 1",
    );
    expect(requests).toHaveLength(0);
    expect(await readFile(resolve(project, "Main.lean"), "utf8")).toContain(
      "n + 1 ≤ n + 1",
    );
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
          page.getByRole("button", { name: "Preview app", exact: true }),
        ).toBeVisible();
        const before = await nativeState(page);
        expect(before.authorized).toBe(false);
        await page
          .getByRole("button", { name: "Preview app", exact: true })
          .click();
        await expect(page.locator("iframe")).toHaveCount(0);
        const port = Number(new URL(url).port);
        await stop();
        await page.goto(await start(port));
      }
      await expect(
        page.getByRole("button", { name: "Preview app", exact: true }),
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
        name: "Extract compiled dependencies",
        exact: true,
      }),
    ).toBeDisabled();
    await expect(
      page.getByText("elan toolchain install leanprover/lean4:v4.35.0", {
        exact: false,
      }),
    ).toBeVisible();
    await page
      .getByRole("button", { name: "Rediscover", exact: true })
      .scrollIntoViewIfNeeded();
    await page.screenshot({
      path: "target/gui-checks/native-atlas-missing-installation.png",
    });
    // Installed resources are a test prerequisite, not a cold-download validation.
    await symlink(lean, installed, "dir");
    await page.getByRole("button", { name: "Rediscover", exact: true }).click();
    await expect(
      page.getByRole("button", {
        name: "Prepare dependency extraction",
        exact: true,
      }),
    ).toBeEnabled();
    await setPaths(page, "App.lean");
    await compileDependencies(page);
    const graph = page.getByRole("region", { name: "Local source graph" });
    await expect(graph).toContainText("Compiled dependencies from Lean Atlas", {
      timeout: 120_000,
    });
    const first = await nativeState(page);
    const firstSource = await nativeSource(page, first);
    expect(first.atlas.prepared).toBe(true);
    expect(Object.keys(firstSource.files).sort()).toEqual([
      "App.lean",
      "Domain.lean",
    ]);
    expect(firstSource.graph.edges).toEqual(
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
      page.getByText("Lean Atlas: Offline extraction ready", { exact: false }),
    ).toBeVisible();
    expect((await nativeState(page)).authorized).toBe(false);
    expect(requests).toHaveLength(0);
  });

  test("prepare real Atlas offline, extract elaborated cross-module scope, analyze it and recover from compilation failure", async ({
    page,
  }) => {
    test.setTimeout(300_000);
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
      page.getByRole("button", {
        name: "Prepare dependency extraction",
        exact: true,
      }),
    ).toBeEnabled();
    await page
      .getByRole("button", {
        name: "Prepare dependency extraction",
        exact: true,
      })
      .click();
    await expect(
      page.getByText("Lean Atlas: Offline extraction ready", { exact: false }),
    ).toBeVisible({ timeout: 120_000 });
    expect(requests).toHaveLength(0);
    await setPaths(page, "App.lean");
    const readyBox = await page
      .getByText("Lean Atlas: Offline extraction ready", { exact: false })
      .boundingBox();
    const labelBox = await page
      .locator("label")
      .filter({ hasText: "Graph source files (one per line)" })
      .boundingBox();
    expect(readyBox).not.toBeNull();
    expect(labelBox).not.toBeNull();
    expect(readyBox!.y).toBeGreaterThanOrEqual(labelBox!.y + labelBox!.height);
    await compileDependencies(page);
    const graph = page.getByRole("region", { name: "Local source graph" });
    await expect(graph).toContainText("Compiled dependencies from Lean Atlas", {
      timeout: 60_000,
    });
    const state = await nativeState(page);
    const stateSource = await nativeSource(page, state);
    expect(Object.keys(stateSource.files).sort()).toEqual([
      "App.lean",
      "Domain.lean",
    ]);
    expect(stateSource.roots).toEqual(["App.lean"]);
    const graphData = stateSource.graph;
    expect(graphData.edges).toEqual(
      expect.arrayContaining([
        {
          source: "Store.permitted",
          target: "eligibility",
          site: "type",
        },
        {
          source: "Store.permitted",
          target: "support",
          site: "value",
        },
        {
          source: "Store.invoice",
          target: "Store.price",
          site: "value",
        },
      ]),
    );
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    await page
      .getByRole("button", { name: "Dependencies: App.lean", exact: true })
      .click({ position: { x: 20, y: 15 } });
    await page
      .getByRole("button", {
        name: /Declaration: Store.unfinished · App.lean:/,
      })
      .click();
    await page
      .getByRole("button", { name: "Explain Store.unfinished", exact: true })
      .hover();
    await expect(
      page.getByRole("region", {
        name: "Meaning: Store.unfinished",
        exact: true,
      }),
    ).toContainText("Unfinished proof detected");
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    await page
      .getByRole("button", { name: /Declaration: Store.permitted · App.lean:/ })
      .click();
    await expect(
      page.locator(".ide-file-page:not([hidden]) [aria-current=location]"),
    ).toContainText("eligibility");
    await expect(
      page.locator(".ide-file-page:not([hidden]) [aria-current=location]"),
    ).toContainText("theorem permitted");
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Use this model", exact: true })
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
      .getByRole("textbox", { name: "Requirement to compare", exact: true })
      .fill(
        "ORIGINAL_SEPARATE_REQUIREMENT: all balances of at least 100 with positive stock permit purchase.",
      );
    await page
      .getByRole("button", { name: "Analyze declaration meaning", exact: true })
      .click();
    await expect(
      page.getByLabel("Meaning analysis result", { exact: true }),
    ).toContainText("Candidate mismatch");
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
    expect(hasCurrentEvidence(await projectEvidence(page))).toBe(false);
    await page
      .getByLabel("Meaning analysis result", { exact: true })
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
      .getByRole("textbox", { name: "Message", exact: true })
      .fill("Change the selected source to exercise compilation failure");
    await page.getByRole("button", { name: "Send", exact: true }).click();
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
      .getByRole("textbox", { name: "Message", exact: true })
      .fill(
        "Restore the valid Lean implementation and keep Domain.lean unchanged",
      );
    await page.getByRole("button", { name: "Send", exact: true }).click();
    await expect(
      page.getByText("ATLAS-SOURCE-REPAIRED", { exact: true }),
    ).toBeVisible();
    await compileDependencies(page);
    await expect(graph).toContainText("Compiled dependencies from Lean Atlas", {
      timeout: 60_000,
    });
    const port = Number(new URL(url).port);
    await stop();
    await page.goto(await start(port));
    await openDependencyControls(page);
    await expect(
      page.getByText("Lean Atlas: Offline extraction ready", { exact: false }),
    ).toBeVisible();
    await expect(graph).not.toBeVisible();
    await setPaths(page, "App.lean\nDomain.lean");
    await compileDependencies(page);
    await expect(graph).toContainText("Compiled dependencies from Lean Atlas", {
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
      .getByRole("button", {
        name: "Prepare dependency extraction",
        exact: true,
      })
      .click();
    await expect(
      page.getByText("Lean Atlas: Offline extraction ready", { exact: false }),
    ).toBeVisible({ timeout: 120_000 });
    const extract = page.getByRole("button", {
      name: "Select Lake targets automatically",
      exact: true,
    });
    await setPaths(page, "");
    await expect(extract).toBeEnabled();
    await compileDependencies(page);
    const graph = page.getByRole("region", { name: "Local source graph" });
    await expect(graph).toContainText("Compiled dependencies from Lean Atlas", {
      timeout: 60_000,
    });
    const state = await nativeState(page);
    const stateSource = await nativeSource(page, state);
    expect(Object.keys(stateSource.files).sort()).toEqual([
      "src/App.lean",
      "vendor/dep/source/Domain.lean",
    ]);
    expect(stateSource.roots).toEqual(["src/App.lean"]);
    await graph
      .getByRole("button", { name: /Domain.answer.*: select/ })
      .click();
    await expect(
      page.locator(".ide-file-page:not([hidden]) [aria-current=location]"),
    ).toContainText("def answer");
    await expect(
      page
        .locator(".ide-file-page:not([hidden]) .root-node")
        .getByRole("button", { name: /Domain.answer.*: select/ }),
    ).toBeVisible();
    await setPaths(page, "missing.lean");
    await compileDependencies(page);
    await expect(
      page.getByRole("alert").filter({ hasText: "Source observation failed" }),
    ).toBeVisible({ timeout: 60_000 });
    expect(await nativeSource(page)).toBeNull();
    await setPaths(page, "");
    await compileDependencies(page);
    await expect(graph).toContainText("Compiled dependencies from Lean Atlas", {
      timeout: 60_000,
    });
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
      name: "Project map",
      exact: true,
    });
    const main = map.getByRole("button", {
      name: /Declaration: Demo.result ·/,
    });
    await expect(main).toHaveAttribute("data-declaration-role", "main_theorem");
    await expect(
      map.getByRole("button", { name: /Declaration: Demo.helper ·/ }),
    ).toHaveAttribute("data-declaration-role", "lemma");
    await expect(
      map.getByRole("button", { name: /Declaration: Demo.value ·/ }),
    ).toHaveAttribute("data-declaration-role", "definition");
    await expect(
      map.getByRole("button", { name: /Declaration: Demo.Container ·/ }),
    ).toHaveAttribute("data-declaration-role", "structure");
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Use this model", exact: true })
      .click();
    await expect(page.getByRole("dialog")).not.toBeVisible();
    const chat = page.getByRole("complementary", {
      name: "Workspace chat",
      exact: true,
    });
    const input = chat.getByRole("textbox", {
      name: "Message",
      exact: true,
    });
    const send = chat.getByRole("button", { name: "Send", exact: true });
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
        .getByLabel("File contents");
      await expect(
        source.locator(".syntax-token").filter({ hasText: /^theorem$/ }),
      ).toHaveCount(1, { timeout: 60000 });
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
        page.getByRole("combobox", { name: "Center declaration" }),
      ).toHaveCount(0);
      await expect(
        page.getByRole("combobox", { name: "Dependency scope" }),
      ).toHaveCount(0);
      await expect(
        page.getByRole("button", { name: "Analyze with Lean", exact: true }),
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
      await expect(chat.getByRole("status")).toContainText("Waiting to send");
      expect(requests).toHaveLength(0);
      release();
      await expect(
        chat.getByText("ONLY-CHAT-SEND-REACTS", { exact: true }),
      ).toBeVisible({ timeout: 60000 });
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
      name: "Project map",
      exact: true,
      includeHidden: true,
    });
    await expect(
      map.getByRole("button", { name: "Category: Networking", exact: true }),
    ).toBeVisible();
    const header = page.getByRole("banner");
    await expect(header).not.toContainText("Local workspace");
    await expect(header.getByRole("alert")).toHaveCount(0);
    await expect(
      header.getByRole("button", { name: "Connection", exact: true }),
    ).toHaveCount(0);
    const chat = page.getByRole("complementary", {
      name: "Workspace chat",
      exact: true,
    });
    await expect(
      chat
        .locator(".composer")
        .getByRole("button", { name: "Choose model", exact: true }),
    ).toBeVisible();
    const treeBox = await page
      .getByRole("complementary", { name: "Files", exact: true })
      .boundingBox();
    const mapBox = await map.boundingBox(),
      chatBox = await chat.boundingBox();
    expect(treeBox!.x + treeBox!.width).toBeLessThanOrEqual(mapBox!.x + 1);
    expect(mapBox!.x + mapBox!.width).toBeLessThanOrEqual(chatBox!.x + 1);
    expect(mapBox!.width).toBeGreaterThan(600);
    await expect
      .poll(async () => (await nativeState(page)).running)
      .toBe(false);
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    const beforeCommands = commands.length;
    await map.locator(".continuous-map-scene").evaluate((element) => {
      (window as any).__originalMap = element;
    });
    const category = map.getByRole("button", {
      name: "Category: Networking",
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
      name: "Dependencies: Networking/Multistream.lean",
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
      name: /Declaration: invoice · Networking\/Multistream.lean:/,
    });
    await expect(invoice).toBeVisible();
    expect(commands.slice(beforeCommands)).toEqual([]);
    expect(await nativeSource(page)).toBeNull();
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
      page.locator(".ide-file-page:not([hidden])").getByLabel("File contents"),
    ).toContainText("def invoice", { timeout: 120000 });
    await expect(page.locator('[aria-current="location"]')).toContainText(
      "def invoice",
    );
    const references = page.getByRole("region", {
      name: "File reference graph",
      exact: true,
    });
    await expect(references).toBeVisible();
    const sourceBox = await page
      .getByRole("region", { name: "Source code", exact: true })
      .boundingBox();
    const graphBox = await references.boundingBox();
    expect(sourceBox!.x + sourceBox!.width).toBeLessThanOrEqual(
      graphBox!.x + 1,
    );
    await expect(chat.getByRole("status")).toHaveCount(0);
    await expect(page.getByRole("tablist", { name: "View mode" })).toHaveCount(
      0,
    );
    await expect(
      page.getByRole("link", { name: "React Flow attribution" }),
    ).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Whole project", exact: true }),
    ).toHaveCount(0);
    const close = page.getByTitle("Close: Networking/Multistream.lean", {
      exact: true,
    });
    expect(
      await close.evaluate(
        (e) => getComputedStyle(e.parentElement!).backgroundColor,
      ),
    ).toBe("rgb(255, 255, 255)");
    await page.screenshot({ path: "target/gui-checks/file-split-source.png" });
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
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
      name: "Local source graph",
      exact: true,
    });
    await expect(compiled).toBeVisible({ timeout: 120000 });
    expect(
      (await nativeSource(page)).files["Networking/Multistream.lean"],
    ).toBe(largeSource);
    expect((await nativeSource(page)).graph.edges).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ source: "invoice", target: "price" }),
      ]),
    );
    await expect(chat.getByRole("alert")).toHaveCount(0);
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    for (const name of [
      "Refresh model list",
      "Select this connection",
      "Reload connection settings on this PC",
    ])
      await expect(page.getByRole("button", { name, exact: true })).toHaveCount(
        0,
      );
    await expect(
      page.getByRole("button", { name: "Use this model", exact: true }),
    ).toBeVisible();
    await expect(page.getByRole("dialog")).not.toContainText(
      "Working directory:",
    );
    await page.getByRole("button", { name: "Close", exact: true }).click();
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    await page.setViewportSize({ width: 390, height: 844 });
    await page.getByRole("button", { name: "Chat", exact: true }).click();
    await expect(
      chat
        .locator(".composer")
        .getByRole("button", { name: "Choose model", exact: true }),
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

  test("opening a file prepares analysis and keeps a bounded-import failure beside its readable source", async ({
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
      name: "Workspace chat",
      exact: true,
    });
    await page
      .getByRole("button", { name: "File: Main.lean", exact: true })
      .click();
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("File contents"),
    ).toContainText("def original", { timeout: 60000 });
    await expect(
      page.getByRole("region", { name: "File reference graph", exact: true }),
    ).toBeVisible();
    expect(await nativeSource(page)).toBeNull();
    await expect(chat.getByRole("status")).toHaveCount(0);
    await compileDependencies(page);
    const pane = page.getByRole("region", {
      name: "File reference graph",
      exact: true,
    });
    await expect(pane.getByRole("alert")).toContainText(
      "Local import closure exceeds 32 files/16 MiB",
      { timeout: 60000 },
    );
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("File contents"),
    ).toContainText("def original", { timeout: 60000 });
    await expect(chat.getByRole("alert")).toHaveCount(0);
    await expect(chat.getByRole("status")).toHaveCount(0);
    expect(await nativeSource(page)).toBeNull();
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
      name: "File",
      exact: true,
    });
    await openDependencyControls(page);
    await openDependencyControls(page);
    await picker.fill("App.lean");
    await page.getByRole("option", { name: "App.lean", exact: true }).click();
    await expect(
      page.getByRole("textbox", { name: "Graph source files (one per line)" }),
    ).toHaveValue("App.lean");
    await compileDependencies(page);
    const missing = page
      .getByRole("alert")
      .filter({ hasText: "Lake dependency mathlib" });
    await expect(missing).toBeVisible({ timeout: 120_000 });
    await expect(missing).toHaveCount(1);
    await expect(missing).toContainText("vendor/mathlib");
    await expect(missing).toContainText("lake exe cache get");
    expect(await nativeSource(page)).toBeNull();

    await missing.locator("summary").click();
    await missing.scrollIntoViewIfNeeded();
    await page.screenshot({
      path: "target/gui-checks/native-lake-missing-dependency-help.png",
    });
    await setPaths(page, "missing.lean");
    const source = page.getByRole("button", {
      name: "Refresh references",
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
    const graph = page.getByRole("region", { name: "Local source graph" });
    await expect(
      graph.getByRole("button", { name: /app.*: select/ }),
    ).toContainText("app");
    let state = await nativeState(page);
    let stateSource = await nativeSource(page, state);
    expect(stateSource.compiled).toBe(false);
    expect(hasCurrentEvidence(await projectEvidence(page, state))).toBe(false);
    await expect(
      readFile(resolve(project, "vendor/mathlib/lakefile.lean")),
    ).rejects.toThrow();
    expect(requests).toHaveLength(0);
    await stop();
    await page.goto(await start());
    await openDependencyControls(page);
    state = await nativeState(page);
    stateSource = await nativeSource(page, state);
    expect(state.authorized).toBe(false);
    expect(stateSource).toBeNull();
    await openDependencyControls(page);
    await picker.fill("App.lean");
    await page.getByRole("option", { name: "App.lean", exact: true }).click();
    await source.click();
    await expect(
      graph.getByRole("button", { name: /app.*: select/ }),
    ).toContainText("app");
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
        .getByRole("button", {
          name: "Prepare dependency extraction",
          exact: true,
        })
        .click();
      await expect(
        page.getByText("Lean Atlas: Offline extraction ready", {
          exact: false,
        }),
      ).toBeVisible({ timeout: 120_000 });
      await setPaths(page, "SandboxProbe.lean");
      const graph = page.getByRole("region", {
        name: "Local source graph",
      });
      await compileDependencies(page);
      await expect(graph).toContainText("1 declarations", { timeout: 60_000 });
      expect(connections).toBe(0);
      expect(requests).toHaveLength(0);
      expect(
        await readFile(resolve(project, "SandboxProbe.lean"), "utf8"),
      ).toBe(source);
      await expect(readFile(resolve(project, "job-write"))).rejects.toThrow();
      const cache = resolve(root, "data/meden/atlas");
      const prepared = (await readdir(cache)).filter((name) =>
        /^[a-f0-9]{64}$/.test(name),
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
        .getByRole("button", {
          name: "Prepare dependency extraction",
          exact: true,
        })
        .click();
      await expect(
        page.getByText("Lean Atlas: Offline extraction ready", {
          exact: false,
        }),
      ).toBeVisible({ timeout: 120_000 });
      await compileDependencies(page);
      await expect(graph).toContainText("1 declarations", { timeout: 60_000 });
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

  async function nativeSource(page: Page, status?: any) {
    const reference = (status || (await nativeState(page))).sources.current;
    if (!reference) return null;
    const response = await page.evaluate(async (revision) => {
      const response = await fetch(
        "/api/source?revision=" + encodeURIComponent(revision),
        {
          headers: {
            Authorization:
              "Bearer " + sessionStorage.getItem("meden-native-session"),
          },
        },
      );
      return { status: response.status, body: await response.json() };
    }, reference.revision);
    if (response.status === 409) return null;
    expect(response.status).toBe(200);
    expect(response.body.revision).toBe(reference.revision);
    return response.body.data;
  }

  async function nativeReader(page: Page, path: string) {
    const state = await nativeState(page);
    const revision = state.readers.files[path]?.revision;
    if (!revision) return null;
    return page.evaluate(async (revision) => {
      const response = await fetch(
        "/api/reader?revision=" + encodeURIComponent(revision),
        {
          headers: {
            Authorization:
              "Bearer " + sessionStorage.getItem("meden-native-session"),
          },
        },
      );
      if (!response.ok) throw Error("Reader query failed");
      return (await response.json()).data;
    }, revision);
  }
  async function nativePreparationLog(page: Page, status: any, path: string) {
    const ref = status.source_preparation[path].log.revision;
    return page.evaluate(
      async ({ ref, job, resource }) => {
        const response = await fetch(
          "/api/preparation-log?revision=" + encodeURIComponent(ref),
          {
            headers: {
              Authorization:
                "Bearer " + sessionStorage.getItem("meden-native-session"),
            },
          },
        );
        if (!response.ok)
          throw new Error("Log unavailable: " + response.status);
        const body = await response.json();
        if (
          body.revision !== ref ||
          body.data.job !== job ||
          body.data.resource !== resource
        )
          throw new Error("Wrong log binding");
        return body.data;
      },
      { ref, job: status.source_preparation[path].job, resource: path },
    );
  }
  async function nativeState(page: Page) {
    return page.evaluate(async () => {
      const response = await fetch("/api/state", {
        signal: AbortSignal.timeout(5000),
        headers: {
          Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
        },
      });
      return response.json();
    });
  }
  async function candidateBody(page: Page, history: boolean) {
    for (let attempt = 0; attempt < 3; attempt++) {
      const state = await nativeState(page);
      const reference = history
        ? state.analyses.history
        : state.analyses.current;
      if (reference.state === "empty") return null;
      expect(reference.state).toBe("ready");
      const response = await page.evaluate(
        async ({ revision, history }) => {
          const response = await fetch(
            `/api/${history ? "analysis-history" : "analysis"}?revision=${encodeURIComponent(revision)}`,
            {
              headers: {
                Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
              },
            },
          );
          return { status: response.status, body: await response.json() };
        },
        { revision: reference.revision, history },
      );
      if (response.status === 409) continue;
      expect(response.status).toBe(200);
      expect(response.body.revision).toBe(reference.revision);
      return response.body;
    }
    throw new Error(
      "Candidate publication did not stabilize after three fresh status reads",
    );
  }
  async function nativeAnalysis(page: Page) {
    const body = await candidateBody(page, false);
    return body ? analysisCurrentSchema.parse(body).data : null;
  }
  async function nativeAnalysisHistory(page: Page) {
    return analysisHistorySchema.parse(await candidateBody(page, true)).data
      .history;
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
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await page.getByRole("button", { name: "Use this model" }).click();
    await page
      .getByRole("textbox", { name: "Message", exact: true })
      .fill(
        "Prepare the original payload for host review, then change the workspace file",
      );
    await page.getByRole("button", { name: "Send", exact: true }).click();
    await expect(
      page.getByText("DELIVERIES-QUEUED", { exact: true }),
    ).toBeVisible();
    await page.getByRole("button", { name: "Review delivery" }).click();
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
          name: "Delivery #0 : review content and destination",
          exact: true,
        })
        .click();
      const dialog = page.getByRole("dialog");
      await expect(dialog).toContainText(target);
      await expect(dialog).toContainText(
        createHash("sha256").update(payload).digest("hex"),
      );
      await expect(dialog.getByLabel("Exact bytes to send")).toContainText(
        "00 ff 46 52",
      );
      await expect(
        dialog.getByRole("button", { name: "Send once to this destination" }),
      ).toBeEnabled();
      const first = (await nativeState(page)).export_review;
      await page.screenshot({
        path: "target/gui-checks/native-delivery-review.png",
        fullPage: true,
      });
      await page.setViewportSize({ width: 390, height: 240 });
      await expect(
        dialog.getByRole("button", { name: "Send once to this destination" }),
      ).toBeDisabled();
      await page.setViewportSize({ width: 390, height: 844 });
      await expect(
        dialog.getByRole("button", { name: "Send once to this destination" }),
      ).toBeEnabled();
      await page.screenshot({
        path: "target/gui-checks/native-delivery-mobile.png",
        fullPage: true,
      });
      await page.setViewportSize({ width: 1440, height: 960 });
      await dialog.getByRole("button", { name: "Next bytes" }).click();
      await expect(dialog).toContainText("Byte offset 2048");
      await expect(dialog).toContainText(target);
      await dialog
        .getByRole("button", { name: "Send once to this destination" })
        .click();
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
          name: "Delivery #1 : review content and destination",
          exact: true,
        })
        .click();
      await dialog
        .getByRole("button", { name: "Reject this delivery" })
        .click();
      await expect(dialog).not.toBeVisible();
      await page
        .getByRole("button", {
          name: "Delivery #1 : review content and destination",
          exact: true,
        })
        .click();
      await expect(
        dialog.getByRole("button", { name: "Send once to this destination" }),
      ).toBeDisabled();
      await dialog.getByRole("button", { name: "Close", exact: true }).click();
      const port = Number(new URL(url).port);
      await stop();
      await page.goto(await start(port));
      await expect(
        page.getByRole("tablist", { name: "Open files" }).getByRole("tab"),
      ).toHaveText(["Project map"]);
      await expect(
        page.getByRole("button", { name: "Review delivery" }),
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
          name: "Delivery #0 : review content and destination",
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
          name: "Delivery #0 : review content and destination",
          exact: true,
        })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", { name: "Send once to this destination" })
        .click();
      await expect(
        page.getByRole("alert").filter({ hasText: "HTTP 307" }),
      ).toBeVisible();
      expect(paths).toEqual(["/redirect"]);
      await page
        .getByRole("button", {
          name: "Delivery #1 : review content and destination",
          exact: true,
        })
        .click();
      await page
        .getByRole("dialog")
        .getByRole("button", { name: "Send once to this destination" })
        .click();
      await expect(
        page
          .getByRole("alert")
          .filter({ hasText: "Delivery uncertain; do not resend" }),
      ).toBeVisible();
      expect(paths).toEqual(["/redirect", "/uncertain"]);
      await page
        .getByRole("button", {
          name: "Delivery #1 : review content and destination",
          exact: true,
        })
        .click();
      await expect(
        page
          .getByRole("dialog")
          .getByRole("button", { name: "Send once to this destination" }),
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
      .getByRole("button", { name: "Preview app", exact: true })
      .click();
    await page
      .getByRole("textbox", { name: "Public folder (inside workspace)" })
      .fill(folder);
    await page.getByRole("textbox", { name: "Entry HTML file" }).fill(entry);
    await page
      .getByRole("button", { name: "Reload files and preview" })
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
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await page.getByRole("button", { name: "Use this model" }).click();
    await page
      .getByRole("textbox", { name: "Message", exact: true })
      .fill("Build a local counter with module assets");
    await page.getByRole("button", { name: "Send", exact: true }).click();
    await expect(
      page.getByText("COUNTER-BUILT", { exact: true }),
    ).toBeVisible();
    await preview(page, "public");
    const frame = page.frameLocator('iframe[title="Local app preview"]');
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
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    await page
      .getByRole("button", { name: "Preview app", exact: true })
      .click();
    await expect(frame.getByLabel("Count")).toHaveText("2");
    await page.screenshot({
      path: "target/gui-checks/native-preview.png",
      fullPage: true,
    });
    await page.setViewportSize({ width: 390, height: 844 });
    await page
      .getByRole("button", { name: "Preview app", exact: true })
      .scrollIntoViewIfNeeded();
    await expect(
      page.getByRole("button", { name: "Reload files and preview" }),
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
      .getByRole("textbox", { name: "Message", exact: true })
      .fill("Change the increment to three");
    await page.getByRole("button", { name: "Send", exact: true }).click();
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
      .getByRole("button", { name: "Preview app", exact: true })
      .click();
    await expect(frame.getByLabel("Count")).toHaveText("0");
    const port = Number(new URL(url).port);
    await stop();
    await page.goto(await start(port));
    await page
      .getByRole("button", { name: "Preview app", exact: true })
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
      const frame = page.frameLocator('iframe[title="Local app preview"]');
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
      page.getByRole("textbox", { name: "Message", exact: true }),
    ).toBeDisabled();
    expect(requests).toHaveLength(0);
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await expect(
      page.getByRole("combobox", { name: "model", exact: true }),
    ).toHaveValue("scripted-gui-fixture");
    await page.getByRole("button", { name: "Use this model" }).click();
    await page
      .getByRole("textbox", { name: "Message", exact: true })
      .fill("Build the fixture and keep the original tests");
    await page.getByRole("button", { name: "Send", exact: true }).click();
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
      .getByRole("button", { name: "File: result.txt", exact: true })
      .click();
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("File contents"),
    ).toContainText("built by native worker");
    await expect(
      page.getByRole("alert").filter({ hasText: "429" }),
    ).toBeVisible();
    await page
      .getByRole("textbox", { name: "Message", exact: true })
      .fill("Continue from the retained files");
    await page.reload();
    await expect(
      page.getByRole("textbox", { name: "Message", exact: true }),
    ).toHaveValue("Continue from the retained files");
    const port = Number(new URL(url).port);
    await stop();
    await page.goto(await start(port));
    await expect(
      page.getByRole("textbox", { name: "Message", exact: true }),
    ).toBeDisabled();
    await expect(
      page.getByText("Build the fixture and keep the original tests", {
        exact: true,
      }),
    ).toBeVisible();
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await page.getByRole("button", { name: "Use this model" }).click();
    await page.getByRole("button", { name: "Send", exact: true }).click();
    await expect(
      page.getByText("RESTART-CONTEXT-RETAINED", { exact: true }),
    ).toBeVisible();
    expect(JSON.stringify(requests[2])).toContain("built by native worker");
    expect(JSON.stringify(requests[2])).toContain("Build the fixture");
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await page.getByRole("button", { name: "Revoke access" }).click();
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Close", exact: true })
      .click();
    await expect(
      page.getByRole("textbox", { name: "Message", exact: true }),
    ).toBeDisabled();
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await expect(
      page.getByRole("button", { name: "Use this model" }),
    ).toBeDisabled();
    await page.getByRole("button", { name: "Close", exact: true }).click();
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
    // A background model-list response must not swallow the explicit approval
    // click. The native binding and complete visible heading are still checked.
    let modelListPending = false,
      releaseModels = () => {};
    const modelsReleased = new Promise<void>(
      (resolve) => (releaseModels = resolve),
    );
    await page.route("**/api/command", async (route) => {
      if (route.request().postDataJSON()?.type !== "list_models")
        return route.continue();
      const response = await route.fetch();
      modelListPending = true;
      await modelsReleased;
      await route.fulfill({ response }).catch(() => {});
    });
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    try {
      await expect.poll(() => modelListPending).toBe(true);
      await page.getByRole("button", { name: "Use this model" }).click();
      await expect(page.getByRole("dialog")).toHaveCount(0);
      expect((await nativeState(page)).authorized).toBe(true);
    } finally {
      releaseModels();
    }
    await page
      .getByRole("textbox", { name: "Message", exact: true })
      .fill("Wait for cancellation");
    await page.getByRole("button", { name: "Send", exact: true }).click();
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
    await page.getByRole("button", { name: "Stop" }).click();
    await expect.poll(() => cancellationReachedHost).toBe(true);
    reply(
      held!,
      tool("write_file", { path: "must-not-exist.txt", content: "bad" }),
    );
    await expect(page.getByRole("button", { name: "Stop" })).not.toBeVisible();
    expect((await nativeState(page)).running).toBe(false);
    await expect(
      readFile(resolve(project, "must-not-exist.txt")),
    ).rejects.toThrow();
    await page
      .getByRole("textbox", { name: "Message", exact: true })
      .fill("Start an explicit new turn");
    await expect.poll(() => cancellationReachedHost).toBe(true);
    await expect(
      page.getByRole("button", { name: "Send", exact: true }),
    ).toBeDisabled();
    releaseCancel();
    await expect(
      page.getByRole("button", { name: "Send", exact: true }),
    ).toBeEnabled();
    await expect(page.locator(".chat-stop-notice > summary")).toHaveText(
      "Response stopped",
    );
    await expect(page.locator(".chat-stop-notice")).not.toHaveAttribute("open");
    await expect(page.locator(".ide-chat > [role=alert]")).toHaveCount(0);
    await page.getByRole("button", { name: "Send", exact: true }).click();
    await expect(
      page.getByText("EXPLICIT-NEW-TURN", { exact: true }),
    ).toBeVisible();
    await page.screenshot({
      path: "target/gui-checks/native-cancel-resumed.png",
      fullPage: true,
    });
    await page.setViewportSize({ width: 390, height: 844 });
    await expect(
      page.getByRole("tab", { name: "Project map", exact: true }),
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
    test.setTimeout(180000);
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
    const graph = page.getByRole("region", { name: "Local source graph" });
    await expect(graph).toContainText(
      "1 files · 2 declarations · 1 reference candidates",
    );
    expect(requests).toHaveLength(0);
    await graph
      .getByRole("button", { name: /Demo.stock_bound.*: select/ })
      .click();
    // Lexical navigation does not present a source-English approximation.
    await expect(
      page.getByRole("button", {
        name: "Explain Demo.stock_bound",
        exact: true,
      }),
    ).toHaveCount(0);
    await page.screenshot({
      path: "target/gui-checks/native-live-graph.png",
      fullPage: true,
    });
    await expect(
      page.locator(".ide-file-page:not([hidden]) [aria-current=location]"),
    ).toContainText("theorem stock_bound");
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("File contents"),
    ).toContainText("sorry");
    await page.screenshot({
      path: "target/gui-checks/native-source-graph.png",
      fullPage: true,
    });
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await page.getByRole("button", { name: "Use this model" }).click();
    await page
      .getByRole("textbox", { name: "Message", exact: true })
      .fill("Rename the property to inventory_bound");
    await page.getByRole("button", { name: "Send", exact: true }).click();
    await expect(
      page.getByText("SOURCE-UPDATED", { exact: true }),
    ).toBeVisible();
    await expect(graph).not.toBeVisible();
    await refreshReferences(page);
    await expect(
      graph.getByRole("button", { name: /Demo.inventory_bound.*: select/ }),
    ).toBeVisible();
    await expect(
      graph.getByRole("button", { name: /Demo.stock_bound.*: select/ }),
    ).not.toBeVisible();
    await setPaths(page, "missing.lean");
    await refreshReferences(page);
    await expect(
      page.getByRole("alert").filter({ hasText: "Source observation failed" }),
    ).toBeVisible();
    await expect(graph).not.toBeVisible();
    await setPaths(page, "Spec.lean");
    await refreshReferences(page);
    await expect(graph).toContainText("2 declarations");
    await page.reload();
    await openDependencyControls(page);
    await expect(
      page.getByRole("textbox", { name: "Graph source files (one per line)" }),
    ).toHaveValue("Spec.lean");
    await expect(
      page.getByRole("button", { name: "Refresh references" }),
    ).toBeEnabled();
    await page.getByRole("button", { name: "Close", exact: true }).click();
    await expect(graph).toContainText("2 declarations");
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
      .getByRole("region", { name: "Local source graph" })
      .getByRole("button", { name: /Demo.stock_bound.*: select/ })
      .click();
    await page
      .getByRole("textbox", { name: "Requirement to compare", exact: true })
      .fill(
        "ORIGINAL_REQUIREMENT_SENTINEL: A purchase always decrements stock regardless of balance.",
      );
    await expect(
      page.getByRole("button", { name: "Analyze declaration meaning" }),
    ).toBeDisabled();
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    if (beforeApproval) await beforeApproval();
    await page.getByRole("button", { name: "Use this model" }).click();
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
      .getByRole("region", { name: "Local source graph" })
      .getByRole("button", { name: /stock.*: select/ })
      .click();
    await page
      .getByRole("textbox", { name: "Requirement to compare", exact: true })
      .fill(stockRequirement);
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await page.getByRole("button", { name: "Use this model" }).click();
    await page
      .getByRole("button", { name: "Analyze declaration meaning" })
      .click();
    await expect(
      page.getByLabel("Meaning analysis result", { exact: true }),
    ).toContainText("Candidate mismatch (Model assessment)");
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
      name: "Prepare patch for this Finding",
    });
    const dialog = page.getByRole("dialog", {
      name: "Review patch and original requirement",
    });
    const apply = dialog.getByRole("button", {
      name: "Apply this patch once and reanalyze",
    });
    await prepare.click();
    await expect(dialog).toBeVisible();
    await expect(dialog.getByLabel("Exact patch to apply")).toContainText(
      "+pub fn stock(n: u64) -> u64 { n.saturating_sub(1) }",
    );
    expect(await readFile(resolve(project, "stock.rs"), "utf8")).toBe(
      stockSource,
    );
    await dialog.getByRole("button", { name: "Reject patch" }).click();
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
    await page
      .getByRole("button", { name: "Analyze declaration meaning" })
      .click();
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
    const result = page.getByLabel("Meaning analysis result", { exact: true });
    await expect(result).toContainText("Candidate match (Model assessment)");
    expect(await readFile(resolve(project, "stock.rs"), "utf8")).toBe(
      concurrent.replace("{ n }", "{ n.saturating_sub(1) }"),
    );
    const state = await nativeState(page);
    expect(hasCurrentEvidence(await projectEvidence(page, state))).toBe(false);
    const savedAnalyses = await nativeAnalysisHistory(page);
    expect(savedAnalyses).toHaveLength(3);
    expect(
      new Set(
        savedAnalyses.map(
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
      page.getByRole("tab", { name: "Project map", exact: true }),
    ).toBeVisible();
    expect((await nativeState(page)).repair_review).toBe(null);
    expect((await nativeState(page)).authorized).toBe(false);
    await page
      .getByRole("button", { name: "File: stock.rs", exact: true })
      .click();
    await expect(result).toContainText("Historical analysis.", {
      timeout: 120000,
    });
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
      .getByRole("button", { name: "Prepare patch for this Finding" })
      .click();
    const dialog = page.getByRole("dialog", {
      name: "Review patch and original requirement",
    });
    await expect(dialog).toBeVisible();
    const reference = (await nativeState(page)).repair_review.reference;
    const port = Number(new URL(url).port);
    await stop();
    await page.goto(await start(port));
    await expect(
      page.getByRole("tab", { name: "Project map", exact: true }),
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
  test("large candidate history transfers once, retries locally and keeps newer selections across delayed reads and restart", async ({
    page,
  }) => {
    test.setTimeout(240000);
    responses.push(
      (response) => reply(response, informalization()),
      (response) => reply(response, judgement()),
    );
    await openMeaning(page);
    await page
      .getByRole("button", { name: "Analyze declaration meaning", exact: true })
      .click();
    const result = page.getByLabel("Meaning analysis result", { exact: true });
    await expect(result).toContainText("Candidate mismatch (Model assessment)");
    const port = Number(new URL(url).port);
    await stop();
    const journal = resolve(
      root,
      "data/meden/projects",
      createHash("sha256").update(project).digest("hex"),
      "meaning-analysis.json",
    );
    const original = JSON.parse(await readFile(journal, "utf8"));
    const marker = "ORIGINAL-CANDIDATE-λ\n<literal> 𝕏 ".repeat(160);
    const seed = {
      version: 2,
      sequence: 400,
      entries: Array.from({ length: 400 }, (_, index) => {
        const row = structuredClone(original.entries[0]);
        row.sequence = index + 1;
        const id = createHash("sha256")
          .update("history-fixture:" + index)
          .digest("hex");
        row.id = "analysis-" + id;
        row.finding_id = "finding-" + id;
        row.assessment.translation.naturalLanguage = `OBSERVATION ${index + 1}: ${marker}`;
        return row;
      }),
    };
    // Seed a retained-history workload from the actual preceding job. This is
    // fixture construction while stopped, not live-model/cold-install evidence.
    await writeFile(journal, JSON.stringify(seed));
    let historyReads = 0,
      failHistory = true,
      currentReads = 0,
      polls = 0;
    const commands: unknown[] = [];
    page.on("request", (request) => {
      if (request.method() === "POST" && request.url().endsWith("/api/command"))
        commands.push(request.postDataJSON());
    });
    page.on("response", (response) => {
      if (response.url().endsWith("/api/state")) polls++;
    });
    await page.route("**/api/analysis-history?*", async (route) => {
      historyReads++;
      if (failHistory) {
        failHistory = false;
        await route.fulfill({
          status: 503,
          contentType: "application/json",
          body: JSON.stringify({
            error: "temporary candidate history transfer",
          }),
        });
      } else await route.continue();
    });
    let holdCurrent = false,
      held = false;
    let release = () => {};
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    await page.route("**/api/analysis?*", async (route) => {
      currentReads++;
      if (holdCurrent) {
        holdCurrent = false;
        const response = await route.fetch();
        held = true;
        await gate;
        await route.fulfill({ response }).catch(() => {});
      } else await route.continue();
    });
    try {
      await page.goto(await start(port));
      await page
        .getByRole("button", { name: "File: Spec.lean", exact: true })
        .click();
      const failure = page
        .getByRole("alert")
        .filter({ hasText: "Could not load saved analyses." });
      await expect(failure).toBeVisible();
      await page.screenshot({
        path: "target/gui-checks/analysis-history-transfer-failure.png",
        fullPage: true,
      });
      const beforeRetry = commands.length;
      await failure
        .getByRole("button", { name: "Retry analyses", exact: true })
        .click();
      await expect(failure).not.toBeVisible();
      expect(commands.slice(beforeRetry)).toEqual([]);
      await expect
        .poll(async () => (await nativeState(page)).running, {
          timeout: 120000,
        })
        .toBe(false);
      await page
        .getByRole("region", { name: "Local source graph" })
        .getByRole("button", { name: /Demo.stock_bound.*: select/ })
        .click();
      const history = page.getByRole("combobox", {
        name: "Saved analyses",
        exact: true,
      });
      await expect(history.locator("option")).toHaveCount(400);
      expect(commands.slice(beforeRetry)).toEqual([]);
      expect(historyReads).toBe(2);
      const status = await nativeState(page);
      expect(status.analyses.history.runs).toBe(400);
      expect(status).not.toHaveProperty("analysis");
      expect(status).not.toHaveProperty("analysis_history");
      expect(JSON.stringify(status)).not.toContain("ORIGINAL-CANDIDATE-");
      const historyBody = await candidateBody(page, true);
      expect(historyBody.data.history).toEqual(
        seed.entries.map((row) => ({
          ...row,
          sequence: String(row.sequence),
          state: "historical",
        })),
      );
      const historyBytes = Buffer.byteLength(JSON.stringify(historyBody));
      expect(historyBytes).toBeGreaterThan(1_000_000);
      expect(Buffer.byteLength(JSON.stringify(status.analyses))).toBeLessThan(
        350,
      );
      const stableReads = historyReads;
      const navigationCommands = commands.length,
        oldPolls = polls;
      await page.getByRole("tab", { name: "Project map", exact: true }).click();
      await page.getByRole("tab", { name: "Spec.lean", exact: true }).click();
      await expect.poll(() => polls).toBeGreaterThan(oldPolls + 3);
      expect(historyReads).toBe(stableReads);
      expect(commands.slice(navigationCommands)).toEqual([]);
      holdCurrent = true;
      await history.selectOption(seed.entries[0]!.id);
      await expect.poll(() => held).toBe(true);
      await expect(history).toBeVisible();
      await expect(history).toBeEnabled();
      await history.selectOption(seed.entries[1]!.id);
      await expect(result).toContainText("OBSERVATION 2:");
      release();
      await expect(result).toContainText("OBSERVATION 2:");
      await expect(result).not.toContainText("OBSERVATION 1:");
      expect(historyReads).toBe(stableReads);
      expect(requests).toHaveLength(2);
      expect((await nativeState(page)).authorized).toBe(false);
      const currentBody = await candidateBody(page, false);
      expect(currentBody.data.id).toBe(seed.entries[1]!.id);
      await test.info().attach("analysis-publication-transfer.json", {
        contentType: "application/json",
        body: Buffer.from(
          JSON.stringify({
            runs: 400,
            history_response_bytes: historyBytes,
            current_response_bytes: Buffer.byteLength(
              JSON.stringify(currentBody),
            ),
            status_projection_bytes: Buffer.byteLength(
              JSON.stringify(status.analyses),
            ),
            commands_on_retry: [],
            history_reads_on_navigation_or_selection:
              historyReads - stableReads,
            current_reads: currentReads,
            scope:
              "Actual host/journal/reader workers; seeded retained observations from one scripted-model job. Not live-model quality, cold installation or a physical-paint measurement.",
          }),
        ),
      });
      await page.screenshot({
        path: "target/gui-checks/revisioned-analysis-history.png",
        fullPage: true,
      });
      await result.scrollIntoViewIfNeeded();
      await page.screenshot({
        path: "target/gui-checks/revisioned-analysis-result.png",
        fullPage: true,
      });
      await stop();
      await page.goto(await start(port));
      await page
        .getByRole("button", { name: "File: Spec.lean", exact: true })
        .click();
      await page
        .getByRole("region", { name: "Local source graph" })
        .getByRole("button", { name: /Demo.stock_bound.*: select/ })
        .click();
      await expect(history.locator("option")).toHaveCount(400);
      expect((await nativeState(page)).authorized).toBe(false);
      expect(requests).toHaveLength(2);
      expect(JSON.parse(await readFile(journal, "utf8"))).toEqual(seed);
    } finally {
      release();
      await page.unroute("**/api/analysis-history?*");
      await page.unroute("**/api/analysis?*");
    }
  });
  test("local meaning history retains requirement and Finding identities across reruns and restart", async ({
    page,
  }) => {
    test.setTimeout(180000);
    responses.push(
      ...Array.from({ length: 3 }, () => [
        (response: ServerResponse) => reply(response, informalization()),
        (response: ServerResponse) => reply(response, judgement()),
      ]).flat(),
    );
    await openMeaning(page);
    const result = page.getByLabel("Meaning analysis result", { exact: true });
    const run = page.getByRole("button", {
      name: "Analyze declaration meaning",
    });
    const requirement = page.getByRole("textbox", {
      name: "Requirement to compare",
      exact: true,
    });
    await run.click();
    await expect(result).toContainText("Candidate mismatch (Model assessment)");
    const first = (await nativeAnalysis(page))!;
    expect(first.finding_id).toMatch(/^finding-/);
    await requirement.fill("A different requirement retained as its own input");
    await run.click();
    await expect
      .poll(async () => (await nativeAnalysisHistory(page)).length)
      .toBe(2);
    const second = (await nativeAnalysis(page))!;
    expect(second.id).not.toBe(first.id);
    expect(second.requirement_id).not.toBe(first.requirement_id);
    const history = page.getByRole("combobox", { name: "Saved analyses" });
    await history.selectOption(first.id);
    await expect(result).toContainText("Historical analysis.", {
      timeout: 120000,
    });
    await expect(requirement).toHaveValue(first.input.requirement);
    expect(requests).toHaveLength(4);
    await run.click();
    await expect
      .poll(async () => (await nativeAnalysisHistory(page)).length)
      .toBe(3);
    const current = (await nativeAnalysis(page))!;
    const savedAnalyses = await nativeAnalysisHistory(page);
    expect(current.requirement_id).toBe(first.requirement_id);
    expect(current.finding_id).not.toBe(first.finding_id);
    expect(savedAnalyses[0]!.assessment).toEqual(first.assessment);
    expect(savedAnalyses[1]!.input).toEqual(second.input);
    await expect(result).toContainText("Candidate mismatch (Model assessment)");
    const port = Number(new URL(url).port);
    await stop();
    await page.goto(await start(port));
    await page
      .getByRole("button", { name: "File: Spec.lean", exact: true })
      .click();
    await history.selectOption(second.id, { timeout: 120000 });
    await expect(result).toContainText(second.input.requirement);
    expect((await nativeState(page)).authorized).toBe(false);
    expect(requests).toHaveLength(6);
    await page.setViewportSize({ width: 390, height: 844 });
    await result
      .getByText("Analysis target and process details", { exact: true })
      .click();
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
          page.getByRole("button", { name: "Use this model" }),
        ).toBeDisabled();
        await page.setViewportSize({ width: 1440, height: 960 });
      },
    );
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await expect(page.getByRole("dialog")).toContainText(
      "separate-judge-fixture",
    );
    await page.getByRole("button", { name: "Close", exact: true }).click();
    await page
      .getByRole("button", { name: "Analyze declaration meaning" })
      .click();
    const result = page.getByLabel("Meaning analysis result", { exact: true });
    await expect(result).toContainText("Candidate mismatch");
    expect((requests[0] as { model: string }).model).toBe(
      "scripted-gui-fixture",
    );
    expect((requests[1] as { model: string }).model).toBe(
      "separate-judge-fixture",
    );
    const record = (await nativeAnalysis(page))!;
    expect(record.judge?.model).toBe("separate-judge-fixture");
    await result.getByText("Analysis target and process details").click();
    await expect(result).toContainText("Comparison: separate-judge-fixture");
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
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    try {
      await expect.poll(() => metadataHeld).toBe(true);
      await page
        .getByRole("button", { name: "Revoke access", exact: true })
        .click();
      await expect
        .poll(async () => (await nativeState(page)).authorized)
        .toBe(false);
    } finally {
      releaseMetadata();
    }
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Close", exact: true })
      .click();
    await expect(
      page.getByRole("button", { name: "Analyze declaration meaning" }),
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
    test.setTimeout(180000);
    responses.push(
      (response) => reply(response, informalization()),
      (response) => reply(response, judgement()),
      (response) => reply(response, informalization()),
      (response) => reply(response, judgement()),
    );
    await openMeaning(page);
    await page
      .getByRole("button", { name: "Analyze declaration meaning" })
      .click();
    const result = page.getByLabel("Meaning analysis result", { exact: true });
    await expect(result).toContainText("Candidate mismatch (Model assessment)");
    await expect(result).toContainText("Stock never increases");
    expect(hasCurrentEvidence(await projectEvidence(page))).toBe(false);
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
      name: "Requirement to compare",
      exact: true,
    });
    const original = await input.inputValue();
    await input.fill("Changed requirement");
    await expect(result).toContainText(
      "This analysis uses different requirements, declarations or source revisions",
    );
    expect(requests).toHaveLength(2);
    const port = Number(new URL(url).port);
    await stop();
    await page.goto(await start(port));
    await openDependencyControls(page);
    await expect(result).toContainText("Historical analysis.", {
      timeout: 120000,
    });
    expect(requests).toHaveLength(2);
    await setPaths(page, "Spec.lean");
    await refreshReferences(page);
    await expect(input).toHaveValue(original);
    await expect(
      page.getByRole("button", { name: "Analyze declaration meaning" }),
    ).toBeDisabled();
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await page.getByRole("button", { name: "Use this model" }).click();
    await page
      .getByRole("button", { name: "Analyze declaration meaning" })
      .click();
    await expect(result).toContainText("Candidate mismatch (Model assessment)");
    await expect(result).not.toContainText("Historical analysis.");
    expect(requests).toHaveLength(4);
    await stop();
    const journals = resolve(root, "data/meden/projects");
    const [projectKey] = await readdir(journals);
    const checkpoint = resolve(journals, projectKey!, "meaning-analysis.json");
    await writeFile(checkpoint, "invalid analysis checkpoint");
    await page.goto(await start(port));
    await page
      .getByRole("button", { name: "File: Spec.lean", exact: true })
      .click();
    await expect(
      page
        .getByRole("alert")
        .filter({ hasText: "Could not load saved analyses." }),
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
      .getByRole("region", { name: "Local source graph" })
      .getByRole("button", { name: /Demo.stock_bound.*: select/ })
      .click();
    await input.fill(original);
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await page.getByRole("button", { name: "Use this model" }).click();
    await page
      .getByRole("button", { name: "Analyze declaration meaning" })
      .click();
    await expect(result).toContainText("Candidate mismatch (Model assessment)");
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
      await page
        .getByRole("button", { name: "Analyze declaration meaning" })
        .click();
      const result = page.getByLabel("Meaning analysis result", {
        exact: true,
      });
      if (mode === "cancel" || mode === "revoke" || mode === "stale") {
        await expect.poll(() => held !== undefined).toBe(true);
        await expect(result).toContainText(
          mode === "cancel" || mode === "revoke"
            ? "Translating declaration"
            : "Comparing with original requirement",
        );
        if (mode === "cancel")
          await page
            .getByRole("button", { name: "Cancel", exact: true })
            .click();
        else if (mode === "revoke") {
          await page
            .getByRole("button", { name: "Choose model", exact: true })
            .click();
          await page
            .getByRole("button", { name: "Revoke access", exact: true })
            .click();
          await page
            .getByRole("dialog")
            .getByRole("button", { name: "Close", exact: true })
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
          ? "Meaning analysis was cancelled"
          : mode === "stale"
            ? "The source changed"
            : "Meaning analysis failed",
      );
      await expect(result).not.toContainText("Candidate match");
      expect(requests).toHaveLength(mode === "stale" ? 2 : 1);
      if (mode === "revoke")
        await expect(
          page.getByRole("button", { name: "Analyze declaration meaning" }),
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
