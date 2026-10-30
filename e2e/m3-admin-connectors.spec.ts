import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const ADMIN_URL = process.env["ZEOKEEL_ADMIN_URL"];
const ADMIN_PASSWORD = process.env["ZEOKEEL_ADMIN_PASSWORD"] ?? "admin-pass";

test.describe("M3 admin connectors (real server)", () => {
  test.skip(ADMIN_URL === undefined, "set ZEOKEEL_ADMIN_URL to run against a real server");

  test("create, grant, oauth and delete a connector", async ({ page }) => {
    await page.goto(`${ADMIN_URL}/`, { waitUntil: "networkidle" });
    await page.locator("input[type=password]").fill(ADMIN_PASSWORD);
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByRole("heading", { name: "用户 / 角色" })).toBeVisible();
    await page.getByRole("button", { name: "连接器" }).click();
    await expect(page.getByRole("heading", { name: "连接器" })).toBeVisible();

    const name = `mcp-${Date.now()}`;
    await page.getByLabel("连接器名").fill(name);
    await page.getByRole("button", { name: "创建连接器" }).click();
    const row = page.getByRole("row", { name: new RegExp(name) });
    await expect(row).toBeVisible();
    await row.getByRole("button", { name: "全员可用" }).click();
    await expect(row).toContainText("all:on");

    await row.getByRole("button", { name: "管理" }).click();
    await page.getByLabel("OAuth client_id").fill("public-client");
    await page.getByRole("button", { name: "保存 OAuth" }).click();
    await expect(row).toContainText("public-client", { timeout: 10_000 });

    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
    await page.screenshot({ path: "target/evidence/m3/admin-connectors.png" });

    await row.getByRole("button", { name: "删除" }).click();
    await expect(row).toHaveCount(0);
  });
});
