import { expect, test } from "@playwright/test";
import {
  composerInput,
  ensureConversation,
  expectNoSeriousAxe,
  expectReply,
  expectUsage,
  openClient,
  recordInteraction,
  sendButton,
  sendMessage,
  startNewConversation,
  userTexts,
} from "./helpers";

/**
 * Composer baseline contract (interaction.md §1, `@zeokeel/ui-chat`).
 * Real serve carrier + fake upstream; each test owns its conversation.
 */
test.describe("composer interaction baseline", () => {
  test(
    "CMP-FOCUS-01: Enter sends, input clears, focus and send stay available",
    { tag: ["@interaction:CMP-FOCUS-01"] },
    async ({ page }) => {
      await openClient(page);
      await startNewConversation(page);
      const input = composerInput(page);

      await input.fill("focus-01 请回复");
      await expect(sendButton(page)).toBeEnabled();
      await expect(input).toBeFocused();
      await page.keyboard.press("Enter");

      await expect(userTexts(page)).toHaveCount(1);
      await expectReply(page);
      await expect(input).toHaveValue("");
      await expect(input).toBeFocused();
      // A new draft re-enables the send action (empty draft = disabled by design).
      await input.fill("focus-恢复");
      await expect(sendButton(page)).toBeEnabled();
      await input.fill("");

      recordInteraction("CMP-FOCUS-01", "CMP-FOCUS-01: Enter sends, input clears, focus and send stay available");
    },
  );

  test(
    "CMP-KEY-01: Shift+Enter inserts a newline, Enter sends without trailing blank line",
    { tag: ["@interaction:CMP-KEY-01"] },
    async ({ page }) => {
      await openClient(page);
      await startNewConversation(page);
      const input = composerInput(page);

      await input.focus();
      await input.pressSequentially("key-line-a");
      await page.keyboard.press("Shift+Enter");
      await input.pressSequentially("key-line-b");

      await expect(input).toHaveValue("key-line-a\nkey-line-b");
      await expect(userTexts(page)).toHaveCount(0);

      await page.keyboard.press("Enter");
      await expect(userTexts(page)).toHaveCount(1);
      const sent: string | null = await userTexts(page).first().evaluate((element) => element.textContent);
      expect(sent).toBe("key-line-a\nkey-line-b");
      await expect(input).toHaveValue("");
      await expectReply(page);

      recordInteraction(
        "CMP-KEY-01",
        "CMP-KEY-01: Shift+Enter inserts a newline, Enter sends without trailing blank line",
      );
    },
  );

  test(
    "CMP-IME-01: Enter while composing does not send and keeps the draft and focus",
    { tag: ["@interaction:CMP-IME-01"] },
    async ({ page }) => {
      await openClient(page);
      await startNewConversation(page);
      const input = composerInput(page);

      await input.fill("中文草稿");
      await input.focus();
      await expect(input).toBeFocused();

      // Synthetic IME composition: compositionstart, then Enter with isComposing=true.
      await page.evaluate(() => {
        const element = document.activeElement;
        if (!(element instanceof HTMLTextAreaElement)) {
          throw new Error("composer textarea is not focused");
        }
        element.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "中文" }));
        element.dispatchEvent(
          new KeyboardEvent("keydown", {
            key: "Enter",
            code: "Enter",
            isComposing: true,
            bubbles: true,
            cancelable: true,
          }),
        );
      });

      // No message was sent; the draft and focus survive the composing Enter.
      await expect(userTexts(page)).toHaveCount(0);
      await expect(input).toHaveValue("中文草稿");
      await expect(input).toBeFocused();

      // Strongest observable guard: the very same Enter sends once composition ends.
      await page.evaluate(() => {
        document.activeElement?.dispatchEvent(
          new CompositionEvent("compositionend", { bubbles: true, data: "中文草稿" }),
        );
      });
      await page.keyboard.press("Enter");
      await expect(userTexts(page)).toHaveCount(1);
      await expect(userTexts(page).first()).toHaveText("中文草稿");
      await expectReply(page);

      recordInteraction(
        "CMP-IME-01",
        "CMP-IME-01: Enter while composing does not send and keeps the draft and focus",
      );
    },
  );

  test(
    "CMP-PERSIST-01: user message, reply and usage survive reload without duplicates",
    { tag: ["@interaction:CMP-PERSIST-01"] },
    async ({ page }) => {
      await openClient(page);
      await startNewConversation(page);
      const marker = "persist-01 请回复";
      await sendMessage(page, marker);
      await expectReply(page);
      await expectUsage(page);

      await page.reload({ waitUntil: "networkidle" });
      await expect(composerInput(page)).toBeVisible();

      await expect(page.getByText(marker, { exact: true })).toBeVisible();
      await expect(userTexts(page)).toHaveCount(1);
      await expect(page.getByText("Hello from ZeoKeel", { exact: true })).toHaveCount(1);
      await expectUsage(page);

      recordInteraction(
        "CMP-PERSIST-01",
        "CMP-PERSIST-01: user message, reply and usage survive reload without duplicates",
      );
    },
  );

  test(
    "CMP-A11Y-01: chat page has no serious axe violations and the composer is named",
    { tag: ["@interaction:CMP-A11Y-01"] },
    async ({ page }) => {
      await openClient(page);
      await ensureConversation(page);
      await expectNoSeriousAxe(page, "chat/composer");

      const input = composerInput(page);
      await expect(input).toHaveAccessibleName("消息输入框");
      await expect(sendButton(page)).toHaveAccessibleName("发送");
      // R4: composer structure baseline (role/name, not visual).
      await expect(input.locator("..")).toMatchAriaSnapshot(`
        - text: 消息输入框
        - textbox "消息输入框":
          - /placeholder: 输入消息，Enter 发送，Shift+Enter 换行
        - text: Enter 发送 · Shift+Enter 换行
        - button "发送" [disabled]
      `);

      recordInteraction(
        "CMP-A11Y-01",
        "CMP-A11Y-01: chat page has no serious axe violations and the composer is named",
      );
    },
  );

  test(
    "CMP-VIS-01: empty composer matches the committed light/1280 visual baseline",
    { tag: ["@interaction:CMP-VIS-01"] },
    async ({ page }) => {
      await openClient(page);
      await startNewConversation(page);
      const input = composerInput(page);
      await expect(input).toBeVisible();
      await expect(input).toHaveValue("");

      // Baseline must not depend on auto-focus (focus-within changes the border).
      await page.evaluate(() => {
        const element = document.activeElement;
        if (element instanceof HTMLElement) {
          element.blur();
        }
      });
      await expect(input.locator("..")).toHaveScreenshot("composer-empty.png");

      recordInteraction(
        "CMP-VIS-01",
        "CMP-VIS-01: empty composer matches the committed light/1280 visual baseline",
      );
    },
  );
});
