import AxeBuilder from "@axe-core/playwright";
import { test, expect } from "@playwright/test";
test("home, conversation and artifact tabs pass automated WCAG AA checks", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
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
  await check();
  await page
    .getByRole("complementary", { name: "会話サイドバー" })
    .getByRole("button", { name: "自販機シミュレーター", exact: true })
    .click();
  await expect(page.locator(".message")).toHaveCount(13);
  await check();
  await page
    .getByRole("button", { name: "証明の状態を表示", exact: true })
    .click();
  for (const name of [
    "証明",
    "依存関係",
    "意味の比較",
    "プレビュー",
    "コード",
  ]) {
    await page.getByRole("tab", { name, exact: true }).click();
    await expect(page.getByRole("tabpanel")).toBeVisible();
    await expect(page.getByRole("tabpanel").locator(".loading")).toHaveCount(0);
    if (name === "依存関係")
      await expect(page.locator(".root-node")).toBeVisible();
    await check();
  }
});
