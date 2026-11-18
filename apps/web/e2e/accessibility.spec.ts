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
    .getByRole("complementary", { name: "会話サイドバー" })
    .getByRole("button", { name: "自販機シミュレーター", exact: true })
    .click();
  await expect(page.locator(".message")).toHaveCount(14);
  await check();
  await page.getByRole("button", { name: "成果物を開く", exact: true }).click();
  for (const name of ["証明", "コード", "プレビュー"]) {
    await page.getByRole("tab", { name, exact: true }).click();
    await expect(page.getByRole("tabpanel")).toBeVisible();
    await expect(page.getByRole("tabpanel").locator(".loading")).toHaveCount(0);
    if (name === "証明") await expect(page.locator(".root-node")).toBeVisible();
    await check();
  }
  await page.getByRole("tab", { name: "証明", exact: true }).click();
  await page.getByRole("button", { name: /Rustの状態遷移.*を選択/ }).click();
  await check();
  await page.getByRole("button", { name: "広げて読む", exact: true }).click();
  const expanded = page.getByRole("dialog", {
    name: "証明の依存関係",
    exact: true,
  });
  await expect(expanded).toBeVisible();
  await check();
  await expanded.getByRole("tab", { name: "Leanソース", exact: true }).click();
  await expect(expanded.locator(".syntax-token").first()).toBeAttached();
  await check();
  await expanded
    .getByRole("button", { name: "会話に戻る", exact: true })
    .click();
  await expect(expanded).toHaveCount(0);
  await page.locator(".proof-statement-link").click();
  await expect(page.locator(".selected-source-line")).toBeVisible();
  await check();
  await page.getByRole("button", { name: "証明に戻る", exact: true }).click();
  await page
    .getByRole("button", { name: "意味解析の仕組み", exact: true })
    .click();
  await expect(
    page.getByRole("dialog", { name: "意味解析のアルゴリズム" }),
  ).toBeVisible();
  await check();
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "意味解析", exact: true }).click();
  await expect(page.locator(".semantic-result-heading")).toContainText(
    "残高150円の境界に差分",
  );
  await check();
  await page.getByRole("button", { name: "要件・定理の記録を確認" }).click();
  await expect(
    page.getByRole("combobox", { name: "比較する要件" }),
  ).toBeVisible();
  await check();
});
