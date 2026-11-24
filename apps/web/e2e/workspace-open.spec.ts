import { test, expect, type Page } from "@playwright/test";
import { spawn, execFile, type ChildProcess } from "node:child_process";
import { once } from "node:events";
import { createServer } from "node:http";
import {
  mkdtemp,
  mkdir,
  writeFile,
  readFile,
  symlink,
  rm,
} from "node:fs/promises";
import { resolve } from "node:path";
import { tmpdir } from "node:os";
import { promisify } from "node:util";
import { projectEvidence } from "./evidence";
import type {
  NativeState,
  WorkspaceOverview,
} from "@meden/agent-protocol/native";

const exec = promisify(execFile);
async function state(page: Page): Promise<NativeState> {
  return page.evaluate(async () =>
    (
      await fetch("/api/state", {
        headers: {
          Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
        },
      })
    ).json(),
  );
}
async function command(page: Page, body: unknown) {
  return page.evaluate(
    async (body) =>
      (
        await fetch("/api/command", {
          method: "POST",
          headers: {
            Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
            "Content-Type": "application/json",
          },
          body: JSON.stringify(body),
        })
      ).status,
    body,
  );
}

test("unreadable portable history stays intact while the GUI graph and ordinary chat remain usable", async ({
  page,
}) => {
  test.setTimeout(120000);
  const root = await mkdtemp(resolve(tmpdir(), "meden-unreadable-history-"));
  const project = resolve(root, "project");
  await mkdir(resolve(project, ".meden"), { recursive: true });
  await writeFile(
    resolve(project, "Proof.lean"),
    "theorem identity (n : Nat) : n = n := rfl\n",
  );
  const original = '{"version":1,"name":"Broken","accepted":true}';
  const record = resolve(project, ".meden/workspace.json");
  await writeFile(record, original);
  let requests = 0;
  const server = createServer(async (request, response) => {
    if (request.url === "/v1/models") {
      response.writeHead(200, { "Content-Type": "application/json" });
      response.end(
        JSON.stringify({ data: [{ id: "unreadable-history-fixture" }] }),
      );
      return;
    }
    for await (const _chunk of request) {
      /* Consume the actual model request. */
    }
    requests++;
    response.writeHead(200, { "Content-Type": "application/json" });
    response.end(
      JSON.stringify({
        choices: [
          {
            message: {
              role: "assistant",
              content: "ORDINARY-GUI-CHAT-AVAILABLE",
            },
            finish_reason: "stop",
          },
        ],
      }),
    );
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const child = spawn(
    process.env.MEDEN_BIN || resolve("target/debug/meden"),
    [
      "gui",
      "--workspace",
      project,
      "--assets",
      resolve("apps/web/dist"),
      "--port",
      "0",
      "--base-url",
      `http://127.0.0.1:${(server.address() as { port: number }).port}/v1`,
      "--model",
      "unreadable-history-fixture",
    ],
    { env: { ...process.env, XDG_DATA_HOME: resolve(root, "data") } },
  );
  try {
    const url = await new Promise<string>((accept, reject) => {
      const timer = setTimeout(
        () => reject(new Error("GUI startup timeout")),
        20_000,
      );
      child.stdout!.on("data", (part) => {
        const match = String(part).match(/Meden GUI: (http:\/\/\S+)/);
        if (match) {
          clearTimeout(timer);
          accept(match[1]!);
        }
      });
      child.once("exit", (code) => {
        clearTimeout(timer);
        reject(new Error(`GUI exited ${code}`));
      });
    });
    await page.goto(url);
    await expect(
      page.getByRole("alert").filter({
        hasText: "Could not read saved conversations and work history.",
      }),
    ).toBeVisible();
    await expect.poll(async () => (await state(page)).overview.scanned).toBe(1);
    expect((await projectEvidence(page, await state(page))).state).toBe(
      "empty",
    );
    expect(requests).toBe(0);
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Use this model", exact: true })
      .click();
    await page
      .getByRole("textbox", { name: "Message", exact: true })
      .fill("Keep ordinary chat available and preserve the unreadable record.");
    await page.getByRole("button", { name: "Send", exact: true }).click();
    await expect(
      page
        .getByRole("region", { name: "Conversation", exact: true })
        .getByText("ORDINARY-GUI-CHAT-AVAILABLE", { exact: true }),
    ).toBeVisible();
    await page
      .getByRole("button", { name: "Work history", exact: true })
      .click();
    await page.getByText("Save notice", { exact: true }).first().click();
    await expect(
      page
        .getByRole("alert")
        .filter({
          hasText: "Portable history was retained but could not be saved",
        })
        .first(),
    ).toBeVisible();
    expect(requests).toBe(1);
    expect(await readFile(record, "utf8")).toBe(original);
    await page.getByRole("button", { name: "Close", exact: true }).click();
    await page.reload();
    await expect(
      page
        .getByRole("region", { name: "Conversation", exact: true })
        .getByText("ORDINARY-GUI-CHAT-AVAILABLE", { exact: true }),
    ).toBeVisible();
    await expect.poll(async () => (await state(page)).overview.scanned).toBe(1);
    expect(await readFile(record, "utf8")).toBe(original);
    await page.screenshot({
      path: "target/gui-checks/workspace-unreadable-history.png",
    });
  } finally {
    if (child.exitCode === null && child.signalCode === null) {
      const end = once(child, "close");
      child.kill("SIGTERM");
      await end;
    }
    server.closeAllConnections();
    await new Promise<void>((accept) => server.close(() => accept()));
    await rm(root, { recursive: true, force: true });
  }
});

test("opening a cloned workspace shows chat and the whole graph, then selects and switches host connections", async ({
  page,
}) => {
  test.setTimeout(180_000);
  const root = await mkdtemp(resolve(tmpdir(), "meden-workspace-open-"));
  const project = resolve(root, "project"),
    clone = resolve(root, "clone"),
    hermes = resolve(root, "hermes");
  const calls: Array<{
    server: string;
    model: string;
    authorization?: string;
    messages: unknown;
  }> = [];
  let modelListAvailable = true;
  const servers = ["first", "second"].map((name) =>
    createServer(async (request, response) => {
      if (request.url === "/v1/models") {
        response.writeHead(modelListAvailable ? 200 : 503, {
          "Content-Type": "application/json",
        });
        response.end(
          JSON.stringify({
            data: [{ id: `${name}-default` }, { id: `${name}-chosen` }],
          }),
        );
        return;
      }
      let text = "";
      for await (const part of request) text += String(part);
      const body = JSON.parse(text);
      calls.push({
        server: name,
        model: body.model,
        authorization: request.headers.authorization,
        messages: body.messages,
      });
      const rawInstruction = body.messages
        .filter((message: { role: string }) => message.role === "user")
        .at(-1)?.content;
      const instruction =
        typeof rawInstruction === "string"
          ? rawInstruction
          : rawInstruction
              ?.filter((part: { type: string }) => part.type === "text")
              .map((part: { text: string }) => part.text)
              .join("\n");
      const fault =
        instruction ===
        "Create an oversized source to inspect its availability.";
      const repair =
        instruction ===
        "Remove the oversized source and reread the whole graph.";
      const message =
        (fault || repair) && body.messages.at(-1)?.role === "user"
          ? {
              role: "assistant",
              content: null,
              tool_calls: [
                {
                  id: fault
                    ? "create_oversized_source"
                    : "remove_oversized_source",
                  type: "function",
                  function: {
                    name: "bash",
                    arguments: JSON.stringify({
                      command: fault
                        ? "python3 -c \"from pathlib import Path; Path('Oversized.lean').write_text('x' * 1048577)\""
                        : "rm Oversized.lean",
                    }),
                  },
                },
              ],
            }
          : {
              role: "assistant",
              content: fault
                ? "OVERSIZED-SOURCE-CREATED"
                : repair
                  ? "OVERSIZED-SOURCE-REMOVED"
                  : `CONTINUED-ON-${name}`,
            };
      response.writeHead(200, { "Content-Type": "application/json" });
      response.end(
        JSON.stringify({
          choices: [
            {
              message,
              finish_reason: "stop",
            },
          ],
        }),
      );
    }),
  );
  let child: ChildProcess | undefined;
  async function stop() {
    if (child && child.exitCode === null && child.signalCode === null) {
      const end = once(child, "close");
      child.kill("SIGTERM");
      await end;
    }
    child = undefined;
  }
  async function start(workspace: string, data: string) {
    child = spawn(
      process.env.MEDEN_BIN || resolve("target/debug/meden"),
      [
        "gui",
        "--workspace",
        workspace,
        "--assets",
        resolve("apps/web/dist"),
        "--port",
        "0",
      ],
      {
        env: {
          ...process.env,
          XDG_DATA_HOME: data,
          HERMES_HOME: hermes,
          CODEX_HOME: resolve(root, "codex"),
          CLAUDE_CONFIG_DIR: resolve(root, "claude"),
          MEDEN_LLM_BASE_URL: "",
          MEDEN_LLM_MODEL: "",
          MEDEN_LLM_API_KEY: "",
          OPENAI_API_KEY: "",
          OLLAMA_API_KEY: "",
          CLAUDE_CODE_OAUTH_TOKEN: "",
          ANTHROPIC_TOKEN: "",
        },
      },
    );
    let stderr = "";
    child.stderr!.on("data", (part) => (stderr += String(part)));
    const url = await new Promise<string>((accept, reject) => {
      const timer = setTimeout(
        () => reject(new Error(`GUI startup: ${stderr}`)),
        20_000,
      );
      child!.stdout!.on("data", (part) => {
        const match = String(part).match(/Meden GUI: (http:\/\/\S+)/);
        if (match) {
          clearTimeout(timer);
          accept(match[1]!);
        }
      });
      child!.once("exit", (code) => {
        clearTimeout(timer);
        reject(new Error(`GUI exited ${code}: ${stderr}`));
      });
    });
    await page.goto(url);
    await expect
      .poll(async () => (await state(page)).overview.state)
      .toBe("ready");
    await expect(
      page.getByRole("region", {
        name: "Project map",
        exact: true,
      }),
    ).toBeVisible();
  }
  try {
    for (const server of servers) {
      server.listen(0, "127.0.0.1");
      await once(server, "listening");
    }
    const urls = servers.map(
      (server) =>
        `http://127.0.0.1:${(server.address() as { port: number }).port}/v1`,
    );
    await mkdir(hermes);
    await writeFile(
      resolve(hermes, "config.yaml"),
      `custom_providers:\n${["first", "second"].map((name, index) => `  - name: ${name}\n    base_url: ${urls[index]}\n    model: ${name}-default\n    key_env: ${name.toUpperCase()}_KEY\n`).join("")}`,
    );
    await writeFile(
      resolve(hermes, ".env"),
      "FIRST_KEY=FIRST_HOST_SECRET\nSECOND_KEY=SECOND_HOST_SECRET\n",
    );
    await mkdir(resolve(project, ".meden"), { recursive: true });
    await mkdir(resolve(project, "Core"));
    await mkdir(resolve(project, "Extra"));
    for (let index = 0; index < 40; index++)
      await writeFile(
        resolve(project, "Extra", `Part${index}.lean`),
        `import Core.Pilot\ndef value${index} (n : Nat) : Nat := n\n`,
      );
    await writeFile(
      resolve(project, "Core/Pilot.lean"),
      "namespace Core.Pilot\ntheorem identity (n : Nat) : n = n := rfl\nend Core.Pilot\n",
    );
    await writeFile(
      resolve(root, "outside.lean"),
      "theorem SECRET_OUTSIDE : True := True.intro\n",
    );
    await symlink(
      resolve(root, "outside.lean"),
      resolve(project, "Outside.lean"),
    );
    await mkdir(resolve(project, "target"));
    await writeFile(
      resolve(project, "target/Generated.lean"),
      "def ignored := 0\n",
    );
    const messages = [
      {
        speaker: "user",
        text: "ORIGINAL-CHAT: prove identity without weakening the requirement",
      },
      {
        speaker: "assistant",
        text: "SAVED-CHAT: the original task remains open",
      },
    ];
    await writeFile(
      resolve(project, ".meden/workspace.json"),
      JSON.stringify({
        version: 1,
        name: "Shared project",
        tasks: [
          {
            id: "identity",
            title: "Identity for every natural number",
            requirement:
              "Every natural number equals itself; retain all inputs.",
            module: "Core.Pilot",
            theorems: ["Core.Pilot.identity"],
            next: "Inspect the existing proof and recheck locally.",
          },
        ],
        events: [],
        conversations: [
          { id: "old_prefix", messages: messages.slice(0, 1) },
          { id: "saved", messages },
        ],
      }),
    );
    await writeFile(resolve(project, ".gitignore"), "target/\n");
    await start(project, resolve(root, "pc-one"));
    const chat = page.getByRole("region", {
      name: "Conversation",
      exact: true,
    });
    await expect(
      chat.getByText(messages[0]!.text, { exact: true }),
    ).toHaveCount(1);
    await expect(
      chat.getByText(messages[1]!.text, { exact: true }),
    ).toBeVisible();
    await expect
      .poll(async () => (await state(page)).overview.scanned)
      .toBe(41);
    await page
      .getByRole("button", { name: "Work history", exact: true })
      .click();
    await expect(
      page.getByRole("region", { name: "Work and shared history" }),
    ).toContainText("Identity for every natural number");
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Close", exact: true })
      .click();
    const overview: WorkspaceOverview = await page.evaluate(async () =>
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
              Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
            },
          },
        )
      )
        .json()
        .then((publication) => publication.data),
    );
    expect(overview.files).toHaveLength(41);
    expect(
      overview.files.some(
        (file) => file.path.includes("Outside") || file.path.includes("target"),
      ),
    ).toBe(false);
    const original = await state(page);
    expect(original.authorized).toBe(false);
    expect((await projectEvidence(page, original)).state).toBe("empty");
    expect(JSON.stringify(original)).not.toContain("HOST_SECRET");
    expect(calls).toHaveLength(0);
    await page
      .getByRole("button", { name: "Folder: Extra", exact: true })
      .click();
    await page
      .getByRole("button", { name: "File: Extra/Part39.lean", exact: true })
      .click();
    await expect(
      page.locator(".ide-file-page:not([hidden])").getByLabel("File contents"),
    ).toContainText("value39", { timeout: 120000 });
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    // Source is readable before compiler preparation finishes. Connection
    // changes still wait for that real host operation's compiler deadline.
    await expect
      .poll(async () => (await state(page)).running, { timeout: 120000 })
      .toBe(false);
    await page
      .getByLabel("provider", { exact: true })
      .selectOption("hermes:first");
    await expect
      .poll(async () => (await state(page)).models)
      .toContain("first-chosen");
    await page.getByRole("combobox", { name: "model", exact: true }).click();
    await page
      .getByRole("option", { name: "first-chosen", exact: true })
      .click();
    await expect(page.getByRole("dialog")).toContainText(
      `${urls[0]}/chat/completions`,
    );
    await expect(
      page.getByRole("combobox", { name: "model", exact: true }),
    ).toHaveValue("first-chosen");
    await page
      .getByRole("button", { name: "Use this model", exact: true })
      .click();
    await page
      .getByRole("textbox", { name: "Message", exact: true })
      .fill("Continue the saved identity work.");
    await page.getByRole("button", { name: "Send", exact: true }).click();
    await expect(
      chat.getByText("CONTINUED-ON-first", { exact: true }),
    ).toBeVisible();
    expect(calls).toHaveLength(1);
    expect(calls[0]!.model).toBe("first-chosen");
    expect(calls[0]!.authorization).toBe("Bearer FIRST_HOST_SECRET");
    expect(JSON.stringify(calls[0]!.messages)).toContain(
      "Every natural number equals itself; retain all inputs.",
    );
    expect(JSON.stringify(calls[0]!.messages)).toContain("SAVED-CHAT:");
    const approved = await state(page);
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    modelListAvailable = false;
    await page
      .getByLabel("provider", { exact: true })
      .selectOption("hermes:second");
    await expect(
      page.getByText("Could not load models.", { exact: false }),
    ).toBeVisible();
    modelListAvailable = true;
    await page.getByRole("button", { name: "Close", exact: true }).click();
    await page
      .getByRole("button", { name: "Choose model", exact: true })
      .click();
    await expect
      .poll(async () => (await state(page)).models)
      .toContain("second-chosen");
    await page.getByRole("combobox", { name: "model", exact: true }).click();
    await page
      .getByRole("option", { name: "second-chosen", exact: true })
      .click();
    await expect.poll(async () => (await state(page)).authorized).toBe(false);
    expect(
      await command(page, {
        type: "approve_processor",
        connection_revision: approved.connection_revision,
        endpoint: approved.endpoint,
        model: approved.model,
        judge: approved.judge,
      }),
    ).toBe(409);
    expect(calls).toHaveLength(1);
    await page
      .getByRole("button", { name: "Use this model", exact: true })
      .click();
    await page
      .getByRole("textbox", { name: "Message", exact: true })
      .fill("Continue on the newly reviewed connection.");
    await page.getByRole("button", { name: "Send", exact: true }).click();
    await expect(
      chat.getByText("CONTINUED-ON-second", { exact: true }),
    ).toBeVisible();
    expect(calls).toHaveLength(2);
    expect(calls[1]!.authorization).toBe("Bearer SECOND_HOST_SECRET");
    expect(calls[1]!.model).toBe("second-chosen");
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    await page
      .getByRole("textbox", { name: "Message", exact: true })
      .fill("Create an oversized source to inspect its availability.");
    await page.getByRole("button", { name: "Send", exact: true }).click();
    await expect(
      chat.getByText("OVERSIZED-SOURCE-CREATED", { exact: true }),
    ).toBeVisible();
    // Unsupported content stays in a complete inventory instead of making the
    // whole project unreadable. The file reader explains its actual limit.
    await expect
      .poll(async () => {
        const overview = (await state(page)).overview;
        return [overview.state, overview.scanned];
      })
      .toEqual(["ready", 42]);
    await page
      .getByRole("region", { name: "File explorer", exact: true })
      .getByRole("button", { name: "File: Oversized.lean", exact: true })
      .click();
    await expect(page.locator(".ide-file-page:not([hidden])")).toContainText(
      "exceeds",
    );
    await expect.poll(async () => (await state(page)).running).toBe(false);
    await page.getByRole("tab", { name: "Project map", exact: true }).click();
    await page
      .getByRole("textbox", { name: "Message", exact: true })
      .fill("Remove the oversized source and reread the whole graph.");
    await page.getByRole("button", { name: "Send", exact: true }).click();
    await expect
      .poll(async () => (await state(page)).overview.state)
      .toBe("ready");
    await expect
      .poll(async () => (await state(page)).overview.scanned)
      .toBe(41);
    await expect(
      chat.getByText("OVERSIZED-SOURCE-REMOVED", { exact: true }),
    ).toBeVisible();
    const completedCalls = calls.length;
    await stop();
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
    await exec("git", ["clone", "-q", project, clone]);
    await start(clone, resolve(root, "pc-two"));
    await expect(
      chat.getByText("CONTINUED-ON-second", { exact: true }),
    ).toBeVisible();
    await expect(
      chat.getByText(messages[0]!.text, { exact: true }),
    ).toHaveCount(1);
    expect((await state(page)).authorized).toBe(false);
    expect(calls).toHaveLength(completedCalls);
    expect(
      await readFile(resolve(clone, ".meden/workspace.json"), "utf8"),
    ).not.toContain("HOST_SECRET");
    await page.screenshot({
      path: "target/gui-checks/workspace-open-clone.png",
    });
    await page.reload();
    await expect(
      chat.getByText("CONTINUED-ON-second", { exact: true }),
    ).toBeVisible();
    await expect
      .poll(async () => (await state(page)).overview.scanned)
      .toBe(41);
    expect((await state(page)).authorized).toBe(false);
    expect(calls).toHaveLength(completedCalls);
  } finally {
    await stop();
    for (const server of servers) {
      server.closeAllConnections();
      await new Promise<void>((accept) => server.close(() => accept()));
    }
    await rm(root, { recursive: true, force: true });
  }
});
