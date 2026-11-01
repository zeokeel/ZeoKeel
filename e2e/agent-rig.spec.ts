import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import type { Page } from "@playwright/test";

async function newConversation(page: Page): Promise<void> {
  // The turn's first events (e.g. approval_requested) are emitted immediately; never send
  // before the SSE subscription is open, otherwise the event is lost with no replay.
  await page.waitForFunction(() => document.documentElement.dataset["zkEventsReady"] === "1");
  await page.getByRole("button", { name: "新建会话" }).last().click();
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(150);
  await expect(page.getByRole("textbox").first()).toBeVisible();
}

test.describe("personal rig agent (real serve + fake upstream)", () => {
  test("unknown tool does not crash the turn", async ({ page }) => {
    await page.goto("/", { waitUntil: "networkidle" });
    await newConversation(page);
    const composer = page.getByRole("textbox").first();
    await composer.fill("unknown");
    await page.keyboard.press("Enter");
    await expect(page.getByText(/unknown handled|AGENT_MAX_STEPS|生成失败|unknown_tool/).last()).toBeVisible({ timeout: 30_000 });
  });

  test("max turns surfaces AGENT_MAX_STEPS", async ({ page }) => {
    await page.goto("/", { waitUntil: "networkidle" });
    await newConversation(page);
    const composer = page.getByRole("textbox").first();
    await composer.fill("loop");
    await page.keyboard.press("Enter");
    await expect(page.getByText(/AGENT_MAX_STEPS|生成失败/).last()).toBeVisible({ timeout: 30_000 });
  });

  test("rejecting an approval is fail-closed", async ({ page }) => {
    await page.goto("/", { waitUntil: "networkidle" });
    await newConversation(page);
    const composer = page.getByRole("textbox").first();
    await composer.fill("write a file");
    await page.keyboard.press("Enter");
    const approval = page.getByRole("group", { name: "需要审批" });
    await expect(approval).toBeVisible();
    await approval.getByRole("button", { name: "拒绝" }).click();
    await expect(page.getByText(/denied by approval policy/).first()).toBeVisible();
  });

  test("write approval, ask-user and final answer", async ({ page }) => {
    await page.goto("/", { waitUntil: "networkidle" });
    await newConversation(page);
    const composer = page.getByRole("textbox").first();
    await composer.fill("write a file");
    await page.keyboard.press("Enter");

    const approval = page.getByRole("group", { name: "需要审批" });
    await expect(approval).toBeVisible();
    await approval.getByRole("button", { name: "允许一次" }).click();
    await expect(page.getByText("已完成工具调用并通过审批。").last()).toBeVisible();
    await page.screenshot({ path: "target/evidence/agent-rig/approval.png" });

    const composer2 = page.getByRole("textbox").first();
    await composer2.fill("ask me");
    await page.keyboard.press("Enter");
    await expect(page.getByRole("group", { name: "需要回答" })).toBeVisible();
    await page.getByLabel("你的名字？").fill("Ada");
    await page.getByRole("button", { name: "提交回答" }).click();
    await expect(page.getByText("已完成工具调用并通过审批。").last()).toBeVisible();
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((violation) => violation.impact === "serious" || violation.impact === "critical");
    expect(serious, JSON.stringify(serious.map((violation) => violation.id))).toEqual([]);
    await page.screenshot({ path: "target/evidence/agent-rig/question.png" });
  });

  test("stops a slow non-streaming turn", async ({ page }) => {
    await page.goto("/", { waitUntil: "networkidle" });
    await newConversation(page);
    const composer = page.getByRole("textbox").first();
    await composer.fill("slow");
    await page.keyboard.press("Enter");
    await expect(page.getByText("生成中…").first()).toBeVisible();
    await page.waitForTimeout(200);
    const stop = page.getByRole("button", { name: "停止生成" });
    await expect(stop).toBeVisible();
    await stop.click();
    await expect(page.getByText("已取消").first()).toBeVisible();
  });

  test("runs an approved bash command", async ({ page }) => {
    await page.goto("/", { waitUntil: "networkidle" });
    await newConversation(page);
    const composer = page.getByRole("textbox").first();
    await composer.fill("bash");
    await page.keyboard.press("Enter");
    const approval = page.getByRole("group", { name: "需要审批" });
    await expect(approval).toBeVisible();
    await approval.getByRole("button", { name: "允许一次" }).click();
    await expect(page.getByText("hi", { exact: false }).first()).toBeVisible();
    await expect(page.getByText("已完成工具调用并通过审批。").last()).toBeVisible();
  });

  test("presents a deliverable after approved write", async ({ page }) => {
    await page.goto("/", { waitUntil: "networkidle" });
    await newConversation(page);
    const composer = page.getByRole("textbox").first();
    await composer.fill("present");
    await page.keyboard.press("Enter");
    const approval = page.getByRole("group", { name: "需要审批" });
    await expect(approval).toBeVisible();
    await approval.getByRole("button", { name: "允许一次" }).click();
    await expect(page.getByText("present", { exact: false }).last()).toBeVisible();
    await expect(page.getByText("已完成工具调用并通过审批。").last()).toBeVisible();
  });

  test("runs a multi-step read/write loop", async ({ page }) => {
    await page.goto("/", { waitUntil: "networkidle" });
    await newConversation(page);
    const composer = page.getByRole("textbox").first();
    await composer.fill("multi");
    await page.keyboard.press("Enter");
    await expect(page.getByText("fs.read", { exact: false }).first()).toBeVisible();
    await expect(page.getByText("fs.read", { exact: false })).toHaveCount(2);
    await expect(page.getByText("已完成工具调用并通过审批。").last()).toBeVisible();
    await page.screenshot({ path: "target/evidence/agent-rig/multi-step.png" });
  });
  test("settings hot-swap switches the model provider", async ({ page }) => {
    const upstream2 = process.env["ZEOKEEL_AGENT_UPSTREAM2"];
    test.skip(upstream2 === undefined, "second upstream not configured");
    await page.goto("/", { waitUntil: "networkidle" });
    await page.getByRole("button", { name: /设置|Settings/ }).first().click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    // Paginated settings: provider credentials live in the "本地 Provider" pane.
    await dialog.getByRole("button", { name: /本地 Provider|Local provider/ }).click();
    await dialog.getByLabel("Provider 类型").selectOption("open_ai_compatible");
    await dialog.getByLabel("Base URL").fill(upstream2 ?? "");
    await dialog.getByLabel("模型", { exact: true }).fill("test-model");
    await dialog.getByRole("button", { name: /保存|Save/ }).click();
    await expect(dialog).toBeHidden({ timeout: 20_000 });
    await newConversation(page);
    const composer = page.getByRole("textbox").first();
    await composer.fill("hello");
    await page.keyboard.press("Enter");
    await expect(page.getByText("second provider").last()).toBeVisible({ timeout: 30_000 });
  });


});
