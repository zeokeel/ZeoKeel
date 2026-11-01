import { expect, test } from "@playwright/test";
import { composerInput, expectNoSeriousAxe, openClient, recordInteraction } from "./helpers";

/** Sessions sidebar baseline contract (interaction.md §5, `@zeokeel/ui-sessions`). */
test.describe("sessions sidebar interaction baseline", () => {
  test(
    "SES-NEW-01: new conversation is selected, the composer is focusable and the row appears",
    { tag: ["@interaction:SES-NEW-01"] },
    async ({ page }) => {
      await openClient(page);
      const navigation = page.getByRole("navigation", { name: "工作区" });

      await page.getByRole("button", { name: "新建会话", exact: true }).last().click();

      const input = composerInput(page);
      await expect(input).toBeVisible();
      await input.focus();
      await expect(input).toBeFocused();

      const sessionRow = navigation.getByRole("button", { name: /^新会话/ }).first();
      await expect(sessionRow).toBeVisible();
      await expect(sessionRow).toHaveAttribute("aria-current", "true");
      await expect(page.getByRole("heading", { level: 1, name: "新会话" })).toBeVisible();

      recordInteraction(
        "SES-NEW-01",
        "SES-NEW-01: new conversation is selected, the composer is focusable and the row appears",
      );
    },
  );

  test(
    "SES-A11Y-01: sidebar has no serious axe violations and named navigation/actions",
    { tag: ["@interaction:SES-A11Y-01"] },
    async ({ page }) => {
      await openClient(page);
      const sidebar = page.getByRole("complementary", { name: "工作区与会话" });
      await expect(sidebar).toBeVisible();
      await expectNoSeriousAxe(page, "sessions/sidebar");

      await expect(sidebar.getByRole("navigation", { name: "工作区" })).toBeVisible();
      await expect(sidebar.getByRole("button", { name: "新建会话" }).first()).toHaveAccessibleName("新建会话");
      await expect(sidebar.getByRole("button", { name: "打开设置" })).toHaveAccessibleName("打开设置");

      recordInteraction(
        "SES-A11Y-01",
        "SES-A11Y-01: sidebar has no serious axe violations and named navigation/actions",
      );
    },
  );
});
