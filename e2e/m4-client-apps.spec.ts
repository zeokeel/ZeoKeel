import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const LOGIN_URL = process.env["ZEOKEEL_LOGIN_URL"];
const SERVER_URL = process.env["ZEOKEEL_LOGIN_SERVER"] ?? "http://127.0.0.1:8795";

test.describe("M4 client apps (real server)", () => {
  test.skip(LOGIN_URL === undefined, "set ZEOKEEL_LOGIN_URL to run");

  test("open isolated app and roundtrip app_request", async ({ page }) => {
    await page.goto(`${LOGIN_URL}/`, { waitUntil: "networkidle" });
    await page.getByLabel("服务器地址").fill(SERVER_URL);
    await page.getByLabel("账号").fill("admin");
    await page.getByLabel("密码").fill("admin-pass");
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByRole("button", { name: "命令面板" }).first()).toBeVisible({ timeout: 15_000 });
    await page.keyboard.press("Control+k");
    await page.getByRole("option", { name: "应用" }).click();
    const apps = page.getByRole("dialog", { name: "应用" });
    await expect(apps.getByText("client-app")).toBeVisible();
    await apps.getByRole("button", { name: "打开" }).click();
    await expect(apps.getByRole("status")).toContainText("已打开 client-app v1.0.0");
    await expect(page.getByRole("dialog", { name: "应用窗口" })).toBeVisible();
    await expect(page.frameLocator("iframe").locator("body")).toContainText("rows:", { timeout: 20_000 });
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
    await page.screenshot({ path: "target/evidence/m4/client-apps-request.png" });
  });
});
