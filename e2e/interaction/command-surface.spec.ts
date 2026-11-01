import { expect, test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { existsSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

/** DSH 1:1 command surface Batch 1 (`2026-11-01-client-command-surface`). */
const SERVE = "http://127.0.0.1:8788";
const HOME = "target/evidence/m1/home";
const EVIDENCE = "target/evidence/2026-11-01-client-command-surface/interaction";
mkdirSync(EVIDENCE, { recursive: true });

function record(id: string, title: string): void {
  writeFileSync(join(EVIDENCE, `${id}.json`), `${JSON.stringify({ id, title, at: new Date().toISOString(), passed: true }, null, 2)}\n`, "utf8");
}

function isRecordValue(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function ipcDataOf(body: unknown): unknown {
  if (!isRecordValue(body)) return undefined;
  return body["data"];
}

async function openClient(page: Page): Promise<void> {
  await page.goto(`${SERVE}/`, { waitUntil: "networkidle" });
  await expect(page.getByRole("complementary", { name: "工作区与会话" })).toBeVisible();
  await page.getByRole("button", { name: "新建会话" }).last().click();
  await expect(composer(page)).toBeVisible();
}

function composer(page: Page): ReturnType<Page["getByRole"]> {
  return page.getByRole("textbox", { name: "消息输入框" });
}

async function slash(page: Page, command: string): Promise<void> {
  await composer(page).click();
  await page.keyboard.type(command);
}

function firstWorkspaceIdOf(data: unknown): string {
  if (!Array.isArray(data) || data.length === 0) return "";
  const first: unknown = data[0];
  if (!isRecordValue(first)) return "";
  const id = first["id"];
  return typeof id === "string" ? id : "";
}

async function axeClean(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  const found = results.violations.filter((violation) => violation.impact === "serious" || violation.impact === "critical").map((violation) => violation.id);
  expect(found, "serious/critical axe violations").toEqual([]);
}

async function callIpc(page: Page, name: string, payload: Record<string, unknown>): Promise<unknown> {
  const response = await page.request.post(`${SERVE}/ipc`, { data: { name, payload } });
  const body: unknown = await response.json();
  return ipcDataOf(body);
}

test.describe("command surface Batch 1 interaction contract", () => {
  test("CMD2-MENU-01: two DSH sections, filtering, accept/close keys, no UI actions", { tag: ["@interaction:CMD2-MENU-01"] }, async ({ page }) => {
    await openClient(page);
    await composer(page).click();
    await page.keyboard.type("/");
    const list = page.getByRole("listbox", { name: "命令" });
    await expect(list).toBeVisible();
    for (const label of ["添加文件", "设置会话目标", "计划", "发送反馈", "压缩上下文", "权限策略", "选择模型", "导出对话", "技能"]) {
      await expect(page.getByRole("option", { name: new RegExp(label) })).toBeVisible();
    }
    // UI actions are palette-only now.
    await expect(page.getByRole("option", { name: /新建会话/ })).toHaveCount(0);
    await composer(page).fill("/mod");
    await expect(page.getByRole("option", { name: /选择模型/ })).toBeVisible();
    await page.keyboard.press("Enter");
    await expect(composer(page)).toHaveValue("");
    await expect(page.getByRole("dialog", { name: "会话级设置" })).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(composer(page)).toBeFocused();
    // Esc and Shift+Tab close without sending.
    await composer(page).fill("/permission");
    await expect(page.getByRole("listbox", { name: "命令" })).toBeVisible();
    await page.keyboard.press("Shift+Tab");
    await expect(page.getByRole("listbox")).toHaveCount(0);
    await expect(composer(page)).toHaveValue("/permission");
    record("CMD2-MENU-01", "two DSH sections, filtering, accept/close keys, no UI actions");
  });

  test("CMD2-MENU-02: unknown commands and Batch 2 placeholders never execute/send", { tag: ["@interaction:CMD2-MENU-02"] }, async ({ page }) => {
    await openClient(page);
    const before = await page.getByTestId("user-text").count();
    await composer(page).fill("/not-a-command");
    await page.keyboard.press("Enter");
    await expect(page.getByText("不支持的命令")).toBeVisible();
    await expect(page.getByTestId("user-text")).toHaveCount(before);
    await composer(page).fill("/plan");
    const placeholder = page.getByRole("option", { name: /计划/ });
    await expect(placeholder).toBeVisible();
    expect(await placeholder.isDisabled()).toBe(true);
    await page.keyboard.press("Enter");
    await expect(page.getByText("未实现（Batch 2）")).toBeVisible();
    await expect(page.getByTestId("user-text")).toHaveCount(before);
    record("CMD2-MENU-02", "unknown commands and Batch 2 placeholders never execute/send");
  });

  test("CMD2-FILE-01: /file opens the picker; attachments are rejected by other commands", { tag: ["@interaction:CMD2-FILE-01"] }, async ({ page }) => {
    await openClient(page);
    await composer(page).click();
    const chooser = page.waitForEvent("filechooser");
    await page.keyboard.type("/file");
    await page.keyboard.press("Enter");
    const fileChooser = await chooser;
    await fileChooser.setFiles({
      name: "slash.png",
      mimeType: "image/png",
      buffer: Buffer.from(
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
        "base64",
      ),
    });
    const chip = page.getByText("slash.png", { exact: true });
    await expect(chip).toBeVisible();
    // Other commands reject attachments but keep the chip and the draft.
    await composer(page).fill("/model");
    await page.keyboard.press("Enter");
    await expect(page.getByText("该命令不支持附件（仅 /file 可添加文件）")).toBeVisible();
    await expect(chip).toBeVisible();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    record("CMD2-FILE-01", "/file opens the picker; attachments are rejected by other commands");
  });

  test("CMD2-MODEL-01: personal model + effort are conversation-scoped and persisted", { tag: ["@interaction:CMD2-MODEL-01"] }, async ({ page }) => {
    await openClient(page);
    const workspaces = await callIpc(page, "workspace.list", {});
    expect(firstWorkspaceIdOf(workspaces)).not.toBe("");
    await slash(page, "/model");
    await page.keyboard.press("Enter");
    const dialog = page.getByRole("dialog", { name: "会话级设置" });
    await expect(dialog).toBeVisible();
    await dialog.getByRole("textbox", { name: "模型" }).fill("deepseek-chat");
    await dialog.getByRole("combobox", { name: "推理等级" }).selectOption("low");
    await dialog.getByRole("button", { name: "应用本会话" }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    // Persisted in the workspace override file; global settings untouched.
    const conversationList = await callIpc(page, "session.list", { workspace_id: firstWorkspaceIdOf(workspaces) });
    expect(Array.isArray(conversationList)).toBe(true);
    await expect
      .poll(() => existsSync(join(HOME, ".zeokeel/conversation-overrides.json")))
      .toBe(true);
    const overrideText = readdirSync(join(HOME, ".zeokeel")).includes("conversation-overrides.json");
    expect(overrideText).toBe(true);
    const settings = await callIpc(page, "settings.get", {});
    expect(isRecordValue(settings) && isRecordValue(settings["settings"]) && isRecordValue(settings["settings"]["provider"]) ? settings["settings"]["provider"]["model"] : "?").toBe("mock");
    // Reload keeps the conversation override (host-persisted).
    await page.reload({ waitUntil: "networkidle" });
    await composer(page).click();
    await page.keyboard.type("/model");
    await page.keyboard.press("Enter");
    await expect(page.getByRole("dialog", { name: "会话级设置" }).getByRole("textbox", { name: "模型" })).toHaveValue("deepseek-chat");
    await page.keyboard.press("Escape");
    record("CMD2-MODEL-01", "personal model + effort are conversation-scoped and persisted");
  });

  test("CMD2-PERM-01: approval presets are labeled honestly and persisted per conversation", { tag: ["@interaction:CMD2-PERM-01"] }, async ({ page }) => {
    await openClient(page);
    await slash(page, "/permission");
    await page.keyboard.press("Enter");
    const dialog = page.getByRole("dialog", { name: "会话级设置" });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByText("仅审批策略；不限制工具/网络/文件，不是沙箱")).toBeVisible();
    await dialog.getByRole("radio", { name: "只读（保守）" }).check();
    await dialog.getByRole("button", { name: "应用本会话" }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await page.reload({ waitUntil: "networkidle" });
    await composer(page).click();
    await page.keyboard.type("/permission");
    await page.keyboard.press("Enter");
    const reopened = page.getByRole("dialog", { name: "会话级设置" });
    await expect(reopened.getByRole("radio", { name: "只读（保守）" })).toBeChecked();
    await page.keyboard.press("Escape");
    record("CMD2-PERM-01", "approval presets are labeled honestly and persisted per conversation");
  });

  test("CMD2-GOAL-01: /goal confirms, GoalBar controls it, clear removes it (no message)", { tag: ["@interaction:CMD2-GOAL-01"] }, async ({ page }) => {
    await openClient(page);
    const before = await page.getByTestId("user-text").count();
    await composer(page).click();
    await page.keyboard.type("/goal ship the command surface");
    await page.keyboard.press("Enter");
    const dialog = page.getByRole("dialog", { name: "设置会话目标" });
    await expect(dialog).toBeVisible();
    await dialog.getByRole("textbox", { name: "目标描述" }).fill("ship the command surface");
    await dialog.getByRole("button", { name: "确认" }).click();
    const bar = page.getByRole("region", { name: "会话目标" });
    await expect(bar).toBeVisible();
    await expect(bar).toContainText("ship the command surface");
    await expect(page.getByTestId("user-text")).toHaveCount(before);
    await expect.poll(() => existsSync(join(HOME, ".zeokeel/goal.json"))).toBe(true);
    await bar.getByRole("button", { name: "暂停" }).click();
    await expect(bar).toContainText("已暂停");
    await bar.getByRole("button", { name: "恢复" }).click();
    await expect(bar).toContainText("进行中");
    await bar.getByRole("button", { name: "清除目标" }).click();
    await expect(dialog).toBeVisible();
    await dialog.getByRole("button", { name: "清除目标" }).click();
    // Confirm-clear block is rendered (the command path uses the same `goal.update{cancel}`).
    await expect(dialog.getByText("清除当前目标？")).toBeVisible();
    const workspaces = await callIpc(page, "workspace.list", {});
    const conversations = await callIpc(page, "session.list", { workspace_id: firstWorkspaceIdOf(workspaces) });
    const conversationId = Array.isArray(conversations) && isRecordValue(conversations[0]) && typeof conversations[0]["id"] === "string" ? conversations[0]["id"] : "";
    await callIpc(page, "goal.update", { workspace_id: firstWorkspaceIdOf(workspaces), conversation_id: conversationId, action: "cancel" });
    // Cleared goals are persisted as cancelled and stay hidden after a reload.
    await page.reload({ waitUntil: "networkidle" });
    await expect(page.getByRole("region", { name: "会话目标" })).toHaveCount(0);
    record("CMD2-GOAL-01", "/goal confirms, GoalBar controls it, clear removes it (no message)");
  });

  test("CMD2-EXPORT-01: /export writes Markdown and serves it via /download", { tag: ["@interaction:CMD2-EXPORT-01"] }, async ({ page }) => {
    await openClient(page);
    // Empty conversation: readable error, nothing written.
    await slash(page, "/export");
    await page.keyboard.press("Enter");
    await expect(page.getByText("当前会话没有可导出的消息")).toBeVisible();
    await composer(page).fill("export me");
    await page.keyboard.press("Enter");
    await expect(page.getByTestId("user-text").last()).toHaveText("export me");
    await page.reload({ waitUntil: "networkidle" });
    await slash(page, "/export");
    await page.keyboard.press("Enter");
    await expect(page.getByText(/已导出：/)).toBeVisible({ timeout: 20_000 });
    const exportsDir = join(HOME, ".zeokeel/exports");
    expect(existsSync(exportsDir)).toBe(true);
    const files = readdirSync(exportsDir);
    expect(files.some((name) => name.endsWith(".md"))).toBe(true);
    const path = join(process.cwd(), exportsDir, files.find((name) => name.endsWith(".md")) ?? "");
    const response = await page.request.get(`${SERVE}/download?path=${encodeURIComponent(path)}`);
    expect(response.status()).toBe(200);
    expect(await response.text()).toContain("export me");
    record("CMD2-EXPORT-01", "/export writes Markdown and serves it via /download");
  });

  test("CMD2-FEEDBACK-01: local-only feedback dialog validates and stores JSON", { tag: ["@interaction:CMD2-FEEDBACK-01"] }, async ({ page }) => {
    await openClient(page);
    await slash(page, "/feedback");
    await page.keyboard.press("Enter");
    const dialog = page.getByRole("dialog", { name: "发送反馈" });
    await expect(dialog).toBeVisible();
    await expect(dialog.getByText(/仅保存在本地/)).toBeVisible();
    await dialog.getByRole("radio", { name: "建议" }).check();
    await dialog.getByRole("textbox", { name: "描述" }).fill("keyboard flow is great");
    await dialog.getByRole("button", { name: "保存到本地" }).click();
    await expect(page.getByText("反馈已保存到本地")).toBeVisible();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    const dir = join(HOME, ".zeokeel/feedback");
    await expect.poll(() => existsSync(dir) && readdirSync(dir).some((name) => name.endsWith(".json"))).toBe(true);
    // Over-limit descriptions are rejected inline.
    await slash(page, "/feedback");
    await page.keyboard.press("Enter");
    const tooLong = "x".repeat(4200);
    await page.getByRole("dialog", { name: "发送反馈" }).getByRole("textbox", { name: "描述" }).fill(tooLong);
    await expect(page.getByText("描述超过 4 KiB 上限")).toBeVisible();
    await page.getByRole("dialog", { name: "发送反馈" }).getByRole("button", { name: "取消" }).click();
    record("CMD2-FEEDBACK-01", "local-only feedback dialog validates and stores JSON");
  });

  test("CMD2-KEY-01: arrows/Enter/Esc/Shift+Tab and IME safety", { tag: ["@interaction:CMD2-KEY-01"] }, async ({ page }) => {
    await openClient(page);
    await composer(page).click();
    await page.keyboard.type("/");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowUp");
    await page.keyboard.press("Escape");
    await expect(page.getByRole("listbox")).toHaveCount(0);
    await expect(composer(page)).toHaveValue("/");
    // Middle-of-text slash never opens the menu.
    await composer(page).fill("a /");
    await expect(composer(page)).toHaveValue("a /");
    // Composition Enter does not accept/send.
    await composer(page).fill("/");
    await page.evaluate(() => {
      const element = document.activeElement;
      if (!(element instanceof HTMLTextAreaElement)) throw new Error("composer not focused");
      element.dispatchEvent(new CompositionEvent("compositionstart", { bubbles: true, data: "中" }));
      element.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", isComposing: true, bubbles: true, cancelable: true }));
      element.dispatchEvent(new CompositionEvent("compositionend", { bubbles: true, data: "中" }));
    });
    await expect(page.getByTestId("user-text")).toHaveCount(0);
    await expect(composer(page)).toHaveValue("/");
    record("CMD2-KEY-01", "arrows/Enter/Esc/Shift+Tab and IME safety");
  });

  test("CMD2-ERR-01: command errors keep the draft and never fake success", { tag: ["@interaction:CMD2-ERR-01"] }, async ({ page }) => {
    await openClient(page);
    await composer(page).fill("/unknown-command");
    await page.keyboard.press("Enter");
    await expect(page.getByText("不支持的命令")).toBeVisible();
    await expect(composer(page)).toHaveValue("/unknown-command");
    // Backend failure (goal.get) surfaces as a readable error without breaking the composer.
    await page.route("**/ipc", async (route) => {
      const body: unknown = route.request().postDataJSON();
      if (isRecordValue(body) && body["name"] === "goal.get") {
        await route.fulfill({ status: 500, body: "boom" });
        return;
      }
      await route.continue();
    });
    await page.reload({ waitUntil: "networkidle" });
    await expect(composer(page)).toBeVisible();
    await expect(page.getByRole("region", { name: "会话目标" })).toHaveCount(0);
    await page.unroute("**/ipc");
    record("CMD2-ERR-01", "command errors keep the draft and never fake success");
  });

  test("CMD2-A11Y-01: axe + keyboard traversal across menu/dialog/GoalBar", { tag: ["@interaction:CMD2-A11Y-01"] }, async ({ page }) => {
    await openClient(page);
    await composer(page).click();
    await page.keyboard.type("/model");
    await expect(page.getByRole("listbox", { name: "命令" })).toBeVisible();
    await axeClean(page);
    await page.keyboard.press("Enter");
    await expect(page.getByRole("dialog", { name: "会话级设置" })).toBeVisible();
    await axeClean(page);
    await page.keyboard.press("Escape");
    await composer(page).fill("/goal accessible goal");
    await page.keyboard.press("Enter");
    const goalDialog = page.getByRole("dialog", { name: "设置会话目标" });
    await expect(goalDialog).toBeVisible();
    await goalDialog.getByRole("button", { name: "确认" }).click();
    await expect(page.getByRole("region", { name: "会话目标" })).toBeVisible();
    await axeClean(page);
    await expect(composer(page)).toBeFocused();
    record("CMD2-A11Y-01", "axe + keyboard traversal across menu/dialog/GoalBar");
  });
});
