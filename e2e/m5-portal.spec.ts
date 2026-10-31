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
    await expect(page.locator("html")).toHaveAttribute("lang", /^(zh|en)/);
    const axe = await new AxeBuilder({ page }).analyze();
    const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
    expect(serious, JSON.stringify(serious.map((v) => v.id))).toEqual([]);
    await page.screenshot({ path: "target/evidence/m5/portal.png" });
  });

  test("keyboard reaches the download links with visible focus", async ({ page }) => {
    await page.goto(PORTAL_URL ?? "");
    await page.keyboard.press("Tab");
    const first = await page.evaluate(() => {
      const element = document.activeElement;
      if (!(element instanceof HTMLElement)) {
        return null;
      }
      const style = getComputedStyle(element);
      return {
        tag: element.tagName.toLowerCase(),
        name: (element.getAttribute("aria-label") ?? element.textContent ?? "").trim().slice(0, 80),
        outline: `${style.outlineStyle} ${style.outlineWidth}`,
        ring: style.boxShadow,
      };
    });
    expect(first).not.toBeNull();
    expect(first?.tag).toBe("a");
    expect(first?.name).not.toBe("");
    expect(first?.outline.startsWith("none") && first?.ring === "none", "portal link has no visible focus styling").toBe(false);
  });
});
