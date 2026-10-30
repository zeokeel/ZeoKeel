import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const PORTAL_URL = process.env["ZEOKEEL_PORTAL_URL"];

test.describe("M5 client portal", () => {
  test.skip(PORTAL_URL === undefined, "set ZEOKEEL_PORTAL_URL");

  test("renders channel/version/download and passes axe", async ({ page }) => {
    await page.goto(PORTAL_URL ?? "");
    await expect(page.getByRole("heading", { name: "ZeoKeel 客户端下载" })).toBeVisible();
    await expect(page.getByText("渠道：official")).toBeVisible();
    await expect(page.getByRole("link", { name: /Linux/ })).toBeVisible();
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
    await page.screenshot({ path: "target/evidence/m5/portal.png" });
  });
});
