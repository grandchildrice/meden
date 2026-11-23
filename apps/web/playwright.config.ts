import { defineConfig } from "@playwright/test";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";
const systemChrome = "/run/current-system/sw/bin/google-chrome-stable";
const chrome =
  process.env.MEDEN_GUI_CHROME ||
  (existsSync(systemChrome) ? systemChrome : undefined);
export default defineConfig({
  testDir: "./e2e",
  outputDir: process.env.MEDEN_GUI_OUTPUT
    ? resolve(process.env.MEDEN_GUI_OUTPUT)
    : "../../target/gui-checks/results",
  timeout: 30_000,
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  reporter: [
    ["list"],
    [
      "html",
      {
        outputFolder: process.env.MEDEN_GUI_REPORT
          ? resolve(process.env.MEDEN_GUI_REPORT)
          : fileURLToPath(
              new URL("../../target/gui-checks/report", import.meta.url),
            ),
        open: "never",
      },
    ],
  ],
  use: {
    baseURL: "http://127.0.0.1:4321",
    viewport: { width: 1440, height: 960 },
    reducedMotion:
      process.env.MEDEN_GUI_MOTION === "no-preference"
        ? "no-preference"
        : "reduce",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
    launchOptions: { executablePath: chrome, args: ["--no-sandbox"] },
  },
  webServer: {
    command:
      "npm run build && npm run preview --workspace @meden/web -- --port 4321",
    cwd: fileURLToPath(new URL("../..", import.meta.url)),
    url: "http://127.0.0.1:4321",
    reuseExistingServer: false,
    timeout: 60_000,
  },
});
