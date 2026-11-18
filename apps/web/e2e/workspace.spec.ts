import { test, expect, type Page } from "@playwright/test";
import { mkdir } from "node:fs/promises";

async function archivedChat(page: Page) {
  await page.goto("/");
  await page
    .getByRole("complementary", { name: "会話サイドバー" })
    .getByRole("button", { name: "自販機シミュレーター", exact: true })
    .click();
  await expect(page.locator(".message")).toHaveCount(13);
}
async function tab(page: Page, name: string) {
  await page.getByRole("tab", { name, exact: true }).click();
  await expect(page.getByRole("tabpanel")).toBeVisible();
  await expect(page.getByRole("tabpanel").locator(".loading")).toHaveCount(0);
}

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
    "何を作りますか？",
  );
  await expect(
    page.getByRole("textbox", { name: "メッセージ", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "送信", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "下書きを保存", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("textbox", { name: "名前", exact: true }),
  ).toHaveCount(0);
  await page
    .getByRole("textbox", { name: "メッセージ", exact: true })
    .fill("自販機の証明を確認して");
  await page.getByRole("button", { name: "送信", exact: true }).click();
  await expect(page.locator(".conversation-title")).toHaveText(
    "自販機シミュレーター",
  );
  await expect(
    page.getByRole("tab", { name: "証明", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await expect(page.locator(".proof-obligations")).toContainText("8 / 8");
  await expect(page.locator(".assistant-message")).toContainText(
    "保存されている検証記録",
  );
  await expect(page.locator(".sidebar-bottom")).toContainText(
    "このブラウザに保存",
  );
  await page.reload();
  await expect(page.locator(".user-message")).toContainText(
    "自販機の証明を確認して",
  );
  await expect(page.locator(".assistant-message")).toContainText(
    "保存されている検証記録",
  );
  expect(requests).toEqual([]);
});

test("the sidebar contains conversations, preserves actual history and gives separate chats automatic titles", async ({
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
  await expect(page.locator(".recording-banner")).toContainText(
    "reasoning本文は未保存",
  );
  const sidebar = page.getByRole("complementary", { name: "会話サイドバー" });
  await expect(
    sidebar.getByRole("button", { name: "依存関係", exact: true }),
  ).toHaveCount(0);
  await expect(
    sidebar.getByRole("button", { name: "意味の比較", exact: true }),
  ).toHaveCount(0);
  await expect(page.locator(".transcript")).toContainText(
    "開発を続けられますか？",
  );
  await expect(page.locator(".transcript")).toContainText("再帰深度の上限");
  await expect(page.locator(".transcript")).toContainText(
    "形式検証に成功しました",
  );
  await sidebar
    .getByRole("button", { name: "新しい会話", exact: true })
    .click();
  const input = page.getByRole("textbox", { name: "メッセージ", exact: true });
  await input.fill("購入条件を確認して");
  await input.press("Enter");
  await expect(page.locator(".conversation-title")).toHaveText(
    "購入条件を確認して",
  );
  await expect(
    sidebar.getByRole("button", { name: "購入条件を確認して", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "パネルを閉じる", exact: true })
    .last()
    .click();
  await sidebar
    .getByRole("button", { name: "自販機シミュレーター", exact: true })
    .click();
  await expect(page.locator(".message")).toHaveCount(13);
  await sidebar
    .getByRole("button", { name: "会話を検索", exact: true })
    .click();
  await page
    .getByRole("textbox", { name: "会話を検索", exact: true })
    .fill("購入条件を確認して");
  await expect(sidebar.locator(".history-item")).toHaveCount(1);
});

test("proof opens the wider source graph and follows a declaration into its exact Lean source", async ({
  page,
}) => {
  await archivedChat(page);
  await page
    .getByRole("button", { name: "証明の状態を表示", exact: true })
    .click();
  await expect(page.getByRole("tab")).toHaveText([
    "コード",
    "プレビュー",
    "証明",
  ]);
  await expect(page.locator(".root-node")).toContainText("実装と仕様の対応");
  expect(await page.locator(".radar-node").count()).toBeGreaterThan(35);
  await page.getByRole("button", { name: /Rustの状態遷移.*を選択/ }).click();
  await expect(
    page.getByRole("complementary", { name: "選択した証明" }),
  ).toContainText("KernelImpl.kernel.step");
  await page.locator(".proof-statement-link").click();
  await expect(
    page.getByRole("tab", { name: "コード", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await expect(page.locator(".selected-source-line")).toContainText(
    "def kernel.step",
  );
  await expect(page.locator(".source-location")).toContainText(
    "Candidate.lean:34",
  );
  await page.getByRole("button", { name: "証明に戻る", exact: true }).click();
  await expect(
    page.getByRole("complementary", { name: "選択した証明" }),
  ).toContainText("KernelImpl.kernel.step");
  await page.getByRole("button", { name: "中心にする", exact: true }).click();
  await expect(page.locator(".root-node")).toContainText("Rustの状態遷移");
  await page.getByRole("button", { name: "前の中心に戻る" }).click();
  await expect(page.locator(".root-node")).toContainText("実装と仕様の対応");
  await page.getByRole("combobox", { name: "依存関係の範囲" }).click();
  await page
    .getByRole("option", { name: "この宣言の依存", exact: true })
    .click();
  expect(await page.locator(".radar-node").count()).toBeGreaterThan(10);
  await page.locator(".proof-obligations>summary").click();
  await page
    .locator(".proof-list>button")
    .filter({ hasText: "150円で商品を購入する" })
    .click();
  await expect(page.locator(".proof-statement-link")).toContainText(
    "残高と在庫が十分なら",
  );
  await page.locator(".proof-statement-link").click();
  await expect(page.locator(".selected-source-line")).toContainText(
    "theorem law_purchase_exact",
  );
  await expect(page.locator(".source-location")).toContainText("Check.lean:");
});

async function inspectMeaning(page: Page) {
  await page.getByRole("button", { name: "意味解析", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "要件・定理の記録を確認", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "要件・定理の記録を確認", exact: true })
    .click();
}

test("meaning analysis has a cancellable CI flow and an explicit sample with visible semantic differences", async ({
  page,
}) => {
  await archivedChat(page);
  await page
    .getByRole("button", { name: "証明の状態を表示", exact: true })
    .click();
  await page.getByRole("button", { name: "意味解析", exact: true }).click();
  await expect(page.getByLabel("解析の進行状況")).toContainText("実行中");
  await page.getByRole("button", { name: "中止", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("中止しました");
  await page
    .getByRole("button", { name: "差分サンプルを試す", exact: true })
    .click();
  await expect(page.locator(".semantic-result-heading")).toContainText(
    "在庫条件に差分",
  );
  await expect(
    page.locator(".semantic-diff .expected-diff").first(),
  ).toBeVisible();
  await expect(
    page.locator(".semantic-diff .actual-diff").first(),
  ).toBeVisible();
  await expect(page.locator(".boundary-example")).toContainText(
    "残高150円・在庫0個",
  );
  await expect(page.locator(".semantic-results")).toContainText(
    "実装の不具合を検出した結果ではありません",
  );
  await tab(page, "コード");
  await tab(page, "証明");
  await expect(page.locator(".semantic-result-heading")).toContainText(
    "在庫条件に差分",
  );
  await page.reload();
  await page
    .getByRole("button", { name: "証明の状態を表示", exact: true })
    .click();
  await expect(page.locator(".proof-obligations")).toContainText("8 / 8");
  await expect(page.locator(".semantic-results")).toHaveCount(0);
});

test("sending opens the real demo preview; purchase, rejection and refund follow its kernel", async ({
  page,
}) => {
  await page.goto("/");
  await page
    .getByRole("button", { name: "自販機シミュレーターを作る", exact: true })
    .click();
  await page.getByRole("button", { name: "送信", exact: true }).click();
  await expect(
    page.getByRole("tab", { name: "プレビュー", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page.getByRole("button", { name: "購入", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("残高が不足");
  await page.getByRole("textbox", { name: "投入金額（円）" }).fill("200");
  await page.getByRole("button", { name: "投入", exact: true }).click();
  await page.getByRole("button", { name: "購入", exact: true }).click();
  await expect(page.locator(".vending-balance")).toContainText("50");
  await expect(page.locator(".stock-count")).toHaveText("在庫 9個");
  await page.getByRole("button", { name: "返金", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("50円を返金");
  await tab(page, "コード");
  await expect(page.locator(".source-code")).toContainText("pub fn step");
});

test("unsent input survives navigation and reload without creating a named empty conversation", async ({
  page,
}) => {
  await page.goto("/");
  await page
    .getByRole("textbox", { name: "メッセージ", exact: true })
    .fill("まだ送信していない命令");
  await page.reload();
  await expect(
    page.getByRole("textbox", { name: "メッセージ", exact: true }),
  ).toHaveValue("まだ送信していない命令");
  await expect(page.locator(".history-item")).toHaveCount(1);
  await page.getByRole("button", { name: "送信", exact: true }).click();
  await expect(page.locator(".conversation-title")).toHaveText(
    "まだ送信していない命令",
  );
  await expect(page.locator(".history-item")).toHaveCount(2);
});

test("requirement edits invalidate retained proof and analysis across restart", async ({
  page,
}) => {
  await archivedChat(page);
  await page
    .getByRole("button", { name: "証明の状態を表示", exact: true })
    .click();
  await inspectMeaning(page);
  await expect(page.locator(".comparison-heading")).toContainText("解析待ち");
  await expect(
    page.getByText(
      "要件または対象版が変更されています。以前の解析結果を表示しています。",
    ),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "要件を編集" }).click();
  await page
    .getByRole("textbox", { name: "要件", exact: true })
    .fill("投入額は残高の上限以下に制限する。");
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await expect(page.locator(".comparison-heading")).toContainText(
    "再解析が必要",
  );
  await page.getByRole("button", { name: "グラフに戻る", exact: true }).click();
  await expect(page.locator(".proof-obligations")).toContainText("0 / 8");
  await expect(page.locator(".sidebar-bottom")).toContainText(
    "このブラウザに保存",
  );
  await page.reload();
  await page
    .getByRole("button", { name: "証明の状態を表示", exact: true })
    .click();
  await page.locator(".proof-obligations>summary").click();
  await expect(page.locator(".proof-list")).toContainText("再検証が必要");
});

test("review export remains revision-bound and omits conversations", async ({
  page,
}) => {
  await archivedChat(page);
  await page
    .getByRole("button", { name: "証明の状態を表示", exact: true })
    .click();
  await inspectMeaning(page);
  await page.getByRole("button", { name: "レビュー依頼を作成" }).click();
  await page.getByRole("textbox", { name: "件名" }).fill("投入条件のレビュー");
  await page
    .getByRole("textbox", { name: "確認してほしい点" })
    .fill("オーバーフローの境界を確認してください。");
  const promise = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "JSONを書き出す", exact: true })
    .click();
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
  await page.locator("input[type=file]").setInputFiles({
    name: "broken.json",
    mimeType: "application/json",
    buffer: Buffer.from("{"),
  });
  await expect(page.getByRole("alert")).toContainText("JSONを読み込めません");
  await expect(page.locator(".conversation-title")).toHaveText(
    "自販機シミュレーター",
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
  await page.locator("input[type=file]").setInputFiles({
    name: "graph.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(graph)),
  });
  await expect(
    page.getByRole("tab", { name: "証明", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page.getByRole("button", { name: /result.*を選択/ }).click();
  await expect(page.locator(".proof-detail")).toContainText("未確認");
  await expect(page.locator(".proof-statement-link")).toContainText(
    "宣言の本文はデータに含まれていません",
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
  await page.getByRole("button", { name: "実行ログ", exact: true }).click();
  await expect(page.locator(".recorded-activities").last()).toBeInViewport();
  await page.screenshot({ path: "target/gui-checks/screenshots/history.png" });

  await page
    .getByRole("button", { name: "証明の状態を表示", exact: true })
    .click();
  for (const name of ["証明", "プレビュー"]) {
    await tab(page, name);
    if (name === "証明") await expect(page.locator(".root-node")).toBeVisible();
    await page.evaluate(() => document.fonts.ready);
    await page.screenshot({
      path:
        "target/gui-checks/screenshots/" +
        {
          証明: "proof",
          プレビュー: "preview",
        }[name] +
        ".png",
    });
  }
  await tab(page, "証明");
  await page.locator(".proof-obligations>summary").click();
  await page
    .locator(".proof-list>button")
    .filter({ hasText: "150円で商品を購入する" })
    .click();
  await expect(page.locator(".root-node>button")).toBeInViewport({ ratio: 1 });
  await page.screenshot({
    path: "target/gui-checks/screenshots/proof-detail.png",
  });
  await page.locator(".proof-statement-link").click();
  await expect(page.locator(".selected-source-line")).toBeVisible();
  await page.screenshot({ path: "target/gui-checks/screenshots/code.png" });
  await page.getByRole("button", { name: "証明に戻る", exact: true }).click();
  await page.getByRole("button", { name: "意味解析", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "差分サンプルを試す", exact: true }),
  ).toBeEnabled();
  await page
    .getByRole("button", { name: "差分サンプルを試す", exact: true })
    .click();
  await expect(page.locator(".semantic-result-heading")).toContainText(
    "在庫条件に差分",
  );
  await page.screenshot({ path: "target/gui-checks/screenshots/analysis.png" });
  await tab(page, "プレビュー");
  await page.setViewportSize({ width: 1000, height: 844 });
  const panelBounds = await page
    .getByRole("region", { name: "実装と検証" })
    .boundingBox();
  expect(panelBounds).not.toBeNull();
  expect(panelBounds!.x).toBeGreaterThanOrEqual(0);
  expect(panelBounds!.x + panelBounds!.width).toBeLessThanOrEqual(1000);
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(
    page.getByRole("tab", { name: "プレビュー", exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "target/gui-checks/screenshots/compact-preview.png",
  });
  await page
    .getByRole("button", { name: "パネルを閉じる", exact: true })
    .click();
  await page.getByRole("button", { name: "会話一覧を開く" }).click();
  await page.getByRole("button", { name: "新しい会話", exact: true }).click();
  await expect(
    page.getByRole("textbox", { name: "メッセージ", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: "target/gui-checks/screenshots/compact-home.png",
  });
  await page.setViewportSize({ width: 1440, height: 960 });
  await page.goto("/");
  await page.keyboard.press("Tab");
  await expect(page.getByRole("link", { name: "本文へ移動" })).toBeFocused();
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
            ?.textContent?.includes("保存を再試行")
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
    .getByRole("textbox", { name: "メッセージ", exact: true })
    .fill("保存失敗の確認");
  await page.getByRole("button", { name: "送信", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText(
    "会話を保存できませんでした",
  );
  await expect(page.locator(".conversation-title")).toHaveText(
    "保存失敗の確認",
  );
  await page.getByRole("button", { name: "保存を再試行", exact: true }).click();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(page.locator(".sidebar-bottom")).toContainText(
    "このブラウザに保存",
  );
  await page.reload();
  await expect(page.locator(".user-message")).toContainText("保存失敗の確認");
});
