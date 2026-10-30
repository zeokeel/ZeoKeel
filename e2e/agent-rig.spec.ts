import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test.describe("personal rig agent (real serve + fake upstream)", () => {
  test("write approval, ask-user and final answer", async ({ page }) => {
    await page.goto("/", { waitUntil: "networkidle" });
    await page.getByRole("button", { name: "新建会话" }).last().click();
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
});
