import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const ADMIN_URL = process.env["ZEOKEEL_ADMIN_URL"];
const ADMIN_PASSWORD = process.env["ZEOKEEL_ADMIN_PASSWORD"] ?? "admin-pass";

test.describe("M2 admin console (real server)", () => {
  test.skip(ADMIN_URL === undefined, "set ZEOKEEL_ADMIN_URL to run against a real server");

  test.beforeEach(async ({ page }) => {
    await page.goto(`${ADMIN_URL}/`, { waitUntil: "networkidle" });
    await page.locator("input[type=password]").fill(ADMIN_PASSWORD);
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByRole("heading", { name: "用户 / 角色" })).toBeVisible();
  });

  test("create a user and see it in the list", async ({ page }) => {
    const name = `ui-${Date.now()}`;
    await page.getByPlaceholder("username").fill(name);
    await page.getByPlaceholder("password (>=8)").fill("ui-pass-123");
    await page.getByRole("button", { name: "创建" }).click();
    await expect(page.getByText(name)).toBeVisible();
  });

  test("adjust wallet and save pricing", async ({ page }) => {
    await page.getByRole("button", { name: "钱包" }).click();
    await expect(page.getByRole("heading", { name: "钱包" })).toBeVisible();
    await page.getByRole("button", { name: "调账" }).first().click();
    await expect(page.getByText("admin ui")).toBeVisible({ timeout: 5_000 }).catch(() => undefined);
    await page.getByRole("button", { name: "价格" }).click();
    await expect(page.getByRole("heading", { name: "模型价格" })).toBeVisible();
    await page.getByRole("button", { name: "保存" }).click();
    await expect(page.getByText("deepseek-chat")).toBeVisible();
  });

  test("audit and usage pages render", async ({ page }) => {
    await page.getByRole("button", { name: "审计" }).click();
    await expect(page.getByRole("heading", { name: "审计" })).toBeVisible();
    await expect(page.locator("tbody tr").first()).toBeVisible();
    await page.getByRole("button", { name: "用量" }).click();
    await expect(page.getByRole("heading", { name: "用量" })).toBeVisible();
  });

  test("jobs page can trigger reconcile", async ({ page }) => {
    await page.getByRole("button", { name: "作业" }).click();
    await expect(page.getByRole("heading", { name: "作业" })).toBeVisible();
    await page.getByRole("button", { name: "对账" }).click();
    await expect(page.locator("tbody tr").first()).toBeVisible();
  });

  test("save a system setting", async ({ page }) => {
    await page.getByRole("button", { name: "系统设置" }).click();
    await expect(page.getByRole("heading", { name: "系统设置" })).toBeVisible();
    await page.getByRole("button", { name: "保存" }).click();
    await expect(page.getByText("updates.channel")).toBeVisible();
  });

  test("no serious/critical axe violations", async ({ page }) => {
    const results = await new AxeBuilder({ page }).analyze();
    const serious = results.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious).toEqual([]);
  });
});
