import { expect, test, type Page } from "@playwright/test";
import { expectNoSeriousAxe, openClient, recordInteraction } from "./helpers";

/** Settings dialog baseline contract (interaction.md §2, `@zeokeel/ui-settings`). */
test.describe("settings dialog interaction baseline", () => {
  const openerOf = (page: Page) => page.getByRole("button", { name: "打开设置" });
  const dialogOf = (page: Page) => page.getByRole("dialog", { name: "设置" });

  test(
    "SET-OPEN-01: dialog opens from the settings button with focus inside and pane nav visible",
    { tag: ["@interaction:SET-OPEN-01"] },
    async ({ page }) => {
      await openClient(page);
      await openerOf(page).click();
      const dialog = dialogOf(page);
      await expect(dialog).toBeVisible();
      await expect(dialog.getByRole("heading", { name: "设置", level: 2 })).toBeVisible();
      // Radix moves focus to the first tabbable control inside the dialog (the close button).
      await expect(dialog.getByRole("button", { name: "关闭" })).toBeFocused();
      const sections = dialog.getByRole("navigation", { name: "设置分区" });
      await expect(sections).toBeVisible();
      expect(await sections.getByRole("button").count()).toBeGreaterThanOrEqual(9);

      recordInteraction(
        "SET-OPEN-01",
        "SET-OPEN-01: dialog opens from the settings button with focus inside and pane nav visible",
      );
    },
  );

  test(
    "SET-ESC-01: Esc closes the dialog and returns focus to the exact opener",
    { tag: ["@interaction:SET-ESC-01"] },
    async ({ page }) => {
      await openClient(page);
      const opener = openerOf(page);
      await opener.click();
      await expect(dialogOf(page)).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(dialogOf(page)).toBeHidden();
      await expect(opener).toBeFocused();

      recordInteraction(
        "SET-ESC-01",
        "SET-ESC-01: Esc closes the dialog and returns focus to the exact opener",
      );
    },
  );

  test(
    "SET-ARIA-01: dialog structure is a named dialog with a single current pane and one save",
    { tag: ["@interaction:SET-ARIA-01"] },
    async ({ page }) => {
      await openClient(page);
      await openerOf(page).click();
      const dialog = dialogOf(page);
      await expect(dialog).toBeVisible();

      await expect(dialog).toMatchAriaSnapshot(`
        - heading "设置" [level=2]
        - button "关闭"
        - navigation "设置分区":
          - button "通用"
          - button "模型"
        - button "取消"
        - button "保存"
      `);
      // The shipped pane nav marks the active pane with `aria-current="page"` (not aria-selected).
      await expect(dialog.locator('[aria-current="page"]')).toHaveCount(1);
      await expect(dialog.getByRole("button", { name: "通用" })).toHaveAttribute("aria-current", "page");
      await expect(dialog.getByRole("button", { name: "保存", exact: true })).toHaveCount(1);

      recordInteraction(
        "SET-ARIA-01",
        "SET-ARIA-01: dialog structure is a named dialog with a single current pane and one save",
      );
    },
  );

  test(
    "SET-A11Y-01: open settings dialog has no serious axe violations",
    { tag: ["@interaction:SET-A11Y-01"] },
    async ({ page }) => {
      await openClient(page);
      await openerOf(page).click();
      await expect(dialogOf(page)).toBeVisible();
      await expectNoSeriousAxe(page, "settings/dialog");

      recordInteraction("SET-A11Y-01", "SET-A11Y-01: open settings dialog has no serious axe violations");
    },
  );
});
