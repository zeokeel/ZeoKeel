import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const LOGIN_URL = process.env["ZEOKEEL_LOGIN_URL"];
const SERVER_URL = process.env["ZEOKEEL_LOGIN_SERVER"] ?? "http://127.0.0.1:8795";

test.describe("M2 enterprise chat through gateway", () => {
  test.skip(LOGIN_URL === undefined, "set ZEOKEEL_LOGIN_URL to run");

  test("logged-in client sends a gateway-backed message", async ({ page }) => {
    await page.goto(`${LOGIN_URL}/`, { waitUntil: "networkidle" });
    const addressField = page.getByLabel("服务器地址");
    if (await addressField.isVisible().catch(() => false)) await addressField.fill(SERVER_URL);
    await page.getByLabel("账号").fill("admin");
    await page.getByLabel("密码").fill("admin-pass");
    await page.getByRole("button", { name: "登录" }).click();
    await page.getByRole("button", { name: "打开设置" }).click();
    await expect(page.getByLabel("企业模型")).toBeVisible({ timeout: 10_000 });
    await expect(page.getByText("模型 provider")).toHaveCount(0);
    await page.keyboard.press("Escape");
    const firstRun = page.getByRole("heading", { name: "先选择一个工作区" });
    if (await firstRun.isVisible().catch(() => false)) {
      await page.getByRole("button", { name: /使用默认/ }).click();
    }
    await page.getByRole("button", { name: "新建会话" }).last().click();
    const composer = page.getByRole("textbox").first();
    await composer.fill("hello gateway");
    await page.keyboard.press("Enter");
    await expect(page.getByText("gateway hello", { exact: false }).first()).toBeVisible({ timeout: 20_000 });
    await page.screenshot({ path: "target/evidence/m2/gateway-chat.png" });
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
  });
});
