import { expect, test, type Page } from "@playwright/test";
import { composerInput, ensureConversation, expectNoSeriousAxe, openClient, recordInteraction } from "./helpers";

/** Inspector baseline contract (interaction.md §3, `@zeokeel/ui-inspector`). */
test.describe("inspector interaction baseline", () => {
  const inspectorOf = (page: Page) => page.getByRole("complementary", { name: "右侧检查器" });

  test(
    "INS-TOGGLE-01: Ctrl+B toggles the inspector even while the composer has focus",
    { tag: ["@interaction:INS-TOGGLE-01"] },
    async ({ page }) => {
      await openClient(page);
      await ensureConversation(page);
      const input = composerInput(page);
      await input.focus();
      await expect(input).toBeFocused();

      await page.keyboard.press("Control+b");
      const inspector = inspectorOf(page);
      await expect(inspector).toBeVisible();
      // The input did not swallow the shortcut and keeps focus.
      await expect(input).toBeFocused();

      await page.keyboard.press("Control+b");
      await expect(inspector).toBeHidden();

      recordInteraction(
        "INS-TOGGLE-01",
        "INS-TOGGLE-01: Ctrl+B toggles the inspector even while the composer has focus",
      );
    },
  );

  test(
    "INS-ARIA-01: inspector exposes one tablist, three tabs and a single selected tab",
    { tag: ["@interaction:INS-ARIA-01"] },
    async ({ page }) => {
      await openClient(page);
      await ensureConversation(page);
      await page.keyboard.press("Control+b");
      const inspector = inspectorOf(page);
      await expect(inspector).toBeVisible();

      await expect(inspector).toMatchAriaSnapshot(`
        - tablist "右侧检查器":
          - tab "文件" [selected]
          - tab "变更"
          - tab "终端"
        - button "收起右栏"
        - tabpanel
      `);
      await expect(inspector.getByRole("tab", { selected: true })).toHaveCount(1);
      await expect(inspector.getByRole("tabpanel")).toBeVisible();

      recordInteraction(
        "INS-ARIA-01",
        "INS-ARIA-01: inspector exposes one tablist, three tabs and a single selected tab",
      );
    },
  );

  test(
    "INS-A11Y-01: open inspector has no serious axe violations",
    { tag: ["@interaction:INS-A11Y-01"] },
    async ({ page }) => {
      await openClient(page);
      await ensureConversation(page);
      await page.keyboard.press("Control+b");
      await expect(inspectorOf(page)).toBeVisible();
      await expectNoSeriousAxe(page, "inspector/panel");

      recordInteraction("INS-A11Y-01", "INS-A11Y-01: open inspector has no serious axe violations");
    },
  );
});
