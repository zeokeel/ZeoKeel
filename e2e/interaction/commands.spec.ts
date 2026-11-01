import { expect, test, type Page } from "@playwright/test";
import { composerInput, ensureConversation, expectNoSeriousAxe, openClient, recordInteraction } from "./helpers";

/** Command palette baseline contract (interaction.md §4, `@zeokeel/ui-commands`). */
test.describe("command palette interaction baseline", () => {
  const paletteOf = (page: Page) => page.getByRole("dialog", { name: "命令面板" });
  const searchOf = (page: Page) => paletteOf(page).getByRole("textbox", { name: "搜索命令" });

  test(
    "CMD-OPEN-01: Ctrl+K from the composer opens the palette with focus in the search box",
    { tag: ["@interaction:CMD-OPEN-01"] },
    async ({ page }) => {
      await openClient(page);
      await ensureConversation(page);
      await composerInput(page).focus();

      await page.keyboard.press("Control+k");
      const palette = paletteOf(page);
      await expect(palette).toBeVisible();
      await expect(searchOf(page)).toBeFocused();
      const options = palette.getByRole("option");
      expect(await options.count()).toBeGreaterThan(0);
      await expect(options.first()).toBeVisible();

      recordInteraction(
        "CMD-OPEN-01",
        "CMD-OPEN-01: Ctrl+K from the composer opens the palette with focus in the search box",
      );
    },
  );

  test(
    "CMD-NAV-01: arrow keys move aria-activedescendant with exactly one selected option",
    { tag: ["@interaction:CMD-NAV-01"] },
    async ({ page }) => {
      await openClient(page);
      await ensureConversation(page);
      await composerInput(page).focus();
      await page.keyboard.press("Control+k");
      const palette = paletteOf(page);
      const search = searchOf(page);
      await expect(palette).toBeVisible();
      await expect(search).toBeFocused();

      const selected = palette.getByRole("option", { selected: true });
      await expect(search).toHaveAttribute("aria-activedescendant", "zk-command-new-conversation");
      await expect(selected).toHaveCount(1);

      await page.keyboard.press("ArrowDown");
      await expect(search).toHaveAttribute("aria-activedescendant", "zk-command-toggle-sidebar");
      await expect(selected).toHaveCount(1);
      await expect(selected).toHaveAttribute("id", "zk-command-toggle-sidebar");

      await page.keyboard.press("ArrowDown");
      await expect(search).toHaveAttribute("aria-activedescendant", "zk-command-open-settings");
      await page.keyboard.press("ArrowUp");
      await expect(search).toHaveAttribute("aria-activedescendant", "zk-command-toggle-sidebar");
      await expect(selected).toHaveCount(1);

      recordInteraction(
        "CMD-NAV-01",
        "CMD-NAV-01: arrow keys move aria-activedescendant with exactly one selected option",
      );
    },
  );

  test(
    "CMD-ESC-01: Esc closes the palette and focus returns to the composer that opened it",
    { tag: ["@interaction:CMD-ESC-01"] },
    async ({ page }) => {
      await openClient(page);
      await ensureConversation(page);
      const input = composerInput(page);
      await input.focus();
      await page.keyboard.press("Control+k");
      await expect(paletteOf(page)).toBeVisible();

      await page.keyboard.press("Escape");
      await expect(paletteOf(page)).toBeHidden();
      await expect(input).toBeFocused();

      recordInteraction(
        "CMD-ESC-01",
        "CMD-ESC-01: Esc closes the palette and focus returns to the composer that opened it",
      );
    },
  );

  test(
    "CMD-A11Y-01: palette has no serious axe violations and keeps dialog/listbox/option semantics",
    { tag: ["@interaction:CMD-A11Y-01"] },
    async ({ page }) => {
      await openClient(page);
      await ensureConversation(page);
      await composerInput(page).focus();
      await page.keyboard.press("Control+k");
      const palette = paletteOf(page);
      await expect(palette).toBeVisible();
      await expectNoSeriousAxe(page, "commands/palette");

      await expect(palette.getByRole("listbox", { name: "命令列表" })).toBeVisible();
      expect(await palette.getByRole("option").count()).toBeGreaterThan(0);
      await expect(palette).toMatchAriaSnapshot(`
        - textbox "搜索命令"
        - listbox "命令列表":
          - option "工作区操作 新建会话" [selected]
      `);

      recordInteraction(
        "CMD-A11Y-01",
        "CMD-A11Y-01: palette has no serious axe violations and keeps dialog/listbox/option semantics",
      );
    },
  );
});
