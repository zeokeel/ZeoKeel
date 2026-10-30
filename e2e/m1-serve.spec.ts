import { test, expect } from "@playwright/test";

const SERVE = "http://127.0.0.1:8788";

test.describe("M1 serve carrier (real dispatch + SQLite)", () => {
  test("send a tool-backed message, then reload and keep the conversation", async ({ page }) => {
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
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

  test("settings dialog opens in serve mode", async ({ page }) => {
    await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
    await page.getByRole("button", { name: /设置|Settings/ }).first().click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toBeHidden();
  });
});
