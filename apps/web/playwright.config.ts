import { defineConfig } from "@playwright/test";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
const systemChrome = "/run/current-system/sw/bin/google-chrome-stable";
const chrome =
  process.env.MEDEN_GUI_CHROME ||
  (existsSync(systemChrome) ? systemChrome : undefined);
export default defineConfig({
  testDir: "./e2e",
  outputDir: "../../target/gui-checks/results",
  timeout: 30_000,
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  reporter: [
    ["list"],
    [
      "html",
      {
        outputFolder: fileURLToPath(
          new URL("../../target/gui-checks/report", import.meta.url),
        ),
        open: "never",
      },
    ],
  ],
  use: {
    baseURL: "http://127.0.0.1:4320",
    viewport: { width: 1440, height: 960 },
    reducedMotion: "reduce",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
    launchOptions: { executablePath: chrome, args: ["--no-sandbox"] },
  },
  webServer: {
    command: "npm run dev",
    cwd: fileURLToPath(new URL("../..", import.meta.url)),
    url: "http://127.0.0.1:4320",
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
  },
});
