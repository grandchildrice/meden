import { test, expect, type Page } from "@playwright/test";
import {
  createVendingWorkspace,
  vendingOpeningRequest,
} from "../src/fixtures/meden-demo";
import { mkdir } from "node:fs/promises";

async function archivedChat(page: Page) {
  await page.goto("/");
  await page
    .getByRole("complementary", { name: "Conversation sidebar" })
    .getByRole("button", { name: "Vending machine simulator", exact: true })
    .click();
  await expect(page.locator(".message")).toHaveCount(14);
  await expect(page.locator(".user-message").first()).toHaveText(
    vendingOpeningRequest,
  );
}
async function tab(page: Page, name: string) {
  await page.getByRole("tab", { name, exact: true }).click();
  await expect(page.getByRole("tabpanel")).toBeVisible();
  await expect(page.getByRole("tabpanel").locator(".loading")).toHaveCount(0);
}

async function openProof(page: Page) {
  await page
    .getByRole("button", { name: "Open artifact", exact: true })
    .click();
  await tab(page, "Proof");
}

test("the conversation sidebar collapses independently of the artifact panel", async ({
  page,
}) => {
  await archivedChat(page);
  const sidebar = page.getByRole("complementary", {
    name: "Conversation sidebar",
  });
  const topbar = page.getByRole("banner");
  const artifacts = page.getByRole("region", {
    name: "Implementation and verification",
  });
  const input = page.getByRole("textbox", { name: "Message", exact: true });
  await input.fill("Work in progress");
  await expect(topbar.getByRole("button")).toHaveCount(2);
  await expect(topbar.getByText("Demo", { exact: true })).toHaveCount(0);
  await topbar.getByRole("button", { name: "Open artifact" }).click();
  await expect(artifacts).toBeVisible();
  await topbar.getByRole("button", { name: "Close conversations" }).click();
  await expect(sidebar).toBeHidden();
  await expect(artifacts).toBeVisible();
  await topbar.getByRole("button", { name: "Close artifact" }).click();
  await expect(artifacts).toBeHidden();
  await expect(sidebar).toBeHidden();
  await expect(input).toHaveValue("Work in progress");
  await topbar
    .getByRole("button", { name: "Open conversations" })
    .press("Enter");
  await expect(sidebar).toBeVisible();
  await expect(
    sidebar.getByRole("button", { name: "Vending machine simulator" }),
  ).toHaveAttribute("aria-current", "page");
  await sidebar.getByRole("button", { name: "Settings" }).click();
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export workspace" }).click();
  expect((await download).suggestedFilename()).toBe("meden-workspace.json");
});

test("home starts with a large composer and Send; instruction creates its title and opens results", async ({
  page,
}) => {
  const requests: string[] = [];
  page.on("request", (request) => {
    if (
      !request.url().startsWith("http://127.0.0.1:4321") &&
      !request.url().startsWith("data:")
    )
      requests.push(request.url());
  });
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "What would you like to build safely?",
  );
  await expect(
    page.getByRole("textbox", { name: "Message", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Send", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Save draft", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("textbox", { name: "Name", exact: true }),
  ).toHaveCount(0);
  await page
    .getByRole("textbox", { name: "Message", exact: true })
    .fill("Check the vending machine proofs");
  await page.getByRole("button", { name: "Send", exact: true }).click();
  await expect(page.locator(".conversation-title")).toHaveText(
    "Vending machine simulator",
  );
  await expect(
    page.getByRole("tab", { name: "Proof", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await expect(page.locator(".proof-obligations")).toContainText("8 / 8");
  await expect(page.locator(".assistant-message").last()).toContainText(
    "The saved record",
  );
  await page.reload();
  await expect(page.locator(".user-message").last()).toContainText(
    "Check the vending machine proofs",
  );
  await expect(page.locator(".assistant-message").last()).toContainText(
    "The saved record",
  );
  expect(requests).toEqual([]);
});

test("home displays the pitch logos below the suggestions at desktop and phone widths", async ({
  page,
}) => {
  await page.goto("/");
  const logos = page.getByRole("list", {
    name: "Languages and verification tools",
  });
  const suggestion = page.getByRole("button", {
    name: "Build a vending machine simulator",
  });
  const names = [
    "Rust",
    "Bend",
    "Lean",
    "Rocq",
    "Dafny",
    "Isabelle",
    "TLA+",
    "cvc5",
    "Z3",
    "Certora",
  ];
  await expect(
    page.getByText("Start a conversation from a message", { exact: true }),
  ).toHaveCount(0);
  await expect(logos.getByRole("img")).toHaveCount(names.length);
  const suggestionElement = await suggestion.elementHandle();
  for (const width of [1440, 390]) {
    await page.setViewportSize({ width, height: 844 });
    await expect
      .poll(() =>
        logos.evaluate(
          (list, button) =>
            list.getBoundingClientRect().top -
            button!.getBoundingClientRect().bottom,
          suggestionElement,
        ),
      )
      .toBeGreaterThan(0);
    for (const name of names) {
      const logo = logos.getByRole("img", { name, exact: true });
      await expect(logo).toBeInViewport({ ratio: 0.9999 });
      await expect(logo).toHaveJSProperty("complete", true);
      expect(
        await logo.evaluate((image: HTMLImageElement) => image.naturalWidth),
      ).toBeGreaterThan(0);
    }
  }
  await suggestion.click();
  await page.getByRole("button", { name: "Send", exact: true }).click();
  await page.getByRole("button", { name: "Close panel", exact: true }).click();
  await expect(logos).toHaveCount(0);
  await expect(
    page.getByText("Enter to send · Shift + Enter for a new line"),
  ).toBeVisible();
});

test("the sidebar keeps one continuous vending history including its original opening request", async ({
  page,
}) => {
  await archivedChat(page);
  await expect(page.locator(".recorded-tool")).toHaveCount(25);
  const failure = page
    .locator(".recorded-tool")
    .filter({
      has: page.locator("summary>code", { hasText: "propose_contract" }),
    })
    .first();
  await failure.locator("summary").click();
  await expect(failure).toContainText("maximum recursion depth");
  const tests = page
    .locator(".recorded-tool")
    .filter({ has: page.locator(".recorded-tool-details") })
    .filter({ hasText: "7 passed; 0 failed" });
  await expect(tests).toHaveCount(1);
  await tests.locator("summary").click();
  await expect(tests.locator("pre").last()).toContainText("7 passed; 0 failed");
  await expect(
    page.getByText("Saved development history", { exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByText("Saved in this browser", { exact: true }),
  ).toHaveCount(0);
  const sidebar = page.getByRole("complementary", {
    name: "Conversation sidebar",
  });
  await expect(
    sidebar.getByRole("button", { name: "Dependencies", exact: true }),
  ).toHaveCount(0);
  await expect(
    sidebar.getByRole("button", { name: "Meaning comparison", exact: true }),
  ).toHaveCount(0);
  await expect(page.locator(".transcript")).toContainText(
    "Can you continue development?",
  );
  await expect(page.locator(".transcript")).toContainText(
    "maximum recursion depth",
  );
  await expect(page.locator(".transcript")).toContainText(
    "Formal verification succeeded",
  );
  await sidebar
    .getByRole("button", { name: "New message", exact: true })
    .click();
  const input = page.getByRole("textbox", { name: "Message", exact: true });
  await input.fill("Check purchase conditions");
  await input.press("Enter");
  await expect(page.locator(".conversation-title")).toHaveText(
    "Vending machine simulator",
  );
  await expect(sidebar.locator(".history-item")).toHaveCount(1);
  await expect(page.locator(".user-message").first()).toHaveText(
    vendingOpeningRequest,
  );
  await expect(page.locator(".reply-activity")).toHaveCount(0);
  await page
    .getByRole("button", { name: "Close panel", exact: true })
    .last()
    .click();
  await sidebar
    .getByRole("button", { name: "Vending machine simulator", exact: true })
    .click();
  await expect(page.locator(".message")).toHaveCount(16);
  await sidebar
    .getByRole("button", { name: "Search conversations", exact: true })
    .click();
  await page
    .getByRole("textbox", { name: "Search conversations", exact: true })
    .fill("Check purchase conditions");
  await expect(sidebar.locator(".history-item")).toHaveCount(1);
});

test("the full conversation artifact opens preview and keeps proof details readable", async ({
  page,
}) => {
  await archivedChat(page);
  const card = page.getByRole("region", {
    name: "Vending machine simulator artifact",
  });
  const statuses = card.getByRole("list", {
    name: "Theorems and proof status",
  });
  const preview = card.getByRole("button", {
    name: "Vending machine simulator: open preview",
  });
  await expect(
    card.getByText("Code · Preview · Proof", { exact: true }),
  ).toHaveCount(0);
  await expect(statuses.getByRole("listitem")).toHaveCount(8);
  await expect(statuses.getByText("Proved", { exact: true })).toHaveCount(8);
  const purchase = statuses
    .getByRole("listitem")
    .filter({ hasText: "KernelProof.law_purchase_exact" });
  await expect(purchase).toContainText(
    "A successful purchase subtracts 150 yen and one item",
  );
  await expect(purchase).toContainText("Proved");
  await expect(statuses).toContainText(
    "Only refunds change the last refunded amount",
  );
  for (const name of ["Rust", "Lean"]) {
    const logo = card.getByRole("img", { name, exact: true });
    await expect(logo).toHaveJSProperty("complete", true);
    expect(
      await logo.evaluate((image: HTMLImageElement) => image.naturalWidth),
    ).toBeGreaterThan(0);
  }
  await preview.scrollIntoViewIfNeeded();
  await page.screenshot({
    path: "target/gui-checks/screenshots/artifact-card.png",
  });
  await preview.press("Enter");
  await expect(
    page.getByRole("tab", { name: "Preview", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page.getByRole("button", { name: "Close panel", exact: true }).click();
  await page.setViewportSize({ width: 390, height: 844 });
  await card.scrollIntoViewIfNeeded();
  await page.screenshot({
    path: "target/gui-checks/screenshots/artifact-card-mobile.png",
  });
  // Click the coordinates of the visible content under the full-card button.
  for (const target of [purchase, card.getByRole("img", { name: "Lean" })]) {
    await target.scrollIntoViewIfNeeded();
    // The toolbar hint can overlap the card on a phone. It must not eat clicks.
    await page
      .getByRole("button", { name: "Open artifact", exact: true })
      .hover();
    await expect(
      page.getByRole("tooltip", { name: "Open artifact", exact: true }),
    ).toBeVisible();
    const bounds = (await target.boundingBox())!;
    await page.mouse.click(
      bounds.x + bounds.width / 2,
      bounds.y + bounds.height / 2,
    );
    await expect(
      page.getByRole("tab", { name: "Preview", exact: true }),
    ).toHaveAttribute("aria-selected", "true");
    await page
      .getByRole("button", { name: "Close panel", exact: true })
      .click();
  }
});

test("proof opens the wider source graph and follows a declaration into its exact Lean source", async ({
  page,
}) => {
  await archivedChat(page);
  await openProof(page);
  await expect(page.getByRole("tab")).toHaveText(["Code", "Preview", "Proof"]);
  await expect(page.locator(".root-node")).toContainText(
    "Implementation/specification correspondence",
  );
  expect(await page.locator(".radar-node").count()).toBeGreaterThan(1);
  await page.getByRole("button", { name: /Rust transition.*: select/ }).click();
  await expect(
    page.getByRole("complementary", { name: "Selected proof" }),
  ).toContainText("KernelImpl.kernel.step");
  await page.locator(".proof-statement-link").click();
  await expect(
    page.getByRole("tab", { name: "Code", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await expect(page.locator(".selected-source-line")).toContainText(
    "def kernel.step",
  );
  await expect(page.locator(".source-location")).toContainText(
    "Candidate.lean:34",
  );
  await page
    .getByRole("button", { name: "Back to proof", exact: true })
    .click();
  await expect(
    page.getByRole("complementary", { name: "Selected proof" }),
  ).toContainText("KernelImpl.kernel.step");
  await page.getByRole("button", { name: "Center here", exact: true }).click();
  await expect(page.locator(".root-node")).toContainText("Rust transition");
  await page
    .getByRole("button", {
      name: /Implementation\/specification correspondence.*: select/,
    })
    .dblclick();
  await expect(page.locator(".root-node")).toContainText(
    "Implementation/specification correspondence",
  );
  expect(await page.locator(".radar-node").count()).toBeGreaterThan(1);
  await page.locator(".proof-obligations>summary").click();
  await page
    .locator(".proof-list>button")
    .filter({ hasText: "A successful purchase subtracts 150 yen and one item" })
    .click();
  await expect(page.locator(".proof-statement-link")).toContainText(
    "With sufficient balance and stock",
  );
  await page.locator(".proof-statement-link").click();
  await expect(page.locator(".selected-source-line")).toContainText(
    "theorem law_purchase_exact",
  );
  await expect(page.locator(".source-location")).toContainText("Check.lean:");
});

test("graph zoom survives pointer movement and canvas resizing until explicit fit", async ({
  page,
}) => {
  await archivedChat(page);
  await openProof(page);
  const canvas = page.locator(".cone-graph");
  await canvas.evaluate((element) => {
    (window as any).__graphTransitions = 0;
    element.addEventListener("transitionrun", () => {
      (window as any).__graphTransitions++;
    });
  });
  const viewport = canvas.locator(".react-flow__viewport");
  const zoom = () =>
    viewport.evaluate(
      (e) => new DOMMatrixReadOnly(getComputedStyle(e).transform).a,
    );
  const initial = await zoom();
  const box = (await canvas.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.wheel(0, -320);
  await expect.poll(zoom).toBeGreaterThan(initial * 1.2);
  // Observe the completed wheel gesture before testing pointer-only changes.
  const settle = () =>
    viewport.evaluate(async (e) => {
      let previous = "",
        stable = 0;
      for (let frame = 0; frame < 90; frame++) {
        await new Promise(requestAnimationFrame);
        const current = getComputedStyle(e).transform;
        stable = current === previous ? stable + 1 : 0;
        previous = current;
        if (stable >= 8) return current;
      }
      throw new Error("Wheel zoom did not settle");
    });
  const camera = await settle();
  for (let i = 0; i < 3; i++) {
    await canvas.locator(".root-node > button").hover();
    await page.mouse.move(box.x + box.width - 8, box.y + 8);
    await expect(viewport).toHaveCSS("transform", camera);
  }
  const beforeLegend = await zoom();
  const legend = (await canvas.getByLabel("Review scope").boundingBox())!;
  await page.mouse.move(
    legend.x + legend.width / 2,
    legend.y + legend.height / 2,
  );
  await page.mouse.wheel(0, -160);
  await expect.poll(zoom).toBeGreaterThan(beforeLegend * 1.1);
  await settle();
  const enlarged = await zoom();
  await page.setViewportSize({ width: 1280, height: 900 });
  await expect
    .poll(async () => (await canvas.boundingBox())!.width)
    .not.toBe(box.width);
  // Wait for resize observers and rendering, then check that they did not fit.
  await viewport.evaluate(async () => {
    for (let i = 0; i < 8; i++) await new Promise(requestAnimationFrame);
  });
  expect(await zoom()).toBeCloseTo(enlarged, 6);
  await canvas.getByRole("button", { name: "Fit View", exact: true }).click();
  await expect.poll(zoom).toBeLessThan(enlarged);
  expect(await page.evaluate(() => (window as any).__graphTransitions)).toBe(0);
});

test("hover keeps dense graph nodes in place until the pointer selects them", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 844 });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await archivedChat(page);
  await openProof(page);

  for (const expanded of [false, true]) {
    if (expanded)
      await page.getByRole("button", { name: "Expand graph" }).click();
    const candidates = page.locator(".radar-node:not(.root-node)>button");
    const node = expanded ? candidates.last() : candidates.first();
    const name = (await node.getAttribute("title"))!.split(" · ")[0]!;
    const hit = () =>
      node.evaluate((button) => {
        const rect = button.getBoundingClientRect();
        return (
          document
            .elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2)
            ?.closest("button") === button
        );
      });
    await expect.poll(hit).toBe(true);
    const before = (await node.boundingBox())!;
    const x = before.x + before.width / 2;
    const y = before.y + before.height / 2;

    await page.mouse.move(x, y);
    await expect(node.locator("..")).toHaveClass(/\bactive\b/);
    const stable = await node.evaluate(
      async (button, { before, x, y }) => {
        for (let frame = 0; frame < 8; frame++) {
          await new Promise(requestAnimationFrame);
          const {
            x: left,
            y: top,
            width,
            height,
          } = button.getBoundingClientRect();
          if (
            document.elementFromPoint(x, y)?.closest("button") !== button ||
            Math.abs(left - before.x) > 0.5 ||
            Math.abs(top - before.y) > 0.5 ||
            Math.abs(width - before.width) > 0.5 ||
            Math.abs(height - before.height) > 0.5
          )
            return false;
        }
        return true;
      },
      { before, x, y },
    );
    expect(stable).toBe(true);
    await page.mouse.down();
    await page.mouse.move(x + 3, y + 2);
    await page.mouse.up();
    const inspector = page.getByRole("complementary", {
      name: "Selected proof",
    });
    await expect(inspector).toContainText(name);
    await inspector
      .getByRole("button", { name: "Close proof details" })
      .click();
  }
});

async function inspectMeaning(page: Page) {
  await page
    .getByRole("button", { name: "Meaning analysis", exact: true })
    .click();
  await expect(
    page.getByRole("button", {
      name: "Inspect requirement and theorem records",
      exact: true,
    }),
  ).toBeVisible();
  await page
    .getByRole("button", {
      name: "Inspect requirement and theorem records",
      exact: true,
    })
    .click();
}

test("meaning analysis shows an NG comparison by default, can switch to OK and keeps cancellation working", async ({
  page,
}) => {
  await archivedChat(page);
  await openProof(page);
  await page
    .getByRole("button", { name: "Meaning analysis", exact: true })
    .click();
  await expect(page.getByLabel("Analysis progress")).toContainText("Running");
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("Display cancelled");
  await page.getByRole("button", { name: "Run again", exact: true }).click();
  await expect(page.locator(".semantic-result-heading")).toContainText(
    "NG · Difference at the 150-yen boundary",
  );
  await expect(page.getByLabel("Analysis progress")).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: "Meaning comparison", exact: true }),
  ).toBeVisible();
  await expect(page.locator(".semantic-diff .expected-diff")).toHaveText(
    "at least 150 yen",
  );
  await expect(page.locator(".semantic-diff .actual-diff")).toHaveText(
    "strictly greater than 150 yen",
  );
  await expect(
    page
      .locator(".semantic-diff section")
      .filter({ hasText: "stock is at least one" }),
  ).toHaveCount(2);
  await expect(
    page.getByRole("heading", {
      name: "Example theorem translated into natural language",
    }),
  ).toBeVisible();
  await expect(page.locator(".boundary-example")).toContainText(
    "At a balance of 150 yen and stock of one",
  );
  await page
    .getByRole("button", { name: "Show matching example", exact: true })
    .click();
  await expect(page.locator(".semantic-result-heading")).toContainText(
    "The displayed text matches",
  );
  await expect(page.getByLabel("Analysis progress")).toHaveCount(0);
  await expect(
    page.getByText(
      "No analysis engine is connected. This display does not establish meaning alignment.",
      { exact: true },
    ),
  ).toHaveCount(0);
  await expect(
    page.getByText(
      "GUI demonstration of progress and results. No LLM or Lean analysis is performed.",
      { exact: true },
    ),
  ).toHaveCount(0);
  await page
    .getByRole("button", { name: "Show mismatch example", exact: true })
    .click();
  await expect(page.locator(".semantic-result-heading")).toContainText(
    "NG · Difference at the 150-yen boundary",
  );
  await tab(page, "Code");
  await tab(page, "Proof");
  await expect(page.locator(".semantic-result-heading")).toContainText(
    "Difference at the 150-yen boundary",
  );
  await page.reload();
  await openProof(page);
  await expect(page.locator(".proof-obligations")).toContainText("8 / 8");
  await expect(page.locator(".semantic-results")).toHaveCount(0);
});

test("sending opens the real demo preview; purchase, rejection and refund follow its kernel", async ({
  page,
}) => {
  await page.goto("/");
  await page
    .getByRole("button", {
      name: "Build a vending machine simulator",
      exact: true,
    })
    .click();
  await page.getByRole("button", { name: "Send", exact: true }).click();
  await expect(
    page.getByRole("tab", { name: "Preview", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page.getByRole("button", { name: "Buy", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Insufficient balance");
  await page.getByRole("textbox", { name: "Insert amount (yen)" }).fill("200");
  await page.getByRole("button", { name: "Insert", exact: true }).click();
  await page.getByRole("button", { name: "Buy", exact: true }).click();
  await expect(page.locator(".vending-balance")).toContainText("50");
  await expect(page.locator(".stock-count")).toHaveText("Stock 9 items");
  await page.getByRole("button", { name: "Refund", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("50 yen refunded");
  await tab(page, "Code");
  await expect(page.locator(".source-code")).toContainText("pub fn step");
});

test("unsent input survives navigation and reload without creating a named empty conversation", async ({
  page,
}) => {
  await page.goto("/");
  await page
    .getByRole("textbox", { name: "Message", exact: true })
    .fill("An unsent instruction");
  await page.reload();
  await expect(
    page.getByRole("textbox", { name: "Message", exact: true }),
  ).toHaveValue("An unsent instruction");
  await expect(page.locator(".history-item")).toHaveCount(1);
  await page.getByRole("button", { name: "Send", exact: true }).click();
  await expect(page.locator(".conversation-title")).toHaveText(
    "Vending machine simulator",
  );
  await expect(page.locator(".history-item")).toHaveCount(1);
});

test("requirement edits invalidate retained proof and analysis across restart", async ({
  page,
}) => {
  await archivedChat(page);
  await openProof(page);
  await inspectMeaning(page);
  await expect(page.locator(".comparison-heading")).toContainText(
    "Analysis pending",
  );
  await expect(
    page.getByText(
      "Requirements or source revisions changed. Showing a previous analysis.",
    ),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Edit requirement" }).click();
  await page
    .getByRole("textbox", { name: "Requirement", exact: true })
    .fill("Limit insertions to the balance upper bound.");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(
    page.getByRole("dialog", { name: "Edit requirement" }),
  ).toBeHidden();
  await expect(page.locator(".comparison-heading")).toContainText(
    "Reanalysis required",
  );
  await page
    .getByRole("button", { name: "Back to graph", exact: true })
    .click();
  await expect(page.locator(".proof-obligations")).toContainText("0 / 8");
  const cardStatuses = page
    .getByRole("region", { name: "Vending machine simulator artifact" })
    .getByRole("list", { name: "Theorems and proof status" });
  await expect(
    cardStatuses.getByText("Recheck required", { exact: true }),
  ).toHaveCount(8);
  await page.reload();
  await expect(
    cardStatuses.getByText("Recheck required", { exact: true }),
  ).toHaveCount(8);
  await expect(cardStatuses.getByText("Proved", { exact: true })).toHaveCount(
    0,
  );
  await openProof(page);
  await page.locator(".proof-obligations>summary").click();
  await expect(page.locator(".proof-list")).toContainText("Recheck required");
});

test("review export remains revision-bound and omits conversations", async ({
  page,
}) => {
  await archivedChat(page);
  await openProof(page);
  await inspectMeaning(page);
  await page.getByRole("button", { name: "Create review request" }).click();
  await page
    .getByRole("textbox", { name: "Subject" })
    .fill("Insertion conditions review");
  await page
    .getByRole("textbox", { name: "Review questions" })
    .fill("Check the overflow boundary.");
  const promise = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export JSON", exact: true }).click();
  const stream = await (await promise).createReadStream();
  const chunks: Buffer[] = [];
  for await (const chunk of stream!) chunks.push(Buffer.from(chunk));
  const result = JSON.parse(Buffer.concat(chunks).toString());
  expect(result.requirements).toHaveLength(1);
  expect(result.requirements[0].id).toBe("purchase_exact");
  expect(result).not.toHaveProperty("conversations");
});

test("invalid imports retain the conversation; Atlas metadata stays unconfirmed", async ({
  page,
}) => {
  await archivedChat(page);
  await page.getByRole("button", { name: "Settings" }).click();
  const brokenFile = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Import JSON" }).click();
  await (
    await brokenFile
  ).setFiles({
    name: "broken.json",
    mimeType: "application/json",
    buffer: Buffer.from("{"),
  });
  await expect(page.getByRole("alert")).toContainText("Could not parse JSON");
  await expect(page.locator(".conversation-title")).toHaveText(
    "Vending machine simulator",
  );
  const graph = {
    metadata: { projectName: "Atlas" },
    nodes: [
      {
        id: "P.result",
        shortName: "result",
        kind: "theorem",
        filePath: "P.lean",
        hasSorry: false,
        meta: { summary: "An explanation", proofProgress: "complete" },
      },
    ],
    edges: [],
  };
  await page.getByRole("button", { name: "Settings" }).click();
  const graphFile = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Import JSON" }).click();
  await (
    await graphFile
  ).setFiles({
    name: "graph.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(graph)),
  });
  await expect(
    page.getByRole("tab", { name: "Proof", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page.getByRole("button", { name: /result.*: select/ }).click();
  await expect(page.locator(".proof-inspector")).toContainText("Unknown");
  await expect(page.locator(".proof-statement-link")).toContainText(
    "The declaration body is not included in this data",
  );
});

test("desktop and compact layouts keep chat and artifact panes usable, with keyboard entry", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await mkdir("target/gui-checks/screenshots", { recursive: true });
  await page.screenshot({ path: "target/gui-checks/screenshots/home.png" });
  await archivedChat(page);
  await page.locator(".recorded-activities").last().scrollIntoViewIfNeeded();
  await expect(page.locator(".recorded-activities").last()).toBeInViewport();
  await page.screenshot({ path: "target/gui-checks/screenshots/history.png" });

  await openProof(page);
  for (const name of ["Proof", "Preview"]) {
    await tab(page, name);
    if (name === "Proof")
      await expect(page.locator(".root-node")).toBeVisible();
    await page.evaluate(() => document.fonts.ready);
    await page.screenshot({
      path:
        "target/gui-checks/screenshots/" +
        {
          Proof: "proof",
          Preview: "preview",
        }[name] +
        ".png",
    });
  }
  await tab(page, "Proof");
  await page.locator(".proof-obligations>summary").click();
  await page
    .locator(".proof-list>button")
    .filter({ hasText: "A successful purchase subtracts 150 yen and one item" })
    .click();
  await expect(page.locator(".root-node>button")).toBeInViewport({
    ratio: 0.9999,
  });
  await page.screenshot({
    path: "target/gui-checks/screenshots/proof-detail.png",
  });
  await page.locator(".proof-statement-link").click();
  await expect(page.locator(".selected-source-line")).toBeVisible();
  await page.screenshot({ path: "target/gui-checks/screenshots/code.png" });
  await page
    .getByRole("button", { name: "Back to proof", exact: true })
    .click();
  await page
    .getByRole("button", { name: "How meaning analysis works", exact: true })
    .click();
  const method = page.getByRole("dialog", {
    name: "Meaning analysis algorithm",
  });
  await expect(method).toContainText("without seeing the original requirement");
  await expect(method).toContainText("translator LLM");
  await expect(method).toContainText("A separate judge");
  await expect(method).toContainText("A design");
  await expect(
    method.getByText("Theorem back-translation", { exact: true }),
  ).toBeVisible();
  await expect(
    method.getByText("Compare meaning with the request", { exact: true }),
  ).toBeVisible();
  await expect(
    method.getByText("Show differences and their effects", { exact: true }),
  ).toBeVisible();
  await expect(method.getByText(/FormalAlign/)).toHaveCount(0);
  await page.screenshot({
    path: "target/gui-checks/screenshots/semantic-method.png",
  });
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("button", {
      name: "How meaning analysis works",
      exact: true,
    }),
  ).toBeFocused();
  await page
    .getByRole("button", { name: "Meaning analysis", exact: true })
    .click();
  await expect(page.locator(".semantic-result-heading")).toContainText(
    "Difference at the 150-yen boundary",
  );
  await page.screenshot({ path: "target/gui-checks/screenshots/analysis.png" });
  await tab(page, "Preview");
  await page.setViewportSize({ width: 1000, height: 844 });
  const panelBounds = await page
    .getByRole("region", { name: "Implementation and verification" })
    .boundingBox();
  expect(panelBounds).not.toBeNull();
  expect(panelBounds!.x).toBeGreaterThanOrEqual(0);
  expect(panelBounds!.x + panelBounds!.width).toBeLessThanOrEqual(1000);
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(
    page.getByRole("tab", { name: "Preview", exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "target/gui-checks/screenshots/compact-preview.png",
  });
  await page.getByRole("button", { name: "Close panel", exact: true }).click();
  await page.getByRole("button", { name: "Open conversations" }).click();
  await page.getByRole("button", { name: "New message", exact: true }).click();
  await expect(
    page.getByRole("textbox", { name: "Message", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: "target/gui-checks/screenshots/compact-home.png",
  });
  await page.setViewportSize({ width: 1440, height: 960 });
  await page.goto("/");
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("link", { name: "Skip to content" }),
  ).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("main")).toBeFocused();
  expect(errors).toEqual([]);
});

test("failed storage retains a sent conversation and recovers through retry", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const original = IDBObjectStore.prototype.put;
    let fail = true;
    document.addEventListener(
      "click",
      (event) => {
        if (
          (event.target as HTMLElement)
            .closest("button")
            ?.textContent?.includes("Retry save")
        )
          fail = false;
      },
      true,
    );
    IDBObjectStore.prototype.put = function (
      ...args: Parameters<IDBObjectStore["put"]>
    ) {
      if (fail)
        throw new DOMException("Fixture quota failure", "QuotaExceededError");
      return original.apply(this, args);
    };
  });
  await page.goto("/");
  await page
    .getByRole("textbox", { name: "Message", exact: true })
    .fill("Check save failure");
  await page.getByRole("button", { name: "Send", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText(
    "Could not save the conversation",
  );
  await expect(page.locator(".conversation-title")).toHaveText(
    "Vending machine simulator",
  );
  await page.getByRole("button", { name: "Retry save", exact: true }).click();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.reload();
  await expect(page.locator(".user-message").last()).toContainText(
    "Check save failure",
  );
});

test("legacy v2 continuations merge permanently and start with the exact supplied request", async ({
  page,
}) => {
  const original = createVendingWorkspace();
  original.requirements.find((item) => item.id === "purchase_exact")!.title =
    "Buy an item for 150 yen";
  original.conversations[0].messages =
    original.conversations[0].messages.filter(
      (message) => message.origin !== "provided",
    );
  for (const [id, time, text] of [
    ["continuation-a", "01:00:00", "Also check purchase conditions"],
    ["continuation-b", "02:00:00", "Check boundaries too"],
  ]) {
    original.conversations.push({
      id,
      title: text,
      messages: [
        {
          id: id + "-user",
          role: "user",
          text,
          status: "sent",
          createdAt: "2026-11-18T" + time + "Z",
        },
        {
          id: id + "-reply",
          role: "assistant",
          text: "Checked: " + text,
          status: "received",
          createdAt: "2026-11-18T" + time.replace(":00:", ":01:") + "Z",
        },
      ],
    });
  }
  const other = {
    ...createVendingWorkspace(),
    id: "another-project",
    name: "Another project",
    conversations: [
      {
        id: "other-chat",
        title: "Another conversation",
        messages: [
          {
            id: "other-message",
            role: "user" as const,
            text: "Another entry",
            status: "sent" as const,
            createdAt: "2026-11-18T03:00:00Z",
          },
        ],
      },
    ],
  };
  await page.route("**/__legacy_fixture__", (route) =>
    route.fulfill({
      contentType: "text/html",
      body: "<title>Legacy storage fixture</title>",
    }),
  );
  await page.goto("/__legacy_fixture__");
  await page.evaluate(
    async ({ original, other }) => {
      await new Promise<void>((resolve, reject) => {
        const request = indexedDB.open("meden-workspaces", 2);
        request.onupgradeneeded = () =>
          request.result
            .createObjectStore("workspaces", { keyPath: "id" })
            .createIndex("updatedAt", "updatedAt");
        request.onerror = () => reject(request.error);
        request.onsuccess = () => {
          const db = request.result;
          const transaction = db.transaction("workspaces", "readwrite");
          transaction.objectStore("workspaces").put(original);
          transaction.objectStore("workspaces").put(other);
          transaction.onerror = () => reject(transaction.error);
          transaction.oncomplete = () => {
            db.close();
            resolve();
          };
        };
      });
      localStorage.setItem(
        "meden-chat-selection",
        JSON.stringify({
          workspaceId: original.id,
          conversationId: "continuation-b",
        }),
      );
      localStorage.setItem(
        "meden-input:" + original.id + ":continuation-b",
        "Unsent continuation",
      );
    },
    { original, other },
  );
  await page.goto("/");
  await expect(page.locator(".history-item")).toHaveCount(2);
  await expect(page.locator(".conversation-title")).toHaveText(
    "Vending machine simulator",
  );
  await expect(page.locator(".message")).toHaveCount(18);
  await expect(page.locator(".recorded-tool")).toHaveCount(25);
  await expect(
    page.getByRole("textbox", { name: "Message", exact: true }),
  ).toHaveValue("Unsent continuation");
  expect(
    (await page.locator(".user-message").allTextContents()).slice(-2),
  ).toEqual(["Also check purchase conditions", "Check boundaries too"]);
  await expect(
    page.getByRole("button", { name: "Another conversation", exact: true }),
  ).toBeVisible();
  const card = page.getByRole("region", {
    name: "Vending machine simulator artifact",
  });
  await expect(card).toContainText(
    "A successful purchase subtracts 150 yen and one item",
  );
  await expect(card).not.toContainText("Buy an item for 150 yen");
  await page.reload();
  await expect(page.locator(".message")).toHaveCount(18);
  await page
    .getByRole("textbox", { name: "Message", exact: true })
    .fill("Check the next continuation");
  await page.getByRole("button", { name: "Send", exact: true }).click();
  await expect(page.locator(".message")).toHaveCount(20);
  await expect(page.locator(".reply-activity")).toHaveCount(0);
  await page.reload();
  await expect(page.locator(".message")).toHaveCount(20);
  await expect(page.locator(".history-item")).toHaveCount(2);
  await page.getByRole("button", { name: "New message", exact: true }).click();
  await page
    .getByRole("textbox", { name: "Message", exact: true })
    .fill("Continue the same development");
  await page.getByRole("button", { name: "Send", exact: true }).click();
  await expect(page.locator(".user-message").last()).toHaveText(
    "Continue the same development",
  );
  await expect(page.locator(".reply-activity")).toHaveCount(0);
  await expect(page.locator(".history-item")).toHaveCount(2);
  await page.reload();
  await expect(page.locator(".message")).toHaveCount(22);
  await expect(page.locator(".user-message").first()).toHaveText(
    vendingOpeningRequest,
  );
});

test("expanded graph keeps source and statement readers tall, scrollable and synchronized", async ({
  page,
}) => {
  await archivedChat(page);
  await page.locator(".user-message").first().scrollIntoViewIfNeeded();
  await expect(page.locator(".user-message").first()).toBeInViewport();
  await page.screenshot({
    path: "target/gui-checks/screenshots/history-start.png",
  });
  await openProof(page);
  await page.locator(".proof-obligations>summary").click();
  await page
    .locator(".proof-list>button")
    .filter({ hasText: "A successful purchase subtracts 150 yen and one item" })
    .click();
  await page
    .getByRole("button", { name: "Expand reader", exact: true })
    .click();
  const expanded = page.getByRole("dialog", {
    name: "Proof dependencies",
    exact: true,
  });
  await expect(expanded).toBeVisible();
  await expect(
    expanded.getByRole("complementary", { name: "Selected proof" }),
  ).toContainText("KernelProof.law_purchase_exact");
  await page.screenshot({
    path: "target/gui-checks/screenshots/expanded-graph.png",
  });
  await expanded.getByRole("tab", { name: "Lean source", exact: true }).click();
  const source = expanded.getByRole("region", { name: "Source code" });
  const code = source.getByLabel("File contents");
  await expect(source.locator(".selected-source-line")).toContainText(
    "theorem law_purchase_exact",
  );
  await expect(source.locator(".syntax-token").first()).toBeAttached();
  expect(await code.evaluate((el) => el.clientHeight)).toBeGreaterThan(450);
  const chatScroll = await page
    .locator(".transcript")
    .evaluate((el) => el.scrollTop);
  await code.hover();
  await page.mouse.wheel(0, 15000);
  await expect
    .poll(() =>
      code.evaluate(
        (el) => el.scrollTop + el.clientHeight >= el.scrollHeight - 2,
      ),
    )
    .toBe(true);
  await expect(source.locator(".source-line").last()).toBeInViewport();
  expect(await page.locator(".transcript").evaluate((el) => el.scrollTop)).toBe(
    chatScroll,
  );
  await page.screenshot({
    path: "target/gui-checks/screenshots/expanded-source.png",
  });
  await expanded
    .getByRole("button", { name: /Rust transition.*: select/ })
    .click();
  await expect(
    expanded.getByRole("complementary", { name: "Selected proof" }),
  ).toContainText("KernelImpl.kernel.step");
  await expanded.getByRole("tab", { name: "Lean source", exact: true }).click();
  await expect(source.locator(".selected-source-line")).toContainText(
    "def kernel.step",
  );
  await expanded
    .getByRole("button", { name: "Back to conversation", exact: true })
    .click();
  await expect(expanded).toHaveCount(0);
  await expect(page.locator(".user-message").first()).toHaveText(
    vendingOpeningRequest,
  );
});
