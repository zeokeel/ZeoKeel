import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const LOGIN_URL = process.env["ZEOKEEL_LOGIN_URL"];
const SERVER_URL = process.env["ZEOKEEL_LOGIN_SERVER"] ?? "http://127.0.0.1:8795";

test.describe("M2 enterprise login (real server + enterprise composition)", () => {
  test.skip(LOGIN_URL === undefined, "set ZEOKEEL_LOGIN_URL to run");

  test("first run connects to server and enters the app", async ({ page }) => {
    await page.goto(`${LOGIN_URL}/`, { waitUntil: "networkidle" });
    await expect(page.getByRole("heading", { name: "连接企业服务器" })).toBeVisible();
    await page.getByLabel("服务器地址").fill(SERVER_URL);
    await page.getByLabel("账号").fill("admin");
    await page.getByLabel("密码").fill("admin-pass");
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByRole("button", { name: "命令面板" }).first()).toBeVisible({ timeout: 15_000 });
    await page.screenshot({ path: "target/evidence/m2/login.png" });
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious).toEqual([]);
  });
});
