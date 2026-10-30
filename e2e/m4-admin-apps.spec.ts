import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const ADMIN_URL = process.env["ZEOKEEL_ADMIN_URL"];
const ADMIN_PASSWORD = process.env["ZEOKEEL_ADMIN_PASSWORD"] ?? "admin-pass";

test.describe("M4 admin apps (real server)", () => {
  test.skip(ADMIN_URL === undefined, "set ZEOKEEL_ADMIN_URL to run");

  test("approve, grant and delist a seeded app", async ({ page }) => {
    await page.goto(`${ADMIN_URL}/`, { waitUntil: "networkidle" });
    await page.locator("input[type=password]").fill(ADMIN_PASSWORD);
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByRole("heading", { name: "用户 / 角色" })).toBeVisible();
    await page.getByRole("button", { name: "应用" }).click();
    await expect(page.getByRole("heading", { name: "应用" })).toBeVisible();
    const row = page.getByRole("row", { name: /admin-app/ });
    await expect(row).toBeVisible();
    await row.getByRole("button", { name: "全员可用" }).click();
    await row.getByRole("button", { name: "审批" }).click();
    await expect(row).toContainText("approved");
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
    await page.screenshot({ path: "target/evidence/m4/admin-apps.png" });
    await row.getByRole("button", { name: "下架" }).click();
    await expect(row).toContainText("delisted");
  });
});
