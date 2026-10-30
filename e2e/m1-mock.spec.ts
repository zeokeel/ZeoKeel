import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test.describe("M1 mock UI (human flow)", () => {
  test("send a tool-backed message and render the tool card", async ({ page }) => {
    await page.goto("/?mock=1", { waitUntil: "networkidle" });
    await page.getByRole("button", { name: "新建会话" }).last().click();
    const composer = page.getByRole("textbox").first();
    await expect(composer).toBeVisible();
    await composer.click();
    await composer.fill("读取 demo-data/hello.txt");
    await page.keyboard.press("Enter");
    await expect(page.getByText("fs.read", { exact: false }).first()).toBeVisible({ timeout: 15_000 });
    await expect(page.getByText(/Hello (from )?ZeoKeel/, { exact: false }).first()).toBeVisible({ timeout: 15_000 });
    await page.screenshot({ path: "target/evidence/m1/e2e-chat.png", fullPage: false });
  });

  test("settings validate and Esc closes; axe has no serious issues", async ({ page }) => {
    await page.goto("/?mock=1", { waitUntil: "networkidle" });
    await page.getByRole("button", { name: /设置|Settings/ }).first().click();
    await expect(page.getByText(/模型 provider|Model provider/)).toBeVisible();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    const results = await new AxeBuilder({ page }).analyze();
    const serious = results.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    await page.screenshot({ path: "target/evidence/m1/e2e-settings-closed.png" });
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
  });

  test("command palette opens with keyboard, filters and runs settings", async ({ page }) => {
    await page.goto("/?mock=1", { waitUntil: "networkidle" });
    await page.keyboard.press("Control+k");
    const palette = page.getByRole("dialog", { name: "命令面板" });
    await expect(palette).toBeVisible();
    await page.getByRole("textbox", { name: "搜索命令" }).fill("设置");
    await expect(page.getByRole("option", { name: /设置/ })).toBeVisible();
    await page.screenshot({ path: "target/evidence/m1/e2e-commands.png" });
    await page.getByRole("option", { name: /设置/ }).click();
    await expect(page.getByRole("dialog", { name: "设置" })).toBeVisible();
    const results = await new AxeBuilder({ page }).analyze();
    const serious = results.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
  });

  test("empty state and composer keyboard hint", async ({ page }) => {
    await page.goto("/?mock=1", { waitUntil: "networkidle" });
    await expect(page.getByText(/今天想完成什么|What would you like/)).toBeVisible();
    await expect(page.getByRole("button", { name: "新建会话" }).last()).toBeVisible();
    await expect(page.getByRole("textbox")).toHaveCount(0);
    await page.screenshot({ path: "target/evidence/m1/e2e-empty.png" });
  });
});
