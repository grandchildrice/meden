import { test, expect, type Page } from "@playwright/test";
import { mkdir } from "node:fs/promises";

async function openSample(page: Page) {
  await page.goto("/");
  await page
    .getByRole("button", { name: "サンプルを開く", exact: true })
    .click();
  await expect(page.getByRole("note")).toContainText("サンプルデータ");
}
async function navigate(page: Page, name: string) {
  if (
    await page.getByRole("button", { name: "ナビゲーションを開く" }).isVisible()
  )
    await page.getByRole("button", { name: "ナビゲーションを開く" }).click();
  await page
    .getByRole("navigation", { name: "メイン" })
    .getByRole("button", { name, exact: true })
    .click();
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  if (name === "依存関係")
    await expect(page.locator(".root-node")).toBeVisible();
}

test("a new workspace saves a draft, restores it after restart and does not invent an agent reply", async ({
  page,
}) => {
  const requests: string[] = [];
  page.on("request", (request) => {
    if (
      !request.url().startsWith("http://127.0.0.1:4320") &&
      !request.url().startsWith("data:")
    )
      requests.push(request.url());
  });
  await page.goto("/");
  await page.getByRole("button", { name: "新規作成", exact: true }).click();
  await page.getByLabel("名前", { exact: true }).fill("在庫管理");
  await page.getByRole("button", { name: "作成", exact: true }).click();
  await page
    .getByRole("textbox", { name: "メッセージ", exact: true })
    .fill("在庫が不足する出荷を拒否してください。");
  await navigate(page, "意味の比較");
  await navigate(page, "会話");
  await expect(
    page.getByRole("textbox", { name: "メッセージ", exact: true }),
  ).toHaveValue("在庫が不足する出荷を拒否してください。");
  await page.reload();
  await expect(
    page.getByRole("textbox", { name: "メッセージ", exact: true }),
  ).toHaveValue("在庫が不足する出荷を拒否してください。");
  await page.getByRole("button", { name: "下書きを保存", exact: true }).click();
  await expect(page.locator(".user-message")).toContainText("在庫が不足する");
  await expect(page.locator(".user-message")).toContainText("未送信");
  await expect(page.locator(".assistant-message")).toHaveCount(0);
  await expect(page.locator(".sidebar-bottom")).toContainText("ブラウザに保存");
  await page.reload();
  await expect(page.locator(".user-message")).toContainText("在庫が不足する");
  await expect(
    page.getByRole("button", { name: "未接続", exact: true }),
  ).toBeVisible();
  expect(requests).toEqual([]);
});

test("dependency selection, recentering and scope preserve the selected declaration", async ({
  page,
}) => {
  await openSample(page);
  await navigate(page, "依存関係");
  await page
    .getByRole("button", { name: /送金の受理条件.*を選択/, exact: true })
    .click();
  await expect(
    page.getByRole("complementary", { name: "宣言の詳細" }),
  ).toContainText("amount < s.debit");
  await expect(
    page
      .getByRole("complementary", { name: "宣言の詳細" })
      .getByText("証明済み"),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "中心にする", exact: true }).click();
  await expect(page.locator(".root-node")).toContainText("送金の受理条件");
  await page.getByRole("button", { name: "前の中心に戻る" }).click();
  await expect(page.locator(".root-node")).toContainText("送金の受理");
  await page.getByRole("combobox", { name: "依存関係の範囲" }).click();
  await page.getByRole("option", { name: "意味に関わる依存" }).click();
  await expect(
    page.getByRole("button", { name: /加減算の補題.*を選択/, exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: /送金の受理条件.*を選択/, exact: true }),
  ).toBeVisible();
});

test("editing meaning invalidates proof and analysis, including after restart", async ({
  page,
}) => {
  await openSample(page);
  await navigate(page, "意味の比較");
  await expect(page.locator(".expected-diff")).toContainText("以下");
  await expect(page.locator(".actual-diff")).toContainText("未満");
  await page.getByRole("button", { name: "要件を編集" }).click();
  await page
    .getByRole("textbox", { name: "要件", exact: true })
    .fill("送金額が残高以下なら、同額の送金も受け付ける。");
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.locator(".comparison-heading")).toContainText(
    "再解析が必要",
  );
  await expect(page.locator(".comparison-card").last()).toContainText(
    "再検証が必要",
  );
  await navigate(page, "会話");
  await expect(page.locator(".requirements-panel")).toContainText(
    "再検証が必要",
  );
  await expect(page.locator(".sidebar-bottom")).toContainText("ブラウザに保存");
  await page.reload();
  await navigate(page, "意味の比較");
  await expect(page.locator(".comparison-heading")).toContainText(
    "再解析が必要",
  );
});

test("review drafts export only selected requirements and linked evidence, without sending", async ({
  page,
}) => {
  await openSample(page);
  await navigate(page, "意味の比較");
  await page.getByRole("button", { name: "レビュー依頼を作成" }).click();
  await page.getByRole("textbox", { name: "件名" }).fill("送金条件のレビュー");
  await page
    .getByRole("textbox", { name: "確認してほしい点" })
    .fill("同額のケースを確認してください。");
  await page.getByRole("button", { name: "下書きを保存", exact: true }).click();
  await expect(page.locator(".review-list")).toContainText(
    "送金条件のレビュー",
  );
  const downloadPromise = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "JSONを書き出す", exact: true })
    .click();
  const download = await downloadPromise;
  const stream = await download.createReadStream();
  const chunks: Buffer[] = [];
  for await (const chunk of stream!) chunks.push(Buffer.from(chunk));
  const data = JSON.parse(Buffer.concat(chunks).toString());
  expect(data.requirements).toHaveLength(1);
  expect(data.requirements[0].id).toBe("REQ-001");
  expect(data.sourceRevision).toBe("sample-ledger-v1");
  expect(data.encryption).toBe("none");
  expect(data).not.toHaveProperty("conversations");
  await expect(page.locator(".sidebar-bottom")).toContainText("ブラウザに保存");
  await page.reload();
  await navigate(page, "レビュー依頼");
  await expect(
    page.getByRole("textbox", { name: "確認してほしい点" }),
  ).toHaveValue("同額のケースを確認してください。");
});

test("invalid imports explain the failure and retain the current workspace", async ({
  page,
}) => {
  await openSample(page);
  await page.locator("input[type=file]").setInputFiles({
    name: "invalid.json",
    mimeType: "application/json",
    buffer: Buffer.from("{"),
  });
  await expect(page.getByRole("alert")).toContainText("JSONを読み込めません");
  await expect(page.locator(".workspace-picker")).toContainText("送金処理");
});

test("Lean Atlas import does not promote metadata to a proof receipt", async ({
  page,
}) => {
  await page.goto("/");
  const graph = {
    metadata: { projectName: "Imported Atlas" },
    nodes: [
      {
        id: "Project.result",
        shortName: "result",
        kind: "theorem",
        filePath: "Project.lean",
        lineStart: 5,
        hasSorry: false,
        meta: { summary: "True", proofProgress: "complete", axioms: [] },
      },
    ],
    edges: [],
  };
  await page.locator("input[type=file]").setInputFiles({
    name: "graph.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(graph)),
  });
  await navigate(page, "依存関係");
  await expect(page.locator(".declaration-inspector")).toContainText("未確認");
  await expect(page.getByRole("note")).toContainText("再検証していません");
});

test("keyboard navigation, compact layouts and all four views render without errors", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await openSample(page);
  await mkdir("target/gui-checks/screenshots", { recursive: true });
  for (const [name, key] of [
    ["会話", "chat"],
    ["依存関係", "graph"],
    ["意味の比較", "analysis"],
    ["レビュー依頼", "review"],
  ]) {
    await navigate(page, name);
    await page.evaluate(() => document.fonts.ready);
    await page.waitForTimeout(250);
    await page.screenshot({
      path: "target/gui-checks/screenshots/desktop-" + key + ".png",
    });
  }
  await page.setViewportSize({ width: 390, height: 844 });
  for (const name of ["会話", "依存関係", "意味の比較", "レビュー依頼"]) {
    await navigate(page, name);
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    ).toBe(true);
    await expect(page.getByRole("main")).toBeVisible();
  }
  await page.screenshot({
    path: "target/gui-checks/screenshots/compact-review.png",
  });
  await page.setViewportSize({ width: 1440, height: 960 });
  await page.goto("/");
  await page.keyboard.press("Tab");
  await expect(page.getByRole("link", { name: "本文へ移動" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("main")).toBeFocused();
  expect(errors).toEqual([]);
});

test("a storage failure retains the workspace and recovers through the visible retry action", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const original = IDBObjectStore.prototype.put;
    let fail = true;
    IDBObjectStore.prototype.put = function (
      ...args: Parameters<IDBObjectStore["put"]>
    ) {
      if (fail) {
        fail = false;
        throw new DOMException("Synthetic quota failure", "QuotaExceededError");
      }
      return original.apply(this, args);
    };
  });
  await page.goto("/");
  await page.getByRole("button", { name: "新規作成", exact: true }).click();
  await page.getByLabel("名前", { exact: true }).fill("未保存の変更");
  await page.getByRole("button", { name: "作成", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText(
    "変更を保存できませんでした",
  );
  await expect(page.locator(".workspace-picker")).toContainText("未保存の変更");
  await page.getByRole("button", { name: "保存を再試行", exact: true }).click();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(page.locator(".sidebar-bottom")).toContainText("ブラウザに保存");
  await page.reload();
  await expect(page.locator(".workspace-picker")).toContainText("未保存の変更");
});
