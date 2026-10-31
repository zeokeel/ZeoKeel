import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { mkdirSync } from "node:fs";

/**
 * B12 a11y matrix — personal client (`serve` carrier, real SQLite + fake upstream).
 * Runs as part of `scripts/m1-e2e.sh` after the baseline composition is restored.
 * Gate: 0 serious/critical axe violations per page + keyboard/focus/reduced-motion checks.
 */
const SERVE = "http://127.0.0.1:8788";
const OUT = "target/evidence/release-1.0-rc1/phase-b/b12b14";

type AxeLike = { violations: Array<{ id: string; impact?: string | null }> };

mkdirSync(OUT, { recursive: true });

function serious(violations: AxeLike["violations"]): string[] {
  return violations
    .filter((violation) => violation.impact === "serious" || violation.impact === "critical")
    .map((violation) => violation.id);
}

async function expectNoSeriousViolations(page: import("@playwright/test").Page, label: string): Promise<void> {
  const results: AxeLike = await new AxeBuilder({ page }).analyze();
  const found = serious(results.violations);
  expect(found, `${label}: serious/critical axe violations`).toEqual([]);
}

async function openClient(page: import("@playwright/test").Page): Promise<void> {
  await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
  await expect(page.getByRole("button", { name: /新建会话|New conversation/ }).first()).toBeVisible();
}

async function ensureConversation(page: import("@playwright/test").Page): Promise<void> {
  if ((await page.getByRole("textbox").count()) === 0) {
    await page.getByRole("button", { name: /新建会话|New conversation/ }).first().click();
  }
  await expect(page.getByRole("textbox").first()).toBeVisible();
}

test.describe("B12 client a11y matrix (real serve carrier)", () => {
  test("empty state (no conversation) has no serious/critical axe violations", async ({ page }) => {
    await openClient(page);
    await expectNoSeriousViolations(page, "client/empty");
    await page.screenshot({ path: `${OUT}/client-empty.png`, fullPage: false });
  });

  test("chat main view has no serious/critical axe violations", async ({ page }) => {
    await openClient(page);
    await ensureConversation(page);
    await expectNoSeriousViolations(page, "client/chat");
    await page.screenshot({ path: `${OUT}/client-chat.png`, fullPage: false });
  });

  test("settings dialog: axe clean, Esc closes, focus returns to opener", async ({ page }) => {
    await openClient(page);
    const opener = page.getByRole("button", { name: /设置|Settings/ }).first();
    await opener.click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    // Radix dialog must move focus inside and expose its accessible name.
    await expect(dialog).toHaveAccessibleName(/设置|Settings/);
    expect(await page.evaluate(() => document.activeElement?.closest("[role=dialog]") !== null)).toBe(true);
    await expectNoSeriousViolations(page, "client/settings-dialog");
    await page.screenshot({ path: `${OUT}/client-settings.png`, fullPage: false });
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    await expect(opener).toBeFocused();
  });

  test("command palette (Ctrl+K): axe clean, arrow-key navigation, Esc closes", async ({ page }) => {
    await openClient(page);
    await page.keyboard.press("Control+k");
    const palette = page.getByRole("dialog", { name: /命令面板|Command palette/ });
    await expect(palette).toBeVisible();
    await expect(palette.getByRole("option").first()).toBeVisible();
    await expectNoSeriousViolations(page, "client/command-palette");
    await page.screenshot({ path: `${OUT}/client-command-palette.png`, fullPage: false });

    await page.keyboard.press("ArrowDown");
    const activeDescendant = await palette.locator("input").getAttribute("aria-activedescendant");
    expect(activeDescendant).not.toBeNull();
    await expect(palette.getByRole("option", { selected: true })).toHaveCount(1);
    await page.keyboard.press("Escape");
    await expect(palette).toBeHidden();
  });

  test("inspector (Ctrl+B): axe clean and toggles closed again", async ({ page }) => {
    await openClient(page);
    await page.keyboard.press("Control+b");
    const inspector = page.getByRole("complementary", { name: /右侧检查器|Inspector/ });
    await expect(inspector).toBeVisible();
    await expectNoSeriousViolations(page, "client/inspector");
    await page.screenshot({ path: `${OUT}/client-inspector.png`, fullPage: false });
    await page.keyboard.press("Control+b");
    await expect(inspector).toBeHidden();
  });

  test("keyboard: skip link + Tab order exposes named, visibly focused controls", async ({ page }) => {
    await openClient(page);
    await ensureConversation(page);

    // 1) First Tab reaches the skip link and reveals it (visible focus proof).
    await page.keyboard.press("Tab");
    const skip = page.locator(".zk-skip-link");
    await expect(skip).toBeFocused();
    await page.waitForTimeout(300);
    const skipTop = await skip.evaluate((element) => getComputedStyle(element).insetBlockStart);
    expect(skipTop, "focused skip link must be moved into view").toBe("12px");

    // 2) Tab through the shell: every stop is an interactive, named control and
    //    keyboard focus is visible (outline or focus ring), never a bare div.
    const stops: Array<{ tag: string; name: string; outline: string; ring: string }> = [];
    for (let index = 0; index < 15; index += 1) {
      await page.keyboard.press("Tab");
      const info = await page.evaluate(() => {
        const element = document.activeElement;
        if (!(element instanceof HTMLElement)) {
          return { tag: "none", name: "", outline: "none", ring: "none" };
        }
        const style = getComputedStyle(element);
        const label = element.getAttribute("aria-label") ?? element.getAttribute("title") ?? (element.textContent ?? "").trim();
        return {
          tag: element.tagName.toLowerCase(),
          name: label.slice(0, 80),
          outline: `${style.outlineStyle} ${style.outlineWidth}`,
          ring: style.boxShadow,
        };
      });
      stops.push(info);
    }
    const interactive = new Set(["a", "button", "input", "textarea", "select"]);
    for (const stop of stops) {
      expect(interactive.has(stop.tag), `non-interactive Tab stop: ${JSON.stringify(stop)}`).toBe(true);
    }
    expect(stops.some((stop) => stop.tag === "button" || stop.tag === "a")).toBe(true);
    // Composer input is wrapped by a :focus-within border; other stops must show an outline.
    const withoutVisibleFocus = stops.filter(
      (stop) => stop.outline.startsWith("none") && stop.ring === "none" && stop.tag !== "textarea",
    );
    expect(withoutVisibleFocus, "Tab stops without visible focus styling").toEqual([]);
    expect(stops.filter((stop) => stop.name === ""), "Tab stops without accessible name").toEqual([]);
    await page.screenshot({ path: `${OUT}/client-keyboard-tab.png`, fullPage: false });
  });
});

test.describe("B12 client reduced motion", () => {
  test.use({ reducedMotion: "reduce" });

  test("prefers-reduced-motion disables transitions/animations", async ({ page }) => {
    await openClient(page);
    expect(await page.evaluate(() => window.matchMedia("(prefers-reduced-motion: reduce)").matches)).toBe(true);
    await page.getByRole("button", { name: /设置|Settings/ }).first().click();
    await expect(page.getByRole("dialog")).toBeVisible();
    const worst = await page.evaluate(() => {
      const seconds = (value: string): number => {
        const trimmed = value.trim();
        if (trimmed.endsWith("ms")) {
          return Number.parseFloat(trimmed) / 1000;
        }
        if (trimmed.endsWith("s")) {
          return Number.parseFloat(trimmed);
        }
        return 0;
      };
      let maxTransition = 0;
      let maxAnimation = 0;
      for (const element of Array.from(document.querySelectorAll("*"))) {
        const style = getComputedStyle(element);
        const durations = style.transitionDuration.split(",").map(seconds);
        const animations = style.animationDuration.split(",").map(seconds);
        maxTransition = Math.max(maxTransition, ...durations);
        maxAnimation = Math.max(maxAnimation, ...animations);
      }
      return { maxTransition, maxAnimation };
    });
    expect(worst.maxTransition, `longest transition under reduced motion: ${worst.maxTransition}s`).toBeLessThanOrEqual(0.02);
    expect(worst.maxAnimation, `longest animation under reduced motion: ${worst.maxAnimation}s`).toBeLessThanOrEqual(0.02);
    await page.screenshot({ path: `${OUT}/client-reduced-motion.png`, fullPage: false });
  });
});
