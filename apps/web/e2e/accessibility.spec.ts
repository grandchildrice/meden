import AxeBuilder from "@axe-core/playwright";
import { test, expect } from "@playwright/test";
test("home, three sidebar tabs, source navigation and semantic differences pass automated WCAG AA", async ({
  page,
}) => {
  const check = async () => {
    const result = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
      .analyze();
    expect(
      result.violations,
      JSON.stringify(
        result.violations.map((item) => ({
          id: item.id,
          nodes: item.nodes.map((node) => node.target),
        })),
      ),
    ).toEqual([]);
  };
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await check();
  await page
    .getByRole("complementary", { name: "Conversation sidebar" })
    .getByRole("button", { name: "Vending machine simulator", exact: true })
    .click();
  await expect(page.locator(".message")).toHaveCount(14);
  await check();
  await page
    .getByRole("button", { name: "Open artifact", exact: true })
    .click();
  for (const name of ["Proof", "Code", "Preview"]) {
    await page.getByRole("tab", { name, exact: true }).click();
    await expect(page.getByRole("tabpanel")).toBeVisible();
    await expect(page.getByRole("tabpanel").locator(".loading")).toHaveCount(0);
    if (name === "Proof")
      await expect(page.locator(".root-node")).toBeVisible();
    await check();
  }
  await page.getByRole("tab", { name: "Proof", exact: true }).click();
  await page.getByRole("button", { name: /Rust transition.*: select/ }).click();
  await check();
  await page
    .getByRole("button", { name: "Expand reader", exact: true })
    .click();
  const expanded = page.getByRole("dialog", {
    name: "Proof dependencies",
    exact: true,
  });
  await expect(expanded).toBeVisible();
  await check();
  await expanded.getByRole("tab", { name: "Lean source", exact: true }).click();
  await expect(expanded.locator(".syntax-token").first()).toBeAttached();
  await check();
  await expanded
    .getByRole("button", { name: "Back to conversation", exact: true })
    .click();
  await expect(expanded).toHaveCount(0);
  await page.locator(".proof-statement-link").click();
  await expect(page.locator(".selected-source-line")).toBeVisible();
  await check();
  await page
    .getByRole("button", { name: "Back to proof", exact: true })
    .click();
  await page
    .getByRole("button", { name: "How meaning analysis works", exact: true })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Meaning analysis algorithm" }),
  ).toBeVisible();
  await check();
  await page.keyboard.press("Escape");
  await page
    .getByRole("button", { name: "Meaning analysis", exact: true })
    .click();
  await expect(page.locator(".semantic-result-heading")).toContainText(
    "Difference at the 150-yen boundary",
  );
  await check();
  await page
    .getByRole("button", { name: "Inspect requirement and theorem records" })
    .click();
  await expect(
    page.getByRole("combobox", { name: "Requirement to compare" }),
  ).toBeVisible();
  await check();
});
