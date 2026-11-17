import AxeBuilder from "@axe-core/playwright";
import { test, expect } from "@playwright/test";

test("workspace views and dialogs meet automated WCAG AA checks", async ({
  page,
}) => {
  await page.goto("/");
  await page
    .getByRole("button", { name: "サンプルを開く", exact: true })
    .click();
  for (const route of ["会話", "依存関係", "意味の比較", "レビュー依頼"]) {
    await page
      .getByRole("navigation", { name: "メイン" })
      .getByRole("button", { name: route, exact: true })
      .click();
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
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
  }
  await page.getByRole("button", { name: "未接続", exact: true }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  expect(
    (
      await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
        .analyze()
    ).violations,
  ).toEqual([]);
  await page.getByRole("button", { name: "閉じる", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "未接続", exact: true }),
  ).toBeFocused();
});
