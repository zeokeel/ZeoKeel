import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const ADMIN_URL = process.env["ZEOKEEL_ADMIN_URL"];
const ADMIN_PASSWORD = process.env["ZEOKEEL_ADMIN_PASSWORD"] ?? "admin-pass";
const SKILL_ZIP = process.env["ZEOKEEL_SKILL_ZIP"] ?? "/tmp/zk-admin-skill.zip";

test.describe("M3 admin skills (real server)", () => {
  test.skip(ADMIN_URL === undefined, "set ZEOKEEL_ADMIN_URL to run against a real server");

  test("create, approve, upload, grant and delete a skill", async ({ page }) => {
    await page.goto(`${ADMIN_URL}/`, { waitUntil: "networkidle" });
    await page.locator("input[type=password]").fill(ADMIN_PASSWORD);
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByRole("heading", { name: "用户 / 角色" })).toBeVisible();
    await page.getByRole("button", { name: "技能" }).click();
    await expect(page.getByRole("heading", { name: "技能" })).toBeVisible();

    const name = `skill-${Date.now()}`;
    await page.getByLabel("技能名").fill(name);
    await page.getByRole("button", { name: "创建技能" }).click();
    const row = page.getByRole("row", { name: new RegExp(name) });
    await expect(row).toBeVisible();

    await row.getByRole("button", { name: "审批" }).click();
    await expect(row).toContainText("approved");

    await row.getByRole("button", { name: "管理" }).click();
    await page.getByRole("textbox", { name: "版本" }).fill("1.0.0");
    await page.getByLabel("技能 zip").setInputFiles(SKILL_ZIP);
    await page.getByRole("button", { name: "上传版本" }).click();
    await expect(row).toContainText("1.0.0", { timeout: 15_000 });
    await page.getByRole("button", { name: "全员可安装" }).click();

    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);

    await row.getByRole("button", { name: "删除" }).click();
    await expect(row).toHaveCount(0);
    await page.screenshot({ path: "target/evidence/m3/admin-skills.png" });
  });
});
