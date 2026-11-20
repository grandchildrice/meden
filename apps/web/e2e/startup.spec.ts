import { test, expect } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, mkdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { createServer } from "node:http";

test("missing GUI assets explain the build step and a built GUI opens with fresh review", async ({
  page,
}) => {
  const root = await mkdtemp(resolve(tmpdir(), "meden-gui-startup-"));
  const workspace = resolve(root, "project");
  await mkdir(workspace);
  let requests = 0;
  const server = createServer((_request, response) => {
    requests++;
    response.writeHead(500);
    response.end();
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const base = `http://127.0.0.1:${(server.address() as { port: number }).port}/v1`;
  let child: ChildProcess | undefined;
  const start = (assets: string) => {
    const started = spawn(
      process.env.MEDEN_BIN || resolve("target/debug/meden"),
      [
        "gui",
        "--workspace",
        workspace,
        "--assets",
        assets,
        "--port",
        "0",
        "--base-url",
        base,
        "--model",
        "startup-fixture-no-inference",
      ],
      {
        env: {
          ...process.env,
          XDG_DATA_HOME: resolve(root, "data"),
        },
      },
    );
    child = started;
    return started;
  };
  try {
    const missing = start(resolve(root, "not-built", "dist"));
    let error = "";
    missing.stderr!.on("data", (data) => (error += String(data)));
    const [exit] = await once(missing, "close");
    expect(exit).not.toBe(0);
    expect(error).toContain("npm run build");
    expect(error).toContain("--assets");
    expect(requests).toBe(0);

    const running = start(resolve("apps/web/dist"));
    const url = await new Promise<string>((accept, reject) => {
      const timer = setTimeout(
        () => reject(new Error("GUI startup timed out")),
        15_000,
      );
      running.stdout!.on("data", (data) => {
        const match = String(data).match(/Meden GUI: (http:\/\/\S+)/);
        if (match) {
          clearTimeout(timer);
          accept(match[1]!);
        }
      });
      running.once("exit", (code) => {
        clearTimeout(timer);
        reject(new Error(`GUI exited before readiness: ${code}`));
      });
    });
    await page.goto(url);
    await expect(
      page.getByRole("button", { name: "接続先を確認", exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "送信", exact: true }),
    ).toBeDisabled();
    await page.getByRole("button", { name: "作業・履歴", exact: true }).click();
    await expect(
      page.getByRole("heading", { name: "作業と共有履歴", exact: true }),
    ).toBeVisible();
    expect(requests).toBe(0);
    await page.screenshot({
      path: resolve("target/gui-checks/startup-recovered.png"),
    });
  } finally {
    if (child && child.exitCode === null && child.signalCode === null) {
      const closed = once(child, "exit");
      child.kill("SIGTERM");
      await closed;
    }
    server.closeAllConnections();
    await new Promise<void>((accept) => server.close(() => accept()));
    await rm(root, { recursive: true, force: true });
  }
});
