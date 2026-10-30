import { test, expect } from "@playwright/test";
import { copyFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import AxeBuilder from "@axe-core/playwright";

const SERVE = "http://127.0.0.1:8788";
const HOME = "target/evidence/m1/home";

test.describe("M1 serve carrier (real dispatch + SQLite)", () => {
  test("send a tool-backed message, then reload and keep the conversation", async ({ page }) => {
    mkdirSync(join(HOME, "demo-data"), { recursive: true });
    copyFileSync("demo-data/hello.txt", join(HOME, "demo-data/hello.txt"));
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    await page.getByRole("button", { name: "新建会话" }).last().click();
    const composer = page.getByRole("textbox").first();
    await expect(composer).toBeVisible();
    await composer.fill("读取 demo-data/hello.txt");
    await page.keyboard.press("Enter");
    await expect(page.getByText("fs.read", { exact: false }).first()).toBeVisible({ timeout: 20_000 });
    await expect(page.getByText(/Hello (from )?ZeoKeel/, { exact: false }).first()).toBeVisible({ timeout: 20_000 });
    await page.screenshot({ path: "target/evidence/m1/serve-chat.png" });

    await page.reload({ waitUntil: "networkidle" });
    await expect(page.getByText("读取 demo-data/hello.txt", { exact: false }).first()).toBeVisible({ timeout: 20_000 });
    await expect(page.getByText(/Hello (from )?ZeoKeel/, { exact: false }).first()).toBeVisible({ timeout: 20_000 });
    await page.screenshot({ path: "target/evidence/m1/serve-reload.png" });
  });

  test("inspector lists workspace files with Ctrl+B", async ({ page }) => {
    mkdirSync(join(HOME, "demo-data"), { recursive: true });
    copyFileSync("demo-data/hello.txt", join(HOME, "demo-data/hello.txt"));
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    await page.keyboard.press("Control+b");
    await expect(page.getByRole("complementary", { name: "右侧检查器" })).toBeVisible();
    await expect(page.getByText("demo-data/hello.txt", { exact: true })).toBeVisible({ timeout: 10_000 });
    await page.screenshot({ path: "target/evidence/m1/serve-inspector.png" });
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
    await page.keyboard.press("Control+b");
    await expect(page.getByRole("complementary", { name: "右侧检查器" })).toBeHidden();
  });

  test("settings dialog opens in serve mode", async ({ page }) => {
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    await page.getByRole("button", { name: /设置|Settings/ }).first().click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toBeHidden();
  });
});
