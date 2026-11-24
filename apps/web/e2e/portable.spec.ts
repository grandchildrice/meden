import { test, expect } from "@playwright/test";
import { spawn, execFile } from "node:child_process";
import { promisify } from "node:util";
import { once } from "node:events";
import { createServer } from "node:http";
import { mkdtemp, mkdir, readFile, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { projectEvidence, verificationJob } from "./evidence";
import { verificationGate } from "./launcher-gate";

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
  const gate = await verificationGate(root, "recorded");
  const requests: unknown[] = [];
  let commands = 0;
  page.on("request", (request) => {
    if (new URL(request.url()).pathname === "/api/command") commands++;
  });
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
    if (
      repair &&
      body.messages.at(-1)?.role === "tool" &&
      body.messages.at(-1)?.tool_call_id === "repair_source"
    )
      message = {
        role: "assistant",
        content: null,
        tool_calls: [
          {
            id: "check_original_work",
            type: "function",
            function: {
              name: "check_lean_work",
              arguments: JSON.stringify({ id: "identity" }),
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
          ...gate.environment,
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
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    await page
      .getByRole("button", { name: "Work history", exact: true })
      .click();
  }
  async function idle() {
    await expect
      .poll(
        async () =>
          page.evaluate(async () =>
            (
              await fetch("/api/state", {
                headers: {
                  Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
                },
              })
            )
              .json()
              .then((x) => x.running),
          ),
        { timeout: 90000 },
      )
      .toBe(false);
  }
  async function approve() {
    const manager = page.getByRole("dialog", { name: /Work history/ });
    if (await manager.isVisible())
      await manager.getByRole("button", { name: "Close", exact: true }).click();
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Use this model", exact: true })
      .click();
  }
  try {
    await start(project, resolve(root, "pc-one"));
    await page
      .getByLabel("Project name", { exact: true })
      .fill("Portable verification");
    await page
      .getByRole("button", { name: "Create shared history", exact: true })
      .click();
    await expect(
      page.getByRole("heading", { name: "Portable verification", exact: true }),
    ).toBeVisible();
    await page
      .getByRole("button", { name: "Add work item", exact: true })
      .click();
    await page.getByLabel("Work item ID", { exact: true }).fill("identity");
    await page
      .getByLabel("Work item title", { exact: true })
      .fill("Natural-number identity");
    await page
      .getByLabel("Work item requirement", { exact: true })
      .fill("Every natural number equals itself.");
    await page.getByLabel("Lean module", { exact: true }).fill("Pilot");
    await page
      .getByLabel("Theorem names (one per line)", { exact: true })
      .fill("portable_identity");
    await page
      .getByLabel("Continuation note", { exact: true })
      .fill("Read the theorem and preserve its statement.");
    await page
      .getByRole("button", { name: "Save work item", exact: true })
      .click();
    await expect(
      page.getByRole("heading", {
        name: "Natural-number identity",
        exact: true,
      }),
    ).toBeVisible();
    const conversation = page.getByRole("region", {
      name: "Conversation",
      exact: true,
      includeHidden: true,
    });
    const chatBefore = await conversation.innerHTML();
    let rejectTransfer = true;
    await page.route("**/api/verification-job?*", async (route) => {
      const response = await route.fetch();
      if (response.status() !== 200) {
        await route.fulfill({ response });
        return;
      }
      const body = await response.json();
      if (
        rejectTransfer &&
        body.data?.kind === "recorded" &&
        body.data.phase.state === "running" &&
        body.data.phase.stage.state === "building"
      ) {
        rejectTransfer = false;
        await route.fulfill({
          status: 503,
          body: "Transient original-view transfer failure",
        });
      } else await route.fulfill({ response });
    });
    await page
      .getByRole("button", { name: "Recheck with Lean", exact: true })
      .click();
    await expect.poll(gate.started).toBe("started");
    const beforeRetry = commands;
    await page
      .getByRole("button", { name: "Retry check status", exact: true })
      .click();
    await expect(page.getByRole("status")).toContainText(
      "Building recorded module",
    );
    const retryCommands = commands - beforeRetry;
    expect(retryCommands).toBe(0);
    const running = await verificationJob(page, "work_job");
    expect(running?.kind).toBe("recorded");
    expect(running?.phase.state).toBe("running");
    expect(await verificationJob(page)).toBeUndefined();
    expect(await conversation.innerHTML()).toBe(chatBefore);
    await page.screenshot({
      path: info.outputPath("recorded-check-running.png"),
    });
    await page.getByRole("button", { name: "Stop check", exact: true }).click();
    await expect(
      page.getByRole("button", { name: "Recheck with Lean", exact: true }),
    ).toBeEnabled();
    const cancelled = await verificationJob(page, "work_job");
    if (
      cancelled?.kind !== "recorded" ||
      cancelled.phase.state !== "historical"
    )
      throw Error("Expected retained cancelled work");
    expect(cancelled.phase.result).toBe("cancelled");
    expect(await conversation.innerHTML()).toBe(chatBefore);
    await expect(
      page.getByRole("button", { name: "Recheck with Lean", exact: true }),
    ).toBeEnabled();
    await gate.reset();
    await page
      .getByRole("button", { name: "Recheck with Lean", exact: true })
      .click();
    await expect.poll(gate.started).toBe("started");
    const retried = await verificationJob(page, "work_job");
    expect(retried?.context.job).not.toBe(running?.context.job);
    await gate.release();
    await expect(
      page.getByText("Saved check: Passed", { exact: true }),
    ).toBeVisible({ timeout: 90_000 });
    expect(requests).toHaveLength(0);
    const checked = await verificationJob(page, "work_job");
    if (checked?.kind !== "recorded" || checked.phase.state !== "historical")
      throw Error("Expected retained original work check");
    expect(checked.phase.result).toBe("checked");
    expect(checked.phase.inputs.work.id).toBe("identity");
    expect(await conversation.innerHTML()).toBe(chatBefore);
    await info.attach("recorded-work-progress.json", {
      body: JSON.stringify({
        running,
        cancelled,
        retried,
        checked,
        retry_commands: retryCommands,
      }),
      contentType: "application/json",
    });
    await page
      .getByLabel("Progress note", { exact: true })
      .fill("The original theorem passed the real local checker.");
    await page
      .getByLabel("Next steps note", { exact: true })
      .fill("Inspect the same theorem after cloning.");
    await page
      .getByRole("button", { name: "Save progress", exact: true })
      .click();
    await expect(
      page.getByText("Inspect the same theorem after cloning.", {
        exact: true,
      }),
    ).toBeVisible();
    await approve();
    await page
      .getByRole("textbox", { name: "Message", exact: true })
      .fill("Retain this conversation for another computer.");
    await page.getByRole("button", { name: "Send", exact: true }).click();
    await expect(
      page.getByRole("region", { name: "Conversation", exact: true }),
    ).toContainText("remaining obligations are retained");
    await idle();
    await page
      .getByRole("button", { name: "Work history", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Save conversation and work", exact: true })
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
      page.getByRole("button", { name: "Resume this work item", exact: true }),
    ).toBeDisabled();
    await expect(
      page.getByText("Saved check: Passed", { exact: true }),
    ).toBeVisible();
    await expect(
      page.getByText("Saved results are historical.", { exact: false }),
    ).toBeVisible();
    await expect(
      page
        .getByRole("region", { name: "Conversation", exact: true })
        .getByText("Saved conversation", { exact: true }),
    ).toHaveCount(0);
    expect(requests).toHaveLength(count);
    expect(await verificationJob(page, "work_job")).toBeUndefined();
    expect(await verificationJob(page)).toBeUndefined();
    await page.getByText(/Saved conversation ·/).click();
    await expect(
      page.getByRole("region", {
        name: "Work and shared history",
        exact: true,
      }),
    ).toContainText("Retain this conversation for another computer.");
    await page
      .getByRole("button", { name: "Read proof source", exact: true })
      .click();
    await expect(
      page
        .locator(".ide-file-page:not([hidden])")
        .getByLabel("File contents", { exact: true }),
    ).toContainText("1: ", { timeout: 120000 });
    await page.getByRole("button", { name: "Next page", exact: true }).click();
    await expect(
      page
        .locator(".ide-file-page:not([hidden])")
        .getByLabel("File contents", { exact: true }),
    ).toContainText("201: ");
    await page.getByRole("button", { name: "Next page", exact: true }).click();
    await expect(
      page
        .locator(".ide-file-page:not([hidden])")
        .getByLabel("File contents", { exact: true }),
    ).toContainText("401: ");
    await expect(
      page
        .locator(".ide-file-page:not([hidden])")
        .getByLabel("File contents", { exact: true }),
    ).toContainText("PORTABLE-SOURCE-END");
    await page
      .getByRole("button", { name: "Previous page", exact: true })
      .click();
    await expect(
      page
        .locator(".ide-file-page:not([hidden])")
        .getByLabel("File contents", { exact: true }),
    ).toContainText("201: ");
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    await page
      .getByRole("button", { name: "Work history", exact: true })
      .click();
    // Deliberately break the source: old transferred success must not admit a fresh check.
    await writeFile(
      resolve(clone, "Pilot.lean"),
      "axiom injected : False\ntheorem portable_identity : False := injected\n",
    );
    await page
      .getByRole("button", { name: "Recheck with Lean", exact: true })
      .click();
    await expect(
      page.getByText("Saved check: Failed or interrupted", { exact: true }),
    ).toBeVisible({ timeout: 90_000 });
    await page
      .getByRole("dialog")
      .getByText(/Failed or interrupted Lean check/)
      .first()
      .click();
    await expect(
      page.getByRole("dialog").getByLabel("Theorem types and axioms").first(),
    ).toContainText("Unexpected axiom");
    expect(requests).toHaveLength(count);
    await approve();
    repair = true;
    await page
      .getByRole("button", { name: "Work history", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Resume this work item", exact: true })
      .click();
    await expect
      .poll(async () => readFile(resolve(clone, "Pilot.lean"), "utf8"))
      .toBe(original);
    await idle();
    const modelCheck = await verificationJob(page);
    if (
      modelCheck?.kind !== "recorded" ||
      modelCheck.phase.state !== "historical"
    )
      throw Error("Expected actual model-requested work check");
    expect(modelCheck.context.origin).toBe("model");
    expect(modelCheck.phase.result).toBe("checked");
    const beforeHostCheck = await verificationJob(page, "work_job");
    expect(beforeHostCheck?.context.job).not.toBe(modelCheck.context.job);
    await page
      .getByRole("button", { name: "Recheck with Lean", exact: true })
      .click();
    await expect
      .poll(async () =>
        page.evaluate(async (old) => {
          const state = await (
            await fetch("/api/state", {
              headers: {
                Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
              },
            })
          ).json();
          return state.work_job.state === "ready" && state.work_job.job !== old;
        }, beforeHostCheck?.context.job),
      )
      .toBe(true);
    await idle();
    const recovered = await verificationJob(page, "work_job");
    if (
      recovered?.kind !== "recorded" ||
      recovered.phase.state !== "historical"
    )
      throw Error("Expected completed host recovery check");
    expect(recovered.phase.result).toBe("checked");
    expect(await verificationJob(page)).toEqual(modelCheck);
    await info.attach("recorded-work-recovered.json", {
      body: JSON.stringify({ modelCheck, recovered }),
      contentType: "application/json",
    });
    await expect(
      page.getByText("Saved check: Passed", { exact: true }),
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
    expect((await projectEvidence(page, state)).state).toBe("empty");
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
    const metrics = await page.evaluate(async () =>
      (
        await fetch("/api/metrics", {
          headers: {
            Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
          },
        })
      ).json(),
    );
    for (const name of [
      "recorded.check.total",
      "recorded.check.lake",
      "recorded.check.inputs",
      "recorded.check.inspect",
      "recorded.history.read",
      "recorded.history.write",
    ])
      expect(
        metrics.records.some((r: { name: string }) => r.name === name),
      ).toBe(true);
    await info.attach("recorded-work-metrics.json", {
      body: JSON.stringify(metrics),
      contentType: "application/json",
    });
    await info.attach("portable-workspace", {
      body: await readFile(resolve(clone, ".meden/workspace.json")),
      contentType: "application/json",
    });
  } finally {
    await page.unrouteAll({ behavior: "wait" });
    await stop();
    await new Promise<void>((resolve) => server.close(() => resolve()));
    await rm(root, { recursive: true, force: true });
  }
});
