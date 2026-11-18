import { test, expect, type Page } from "@playwright/test";
import {
  createVendingWorkspace,
  vendingOpeningRequest,
} from "../src/fixtures/meden-demo";
import { mkdir } from "node:fs/promises";

async function archivedChat(page: Page) {
  await page.goto("/");
  await page
    .getByRole("complementary", { name: "会話サイドバー" })
    .getByRole("button", { name: "自販機シミュレーター", exact: true })
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
  await page.getByRole("button", { name: "成果物を開く", exact: true }).click();
  await tab(page, "証明");
}

test("the conversation sidebar collapses independently of the artifact panel", async ({
  page,
}) => {
  await archivedChat(page);
  const sidebar = page.getByRole("complementary", { name: "会話サイドバー" });
  const topbar = page.getByRole("banner");
  const artifacts = page.getByRole("region", { name: "実装と検証" });
  const input = page.getByRole("textbox", { name: "メッセージ", exact: true });
  await input.fill("作業中の下書き");
  await expect(topbar.getByRole("button")).toHaveCount(2);
  await expect(topbar.getByText("デモ", { exact: true })).toHaveCount(0);
  await topbar.getByRole("button", { name: "成果物を開く" }).click();
  await expect(artifacts).toBeVisible();
  await topbar.getByRole("button", { name: "会話一覧を閉じる" }).click();
  await expect(sidebar).toBeHidden();
  await expect(artifacts).toBeVisible();
  await topbar.getByRole("button", { name: "成果物を閉じる" }).click();
  await expect(artifacts).toBeHidden();
  await expect(sidebar).toBeHidden();
  await expect(input).toHaveValue("作業中の下書き");
  await topbar.getByRole("button", { name: "会話一覧を開く" }).press("Enter");
  await expect(sidebar).toBeVisible();
  await expect(
    sidebar.getByRole("button", { name: "自販機シミュレーター" }),
  ).toHaveAttribute("aria-current", "page");
  await sidebar.getByRole("button", { name: "設定" }).click();
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "ワークスペースを書き出す" }).click();
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
    "何を安全に作りますか？",
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
  await expect(page.locator(".assistant-message").last()).toContainText(
    "保存されている検証記録",
  );
  await page.reload();
  await expect(page.locator(".user-message").last()).toContainText(
    "自販機の証明を確認して",
  );
  await expect(page.locator(".assistant-message").last()).toContainText(
    "保存されている検証記録",
  );
  expect(requests).toEqual([]);
});

test("home displays the pitch logos below the suggestions at desktop and phone widths", async ({
  page,
}) => {
  await page.goto("/");
  const logos = page.getByRole("list", { name: "言語・形式検証ツールの例" });
  const suggestion = page.getByRole("button", {
    name: "自販機シミュレーターを作る",
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
    page.getByText("メッセージから会話を始めます", { exact: true }),
  ).toHaveCount(0);
  await expect(logos.getByRole("img")).toHaveCount(names.length);
  for (const width of [1440, 390]) {
    await page.setViewportSize({ width, height: 844 });
    const suggestionBox = (await suggestion.boundingBox())!;
    const logoBox = (await logos.boundingBox())!;
    expect(logoBox.y).toBeGreaterThan(suggestionBox.y + suggestionBox.height);
    for (const name of names) {
      const logo = logos.getByRole("img", { name, exact: true });
      await expect(logo).toBeInViewport({ ratio: 1 });
      await expect(logo).toHaveJSProperty("complete", true);
      expect(
        await logo.evaluate((image: HTMLImageElement) => image.naturalWidth),
      ).toBeGreaterThan(0);
    }
  }
  await suggestion.click();
  await page.getByRole("button", { name: "送信", exact: true }).click();
  await page
    .getByRole("button", { name: "パネルを閉じる", exact: true })
    .click();
  await expect(logos).toHaveCount(0);
  await expect(
    page.getByText("Enterで送信・Shift + Enterで改行"),
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
    page.getByText("保存された開発履歴", { exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByText("このブラウザに保存", { exact: true }),
  ).toHaveCount(0);
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
    .getByRole("button", { name: "メッセージを書く", exact: true })
    .click();
  const input = page.getByRole("textbox", { name: "メッセージ", exact: true });
  await input.fill("購入条件を確認して");
  await input.press("Enter");
  await expect(page.locator(".conversation-title")).toHaveText(
    "自販機シミュレーター",
  );
  await expect(sidebar.locator(".history-item")).toHaveCount(1);
  await expect(page.locator(".user-message").first()).toHaveText(
    vendingOpeningRequest,
  );
  await expect(page.locator(".reply-activity")).toHaveCount(0);
  await page
    .getByRole("button", { name: "パネルを閉じる", exact: true })
    .last()
    .click();
  await sidebar
    .getByRole("button", { name: "自販機シミュレーター", exact: true })
    .click();
  await expect(page.locator(".message")).toHaveCount(16);
  await sidebar
    .getByRole("button", { name: "会話を検索", exact: true })
    .click();
  await page
    .getByRole("textbox", { name: "会話を検索", exact: true })
    .fill("購入条件を確認して");
  await expect(sidebar.locator(".history-item")).toHaveCount(1);
});

test("the full conversation artifact opens preview and keeps proof details readable", async ({
  page,
}) => {
  await archivedChat(page);
  const card = page.getByRole("region", {
    name: "自販機シミュレーターの成果物",
  });
  const statuses = card.getByRole("list", { name: "定理と証明ステータス" });
  const preview = card.getByRole("button", {
    name: "自販機シミュレーターのプレビューを開く",
  });
  await expect(
    card.getByText("コード・プレビュー・証明", { exact: true }),
  ).toHaveCount(0);
  await expect(statuses.getByRole("listitem")).toHaveCount(8);
  await expect(statuses.getByText("証明済み", { exact: true })).toHaveCount(8);
  const purchase = statuses
    .getByRole("listitem")
    .filter({ hasText: "KernelProof.law_purchase_exact" });
  await expect(purchase).toContainText(
    "購入が成立すると、残高が150円、在庫が1個減る",
  );
  await expect(purchase).toContainText("証明済み");
  await expect(statuses).toContainText(
    "返金操作以外では、直近の返金額は変わらない",
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
    page.getByRole("tab", { name: "プレビュー", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page
    .getByRole("button", { name: "パネルを閉じる", exact: true })
    .click();
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
      .getByRole("button", { name: "成果物を開く", exact: true })
      .hover();
    await expect(
      page.getByRole("tooltip", { name: "成果物を開く", exact: true }),
    ).toBeVisible();
    const bounds = (await target.boundingBox())!;
    await page.mouse.click(
      bounds.x + bounds.width / 2,
      bounds.y + bounds.height / 2,
    );
    await expect(
      page.getByRole("tab", { name: "プレビュー", exact: true }),
    ).toHaveAttribute("aria-selected", "true");
    await page
      .getByRole("button", { name: "パネルを閉じる", exact: true })
      .click();
  }
});

test("proof opens the wider source graph and follows a declaration into its exact Lean source", async ({
  page,
}) => {
  await archivedChat(page);
  await openProof(page);
  await expect(page.getByRole("tab")).toHaveText([
    "コード",
    "プレビュー",
    "証明",
  ]);
  await expect(page.locator(".root-node")).toContainText("実装と仕様の対応");
  expect(await page.locator(".radar-node").count()).toBeGreaterThan(10);
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
    .filter({ hasText: "購入が成立すると、残高が150円、在庫が1個減る" })
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

test("hover keeps dense graph nodes in place until the pointer selects them", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1440, height: 844 });
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await archivedChat(page);
  await openProof(page);

  for (const expanded of [false, true]) {
    if (expanded)
      await page.getByRole("button", { name: "グラフを広げる" }).click();
    await page.getByRole("combobox", { name: "依存関係の範囲" }).click();
    await page.getByRole("option", { name: "プロジェクト全体" }).click();
    const name = expanded
      ? "KernelContract.State"
      : "KernelContract.transition";
    const node = page.getByRole("button", {
      name: new RegExp(name + ".*を選択"),
    });
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
    const inspector = page.getByRole("complementary", { name: "選択した証明" });
    await expect(inspector).toContainText(name);
    await inspector.getByRole("button", { name: "証明の詳細を閉じる" }).click();
  }
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

test("meaning analysis shows an NG comparison by default, can switch to OK and keeps cancellation working", async ({
  page,
}) => {
  await archivedChat(page);
  await openProof(page);
  await page.getByRole("button", { name: "意味解析", exact: true }).click();
  await expect(page.getByLabel("解析の進行状況")).toContainText("実行中");
  await page.getByRole("button", { name: "中止", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("中止しました");
  await page.getByRole("button", { name: "もう一度実行", exact: true }).click();
  await expect(page.locator(".semantic-result-heading")).toContainText(
    "NG · 残高150円の境界に差分",
  );
  await expect(page.getByLabel("解析の進行状況")).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: "意味の比較", exact: true }),
  ).toBeVisible();
  await expect(page.locator(".semantic-diff .expected-diff")).toHaveText(
    "150円以上",
  );
  await expect(page.locator(".semantic-diff .actual-diff")).toHaveText(
    "150円を超え",
  );
  await expect(
    page.locator(".semantic-diff section").filter({ hasText: "在庫が1個以上" }),
  ).toHaveCount(2);
  await expect(
    page.getByRole("heading", { name: "仮の定理を自然文にした例" }),
  ).toBeVisible();
  await expect(page.locator(".boundary-example")).toContainText(
    "残高150円・在庫1個",
  );
  await page.getByRole("button", { name: "一致例を表示", exact: true }).click();
  await expect(page.locator(".semantic-result-heading")).toContainText(
    "差分はありません",
  );
  await expect(page.getByLabel("解析の進行状況")).toHaveCount(0);
  await expect(
    page.getByText(
      "解析エンジンは未接続です。この表示は意味の一致を保証しません。",
      { exact: true },
    ),
  ).toHaveCount(0);
  await expect(
    page.getByText(
      "処理の進行と結果表示を試すGUIデモです。LLMやLeanによる解析は実行していません。",
      { exact: true },
    ),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "NG例を表示", exact: true }).click();
  await expect(page.locator(".semantic-result-heading")).toContainText(
    "NG · 残高150円の境界に差分",
  );
  await tab(page, "コード");
  await tab(page, "証明");
  await expect(page.locator(".semantic-result-heading")).toContainText(
    "残高150円の境界に差分",
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
    "自販機シミュレーター",
  );
  await expect(page.locator(".history-item")).toHaveCount(1);
});

test("requirement edits invalidate retained proof and analysis across restart", async ({
  page,
}) => {
  await archivedChat(page);
  await openProof(page);
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
  await expect(page.getByRole("dialog", { name: "要件を編集" })).toBeHidden();
  await expect(page.locator(".comparison-heading")).toContainText(
    "再解析が必要",
  );
  await page.getByRole("button", { name: "グラフに戻る", exact: true }).click();
  await expect(page.locator(".proof-obligations")).toContainText("0 / 8");
  const cardStatuses = page
    .getByRole("region", { name: "自販機シミュレーターの成果物" })
    .getByRole("list", { name: "定理と証明ステータス" });
  await expect(
    cardStatuses.getByText("再検証が必要", { exact: true }),
  ).toHaveCount(8);
  await page.reload();
  await expect(
    cardStatuses.getByText("再検証が必要", { exact: true }),
  ).toHaveCount(8);
  await expect(cardStatuses.getByText("証明済み", { exact: true })).toHaveCount(
    0,
  );
  await openProof(page);
  await page.locator(".proof-obligations>summary").click();
  await expect(page.locator(".proof-list")).toContainText("再検証が必要");
});

test("review export remains revision-bound and omits conversations", async ({
  page,
}) => {
  await archivedChat(page);
  await openProof(page);
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
  await page.getByRole("button", { name: "設定" }).click();
  const brokenFile = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "JSONを読み込む" }).click();
  await (
    await brokenFile
  ).setFiles({
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
  await page.getByRole("button", { name: "設定" }).click();
  const graphFile = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "JSONを読み込む" }).click();
  await (
    await graphFile
  ).setFiles({
    name: "graph.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(graph)),
  });
  await expect(
    page.getByRole("tab", { name: "証明", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page.getByRole("button", { name: /result.*を選択/ }).click();
  await expect(page.locator(".proof-inspector")).toContainText("未確認");
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
  await page.locator(".recorded-activities").last().scrollIntoViewIfNeeded();
  await expect(page.locator(".recorded-activities").last()).toBeInViewport();
  await page.screenshot({ path: "target/gui-checks/screenshots/history.png" });

  await openProof(page);
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
    .filter({ hasText: "購入が成立すると、残高が150円、在庫が1個減る" })
    .click();
  await expect(page.locator(".root-node>button")).toBeInViewport({ ratio: 1 });
  await page.screenshot({
    path: "target/gui-checks/screenshots/proof-detail.png",
  });
  await page.locator(".proof-statement-link").click();
  await expect(page.locator(".selected-source-line")).toBeVisible();
  await page.screenshot({ path: "target/gui-checks/screenshots/code.png" });
  await page.getByRole("button", { name: "証明に戻る", exact: true }).click();
  await page
    .getByRole("button", { name: "意味解析の仕組み", exact: true })
    .click();
  const method = page.getByRole("dialog", { name: "意味解析のアルゴリズム" });
  await expect(method).toContainText("元の依頼文を見せず");
  await expect(method).toContainText("翻訳用の大規模言語モデル（LLM）");
  await expect(method).toContainText("比較用の別のLLM");
  await expect(method).toContainText("実装案");
  await expect(method.getByText("定理の逆翻訳", { exact: true })).toBeVisible();
  await expect(
    method.getByText("依頼との意味比較", { exact: true }),
  ).toBeVisible();
  await expect(
    method.getByText("差分と影響の提示", { exact: true }),
  ).toBeVisible();
  await expect(method.getByText(/FormalAlign/)).toHaveCount(0);
  await page.screenshot({
    path: "target/gui-checks/screenshots/semantic-method.png",
  });
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("button", { name: "意味解析の仕組み", exact: true }),
  ).toBeFocused();
  await page.getByRole("button", { name: "意味解析", exact: true }).click();
  await expect(page.locator(".semantic-result-heading")).toContainText(
    "残高150円の境界に差分",
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
  await page
    .getByRole("button", { name: "メッセージを書く", exact: true })
    .click();
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
    "自販機シミュレーター",
  );
  await page.getByRole("button", { name: "保存を再試行", exact: true }).click();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.reload();
  await expect(page.locator(".user-message").last()).toContainText(
    "保存失敗の確認",
  );
});

test("legacy v2 continuations merge permanently and start with the exact supplied request", async ({
  page,
}) => {
  const original = createVendingWorkspace();
  original.requirements.find((item) => item.id === "purchase_exact")!.title =
    "150円で商品を購入する";
  original.conversations[0].messages =
    original.conversations[0].messages.filter(
      (message) => message.origin !== "provided",
    );
  for (const [id, time, text] of [
    ["continuation-a", "01:00:00", "購入条件も確認して"],
    ["continuation-b", "02:00:00", "境界もお願いします"],
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
          text: "確認しました：" + text,
          status: "received",
          createdAt: "2026-11-18T" + time.replace(":00:", ":01:") + "Z",
        },
      ],
    });
  }
  const other = {
    ...createVendingWorkspace(),
    id: "another-project",
    name: "別のプロジェクト",
    conversations: [
      {
        id: "other-chat",
        title: "別件の会話",
        messages: [
          {
            id: "other-message",
            role: "user" as const,
            text: "別件",
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
        "送信前の続き",
      );
    },
    { original, other },
  );
  await page.goto("/");
  await expect(page.locator(".history-item")).toHaveCount(2);
  await expect(page.locator(".conversation-title")).toHaveText(
    "自販機シミュレーター",
  );
  await expect(page.locator(".message")).toHaveCount(18);
  await expect(page.locator(".recorded-tool")).toHaveCount(25);
  await expect(
    page.getByRole("textbox", { name: "メッセージ", exact: true }),
  ).toHaveValue("送信前の続き");
  expect(
    (await page.locator(".user-message").allTextContents()).slice(-2),
  ).toEqual(["購入条件も確認して", "境界もお願いします"]);
  await expect(
    page.getByRole("button", { name: "別件の会話", exact: true }),
  ).toBeVisible();
  const card = page.getByRole("region", {
    name: "自販機シミュレーターの成果物",
  });
  await expect(card).toContainText(
    "購入が成立すると、残高が150円、在庫が1個減る",
  );
  await expect(card).not.toContainText("150円で商品を購入する");
  await page.reload();
  await expect(page.locator(".message")).toHaveCount(18);
  await page
    .getByRole("textbox", { name: "メッセージ", exact: true })
    .fill("さらに続きを確認");
  await page.getByRole("button", { name: "送信", exact: true }).click();
  await expect(page.locator(".message")).toHaveCount(20);
  await expect(page.locator(".reply-activity")).toHaveCount(0);
  await page.reload();
  await expect(page.locator(".message")).toHaveCount(20);
  await expect(page.locator(".history-item")).toHaveCount(2);
  await page
    .getByRole("button", { name: "メッセージを書く", exact: true })
    .click();
  await page
    .getByRole("textbox", { name: "メッセージ", exact: true })
    .fill("同じ開発の続きです");
  await page.getByRole("button", { name: "送信", exact: true }).click();
  await expect(page.locator(".user-message").last()).toHaveText(
    "同じ開発の続きです",
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
    .filter({ hasText: "購入が成立すると、残高が150円、在庫が1個減る" })
    .click();
  await page.getByRole("button", { name: "広げて読む", exact: true }).click();
  const expanded = page.getByRole("dialog", {
    name: "証明の依存関係",
    exact: true,
  });
  await expect(expanded).toBeVisible();
  await expect(
    expanded.getByRole("complementary", { name: "選択した証明" }),
  ).toContainText("KernelProof.law_purchase_exact");
  await page.screenshot({
    path: "target/gui-checks/screenshots/expanded-graph.png",
  });
  await expanded.getByRole("tab", { name: "Leanソース", exact: true }).click();
  const source = expanded.getByRole("region", { name: "ソースコード" });
  const code = source.getByLabel("ファイルの内容");
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
    .getByRole("button", { name: /Rustの状態遷移.*を選択/ })
    .click();
  await expect(
    expanded.getByRole("complementary", { name: "選択した証明" }),
  ).toContainText("KernelImpl.kernel.step");
  await expanded.getByRole("tab", { name: "Leanソース", exact: true }).click();
  await expect(source.locator(".selected-source-line")).toContainText(
    "def kernel.step",
  );
  await expanded
    .getByRole("button", { name: "会話に戻る", exact: true })
    .click();
  await expect(expanded).toHaveCount(0);
  await expect(page.locator(".user-message").first()).toHaveText(
    vendingOpeningRequest,
  );
});
