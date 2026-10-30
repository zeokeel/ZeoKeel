import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test.describe("M1 mock UI (human flow)", () => {
  test("send a tool-backed message and render the tool card", async ({ page }) => {
    await page.goto("/?mock=1", { waitUntil: "networkidle" });
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
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    const results = await new AxeBuilder({ page }).analyze();
    const serious = results.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    await page.screenshot({ path: "target/evidence/m1/e2e-settings-closed.png" });
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
  });

  test("empty state and composer keyboard hint", async ({ page }) => {
    await page.goto("/?mock=1", { waitUntil: "networkidle" });
    await expect(page.getByText(/今天想完成什么|What would you like/)).toBeVisible();
    await expect(page.getByRole("textbox").first()).toBeVisible();
    await page.screenshot({ path: "target/evidence/m1/e2e-empty.png" });
  });
});
