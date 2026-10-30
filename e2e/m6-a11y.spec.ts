import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const ADMIN_URL = process.env["ZEOKEEL_ADMIN_URL"];
const ADMIN_PASSWORD = process.env["ZEOKEEL_ADMIN_PASSWORD"] ?? "admin-pass";

test.describe("M6 admin a11y matrix", () => {
  test.skip(ADMIN_URL === undefined, "set ZEOKEEL_ADMIN_URL");
  test("every admin page has no serious/critical axe violations", async ({ page }) => {
    await page.goto(`${ADMIN_URL}/`, { waitUntil: "networkidle" });
    await page.locator("input[type=password]").fill(ADMIN_PASSWORD);
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByRole("heading", { name: "用户 / 角色" })).toBeVisible();
    const pages = ["用户/角色", "钱包", "价格", "审计", "用量", "作业", "技能", "连接器", "应用", "系统设置"];
    for (const name of pages) {
      await page.getByRole("button", { name }).click();
      const axe = await new AxeBuilder({ page }).analyze();
      const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
      expect(serious, `${name}: ${JSON.stringify(serious.map((v) => v.id))}`).toEqual([]);
    }
    await page.screenshot({ path: "target/evidence/m6/admin-a11y.png" });
  });
});
