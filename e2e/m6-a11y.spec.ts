import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

/**
 * B12 a11y matrix — admin SPA (login + every admin page) and the deploy portal.
 * Admin rows need a real server (`scripts/admin-e2e.sh` sets ZEOKEEL_ADMIN_URL);
 * the static `deploy/portal/index.html` row is self-contained and always runs.
 * Gate: 0 serious/critical axe violations + keyboard reachability/focus checks.
 */
const ADMIN_URL = process.env["ZEOKEEL_ADMIN_URL"];
const ADMIN_PASSWORD = process.env["ZEOKEEL_ADMIN_PASSWORD"] ?? "admin-pass";
const OUT = "target/evidence/release-1.0-rc1/phase-b/b12b14";
const STATIC_PORTAL = resolve("deploy/portal/index.html");

mkdirSync(OUT, { recursive: true });

async function expectNoSerious(page: import("@playwright/test").Page, label: string): Promise<void> {
  const axe = await new AxeBuilder({ page }).analyze();
  const serious = axe.violations.filter((v) => v.impact === "serious" || v.impact === "critical");
  expect(serious, `${label}: ${JSON.stringify(serious.map((v) => v.id))}`).toEqual([]);
}

async function focusedControl(page: import("@playwright/test").Page): Promise<{ tag: string; name: string; outline: string; ring: string }> {
  return page.evaluate(() => {
    const element = document.activeElement;
    if (!(element instanceof HTMLElement)) {
      return { tag: "none", name: "", outline: "none", ring: "none" };
    }
    const style = getComputedStyle(element);
    const label = element.getAttribute("aria-label") ?? element.getAttribute("title") ?? (element.textContent ?? "").trim();
    return { tag: element.tagName.toLowerCase(), name: label.slice(0, 80), outline: `${style.outlineStyle} ${style.outlineWidth}`, ring: style.boxShadow };
  });
}

test.describe("M6 admin a11y matrix", () => {
  test.skip(ADMIN_URL === undefined, "set ZEOKEEL_ADMIN_URL");

  test("login page: axe clean + keyboard-only submit", async ({ page }) => {
    await page.goto(`${ADMIN_URL}/`, { waitUntil: "networkidle" });
    await expectNoSerious(page, "admin/login");
    await page.screenshot({ path: `${OUT}/admin-login.png` });

    // Keyboard-only path: focus the password field, type, submit with Enter.
    const password = page.locator("input[type=password]");
    await password.focus();
    await expect(password).toBeFocused();
    await password.fill(ADMIN_PASSWORD);
    await page.keyboard.press("Enter");
    await expect(page.getByRole("heading", { name: "用户 / 角色" })).toBeVisible({ timeout: 15_000 });
  });

  test("every admin page has no serious/critical axe violations", async ({ page }) => {
    await page.goto(`${ADMIN_URL}/`, { waitUntil: "networkidle" });
    await page.locator("input[type=password]").fill(ADMIN_PASSWORD);
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByRole("heading", { name: "用户 / 角色" })).toBeVisible();
    const pages = ["用户/角色", "钱包", "价格", "审计", "用量", "作业", "技能", "连接器", "应用", "系统设置"];
    for (const name of pages) {
      await page.getByRole("button", { name }).click();
      await expectNoSerious(page, `admin/${name}`);
    }
    await page.screenshot({ path: `${OUT}/admin-matrix.png` });
  });

  test("admin shell keeps keyboard focus on named, visible controls", async ({ page }) => {
    await page.goto(`${ADMIN_URL}/`, { waitUntil: "networkidle" });
    await page.locator("input[type=password]").fill(ADMIN_PASSWORD);
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByRole("heading", { name: "用户 / 角色" })).toBeVisible();
    await page.locator("body").click({ position: { x: 2, y: 2 } });
    for (let index = 0; index < 6; index += 1) {
      await page.keyboard.press("Tab");
      const focused = await focusedControl(page);
      expect(["a", "button", "input", "textarea", "select"]).toContain(focused.tag);
      expect(focused.name, `admin Tab stop ${index} has no accessible name`).not.toBe("");
      expect(
        focused.outline.startsWith("none") && focused.ring === "none",
        `admin Tab stop ${index} has no visible focus styling: ${JSON.stringify(focused)}`,
      ).toBe(false);
    }
    await page.screenshot({ path: `${OUT}/admin-keyboard-tab.png` });
  });
});

test.describe("B12 static deploy portal a11y", () => {
  test("deploy/portal/index.html: axe clean, lang set, keyboard reaches a link", async ({ page }) => {
    await page.goto(pathToFileURL(STATIC_PORTAL).href);
    await expect(page.getByRole("heading").first()).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("lang", /^(zh|en)/);
    await expectNoSerious(page, "portal/static");
    await page.screenshot({ path: `${OUT}/portal-static.png` });

    await page.keyboard.press("Tab");
    const focused = await focusedControl(page);
    expect(focused.tag).toBe("a");
    expect(focused.name).not.toBe("");
    expect(focused.outline.startsWith("none") && focused.ring === "none", "portal link has no visible focus styling").toBe(false);
  });
});
