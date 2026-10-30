import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import type { Page } from "@playwright/test";

async function newConversation(page: Page): Promise<void> {
  await page.getByRole("button", { name: "新建会话" }).last().click();
  await page.waitForLoadState("networkidle");
  await expect(page.getByRole("textbox").first()).toBeVisible();
}

test.describe("personal rig agent (real serve + fake upstream)", () => {
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

  test("stops a slow non-streaming turn", async ({ page }) => {
    await page.goto("/", { waitUntil: "networkidle" });
    await newConversation(page);
    const composer = page.getByRole("textbox").first();
    await composer.fill("slow");
    await page.keyboard.press("Enter");
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
});
